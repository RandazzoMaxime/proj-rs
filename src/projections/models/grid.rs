//! Shift grids used by `+proj=deformation` and `+proj=defmodel`.
//!
//! GeoTIFF follows `GTiffDataset::nextGrid` in PROJ `grids.cpp`: tie points,
//! the transformation matrix, and the bottom-up flag. CTable2 and GTX follow
//! the same file. Values are stored with row 0 at the south, which is what
//! `valueAt` indexes.

use super::grid_tiff::open_tiff;

pub(super) const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

#[derive(Clone, Debug)]
pub(super) struct Grid {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) samples: usize,
    /// South-first, row-major, sample interleaved.
    pub(super) values: Vec<f32>,
    pub(super) west: f64,
    pub(super) south: f64,
    pub(super) east: f64,
    pub(super) north: f64,
    pub(super) res_x: f64,
    pub(super) res_y: f64,
    pub(super) geographic: bool,
    pub(super) desc: Vec<String>,
    pub(super) unit: Vec<String>,
}

impl Grid {
    pub(super) fn value(&self, x: usize, y: usize, sample: usize) -> Option<f32> {
        if x >= self.width || y >= self.height || sample >= self.samples {
            return None;
        }
        self.values
            .get(((y * self.width) + x) * self.samples + sample)
            .copied()
    }

    pub(super) fn description(&self, sample: usize) -> &str {
        self.desc.get(sample).map(String::as_str).unwrap_or("")
    }

    pub(super) fn unit(&self, sample: usize) -> &str {
        self.unit.get(sample).map(String::as_str).unwrap_or("")
    }

    /// Point in extent, with the same longitude wrap as `isPointInExtent`.
    pub(super) fn contains(&self, x: f64, y: f64, eps: f64) -> bool {
        if !(y + eps >= self.south && y - eps <= self.north) {
            return false;
        }
        let mut x = x;
        if self.geographic {
            if x + eps < self.west {
                x += 2.0 * std::f64::consts::PI;
            } else if x - eps > self.east {
                x -= 2.0 * std::f64::consts::PI;
            }
        }
        x + eps >= self.west && x - eps <= self.east
    }
}

fn name_ends(name: &str, suffix: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
        .ends_with(suffix)
}

pub(super) fn open_named(name: &str, bytes: &[u8]) -> Option<Grid> {
    if name_ends(name, ".gtx") {
        open_gtx(bytes)
    } else if name_ends(name, ".ct2") || bytes.starts_with(b"CTABLE V2.") {
        open_ctable2(bytes)
    } else if name_ends(name, ".tif") || name_ends(name, ".tiff") || is_le_tiff(bytes) {
        open_tiff(bytes)
    } else {
        None
    }
}

fn is_le_tiff(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == b'I' && bytes[1] == b'I' && bytes[2] == 42 && bytes[3] == 0
}

fn open_gtx(bytes: &[u8]) -> Option<Grid> {
    if bytes.len() < 40 {
        return None;
    }
    let yorigin = f64_at(bytes, 0, false)?;
    let mut xorigin = f64_at(bytes, 8, false)?;
    let ystep = f64_at(bytes, 16, false)?;
    let xstep = f64_at(bytes, 24, false)?;
    let rows = i32_at(bytes, 32, false)?;
    let columns = i32_at(bytes, 36, false)?;
    if columns <= 0
        || rows <= 0
        || !(-360.0..=360.0).contains(&xorigin)
        || !(-90.0..=90.0).contains(&yorigin)
    {
        return None;
    }
    if xorigin >= 180.0 {
        xorigin -= 360.0;
    }
    let width = usize::try_from(columns).ok()?;
    let height = usize::try_from(rows).ok()?;
    let count = width.checked_mul(height)?;
    let data = bytes.get(40..40 + count * 4)?;
    let mut values = Vec::with_capacity(count);
    for index in 0..count {
        values.push(f32_at(data, index * 4, false)?);
    }
    let west = xorigin * DEG_TO_RAD;
    let south = yorigin * DEG_TO_RAD;
    let res_x = xstep * DEG_TO_RAD;
    let res_y = ystep * DEG_TO_RAD;
    Some(Grid {
        width,
        height,
        samples: 1,
        values,
        west,
        south,
        east: west + res_x * dim(width - 1),
        north: south + res_y * dim(height - 1),
        res_x,
        res_y,
        geographic: true,
        desc: vec![String::new()],
        unit: vec![String::new()],
    })
}

fn open_ctable2(bytes: &[u8]) -> Option<Grid> {
    if bytes.len() < 160 {
        return None;
    }
    // Little-endian on this host. PROJ swaps only when the host is big-endian.
    let west = f64_at(bytes, 96, true)?;
    let south = f64_at(bytes, 104, true)?;
    let res_x = f64_at(bytes, 112, true)?;
    let res_y = f64_at(bytes, 120, true)?;
    if west.abs() > 4.0 * std::f64::consts::PI
        || south.abs() > std::f64::consts::PI + 1e-5
        || res_x <= 1e-10
        || res_y <= 1e-10
    {
        return None;
    }
    let width = usize::try_from(i32_at(bytes, 128, true)?).ok()?;
    let height = usize::try_from(i32_at(bytes, 132, true)?).ok()?;
    if width == 0 || height == 0 {
        return None;
    }
    let count = width.checked_mul(height)?;
    let data = bytes.get(160..)?;
    if data.len() < count * 8 {
        return None;
    }
    // PROJ stores (lat, long) after an optional sign flip. deformation calls
    // `pj_hgrid_value`, which passes compensateNTConvention = false, so the
    // first float is east and the second is north, both in mm/year.
    let mut values = Vec::with_capacity(count * 2);
    for index in 0..count {
        let east = f32_at(data, index * 8, true)?;
        let north = f32_at(data, index * 8 + 4, true)?;
        values.push(east);
        values.push(north);
    }
    Some(Grid {
        width,
        height,
        samples: 2,
        values,
        west,
        south,
        east: west + res_x * dim(width - 1),
        // `CTable2Grid::open` adds `(height - 1) * resX`, not `resY`.
        north: south + res_x * dim(height - 1),
        res_x,
        res_y,
        geographic: true,
        desc: vec!["east".to_owned(), "north".to_owned()],
        unit: vec![String::new(), String::new()],
    })
}

pub(super) fn f64_at(bytes: &[u8], offset: usize, little: bool) -> Option<f64> {
    let chunk = bytes.get(offset..offset + 8)?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(chunk);
    if !little {
        raw.reverse();
    }
    Some(f64::from_le_bytes(raw))
}

pub(super) fn f32_at(bytes: &[u8], offset: usize, little: bool) -> Option<f32> {
    let chunk = bytes.get(offset..offset + 4)?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(chunk);
    if !little {
        raw.reverse();
    }
    Some(f32::from_le_bytes(raw))
}

fn i32_at(bytes: &[u8], offset: usize, little: bool) -> Option<i32> {
    let chunk = bytes.get(offset..offset + 4)?;
    let mut raw = [0u8; 4];
    raw.copy_from_slice(chunk);
    if !little {
        raw.reverse();
    }
    Some(i32::from_le_bytes(raw))
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "grid dimensions used here are far below the f64 mantissa"
)]
pub(super) fn dim(value: usize) -> f64 {
    value as f64
}
