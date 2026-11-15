//! Grid samples read chunk by chunk (tile by tile, or strip by strip) from a remote GeoTIFF, as PROJ
//! reads them (`GTiffGrid`): a chunk is read when one of its samples is needed, not when the grid is
//! opened.
//!
//! An uncompressed chunk without predictor is read as its bytes: its `TileByteCounts` (or
//! `StripByteCounts`) size, from its offset, so the ranges asked of the callbacks are PROJ's. A
//! compressed chunk, or one with a predictor, is read the same way, and then decoded by the TIFF
//! decoder, which undoes the compression and the predictor. Its bytes are read once: the decoder
//! takes them from a window, and reads nothing else.
//!
//! A sample of a chunk that is not read yet gives `None`, and the chunk is noted. [`with_tiles`]
//! then reads the noted chunks and runs the point again, so that the answer is the one a grid read
//! whole would give: an interpolation never sees a sample that is not there.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use tiff::decoder::Decoder;

use super::geotiff::{self, ByteOrder};
use super::lzw::{self, Predictor};
use crate::engine::TransformError;
use crate::network_chunks::{RemoteFile, RemoteReader};

/// The samples of one image: in memory, or in the remote file, by chunk.
pub(super) enum Samples {
    Flat(Vec<f32>),
    Lazy(Arc<LazyIfd>),
}

impl Samples {
    /// The sample `sample` of pixel `(x, y)`, in a raster `width` pixels wide with `spp` samples
    /// per pixel. `None` outside the raster, or in a chunk that is not read yet: the chunk is then
    /// noted for [`with_tiles`].
    pub(super) fn get(
        &self,
        x: usize,
        y: usize,
        sample: usize,
        width: usize,
        spp: usize,
    ) -> Option<f32> {
        match self {
            Self::Flat(values) => {
                let index = y
                    .checked_mul(width)?
                    .checked_add(x)?
                    .checked_mul(spp)?
                    .checked_add(sample)?;
                values.get(index).copied()
            }
            Self::Lazy(lazy) => {
                let (chunk, inside) = lazy.locate(x, y, sample)?;
                match lazy.tiles.get(chunk)?.get() {
                    Some(values) => values.get(inside).copied(),
                    None => {
                        note_missing(lazy, chunk);
                        None
                    }
                }
            }
        }
    }
}

/// How the samples of an image are cut in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layout {
    /// Tiles, all of the tile size, with padding at the right and bottom edges.
    Tiles,
    /// Strips of the full image width. The last one holds only the rows left.
    Strips,
}

/// How the samples of a pixel are stored: together, pixel after pixel (`PlanarConfiguration` 1), or
/// one plane per sample, each with its own chunks (`PlanarConfiguration` 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Storage {
    Chunky,
    Separate,
}

/// How a chunk is read: its bytes as they are, which is an uncompressed chunk without predictor,
/// or its bytes decoded by the TIFF decoder, which undoes the compression and the predictor. LZW
/// chunks are decoded by [`super::lzw`], with their predictor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    Raw,
    Decoded,
    Lzw(Predictor),
}

/// The layout of one image of a TIFF, as its tags give it.
pub(super) struct ImageTiles {
    /// The image size, in pixels.
    pub(super) size: (u32, u32),
    /// The tile size (`TileWidth`, `TileLength`), or the strip size (the width, `RowsPerStrip`).
    pub(super) tile: (u32, u32),
    pub(super) spp: usize,
    /// `BitsPerSample` and `SampleFormat` of the samples.
    pub(super) bits: u32,
    pub(super) format: u32,
    pub(super) order: ByteOrder,
    pub(super) layout: Layout,
    pub(super) storage: Storage,
    pub(super) reading: Reading,
    /// The index of the image in the file, the one [`Decoder::seek_to_image`] takes.
    pub(super) ifd: usize,
    /// The file offset and the byte count of each chunk, as `TileOffsets` and `TileByteCounts`,
    /// or `StripOffsets` and `StripByteCounts`. With planar samples, the chunks of each plane follow
    /// one another.
    pub(super) places: Vec<(u64, u64)>,
}

