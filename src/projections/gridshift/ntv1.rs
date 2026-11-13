//! Canadian NTv1 (`.dat`) horizontal grids.
//!
//! Port of `NTv1Grid::open` and `NTv1Grid::valueAt` in PROJ `src/grids.cpp`. The file is
//! big-endian, its header holds degrees, and each record holds a latitude then a longitude
//! shift in seconds. Records run east to west, south to north. The samples are stored as the
//! NTv2 reader stores them (south to north, west to east, longitude negated), so both formats
//! share one grid set.

use super::ntv2::NtvGrid;
use super::{Domain, Extent};

const HEADER: usize = 192;
const RECORD: usize = 16;
const ARCSEC_TO_RAD: f64 = (std::f64::consts::PI / 180.0) / 3600.0;
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// `header[0..6]`, `header[96..102]` and `header[144..160]` of PROJ's `pj_gridinfo_init`.
pub(super) fn is_ntv1(bytes: &[u8]) -> bool {
    bytes.len() >= 160
        && bytes.starts_with(b"HEADER")
        && &bytes[96..102] == b"W GRID"
        && &bytes[144..160] == b"TO      NAD83   "
}

/// The one grid of an NTv1 file.
pub(super) fn load(bytes: &[u8]) -> Result<NtvGrid, ()> {
    if bytes.len() < HEADER || read_i32(&bytes[8..12]) != 12 {
        return Err(());
    }
    let south = read_f64(&bytes[24..32]) * DEG_TO_RAD;
    let north = read_f64(&bytes[40..48]) * DEG_TO_RAD;
    let east = -read_f64(&bytes[56..64]) * DEG_TO_RAD;
    let west = -read_f64(&bytes[72..80]) * DEG_TO_RAD;
    let res_y = read_f64(&bytes[88..96]) * DEG_TO_RAD;
    let res_x = read_f64(&bytes[104..112]) * DEG_TO_RAD;
    let extent = Extent::try_new(Domain::Geographic, west, south, east, north, res_x, res_y)?;
    let columns = trunc_positive(((east - west) * extent.inv_x + 0.5).abs() + 1.0)?;
    let rows = trunc_positive(((north - south) * extent.inv_y + 0.5).abs() + 1.0)?;
    let cols = usize::try_from(columns).map_err(|_| ())?;
    let rows_us = usize::try_from(rows).map_err(|_| ())?;
    let records = cols.checked_mul(rows_us).ok_or(())?;
    if bytes.len() < HEADER + records * RECORD {
        return Err(());
    }
    let data = &bytes[HEADER..HEADER + records * RECORD];
    let mut samples = vec![0.0_f32; records * 2];
    for row in 0..rows_us {
        for col in 0..cols {
            // NTv1 is organized from east to west (`NTv1Grid::valueAt`).
            let record = (row * cols + (cols - 1 - col)) * RECORD;
            let lat = read_f64(&data[record..record + 8]);
            let lon = read_f64(&data[record + 8..record + 16]);
            let dest = (row * cols + col) * 2;
            samples[dest] = f32_from_f64(lat * ARCSEC_TO_RAD);
            // West longitude positive: the compensated convention of `valueAt`.
            samples[dest + 1] = f32_from_f64(-lon * ARCSEC_TO_RAD);
        }
    }
    Ok(NtvGrid {
        width: columns,
        height: rows,
        extent,
        samples,
        grid_name: String::new(),
        parent_name: String::new(),
    })
}

fn f32_from_f64(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "NTv1 shifts are stored as float, matching NTv1Grid::valueAt"
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

fn read_i32(bytes: &[u8]) -> i32 {
    let mut buf = [0_u8; 4];
    buf.copy_from_slice(&bytes[..4]);
    i32::from_be_bytes(buf)
}

fn read_f64(bytes: &[u8]) -> f64 {
    let mut buf = [0_u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    f64::from_be_bytes(buf)
}
