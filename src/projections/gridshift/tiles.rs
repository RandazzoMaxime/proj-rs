//! Grid samples read tile by tile (or strip by strip) from a remote GeoTIFF, as PROJ reads them
//! (`GTiffGrid`): a tile is read when one of its samples is needed, not when the grid is opened.
//! The tile is read whole, as its `TileByteCounts` (or `StripByteCounts`) size, so the ranges
//! asked of the callbacks are PROJ's.
//!
//! A sample of a tile that is not read yet gives `None`, and the tile is noted. [`with_tiles`]
//! then reads the noted tiles and runs the point again, so that the answer is the one a grid
//! read whole would give: an interpolation never sees a sample that is not there.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use super::geotiff::{self, ByteOrder};
use crate::engine::TransformError;
use crate::network_chunks::RemoteFile;

/// The samples of one image: in memory, or in the remote file, by tile.
pub(super) enum Samples {
    Flat(Vec<f32>),
    Lazy(Arc<LazyIfd>),
}

impl Samples {
    /// The sample `sample` of pixel `(x, y)`, in a raster `width` pixels wide with `spp` samples
    /// per pixel. `None` outside the raster, or in a tile that is not read yet: the tile is then
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
                let (tile, inside) = lazy.locate(x, y, sample)?;
                match lazy.tiles.get(tile)?.get() {
                    Some(values) => values.get(inside).copied(),
                    None => {
                        note_missing(lazy, tile);
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
    /// The file offset and the byte count of each tile or strip, as `TileOffsets` and
    /// `TileByteCounts`, or `StripOffsets` and `StripByteCounts`.
    pub(super) places: Vec<(u64, u64)>,
}

/// One image of a remote TIFF, whose tiles or strips are read on first need.
pub(super) struct LazyIfd {
    file: Arc<Mutex<RemoteFile>>,
    tile_w: usize,
    tile_h: usize,
    /// Tiles per row.
    across: usize,
    /// The image height, which bounds the rows of the last strip.
    height: usize,
    layout: Layout,
    spp: usize,
    bits: u32,
    format: u32,
    order: ByteOrder,
    /// The number of samples of one tile: `tile_w * tile_h * spp`.
    tile_len: usize,
    places: Vec<(u64, u64)>,
    /// The decoded samples of each tile, once read.
    tiles: Vec<OnceLock<Box<[f32]>>>,
}

impl LazyIfd {
    pub(super) fn new(file: Arc<Mutex<RemoteFile>>, image: ImageTiles) -> Result<Self, ()> {
        let tile_w = usize::try_from(image.tile.0).map_err(|_| ())?;
        let width = usize::try_from(image.size.0).map_err(|_| ())?;
        let height = usize::try_from(image.size.1).map_err(|_| ())?;
        // A single strip may be `RowsPerStrip` = 2^32 - 1: the rows of the image are the strip.
        let tile_h = usize::try_from(image.tile.1).map_err(|_| ())?;
        let tile_h = match image.layout {
            Layout::Tiles => tile_h,
            Layout::Strips => tile_h.min(height),
        };
        if tile_w == 0 || tile_h == 0 || image.spp == 0 {
            return Err(());
        }
        if image.layout == Layout::Strips && tile_w != width {
            return Err(());
        }
        let across = width.div_ceil(tile_w);
        let down = height.div_ceil(tile_h);
        let count = across.checked_mul(down).ok_or(())?;
        if image.places.len() != count {
            return Err(());
        }
        let tile_len = tile_w
            .checked_mul(tile_h)
            .and_then(|pixels| pixels.checked_mul(image.spp))
            .ok_or(())?;
        Ok(Self {
            file,
            tile_w,
            tile_h,
            across,
            height,
            layout: image.layout,
            spp: image.spp,
            bits: image.bits,
            format: image.format,
            order: image.order,
            tile_len,
            places: image.places,
            tiles: (0..count).map(|_| OnceLock::new()).collect(),
        })
    }

    /// The number of samples a tile holds in the file: a whole tile, or for a strip the rows that
    /// the image has in it.
    fn expected_len(&self, tile: usize) -> usize {
        match self.layout {
            Layout::Tiles => self.tile_len,
            Layout::Strips => {
                let top = tile.saturating_mul(self.tile_h);
                let rows = self.height.saturating_sub(top).min(self.tile_h);
                rows * self.tile_w * self.spp
            }
        }
    }

    /// The tile that holds sample `sample` of pixel `(x, y)`, and its position in that tile.
    fn locate(&self, x: usize, y: usize, sample: usize) -> Option<(usize, usize)> {
        if sample >= self.spp {
            return None;
        }
        let col = x / self.tile_w;
        let row = y / self.tile_h;
        let tile = row.checked_mul(self.across)?.checked_add(col)?;
        let within_x = x - col * self.tile_w;
        let within_y = y - row * self.tile_h;
        let inside = within_y
            .checked_mul(self.tile_w)?
            .checked_add(within_x)?
            .checked_mul(self.spp)?
            .checked_add(sample)?;
        Some((tile, inside))
    }

    /// Reads tile `tile` from the file, unless it is read already. The read is the tile's whole
    /// byte count, from its offset.
    pub(super) fn load(&self, tile: usize) -> Result<(), ()> {
        let cell = self.tiles.get(tile).ok_or(())?;
        if cell.get().is_some() {
            return Ok(());
        }
        let (offset, count) = *self.places.get(tile).ok_or(())?;
        let size = usize::try_from(count).map_err(|_| ())?;
        let bytes = self
            .file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read_at(offset, size)
            .map_err(|_| ())?;
        if bytes.len() != size {
            return Err(());
        }
        let values = geotiff::raw_samples(&bytes, self.format, self.bits, self.order)?;
        if values.len() != self.expected_len(tile) {
            return Err(());
        }
        // A concurrent load of the same tile gives the same samples: either set is kept.
        let _ = cell.set(values.into_boxed_slice());
        Ok(())
    }
}

thread_local! {
    /// The tiles that the current pass of a point found missing.
    static MISSING: RefCell<Vec<(Arc<LazyIfd>, usize)>> = const { RefCell::new(Vec::new()) };
}

fn note_missing(lazy: &Arc<LazyIfd>, tile: usize) {
    MISSING.with(|missing| missing.borrow_mut().push((Arc::clone(lazy), tile)));
}

fn take_missing() -> Vec<(Arc<LazyIfd>, usize)> {
    MISSING.with(|missing| std::mem::take(&mut *missing.borrow_mut()))
}

/// The most passes one point takes. A pass reads every tile it found missing, so the passes
/// needed are the tiles that a point's interpolation reaches, one set per nested grid.
const MAX_PASSES: usize = 16;

/// Runs `body`, the transform of one point. When a pass finds a tile missing, the tile is read
/// and the pass is run again, until a pass finds none missing. Nested calls keep the tiles that
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
/// `None` when `body` gives none, and also when a tile it needs cannot be read.
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
        for (lazy, tile) in missing {
            lazy.load(tile)
                .map_err(|()| TransformError::NetworkUnavailable)?;
        }
    }
    Err(TransformError::NetworkUnavailable)
}
