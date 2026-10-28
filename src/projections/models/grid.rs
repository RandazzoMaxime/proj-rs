//! Shift grids used by `+proj=deformation` and `+proj=defmodel`.
//!
//! GeoTIFF follows `GTiffDataset::nextGrid` in PROJ `grids.cpp`: tie points,
//! the transformation matrix, and the bottom-up flag. CTable2 and GTX follow
//! the same file. Values are stored with row 0 at the south, which is what
//! `valueAt` indexes.

use super::inflate::zlib_decode;

const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

#[derive(Clone, Debug)]
pub(super) struct Grid {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) samples: usize,
    /// South-first, row-major, sample interleaved.
    values: Vec<f32>,
    pub(super) west: f64,
    pub(super) south: f64,
    pub(super) east: f64,
    pub(super) north: f64,
    pub(super) res_x: f64,
    pub(super) res_y: f64,
    pub(super) geographic: bool,
    desc: Vec<String>,
    unit: Vec<String>,
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

/// Inverse of libtiff `fpAcc`: byte prefix-sum, then little-endian byte gather.
fn float_predictor(row: &mut [u8], stride: usize, bps: usize) {
    if stride == 0 || bps == 0 || row.len() % (bps * stride) != 0 {
        return;
    }
    let mut count = row.len();
    if stride == 1 {
        let mut cursor = 0usize;
        while count > 1 {
            row[cursor + 1] = row[cursor + 1].wrapping_add(row[cursor]);
            cursor += 1;
            count -= 1;
        }
    } else {
        let mut cursor = 0usize;
        while count > stride {
            for _ in 0..stride {
                row[cursor + stride] = row[cursor + stride].wrapping_add(row[cursor]);
                cursor += 1;
            }
            count -= stride;
        }
    }
    let words = row.len() / bps;
    let tmp = row.to_vec();
    for word in 0..words {
        for byte in 0..bps {
            row[bps * word + byte] = tmp[(bps - byte - 1) * words + word];
        }
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
    } else if name_ends(name, ".ct2") {
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

fn open_tiff(bytes: &[u8]) -> Option<Grid> {
    if bytes.len() < 8 || bytes[0] != b'I' || bytes[1] != b'I' || u16_at(bytes, 2)? != 42 {
        return None;
    }
    let mut ifd = usize::try_from(u32_at(bytes, 4)?).ok()?;
    let mut entries = Vec::new();
    let count = usize::from(u16_at(bytes, ifd)?);
    ifd += 2;
    for _ in 0..count {
        let tag = u16_at(bytes, ifd)?;
        let kind = u16_at(bytes, ifd + 2)?;
        let n = u32_at(bytes, ifd + 4)?;
        let raw = u32_at(bytes, ifd + 8)?;
        entries.push(Entry { tag, kind, n, raw });
        ifd += 12;
    }
    let width = entry_usize(&entries, bytes, 256)?;
    let height = entry_usize(&entries, bytes, 257)?;
    if width == 0 || height == 0 || width > 100_000 || height > 100_000 {
        return None;
    }
    let samples = entry_usize(&entries, bytes, 277).unwrap_or(1);
    if samples == 0 || samples > 16 {
        return None;
    }
    let compression = entry_u16(&entries, 259).unwrap_or(1);
    if compression != 1 && compression != 8 {
        return None;
    }
    let planar = entry_u16(&entries, 284).unwrap_or(1);
    let rows_per = entry_usize(&entries, bytes, 278).unwrap_or(height);
    if rows_per == 0 {
        return None;
    }
    if entry_u16(&entries, 322).is_some() {
        return None;
    }
    let predictor = entry_u16(&entries, 317).unwrap_or(1);
    if predictor != 1 && predictor != 3 {
        return None;
    }
    let bits = entry_shorts(&entries, bytes, 258)?;
    let formats = entry_shorts(&entries, bytes, 339)?;
    if bits.len() != samples || formats.len() != samples {
        return None;
    }
    if bits.iter().any(|bit| *bit != 32) || formats.iter().any(|fmt| *fmt != 3) {
        return None;
    }
    let offsets = entry_u32s(&entries, bytes, 273)?;
    let counts = entry_u32s(&entries, bytes, 279)?;
    let strips_per_plane = height.div_ceil(rows_per);
    let planes = if planar == 2 { samples } else { 1 };
    if offsets.len() != strips_per_plane * planes || counts.len() != offsets.len() {
        return None;
    }
    let mut strips = Vec::with_capacity(offsets.len());
    for (offset, count) in offsets.iter().zip(counts.iter()) {
        let start = usize::try_from(*offset).ok()?;
        let size = usize::try_from(*count).ok()?;
        let raw = bytes.get(start..start + size)?.to_vec();
        let decoded = if compression == 8 {
            zlib_decode(&raw)?
        } else {
            raw
        };
        strips.push(decoded);
    }
    if predictor == 3 {
        let stride = if planar == 2 { 1 } else { samples };
        let row_bytes = width.checked_mul(4)?.checked_mul(stride)?;
        for strip in &mut strips {
            if row_bytes == 0 || strip.len() % row_bytes != 0 {
                return None;
            }
            for row in strip.chunks_mut(row_bytes) {
                float_predictor(row, stride, 4);
            }
        }
    }
    let placed = tiff_extent(&entries, bytes, width, height)?;
    let Placed {
        west,
        south,
        east,
        north,
        res_x,
        res_y,
        geographic,
        bottom_up,
    } = placed;
    let mut values = vec![0.0_f32; width * height * samples];
    for tiff_row in 0..height {
        let y_south = if bottom_up {
            tiff_row
        } else {
            height - 1 - tiff_row
        };
        let strip_in_plane = tiff_row / rows_per;
        let row_in_strip = tiff_row % rows_per;
        for x in 0..width {
            for sample in 0..samples {
                let strip_index = if planar == 2 {
                    sample * strips_per_plane + strip_in_plane
                } else {
                    strip_in_plane
                };
                let strip = strips.get(strip_index)?;
                let pixel = if planar == 2 {
                    (row_in_strip * width + x) * 4
                } else {
                    (row_in_strip * width + x) * samples * 4 + sample * 4
                };
                let value = f32_at(strip, pixel, true)?;
                let slot = (y_south * width + x) * samples + sample;
                values[slot] = value;
            }
        }
    }
    let (desc, unit) = gdal_bands(&entries, bytes, samples);
    Some(Grid {
        width,
        height,
        samples,
        values,
        west,
        south,
        east,
        north,
        res_x,
        res_y,
        geographic,
        desc,
        unit,
    })
}

struct Entry {
    tag: u16,
    kind: u16,
    n: u32,
    raw: u32,
}

fn find(entries: &[Entry], tag: u16) -> Option<&Entry> {
    entries.iter().find(|entry| entry.tag == tag)
}

fn entry_u16(entries: &[Entry], tag: u16) -> Option<u16> {
    let entry = find(entries, tag)?;
    if entry.kind == 3 && entry.n == 1 {
        u16::try_from(entry.raw & 0xffff).ok()
    } else if entry.kind == 4 && entry.n == 1 {
        u16::try_from(entry.raw).ok()
    } else {
        None
    }
}

fn entry_usize(entries: &[Entry], bytes: &[u8], tag: u16) -> Option<usize> {
    let entry = find(entries, tag)?;
    let value = if entry.n == 1 && (entry.kind == 3 || entry.kind == 4) {
        if entry.kind == 3 {
            entry.raw & 0xffff
        } else {
            entry.raw
        }
    } else {
        let values = entry_u32s(entries, bytes, tag)?;
        *values.first()?
    };
    usize::try_from(value).ok()
}

fn entry_shorts(entries: &[Entry], bytes: &[u8], tag: u16) -> Option<Vec<u16>> {
    let entry = find(entries, tag)?;
    if entry.kind != 3 {
        return None;
    }
    let n = usize::try_from(entry.n).ok()?;
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(vec![u16::try_from(entry.raw & 0xffff).ok()?]);
    }
    if n == 2 && entry.kind == 3 {
        let lo = u16::try_from(entry.raw & 0xffff).ok()?;
        let hi = u16::try_from(entry.raw >> 16).ok()?;
        return Some(vec![lo, hi]);
    }
    let start = usize::try_from(entry.raw).ok()?;
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        out.push(u16_at(bytes, start + index * 2)?);
    }
    Some(out)
}

