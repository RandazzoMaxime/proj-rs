//! The `+grids` of `+proj=deformation`: `pj_generic_grid_init` and
//! `pj_deformation_get_grid_values`. The grids are `GenericShiftGridSet`s, read by tile on demand
//! as PROJ reads them, so a remote grid is not downloaded whole.

use crate::engine::{CreateError, TransformError};
use crate::projections::{GenericShiftGridSet, Grid, GridError};

use super::deformation::{dim, index, trunc_i32};

/// The sets of a `+grids` list. A name with `@` is optional: a missing file is skipped.
pub(super) fn load(text: &str) -> Result<Vec<GenericShiftGridSet>, CreateError> {
    if text.is_empty() {
        return Err(CreateError::MissingArgument("grids"));
    }
    let mut sets = Vec::new();
    for raw in text.split(',') {
        let mut name = raw.trim();
        if name.is_empty() {
            return Err(CreateError::IllegalValue("grids"));
        }
        let optional = name.starts_with('@');
        if optional {
            name = &name[1..];
        }
        match GenericShiftGridSet::open(name) {
            Ok(set) => {
                let thin = set
                    .grids()
                    .iter()
                    .any(|grid| !grid.is_null_grid() && grid.samples_per_pixel() < 3);
                if thin {
                    return Err(CreateError::IllegalValue("grids"));
                }
                sets.push(set);
            }
            Err(GridError::NotFound { .. }) if optional => {}
            Err(GridError::NotFound { .. }) => return Err(CreateError::IllegalValue("file")),
            Err(_) => return Err(CreateError::IllegalValue("grids")),
        }
    }
    if sets.is_empty() {
        return Err(CreateError::IllegalValue("grids"));
    }
    Ok(sets)
}

/// The east, north and up velocities at `(lam, phi)`, in millimetres per year, from the first set
/// with a grid there (`pj_find_generic_grid`). The null grid gives zero.
pub(super) fn sample(
    sets: &[GenericShiftGridSet],
    lam: f64,
    phi: f64,
) -> Result<(f64, f64, f64), TransformError> {
    let grid = sets
        .iter()
        .find_map(|set| set.grid_at(lam, phi))
        .ok_or(TransformError::OutsideGrid)?;
    if grid.is_null_grid() {
        return Ok((0.0, 0.0, 0.0));
    }
    let (east, north, up) = velocity_bands(&grid)?;
    bilinear3(&grid, lam, phi, [east, north, up])
}

fn velocity_bands(grid: &Grid<'_>) -> Result<(usize, usize, usize), TransformError> {
    let (mut east, mut north, mut up) = (0usize, 1usize, 2usize);
    for sample in 0..grid.samples_per_pixel() {
        match grid.description(sample).as_str() {
            "east_velocity" => east = sample,
            "north_velocity" => north = sample,
            "up_velocity" => up = sample,
            _ => {}
        }
    }
    let unit = grid.unit(east);
    if !unit.is_empty() && unit != "millimetres per year" {
        return Err(TransformError::OutsideDomain);
    }
    Ok((east, north, up))
}

/// `pj_bilinear_interpolation_three_samples`.
fn bilinear3(
    grid: &Grid<'_>,
    lam: f64,
    phi: f64,
    bands: [usize; 3],
) -> Result<(f64, f64, f64), TransformError> {
    let extent = grid.extent();
    if !extent.is_geographic {
        return Err(TransformError::OutsideDomain);
    }
    let mut grid_x = (lam - extent.west) / extent.res_x;
    if lam < extent.west {
        grid_x = (lam + 2.0 * std::f64::consts::PI - extent.west) / extent.res_x;
    } else if lam > extent.east {
        grid_x = (lam - 2.0 * std::f64::consts::PI - extent.west) / extent.res_x;
    }
    let grid_y = (phi - extent.south) / extent.res_y;
    let ix = index(trunc_i32(grid_x))?;
    let iy = index(trunc_i32(grid_y))?;
    if ix >= grid.width() || iy >= grid.height() {
        return Err(TransformError::OutsideGrid);
    }
    let ix2 = (ix + 1).min(grid.width() - 1);
    let iy2 = (iy + 1).min(grid.height() - 1);
    let corner = |x, y| -> Result<[f64; 3], TransformError> {
        let mut out = [0.0; 3];
        for (slot, &band) in out.iter_mut().zip(&bands) {
            *slot = grid
                .value_at(x, y, band)
                .map(f64::from)
                .ok_or(TransformError::OutsideGrid)?;
        }
        Ok(out)
    };
    let c00 = corner(ix, iy)?;
    let c10 = corner(ix2, iy)?;
    let c01 = corner(ix, iy2)?;
    let c11 = corner(ix2, iy2)?;
    let frct_x = grid_x - dim(ix);
    let frct_y = grid_y - dim(iy);
    let m10 = frct_x * (1.0 - frct_y);
    let m11 = frct_x * frct_y;
    let m01 = (1.0 - frct_x) * frct_y;
    let m00 = (1.0 - frct_x) * (1.0 - frct_y);
    let mix = |band: usize| m00 * c00[band] + m10 * c10[band] + m01 * c01[band] + m11 * c11[band];
    Ok((mix(0), mix(1), mix(2)))
}