/// One image of a remote TIFF, whose chunks are read on first need.
pub(super) struct LazyIfd {
    file: Arc<Mutex<RemoteFile>>,
    /// The image width and height, which bound the chunks at the right and bottom edges.
    width: usize,
    height: usize,
    tile_w: usize,
    tile_h: usize,
    /// Chunks per row of a plane.
    across: usize,
    /// Chunks per plane: `across` times the chunk rows.
    per_plane: usize,
    layout: Layout,
    spp: usize,
    /// The number of samples of one chunk: `spp` when chunky, one when planar.
    chunk_spp: usize,
    storage: Storage,
    reading: Reading,
    ifd: usize,
    bits: u32,
    format: u32,
    order: ByteOrder,
    /// The number of samples of one whole tile: `tile_w * tile_h * chunk_spp`.
    tile_len: usize,
    places: Vec<(u64, u64)>,
    /// The decoded samples of each chunk, once read. A chunk holds its rows `tile_w` pixels
    /// apart, `chunk_spp` samples per pixel.
    tiles: Vec<OnceLock<Box<[f32]>>>,
}

impl LazyIfd {
    pub(super) fn new(file: Arc<Mutex<RemoteFile>>, image: ImageTiles) -> Result<Self, ()> {
        let width = usize::try_from(image.size.0).map_err(|_| ())?;
        let height = usize::try_from(image.size.1).map_err(|_| ())?;
        let tile_w = usize::try_from(image.tile.0).map_err(|_| ())?;
        // A single strip may be `RowsPerStrip` = 2^32 - 1: the rows of the image are the strip.
        let tile_h = usize::try_from(image.tile.1).map_err(|_| ())?;
        let tile_h = match image.layout {
            Layout::Tiles => tile_h,
            Layout::Strips => tile_h.min(height),
        };
        if width == 0 || height == 0 || tile_w == 0 || tile_h == 0 || image.spp == 0 {
            return Err(());
        }
        if image.layout == Layout::Strips && tile_w != width {
            return Err(());
        }
        let across = width.div_ceil(tile_w);
        let per_plane = across.checked_mul(height.div_ceil(tile_h)).ok_or(())?;
        let (planes, chunk_spp) = match image.storage {
            Storage::Chunky => (1, image.spp),
            Storage::Separate => (image.spp, 1),
        };
        let count = per_plane.checked_mul(planes).ok_or(())?;
        if image.places.len() != count {
            return Err(());
        }
        let tile_len = tile_w
            .checked_mul(tile_h)
            .and_then(|pixels| pixels.checked_mul(chunk_spp))
            .ok_or(())?;
        Ok(Self {
            file,
            width,
            height,
            tile_w,
            tile_h,
            across,
            per_plane,
            layout: image.layout,
            spp: image.spp,
            chunk_spp,
            storage: image.storage,
            reading: image.reading,
            ifd: image.ifd,
            bits: image.bits,
            format: image.format,
            order: image.order,
            tile_len,
            places: image.places,
            tiles: (0..count).map(|_| OnceLock::new()).collect(),
        })
    }

    /// Whether the file of this image changed while it was open (see `RemoteFile::has_changed`).
    fn has_changed(&self) -> bool {
        self.file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .has_changed()
    }

    /// Closes the file of this image, if it is open. A chunk read after this opens it again.
    pub(super) fn release(&self) {
        self.file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .release();
    }

    /// The chunk that holds sample `sample` of pixel `(x, y)`, and the position of the sample in
    /// that chunk.
    fn locate(&self, x: usize, y: usize, sample: usize) -> Option<(usize, usize)> {
        if sample >= self.spp {
            return None;
        }
        let col = x / self.tile_w;
        let row = y / self.tile_h;
        let tile = row.checked_mul(self.across)?.checked_add(col)?;
        let within = (y - row * self.tile_h)
            .checked_mul(self.tile_w)?
            .checked_add(x - col * self.tile_w)?;
        match self.storage {
            Storage::Chunky => Some((tile, within.checked_mul(self.spp)?.checked_add(sample)?)),
            Storage::Separate => Some((
                sample.checked_mul(self.per_plane)?.checked_add(tile)?,
                within,
            )),
        }
    }

    /// The chunk of `chunk` within its plane.
    fn plane_tile(&self, chunk: usize) -> usize {
        chunk % self.per_plane
    }

    /// The width and the height of the pixels of the chunk `tile` of a plane that are in the image:
    /// a whole tile, or less at the right and bottom edges. A strip is the image width.
    fn data_dims(&self, tile: usize) -> (usize, usize) {
        let col = tile % self.across;
        let row = tile / self.across;
        let width = self
            .width
            .saturating_sub(col * self.tile_w)
            .min(self.tile_w);
        let height = self
            .height
            .saturating_sub(row * self.tile_h)
            .min(self.tile_h);
        (width, height)
    }

