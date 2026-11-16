//! GeoTIFF shift grids of `+proj=deformation` and `+proj=defmodel`, read whole.
//!
//! `GTiffDataset::nextGrid` in PROJ `grids.cpp`: tie points, the transformation matrix, the
//! bottom-up flag, GDAL band metadata.

use super::grid::{dim, f32_at, f64_at, Grid, DEG_TO_RAD};
use super::inflate::zlib_decode;
use crate::projections::tiff_codec::predictor::{undo_predictor, Predictor};
use crate::projections::tiff_codec::ByteOrder;

pub(super) fn open_tiff(bytes: &[u8]) -> Option<Grid> {
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
    let tile = match (
        entry_usize(&entries, bytes, 322),
        entry_usize(&entries, bytes, 323),
    ) {
        (Some(tile_w), Some(tile_h)) if tile_w > 0 && tile_h > 0 => Some((tile_w, tile_h)),
        (None, None) => None,
        _ => return None,
    };
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
    let (block_w, block_h, offsets_tag, counts_tag) = match tile {
        Some((tile_w, tile_h)) => (tile_w, tile_h, 324, 325),
        None => (width, rows_per, 273, 279),
    };
    let across = width.div_ceil(block_w);
    let per_plane = across * height.div_ceil(block_h);
    let offsets = entry_u32s(&entries, bytes, offsets_tag)?;
    let counts = entry_u32s(&entries, bytes, counts_tag)?;
    let planes = if planar == 2 { samples } else { 1 };
    if offsets.len() != per_plane * planes || counts.len() != offsets.len() {
        return None;
    }
    let mut blocks = Vec::with_capacity(offsets.len());
    for (offset, count) in offsets.iter().zip(counts.iter()) {
        let start = usize::try_from(*offset).ok()?;
        let size = usize::try_from(*count).ok()?;
        let raw = bytes.get(start..start + size)?.to_vec();
        let decoded = if compression == 8 {
            zlib_decode(&raw)?
        } else {
            raw
        };
        blocks.push(decoded);
    }
    if predictor == 3 {
        let stride = if planar == 2 { 1 } else { samples };
        let row_bytes = block_w.checked_mul(4)?.checked_mul(stride)?;
        for block in &mut blocks {
            undo_predictor(
                block,
                row_bytes,
                stride,
                32,
                Predictor::Float,
                ByteOrder::Little,
            )
            .ok()?;
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
        let block_row = tiff_row / block_h;
        let row_in_block = tiff_row % block_h;
        for x in 0..width {
            let index_in_plane = block_row * across + x / block_w;
            let col_in_block = x % block_w;
            for sample in 0..samples {
                let block_index = if planar == 2 {
                    sample * per_plane + index_in_plane
                } else {
                    index_in_plane
                };
                let block = blocks.get(block_index)?;
                let pixel = if planar == 2 {
                    (row_in_block * block_w + col_in_block) * 4
                } else {
                    (row_in_block * block_w + col_in_block) * samples * 4 + sample * 4
                };
                let value = f32_at(block, pixel, true)?;
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

#[cfg(test)]
#[path = "grid_tiff_tests.rs"]
mod tests;