fn entry_u32s(entries: &[Entry], bytes: &[u8], tag: u16) -> Option<Vec<u32>> {
    let entry = find(entries, tag)?;
    let n = usize::try_from(entry.n).ok()?;
    if entry.kind == 3 {
        return Some(
            entry_shorts(entries, bytes, tag)?
                .into_iter()
                .map(u32::from)
                .collect(),
        );
    }
    if entry.kind != 4 {
        return None;
    }
    if n == 1 {
        return Some(vec![entry.raw]);
    }
    let start = usize::try_from(entry.raw).ok()?;
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        out.push(u32_at(bytes, start + index * 4)?);
    }
    Some(out)
}

fn entry_f64s(entries: &[Entry], bytes: &[u8], tag: u16) -> Option<Vec<f64>> {
    let entry = find(entries, tag)?;
    if entry.kind != 12 {
        return None;
    }
    let n = usize::try_from(entry.n).ok()?;
    let start = usize::try_from(entry.raw).ok()?;
    let mut out = Vec::with_capacity(n);
    for index in 0..n {
        out.push(f64_at(bytes, start + index * 8, true)?);
    }
    Some(out)
}

struct Placed {
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    res_x: f64,
    res_y: f64,
    geographic: bool,
    bottom_up: bool,
}

fn tiff_extent(entries: &[Entry], bytes: &[u8], width: usize, height: usize) -> Option<Placed> {
    let mut geographic = true;
    let mut pixel_is_area = false;
    if let Some(keys) = entry_shorts(entries, bytes, 34735) {
        if keys.len() < 4 || keys.len() % 4 != 0 || keys[0] != 1 {
            return None;
        }
        let mut index = 4;
        while index + 3 < keys.len() {
            let key = keys[index];
            let value = keys[index + 3];
            if key == 1024 {
                if value == 1 {
                    geographic = false;
                } else if value != 2 {
                    return None;
                }
            } else if key == 1025 && value == 1 {
                pixel_is_area = true;
            }
            index += 4;
        }
    }
    let (h_res, v_res, mut west, mut north) =
        if let Some(matrix) = entry_f64s(entries, bytes, 34264) {
            if matrix.len() != 16 || matrix[1] != 0.0 || matrix[4] != 0.0 {
                return None;
            }
            (matrix[0], -matrix[5], matrix[3], matrix[7])
        } else {
            let scale = entry_f64s(entries, bytes, 33550)?;
            let tie = entry_f64s(entries, bytes, 33922)?;
            if scale.len() != 3 || tie.len() != 6 {
                return None;
            }
            let h = scale[0];
            let v = scale[1];
            (h, v, tie[3] - tie[0] * h, tie[4] + tie[1] * v)
        };
    if pixel_is_area {
        west += 0.5 * h_res;
        north -= 0.5 * v_res;
    }
    let mul = if geographic { DEG_TO_RAD } else { 1.0 };
    let extent_west = west * mul;
    let mut extent_north = north * mul;
    let res_x = h_res * mul;
    let res_y = v_res.abs() * mul;
    let extent_east = (west + h_res * dim(width - 1)) * mul;
    let mut extent_south = (north - v_res * dim(height - 1)) * mul;
    let bottom_up = v_res < 0.0;
    if v_res < 0.0 {
        std::mem::swap(&mut extent_north, &mut extent_south);
    }
    let pi = std::f64::consts::PI;
    let sane = (!geographic
        || (extent_west.abs() <= 4.0 * pi
            && extent_east.abs() <= 4.0 * pi
            && extent_north.abs() <= pi + 1e-5
            && extent_south.abs() <= pi + 1e-5))
        && extent_west < extent_east
        && extent_south < extent_north
        && res_x > 1e-10
        && res_y > 1e-10;
    if !sane {
        return None;
    }
    Some(Placed {
        west: extent_west,
        south: extent_south,
        east: extent_east,
        north: extent_north,
        res_x,
        res_y,
        geographic,
        bottom_up,
    })
}

