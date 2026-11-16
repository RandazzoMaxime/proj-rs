//! Grid samples read chunk by chunk (tile by tile, or strip by strip) from a remote GeoTIFF, as PROJ
//! reads them (`GTiffGrid`): a chunk is read when one of its samples is needed, not when the grid is
//! opened.
//!
//! An uncompressed chunk without predictor is read as its bytes: its `TileByteCounts` (or
//! `StripByteCounts`) size, from its offset, so the ranges asked of the callbacks are PROJ's. A
//! compressed chunk, or one with a predictor, is read the same way, and then decoded: by the TIFF
//! decoder for Deflate, by [`crate::projections::tiff_codec`] for LZW and PackBits. Its bytes are
//! read once.
//!
//! A sample of a chunk that is not read yet gives `None`, and the chunk is noted. [`with_tiles`]
//! then reads the noted chunks and runs the point again, so that the answer is the one a grid read
//! whole would give: an interpolation never sees a sample that is not there.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use tiff::decoder::Decoder;

use super::geotiff;
use crate::engine::TransformError;
use crate::network_chunks::{RemoteFile, RemoteReader};
use crate::projections::tiff_codec::predictor::{self, Predictor};
use crate::projections::tiff_codec::{lzw, packbits, ByteOrder};

/// The samples of one image: in memory, in the remote file by chunk, or not read at all (JPEG and
/// CCITT, see `is_undecoded`), where every value is `inf` as libproj 9.9.0 gives it.
pub(super) enum Samples {
    Flat(Vec<f32>),
    Lazy(Arc<LazyIfd>),
    Undecoded,
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
                let (chunk, inside) = lazy.geo.locate(x, y, sample)?;
                match lazy.tiles.get(chunk)?.get() {
                    Some(values) => values.get(inside).copied(),
                    None => {
                        note_missing(lazy, chunk);
                        None
                    }
                }
            }
            Self::Undecoded => Some(f32::INFINITY),
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
/// its bytes decoded by the TIFF decoder, or its bytes decoded by [`decode_coded`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    Raw,
    Decoded,
    Coded(Coded),
}

/// A compression that the TIFF decoder of this build does not read: LZW, with its predictor, or
/// PackBits. libtiff applies the predictor to LZW only, so PackBits chunks are read without one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Coded {
    Lzw(Predictor),
    PackBits,
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

/// The cut of an image into chunks: how many chunks across and down, and how many samples each
/// one holds.
#[derive(Clone, Copy, Debug)]
pub(super) struct Geometry {
    pub(super) width: usize,
    pub(super) height: usize,
    /// The chunk size: a tile, or a strip of the image width.
    pub(super) tile_w: usize,
    pub(super) tile_h: usize,
    /// Chunks per row of a plane.
    pub(super) across: usize,
    /// Chunks per plane: `across` times the chunk rows.
    pub(super) per_plane: usize,
    pub(super) layout: Layout,
    pub(super) spp: usize,
    /// The number of samples of one chunk pixel: `spp` when chunky, one when planar.
    pub(super) chunk_spp: usize,
    pub(super) storage: Storage,
}

impl Geometry {
    /// The cut of an image of `size` pixels into chunks of `tile` pixels. A strip's height is
    /// clamped to the image: a single strip may be `RowsPerStrip` = 2^32 - 1.
    pub(super) fn new(
        size: (u32, u32),
        tile: (u32, u32),
        layout: Layout,
        spp: usize,
        storage: Storage,
    ) -> Result<Self, ()> {
        let width = usize::try_from(size.0).map_err(|_| ())?;
        let height = usize::try_from(size.1).map_err(|_| ())?;
        let tile_w = usize::try_from(tile.0).map_err(|_| ())?;
        let tile_h = usize::try_from(tile.1).map_err(|_| ())?;
        let tile_h = match layout {
            Layout::Tiles => tile_h,
            Layout::Strips => tile_h.min(height),
        };
        if width == 0 || height == 0 || tile_w == 0 || tile_h == 0 || spp == 0 {
            return Err(());
        }
        if layout == Layout::Strips && tile_w != width {
            return Err(());
        }
        let across = width.div_ceil(tile_w);
        let per_plane = across.checked_mul(height.div_ceil(tile_h)).ok_or(())?;
        let chunk_spp = match storage {
            Storage::Chunky => spp,
            Storage::Separate => 1,
        };
        tile_w
            .checked_mul(tile_h)
            .and_then(|pixels| pixels.checked_mul(chunk_spp))
            .ok_or(())?;
        Ok(Self {
            width,
            height,
            tile_w,
            tile_h,
            across,
            per_plane,
            layout,
            spp,
            chunk_spp,
            storage,
        })
    }

    /// The number of chunks of the image, all planes together.
    pub(super) fn chunk_count(&self) -> Result<usize, ()> {
        let planes = match self.storage {
            Storage::Chunky => 1,
            Storage::Separate => self.spp,
        };
        self.per_plane.checked_mul(planes).ok_or(())
    }