    /// The number of samples a raw read of the chunk holds: a whole tile, or for a strip the rows
    /// that the image has in it.
    fn expected_len(&self, tile: usize) -> usize {
        match self.layout {
            Layout::Tiles => self.tile_len,
            Layout::Strips => {
                let (width, rows) = self.data_dims(tile);
                width * rows * self.chunk_spp
            }
        }
    }

    /// Reads `size` bytes at `offset` of the file: a chunk's bytes, one read of the file for the
    /// chunk, as PROJ's measured reads are.
    fn read_file(&self, offset: u64, size: usize) -> Result<Vec<u8>, ()> {
        let bytes = self
            .file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read_at(offset, size)
            .map_err(|_| ())?;
        if bytes.len() != size {
            return Err(());
        }
        Ok(bytes)
    }

    /// Reads chunk `chunk` from the file, unless it is read already. The read is the chunk's whole
    /// byte count, from its offset.
    pub(super) fn load(&self, chunk: usize) -> Result<(), ()> {
        let cell = self.tiles.get(chunk).ok_or(())?;
        if cell.get().is_some() {
            return Ok(());
        }
        let values = match self.reading {
            Reading::Raw => self.read_raw(chunk)?,
            Reading::Decoded => self.read_decoded(chunk)?,
            Reading::Lzw(predictor) => self.read_lzw(chunk, predictor)?,
        };
        // A concurrent load of the same chunk gives the same samples: either set is kept.
        let _ = cell.set(values.into_boxed_slice());
        Ok(())
    }

    /// The samples of an uncompressed chunk, as they are in the file.
    fn read_raw(&self, chunk: usize) -> Result<Vec<f32>, ()> {
        let (offset, count) = *self.places.get(chunk).ok_or(())?;
        let size = usize::try_from(count).map_err(|_| ())?;
        let bytes = self.read_file(offset, size)?;
        let values = geotiff::raw_samples(&bytes, self.format, self.bits, self.order)?;
        if values.len() != self.expected_len(self.plane_tile(chunk)) {
            return Err(());
        }
        Ok(values)
    }

    /// The samples of an LZW chunk: its codes decoded, then its predictor undone row by row. Its
    /// rows are `tile_w` pixels wide, as the file stores them, and the chunk is as long as
    /// [`Self::expected_len`] says, as [`Self::read_raw`] checks.
    fn read_lzw(&self, chunk: usize, predictor: Predictor) -> Result<Vec<f32>, ()> {
        let (offset, count) = *self.places.get(chunk).ok_or(())?;
        let size = usize::try_from(count).map_err(|_| ())?;
        let bytes = self.read_file(offset, size)?;
        let len = self.expected_len(self.plane_tile(chunk));
        let sample_size = usize::try_from(self.bits / 8).map_err(|_| ())?;
        let mut raw = lzw::decode(&bytes, len.checked_mul(sample_size).ok_or(())?)?;
        let row_samples = self.tile_w.checked_mul(self.chunk_spp).ok_or(())?;
        let row_bytes = row_samples.checked_mul(sample_size).ok_or(())?;
        lzw::undo_predictor(
            &mut raw,
            row_bytes,
            self.chunk_spp,
            self.bits,
            predictor,
            self.order,
        )?;
        let values = geotiff::raw_samples(&raw, self.format, self.bits, self.order)?;
        if values.len() != len {
            return Err(());
        }
        Ok(values)
    }