fn gdal_bands(entries: &[Entry], bytes: &[u8], samples: usize) -> (Vec<String>, Vec<String>) {
    let mut desc = vec![String::new(); samples];
    let mut unit = vec![String::new(); samples];
    let Some(entry) = find(entries, 42112) else {
        return (desc, unit);
    };
    let text = if entry.kind != 2 {
        return (desc, unit);
    } else if entry.n <= 4 {
        return (desc, unit);
    } else {
        let start = usize::try_from(entry.raw).unwrap_or(0);
        let len = usize::try_from(entry.n.saturating_sub(1)).unwrap_or(0);
        bytes
            .get(start..start + len)
            .and_then(|slice| std::str::from_utf8(slice).ok())
            .unwrap_or("")
    };
    let mut rest = text;
    while let Some(start) = rest.find("<Item ") {
        rest = &rest[start + 6..];
        let Some(end_tag) = rest.find('>') else { break };
        let tag = &rest[..end_tag];
        rest = &rest[end_tag + 1..];
        let Some(end_value) = rest.find('<') else {
            break;
        };
        let value = &rest[..end_value];
        let Some(name) = attr(tag, "name") else { break };
        let sample = attr(tag, "sample").and_then(|text| text.parse::<usize>().ok());
        let role = attr(tag, "role").unwrap_or("");
        if let Some(sample) = sample {
            if sample < samples {
                if role == "description" || name == "DESCRIPTION" {
                    desc[sample] = value.to_owned();
                }
                if role == "unittype" || name == "UNITTYPE" {
                    unit[sample] = value.to_owned();
                }
            }
        }
    }
    (desc, unit)
}

fn attr<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let chunk = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([chunk[0], chunk[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let chunk = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
}

fn f64_at(bytes: &[u8], offset: usize, little: bool) -> Option<f64> {
    let chunk = bytes.get(offset..offset + 8)?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(chunk);
    if !little {
        raw.reverse();
    }
    Some(f64::from_le_bytes(raw))
}

fn f32_at(bytes: &[u8], offset: usize, little: bool) -> Option<f32> {
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
fn dim(value: usize) -> f64 {
    value as f64
}