    /// The chunk that holds sample `sample` of pixel `(x, y)`, and the position of the sample in
    /// that chunk.
    pub(super) fn locate(&self, x: usize, y: usize, sample: usize) -> Option<(usize, usize)> {
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
    pub(super) fn plane_tile(&self, chunk: usize) -> usize {
        chunk % self.per_plane
    }

    /// The width and the height of the pixels of the chunk `tile` of a plane that are in the image:
    /// a whole tile, or less at the right and bottom edges. A strip is the image width.
    pub(super) fn data_dims(&self, tile: usize) -> (usize, usize) {
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
    pub(super) fn expected_len(&self, tile: usize) -> usize {
        match self.layout {
            Layout::Tiles => self.tile_w * self.tile_h * self.chunk_spp,
            Layout::Strips => {
                let (width, rows) = self.data_dims(tile);
                width * rows * self.chunk_spp
            }
        }
    }
}

/// What [`decode_coded`] needs of a chunk: its length in samples, and how its samples are laid out.
#[derive(Clone, Copy, Debug)]
pub(super) struct Chunk {
    pub(super) len: usize,
    /// The samples of one row of the chunk, as the file stores them: `tile_w * chunk_spp`.
    pub(super) row_samples: usize,
    pub(super) chunk_spp: usize,
    pub(super) bits: u32,
    pub(super) format: u32,
    pub(super) order: ByteOrder,
}

/// The samples of a chunk stored with `coded`, as `f32`: its codes decoded, its predictor undone
/// for LZW, and its samples read in the file's byte order.
pub(super) fn decode_coded(bytes: &[u8], coded: Coded, chunk: Chunk) -> Result<Vec<f32>, ()> {
    let sample_size = usize::try_from(chunk.bits / 8).map_err(|_| ())?;
    let expected = chunk.len.checked_mul(sample_size).ok_or(())?;
    let mut raw = match coded {
        Coded::Lzw(_) => lzw::decode(bytes, expected)?,
        Coded::PackBits => packbits::decode(bytes, expected)?,
    };
    if let Coded::Lzw(predictor) = coded {
        let row_bytes = chunk.row_samples.checked_mul(sample_size).ok_or(())?;
        predictor::undo_predictor(
            &mut raw,
            row_bytes,
            chunk.chunk_spp,
            chunk.bits,
            predictor,
            chunk.order,
        )?;
    }
    let values = geotiff::raw_samples(&raw, chunk.format, chunk.bits, chunk.order)?;
    if values.len() != chunk.len {
        return Err(());
    }
    Ok(values)
}

/// One image of a remote TIFF, whose chunks are read on first need.
pub(super) struct LazyIfd {
    file: Arc<Mutex<RemoteFile>>,
    geo: Geometry,
    reading: Reading,
    ifd: usize,
    bits: u32,
    format: u32,
    order: ByteOrder,
    places: Vec<(u64, u64)>,
    /// The decoded samples of each chunk, once read. A chunk holds its rows `tile_w` pixels
    /// apart, `chunk_spp` samples per pixel.
    tiles: Vec<OnceLock<Box<[f32]>>>,
}

impl LazyIfd {
    pub(super) fn new(file: Arc<Mutex<RemoteFile>>, image: ImageTiles) -> Result<Self, ()> {
        let image = chop_single_strip(image)?;
        let geo = Geometry::new(
            image.size,
            image.tile,
            image.layout,
            image.spp,
            image.storage,
        )?;
        let count = geo.chunk_count()?;
        if image.places.len() != count {
            return Err(());
        }
        Ok(Self {
            file,
            geo,
            reading: image.reading,
            ifd: image.ifd,
            bits: image.bits,
            format: image.format,
            order: image.order,
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
            Reading::Coded(coded) => self.read_coded(chunk, coded)?,
        };
        // A concurrent load of the same chunk gives the same samples: either set is kept.
        let _ = cell.set(values.into_boxed_slice());
        Ok(())
    }

    /// The bytes of chunk `chunk` in the file, as one read.
    fn chunk_bytes(&self, chunk: usize) -> Result<Vec<u8>, ()> {
        let (offset, count) = *self.places.get(chunk).ok_or(())?;
        let size = usize::try_from(count).map_err(|_| ())?;
        self.read_file(offset, size)
    }

    /// The samples of an uncompressed chunk, as they are in the file.
    fn read_raw(&self, chunk: usize) -> Result<Vec<f32>, ()> {
        let bytes = self.chunk_bytes(chunk)?;
        let values = geotiff::raw_samples(&bytes, self.format, self.bits, self.order)?;
        if values.len() != self.geo.expected_len(self.geo.plane_tile(chunk)) {
            return Err(());
        }
        Ok(values)
    }

    /// The samples of a chunk stored with `coded`, decoded by [`decode_coded`].
    fn read_coded(&self, index: usize, coded: Coded) -> Result<Vec<f32>, ()> {
        let bytes = self.chunk_bytes(index)?;
        let chunk = Chunk {
            len: self.geo.expected_len(self.geo.plane_tile(index)),
            row_samples: self.geo.tile_w * self.geo.chunk_spp,
            chunk_spp: self.geo.chunk_spp,
            bits: self.bits,
            format: self.format,
            order: self.order,
        };
        decode_coded(&bytes, coded, chunk)
    }

    /// The samples of a compressed chunk, or one with a predictor, decoded by the TIFF decoder. The
    /// decoder reads the chunk's bytes from a window read once here, and nothing else. The rows of
    /// the chunk are stored `tile_w` pixels apart, as the raw samples are.
    fn read_decoded(&self, chunk: usize) -> Result<Vec<f32>, ()> {
        geotiff::contained(|| self.decode_chunk(chunk))
    }

    fn decode_chunk(&self, chunk: usize) -> Result<Vec<f32>, ()> {
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
        let (data_w, data_h) = self.geo.data_dims(self.geo.plane_tile(chunk));
        let row_len = data_w.checked_mul(self.geo.chunk_spp).ok_or(())?;
        let rows_len = row_len.checked_mul(data_h).ok_or(())?;
        // The decoder's width is the image's: its padding is by column. Its height is not always
        // the image's: `tiff` 0.10.3 takes the bottom padding of a tile of a planar image from the
        // chunk index, not from its row within the plane, so a partial tile of a plane after the
        // first comes back with its padding rows. The rows of the image come first.
        if row_len == 0 || values.len() < rows_len || values.len() % row_len != 0 {
            return Err(());
        }
        let stride = self.geo.tile_w.checked_mul(self.geo.chunk_spp).ok_or(())?;
        let mut out = vec![0.0_f32; stride.checked_mul(data_h).ok_or(())?];
        for (row, source) in values.chunks_exact(row_len).take(data_h).enumerate() {
            let start = row.checked_mul(stride).ok_or(())?;
            let end = start.checked_add(row_len).ok_or(())?;
            out.get_mut(start..end).ok_or(())?.copy_from_slice(source);
        }
        Ok(out)
    }
}

/// libtiff's size of a strip it cuts from a single uncompressed strip (`STRIP_SIZE_DEFAULT`).
const STRIP_SIZE_DEFAULT: u64 = 8192;

/// The strips libtiff reads of an image stored as one uncompressed chunky strip: it cuts the strip
/// into strips of about 8 KiB, whole rows (`ChopUpSingleUncompressedStrip`, libtiff 4.7). PROJ reads
/// a point's strip through libtiff, so the bytes asked of a remote file are the same.
fn chop_single_strip(mut image: ImageTiles) -> Result<ImageTiles, ()> {
    let &[(offset, count)] = image.places.as_slice() else {
        return Ok(image);
    };
    let single = image.layout == Layout::Strips
        && image.storage == Storage::Chunky
        && matches!(image.reading, Reading::Raw);
    if !single {
        return Ok(image);
    }
    let row_bits = u64::from(image.size.0)
        .checked_mul(u64::try_from(image.spp).map_err(|_| ())?)
        .and_then(|samples| samples.checked_mul(u64::from(image.bits)))
        .ok_or(())?;
    let row_bytes = row_bits.div_ceil(8);
    if row_bytes == 0 {
        return Ok(image);
    }
    let rows = (STRIP_SIZE_DEFAULT / row_bytes).max(1);
    let strip_bytes = rows * row_bytes;
    let rows = u32::try_from(rows).map_err(|_| ())?;
    if rows >= image.tile.1 {
        return Ok(image);
    }
    let strips = image.size.1.div_ceil(rows);
    let mut left = count;
    let mut at = offset;
    image.places = (0..strips)
        .map(|_| {
            let bytes = strip_bytes.min(left);
            let place = (if bytes == 0 { 0 } else { at }, bytes);
            at += bytes;
            left -= bytes;
            place
        })
        .collect();
    image.tile.1 = rows;
    Ok(image)
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

/// [`passes`], which reports a change once the chunks of the pass are read: PROJ finishes the
/// reads of an interpolation before it checks `hasChanged`, and the later reads of the pass
/// are in the cache for the reloaded grid.
fn passes_watch<T>(
    body: &impl Fn() -> Result<T, TransformError>,
) -> (Result<T, TransformError>, bool) {
    for _ in 0..MAX_PASSES {
        let answer = body();
        let missing = take_missing();
        if missing.is_empty() {
            return (answer, false);
        }
        let mut changed = false;
        for (lazy, chunk) in missing {
            if lazy.load(chunk).is_err() {
                return (Err(TransformError::NetworkUnavailable), false);
            }
            changed |= lazy.has_changed();
        }
        if changed {
            return (Err(TransformError::NetworkUnavailable), true);
        }
    }
    (Err(TransformError::NetworkUnavailable), false)
}
