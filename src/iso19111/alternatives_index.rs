//! A coarse grid over the candidate boxes, so that a point is tested against the few candidates
//! near it instead of all of them. The grid only narrows the list: the caller still applies the
//! exact `covers` test, in the original candidate order, so the choice is the one of the full scan.

use super::alternatives::{normalize_longitude, Bounds, DegreeOrder};

const COLUMNS: usize = 48;
const ROWS: usize = 24;
const COLUMNS_F: f64 = 48.0;
const ROWS_F: f64 = 24.0;
/// Bounds beyond this (an unbounded box is stored as `f64::MAX`) do not stretch the grid: they
/// fall in its edge cells.
const EXTENT_LIMIT: f64 = 1.0e8;

/// Lists of candidate indices per grid cell, each list in increasing index order.
#[derive(Clone, Debug)]
pub(super) struct BoxIndex {
    min_x: f64,
    min_y: f64,
    x_scale: f64,
    y_scale: f64,
    offsets: Vec<usize>,
    entries: Vec<usize>,
    any_lon_lat: bool,
    any_lat_lon: bool,
}

/// A box no point can be inside of: a NaN bound, or a minimum above its maximum.
fn is_empty(bounds: &Bounds) -> bool {
    !(bounds.min_x <= bounds.max_x && bounds.min_y <= bounds.max_y)
}

#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to the cell range just before"
)]
fn cell(value: f64, origin: f64, scale: f64, count: f64) -> usize {
    // A monotone clamp: an infinite bound lands in the edge cell. Same result as
    // `((value - origin) * scale).floor().clamp(0.0, count - 1.0) as usize`, without the
    // libm `floor`: on `0 < t < count - 1` the truncation of `t as usize` is its floor, and
    // outside, one of the two comparisons takes the clamped edge (`NaN as usize` is 0, as
    // the `floor` of `NaN` clamped to 0 is).
    let t = (value - origin) * scale;
    if t < 1.0 {
        0
    } else if t > count - 1.0 {
        (count - 1.0) as usize
    } else {
        t as usize
    }
}

fn scale(min: f64, max: f64, count: f64) -> f64 {
    if max > min && (max - min).is_finite() {
        count / (max - min)
    } else {
        0.0
    }
}

impl BoxIndex {
    pub(super) fn build(boxes: &[(Bounds, DegreeOrder)]) -> Self {
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (bounds, _) in boxes.iter().filter(|(bounds, _)| !is_empty(bounds)) {
            for (low, high, lo, hi) in [
                (bounds.min_x, bounds.max_x, &mut min_x, &mut max_x),
                (bounds.min_y, bounds.max_y, &mut min_y, &mut max_y),
            ] {
                if low.abs() <= EXTENT_LIMIT {
                    *lo = lo.min(low);
                    *hi = hi.max(low);
                }
                if high.abs() <= EXTENT_LIMIT {
                    *lo = lo.min(high);
                    *hi = hi.max(high);
                }
            }
        }
        if min_x > max_x {
            (min_x, max_x) = (0.0, 0.0);
        }
        if min_y > max_y {
            (min_y, max_y) = (0.0, 0.0);
        }
        let x_scale = scale(min_x, max_x, COLUMNS_F);
        let y_scale = scale(min_y, max_y, ROWS_F);
        let span = |bounds: &Bounds| {
            (
                cell(bounds.min_x, min_x, x_scale, COLUMNS_F)
                    ..=cell(bounds.max_x, min_x, x_scale, COLUMNS_F),
                cell(bounds.min_y, min_y, y_scale, ROWS_F)
                    ..=cell(bounds.max_y, min_y, y_scale, ROWS_F),
            )
        };
        let mut offsets = vec![0_usize; COLUMNS * ROWS + 1];
        for (bounds, _) in boxes.iter().filter(|(bounds, _)| !is_empty(bounds)) {
            let (columns, rows) = span(bounds);
            for row in rows {
                for column in columns.clone() {
                    offsets[row * COLUMNS + column + 1] += 1;
                }
            }
        }
        for slot in 1..offsets.len() {
            offsets[slot] += offsets[slot - 1];
        }
        let mut fill = offsets.clone();
        let mut entries = vec![0_usize; offsets[offsets.len() - 1]];
        for (index, (bounds, _)) in boxes.iter().enumerate() {
            if is_empty(bounds) {
                continue;
            }
            let (columns, rows) = span(bounds);
            for row in rows {
                for column in columns.clone() {
                    let slot = &mut fill[row * COLUMNS + column];
                    entries[*slot] = index;
                    *slot += 1;
                }
            }
        }
        Self {
            min_x,
            min_y,
            x_scale,
            y_scale,
            offsets,
            entries,
            any_lon_lat: boxes.iter().any(|(_, order)| order.lon_lat),
            any_lat_lon: boxes.iter().any(|(_, order)| order.lat_lon),
        }
    }

    /// The candidates that may cover `(x, y)`, or `None` when the grid cannot tell and the whole
    /// list must be scanned: a NaN, or a longitude the box test would normalise for a box read
    /// as longitude and latitude.
    pub(super) fn candidates(&self, x: f64, y: f64) -> Option<&[usize]> {
        if x.is_nan() || y.is_nan() {
            return None;
        }
        if (self.any_lon_lat && normalize_longitude(x) != x)
            || (self.any_lat_lon && normalize_longitude(y) != y)
        {
            return None;
        }
        let column = cell(x, self.min_x, self.x_scale, COLUMNS_F);
        let row = cell(y, self.min_y, self.y_scale, ROWS_F);
        let slot = row * COLUMNS + column;
        Some(&self.entries[self.offsets[slot]..self.offsets[slot + 1]])
    }
}
