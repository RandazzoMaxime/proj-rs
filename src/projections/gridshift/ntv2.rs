//! Canadian NTv2 (`.gsb`) horizontal grids.
//!
//! Port of `NTv2GridSet::open` and `NTv2Grid::valueAt` in PROJ `src/grids.cpp`.
//! Records run south to north and east to west. Longitude shifts are positive west.

use super::{Domain, Extent};

#[derive(Clone, Copy)]
enum Order {
    Little,
    Big,
}

const RECORD: usize = 16;
const HEADER: usize = 11 * RECORD;
const ARCSEC_TO_RAD: f64 = (std::f64::consts::PI / 180.0) / 3600.0;

pub(super) struct NtvGrid {
    pub width: i32,
    pub height: i32,
    pub extent: Extent,
    /// South-to-north, west-to-east, interleaved latitude then longitude, radians.
    /// Longitude is already negated (PROJ's `compensateNTConvention`).
    pub samples: Vec<f32>,
    pub grid_name: String,
    pub parent_name: String,
}

pub(super) fn is_ntv2(bytes: &[u8]) -> bool {
    bytes.len() >= 48 + 7 && bytes.starts_with(b"NUM_OREC") && &bytes[48..55] == b"GS_TYPE"
}

pub(super) fn load(bytes: &[u8]) -> Result<Vec<NtvGrid>, ()> {
    if bytes.len() < HEADER || &bytes[56..63] != b"SECONDS" {
        return Err(());
    }
    let order = if bytes[8] == 11 {
        Order::Little
    } else {
        Order::Big
    };
    let count = read_u32(&bytes[40..44], order);
    if count == 0 || count > 10_000 {
        return Err(());
    }
    let mut cursor = HEADER;
    let mut grids = Vec::new();
    for _ in 0..count {
        if bytes.len() < cursor + HEADER {
            return Err(());
        }
        let header = &bytes[cursor..cursor + HEADER];
        cursor += HEADER;
        if &header[0..8] != b"SUB_NAME" {
            return Err(());
        }
        let grid_name = latin8(&header[8..16]);
        let parent_name = latin8(&header[24..32]);
        let south = read_f64(&header[72..80], order) * ARCSEC_TO_RAD;
        let north = read_f64(&header[88..96], order) * ARCSEC_TO_RAD;
        let east = -read_f64(&header[104..112], order) * ARCSEC_TO_RAD;
        let west = -read_f64(&header[120..128], order) * ARCSEC_TO_RAD;
        let res_y = read_f64(&header[136..144], order) * ARCSEC_TO_RAD;
        let res_x = read_f64(&header[152..160], order) * ARCSEC_TO_RAD;
        let extent = Extent::try_new(Domain::Geographic, west, south, east, north, res_x, res_y)?;
        let columns = trunc_positive(((east - west) * extent.inv_x + 0.5).abs() + 1.0)?;
        let rows = trunc_positive(((north - south) * extent.inv_y + 0.5).abs() + 1.0)?;
        if columns <= 0 || rows <= 0 {
            return Err(());
        }
        let gs_count = read_u32(&header[168..172], order);
        let columns_u = u32::try_from(columns).map_err(|_| ())?;
        let rows_u = u32::try_from(rows).map_err(|_| ())?;
        if columns_u == 0 || gs_count / columns_u != rows_u {
            return Err(());
        }
        let bytes_len = usize::try_from(gs_count)
            .map_err(|_| ())?
            .checked_mul(16)
            .ok_or(())?;
        if bytes.len() < cursor + bytes_len {
            return Err(());
        }
        let data = &bytes[cursor..cursor + bytes_len];
        cursor += bytes_len;
        let cols_us = usize::try_from(columns).map_err(|_| ())?;
        let rows_us = usize::try_from(rows).map_err(|_| ())?;
        let mut samples = vec![0.0_f32; rows_us * cols_us * 2];
        for row in 0..rows_us {
            for col in 0..cols_us {
                let record = (row * cols_us + col) * 4;
                let lat = read_f32_at(data, record, order)?;
                let lon = read_f32_at(data, record + 1, order)?;
                let dest_col = cols_us - 1 - col;
                let dest = (row * cols_us + dest_col) * 2;
                samples[dest] = seconds_to_rad(lat);
                samples[dest + 1] = -seconds_to_rad(lon);
            }
        }
        grids.push(NtvGrid {
            width: columns,
            height: rows,
            extent,
            samples,
            grid_name,
            parent_name,
        });
    }
    Ok(grids)
}

fn seconds_to_rad(seconds: f32) -> f32 {
    f32_from_f64(f64::from(seconds) * ARCSEC_TO_RAD)
}

fn f32_from_f64(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "NTv2 shifts are stored as float, matching NTv2Grid::valueAt"
    )]
    {
        value as f32
    }
}

fn trunc_positive(value: f64) -> Result<i32, ()> {
    if !value.is_finite() || value < 0.0 || value > f64::from(i32::MAX) {
        return Err(());
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the value was range-checked against i32"
    )]
    Ok(value as i32)
}

fn latin8(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn read_u32(bytes: &[u8], order: Order) -> u32 {
    let mut buf = [0_u8; 4];
    buf.copy_from_slice(&bytes[..4]);
    match order {
        Order::Little => u32::from_le_bytes(buf),
        Order::Big => u32::from_be_bytes(buf),
    }
}

fn read_f64(bytes: &[u8], order: Order) -> f64 {
    let mut buf = [0_u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    match order {
        Order::Little => f64::from_le_bytes(buf),
        Order::Big => f64::from_be_bytes(buf),
    }
}

fn read_f32_at(bytes: &[u8], index: usize, order: Order) -> Result<f32, ()> {
    let start = index.checked_mul(4).ok_or(())?;
    let end = start + 4;
    if end > bytes.len() {
        return Err(());
    }
    let mut buf = [0_u8; 4];
    buf.copy_from_slice(&bytes[start..end]);
    Ok(match order {
        Order::Little => f32::from_le_bytes(buf),
        Order::Big => f32::from_be_bytes(buf),
    })
}