    /// The samples of a compressed chunk, or one with a predictor, decoded by the TIFF decoder. The
    /// decoder reads the chunk's bytes from a window read once here, and nothing else. The rows of
    /// the chunk are stored `tile_w` pixels apart, as the raw samples are.
    fn read_decoded(&self, chunk: usize) -> Result<Vec<f32>, ()> {
        let (offset, count) = *self.places.get(chunk).ok_or(())?;
        let size = usize::try_from(count).map_err(|_| ())?;
        let mut decoder =
            Decoder::new(RemoteReader::new(Arc::clone(&self.file))).map_err(|_| ())?;
        decoder.seek_to_image(self.ifd).map_err(|_| ())?;
        let bytes = self.read_file(offset, size)?;
        decoder.inner().set_window(offset, bytes);
        let index = u32::try_from(chunk).map_err(|_| ())?;
        let decoded = decoder.read_chunk(index).map_err(|_| ())?;
        let values = geotiff::to_f32(decoded)?;
        let (data_w, data_h) = self.data_dims(self.plane_tile(chunk));
        let row_len = data_w.checked_mul(self.chunk_spp).ok_or(())?;
        let rows_len = row_len.checked_mul(data_h).ok_or(())?;
        // The decoder's width is the image's: its padding is by column. Its height is not always
        // the image's: `tiff` 0.10.3 takes the bottom padding of a tile of a planar image from the
        // chunk index, not from its row within the plane, so a partial tile of a plane after the
        // first comes back with its padding rows. The rows of the image come first.
        if row_len == 0 || values.len() < rows_len || values.len() % row_len != 0 {
            return Err(());
        }
        let stride = self.tile_w.checked_mul(self.chunk_spp).ok_or(())?;
        let mut out = vec![0.0_f32; stride.checked_mul(data_h).ok_or(())?];
        for (row, source) in values.chunks_exact(row_len).take(data_h).enumerate() {
            let start = row.checked_mul(stride).ok_or(())?;
            let end = start.checked_add(row_len).ok_or(())?;
            out.get_mut(start..end).ok_or(())?.copy_from_slice(source);
        }
        Ok(out)
    }
}

thread_local! {
    /// The chunks that the current pass of a point found missing.
    static MISSING: RefCell<Vec<(Arc<LazyIfd>, usize)>> = const { RefCell::new(Vec::new()) };
}

fn note_missing(lazy: &Arc<LazyIfd>, chunk: usize) {
    MISSING.with(|missing| missing.borrow_mut().push((Arc::clone(lazy), chunk)));
}

fn take_missing() -> Vec<(Arc<LazyIfd>, usize)> {
    MISSING.with(|missing| std::mem::take(&mut *missing.borrow_mut()))
}

/// The most passes one point takes. A pass reads every chunk it found missing, so the passes
/// needed are the chunks that a point's interpolation reaches, one set per nested grid.
const MAX_PASSES: usize = 16;

/// Runs `body`, the transform of one point. When a pass finds a chunk missing, the chunk is read
/// and the pass is run again, until a pass finds none missing. Nested calls keep the chunks that
/// the outer call has noted.
pub(super) fn with_tiles<T>(
    body: impl Fn() -> Result<T, TransformError>,
) -> Result<T, TransformError> {
    let outer = take_missing();
    let answer = passes(&body);
    MISSING.with(|missing| *missing.borrow_mut() = outer);
    answer
}

/// [`with_tiles`] for a read of samples that is not a transform, such as a grid's `value_at`.
/// `None` when `body` gives none, and also when a chunk it needs cannot be read.
pub(super) fn read_samples<T>(body: impl Fn() -> Option<T>) -> Option<T> {
    with_tiles(|| Ok::<_, TransformError>(body()))
        .ok()
        .flatten()
}

fn passes<T>(body: &impl Fn() -> Result<T, TransformError>) -> Result<T, TransformError> {
    for _ in 0..MAX_PASSES {
        let answer = body();
        let missing = take_missing();
        if missing.is_empty() {
            return answer;
        }
        for (lazy, chunk) in missing {
            lazy.load(chunk)
                .map_err(|()| TransformError::NetworkUnavailable)?;
        }
    }
    Err(TransformError::NetworkUnavailable)
}

/// [`with_tiles`] for a grid whose file may change while it is open. The second value is `true`
/// when a chunk read found its file changed: the first value is then not an answer, and the grid
/// must be loaded again before the point is run again (PROJ's `shouldRetry`).
pub(super) fn with_tiles_watch<T>(
    body: impl Fn() -> Result<T, TransformError>,
) -> (Result<T, TransformError>, bool) {
    let outer = take_missing();
    let answer = passes_watch(&body);
    MISSING.with(|missing| *missing.borrow_mut() = outer);
    answer
}

/// [`passes`], which stops at the first chunk read that finds its file changed.
fn passes_watch<T>(
    body: &impl Fn() -> Result<T, TransformError>,
) -> (Result<T, TransformError>, bool) {
    for _ in 0..MAX_PASSES {
        let answer = body();
        let missing = take_missing();
        if missing.is_empty() {
            return (answer, false);
        }
        for (lazy, chunk) in missing {
            if lazy.load(chunk).is_err() {
                return (Err(TransformError::NetworkUnavailable), false);
            }
            if lazy.has_changed() {
                return (Err(TransformError::NetworkUnavailable), true);
            }
        }
    }
    (Err(TransformError::NetworkUnavailable), false)
}
