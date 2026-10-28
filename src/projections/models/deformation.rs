//! Kinematic grid shift (`+proj=deformation`).
//!
//! Cartesian in, cartesian out. Grid values are millimetres per year in
//! east, north, up. The ENU to geocentric rotation and the reverse iteration
//! are `pj_deformation_get_grid_shift` and `pj_deformation_reverse_shift`.

use super::grid::Grid;
use crate::ellipsoid::Ellipsoid;
use crate::engine::{
    cartesian_to_geodetic, Coord, CreateError, Frame, IoKind, Params, Step, TransformError,
};

const TOL: f64 = 1e-8;
const REL: f64 = 1e-5;

enum Grids {
    Generic(Vec<Grid>),
    Split {
        horizontal: Vec<Grid>,
        vertical: Vec<Grid>,
    },
}

struct Deformation {
    grids: Grids,
    dt: Option<f64>,
    t_epoch: Option<f64>,
    earth: Ellipsoid,
}

pub(super) fn keys() -> &'static [&'static str] {
    &["dt", "t_epoch", "t_obs", "grids", "xy_grids", "z_grids"]
}

pub(super) fn build(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    if params.flag("t_obs") {
        return Err(CreateError::MissingArgument("dt"));
    }
    let has_grids = params.flag("grids");
    let has_xy = params.flag("xy_grids");
    let has_z = params.flag("z_grids");
    if !has_grids && !(has_xy && has_z) {
        return Err(CreateError::MissingArgument("grids"));
    }
    let grids = if has_grids {
        Grids::Generic(load_list(params.text("grids").unwrap_or(""), true)?)
    } else {
        Grids::Split {
            horizontal: load_list(params.text("xy_grids").unwrap_or(""), false)?,
            vertical: load_list(params.text("z_grids").unwrap_or(""), false)?,
        }
    };
    let dt = params.number("dt")?;
    let t_epoch = params.number("t_epoch")?;
    if dt.is_none() && t_epoch.is_none() {
        return Err(CreateError::MissingArgument("dt"));
    }
    if dt.is_some() && t_epoch.is_some() {
        return Err(CreateError::IllegalValue("dt"));
    }
    Ok(Box::new(Deformation {
        grids,
        dt,
        t_epoch,
        earth: frame.earth,
    }))
}

fn load_list(text: &str, generic: bool) -> Result<Vec<Grid>, CreateError> {
    let mut grids = Vec::new();
    if text.is_empty() {
        return Err(CreateError::MissingArgument("grids"));
    }
    for raw in text.split(',') {
        let mut name = raw.trim();
        if name.is_empty() {
            return Err(CreateError::IllegalValue("grids"));
        }
        let optional = name.starts_with('@');
        if optional {
            name = &name[1..];
        }
        match super::read_resource(name) {
            Ok(bytes) => {
                let Some(grid) = super::grid::open_named(name, &bytes) else {
                    // A known suffix that is not a shift grid is an illegal file.
                    // A bare name that is not one stays a missing-grid error.
                    return Err(CreateError::IllegalValue("grids"));
                };
                if generic && grid.samples < 3 {
                    return Err(CreateError::IllegalValue("grids"));
                }
                grids.push(grid);
            }
            Err(CreateError::IllegalValue("file") | CreateError::Unsupported(_)) if optional => {}
            Err(error) => return Err(error),
        }
    }
    if grids.is_empty() {
        return Err(CreateError::IllegalValue("grids"));
    }
    Ok(grids)
}

impl Step for Deformation {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let dt = self.delta_t(coord.t)?;
        let shift = self.shift(coord)?;
        Ok(Coord {
            x: coord.x + dt * shift.0,
            y: coord.y + dt * shift.1,
            z: coord.z + dt * shift.2,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let dt = self.delta_t(coord.t)?;
        let shifted = reverse(self, coord, dt)?;
        Ok(Coord {
            x: shifted.0,
            y: shifted.1,
            z: shifted.2,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.earth.a(), self.earth.es())
    }
}

impl Deformation {
    fn delta_t(&self, time: f64) -> Result<f64, TransformError> {
        if let Some(dt) = self.dt {
            Ok(dt)
        } else if !time.is_finite() {
            Err(TransformError::MissingTime)
        } else {
            Ok(time - self.t_epoch.unwrap_or(0.0))
        }
    }

    fn shift(&self, coord: Coord) -> Result<(f64, f64, f64), TransformError> {
        let (lam, phi, _) = cartesian_to_geodetic(self.earth, coord.x, coord.y, coord.z);
        let (east, north, up) = match &self.grids {
            Grids::Generic(grids) => sample_generic(grids, lam, phi)?,
            Grids::Split {
                horizontal,
                vertical,
            } => {
                let (east, north) = sample_horizontal(horizontal, lam, phi)?;
                let up = sample_vertical(vertical, lam, phi)?;
                (east, north, up)
            }
        };
        let east = east / 1000.0;
        let north = north / 1000.0;
        let up = up / 1000.0;
        Ok(enu_to_geocentric(lam, phi, east, north, up))
    }
}

fn reverse(step: &Deformation, input: Coord, dt: f64) -> Result<(f64, f64, f64), TransformError> {
    let delta = step.shift(input)?;
    let z0 = delta.2;
    let mut x = input.x - dt * delta.0;
    let mut y = input.y - dt * delta.1;
    let mut z = input.z + dt * delta.2;
    let mut left = 10i32;
    loop {
        let at = Coord {
            x,
            y,
            z,
            t: input.t,
        };
        let delta = step.shift(at)?;
        let dif_x = x + dt * delta.0 - input.x;
        let dif_y = y + dt * delta.1 - input.y;
        let dif_z = z - dt * delta.2 - input.z;
        x += dif_x;
        y += dif_y;
        z += dif_z;
        left -= 1;
        if left == 0 || dif_x.hypot(dif_y) <= TOL {
            break;
        }
    }
    Ok((x, y, input.z - dt * z0))
}

fn sample_generic(grids: &[Grid], lam: f64, phi: f64) -> Result<(f64, f64, f64), TransformError> {
    let grid = grids
        .iter()
        .find(|grid| grid.contains(lam, phi, 0.0))
        .ok_or(TransformError::OutsideGrid)?;
    let (east_i, north_i, up_i) = velocity_bands(grid)?;
    bilinear3(grid, lam, phi, east_i, north_i, up_i)
}

fn velocity_bands(grid: &Grid) -> Result<(usize, usize, usize), TransformError> {
    let mut east = 0usize;
    let mut north = 1usize;
    let mut up = 2usize;
    for sample in 0..grid.samples {
        match grid.description(sample) {
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

fn bilinear3(
    grid: &Grid,
    lam: f64,
    phi: f64,
    i0: usize,
    i1: usize,
    i2: usize,
) -> Result<(f64, f64, f64), TransformError> {
    if !grid.geographic {
        return Err(TransformError::OutsideDomain);
    }
    let mut grid_x = (lam - grid.west) / grid.res_x;
    if lam < grid.west {
        grid_x = (lam + 2.0 * std::f64::consts::PI - grid.west) / grid.res_x;
    } else if lam > grid.east {
        grid_x = (lam - 2.0 * std::f64::consts::PI - grid.west) / grid.res_x;
    }
    let grid_y = (phi - grid.south) / grid.res_y;
    let ix = trunc_i32(grid_x);
    let iy = trunc_i32(grid_y);
    if ix < 0 || iy < 0 {
        return Err(TransformError::OutsideGrid);
    }
    let ix = index(ix)?;
    let iy = index(iy)?;
    if ix >= grid.width || iy >= grid.height {
        return Err(TransformError::OutsideGrid);
    }
    let ix2 = (ix + 1).min(grid.width - 1);
    let iy2 = (iy + 1).min(grid.height - 1);
    let sample = |x, y, sample| -> Result<f64, TransformError> {
        grid.value(x, y, sample)
            .map(f64::from)
            .ok_or(TransformError::OutsideGrid)
    };
    let corners = [
        (
            sample(ix, iy, i0)?,
            sample(ix, iy, i1)?,
            sample(ix, iy, i2)?,
        ),
        (
            sample(ix2, iy, i0)?,
            sample(ix2, iy, i1)?,
            sample(ix2, iy, i2)?,
        ),
        (
            sample(ix, iy2, i0)?,
            sample(ix, iy2, i1)?,
            sample(ix, iy2, i2)?,
        ),
        (
            sample(ix2, iy2, i0)?,
            sample(ix2, iy2, i1)?,
            sample(ix2, iy2, i2)?,
        ),
    ];
    let frct_x = grid_x - dim(ix);
    let frct_y = grid_y - dim(iy);
    let m10 = frct_x * (1.0 - frct_y);
    let m11 = frct_x * frct_y;
    let m01 = (1.0 - frct_x) * frct_y;
    let m00 = (1.0 - frct_x) * (1.0 - frct_y);
    Ok((
        m00 * corners[0].0 + m10 * corners[1].0 + m01 * corners[2].0 + m11 * corners[3].0,
        m00 * corners[0].1 + m10 * corners[1].1 + m01 * corners[2].1 + m11 * corners[3].1,
        m00 * corners[0].2 + m10 * corners[1].2 + m01 * corners[2].2 + m11 * corners[3].2,
    ))
}

fn sample_horizontal(grids: &[Grid], lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
    let grid = pick(grids, lam, phi)?;
    let eps = (grid.res_x + grid.res_y) * REL;
    let mut x = lam - grid.west;
    if x + eps < 0.0 {
        x += 2.0 * std::f64::consts::PI;
    } else if x - eps > grid.east - grid.west {
        x -= 2.0 * std::f64::consts::PI;
    }
    let y = phi - grid.south;
    let (east_band, north_band, factor) = horizontal_layout(grid);
    let (east, north) = hgrid_interpolate(grid, x, y, east_band, north_band)?;
    Ok((east * factor, north * factor))
}

fn horizontal_layout(grid: &Grid) -> (usize, usize, f64) {
    let mut east = 0usize;
    let mut north = 1usize;
    for sample in 0..grid.samples {
        match grid.description(sample) {
            "longitude_offset" | "east_velocity" | "east" => east = sample,
            "latitude_offset" | "north_velocity" | "north" => north = sample,
            _ => {}
        }
    }
    // `pj_hgrid_value` converts arc-seconds to radians. Millimetres stay raw;
    // the caller divides by 1000 either way.
    let factor = match grid.unit(east) {
        "arc-second" | "arc-seconds per year" => (std::f64::consts::PI / 180.0) / 3600.0,
        "degree" => std::f64::consts::PI / 180.0,
        _ => 1.0,
    };
    (east, north, factor)
}

fn hgrid_interpolate(
    grid: &Grid,
    x: f64,
    y: f64,
    east_band: usize,
    north_band: usize,
) -> Result<(f64, f64), TransformError> {
    let lam = x / grid.res_x;
    let phi = y / grid.res_y;
    let mut ix = trunc_i32(lam.floor());
    let mut iy = trunc_i32(phi.floor());
    let mut frct_x = lam - dim_i(ix);
    let mut frct_y = phi - dim_i(iy);
    if ix < 0 {
        if ix == -1 && frct_x > 1.0 - 10.0 * REL {
            ix = 0;
            frct_x = 0.0;
        } else {
            return Err(TransformError::OutsideGrid);
        }
    } else if ix + 1 >= i32_width(grid.width) {
        if ix + 1 == i32_width(grid.width) && frct_x < 10.0 * REL {
            ix -= 1;
            frct_x = 1.0;
        } else {
            return Err(TransformError::OutsideGrid);
        }
    }
    if iy < 0 {
        if iy == -1 && frct_y > 1.0 - 10.0 * REL {
            iy = 0;
            frct_y = 0.0;
        } else {
            return Err(TransformError::OutsideGrid);
        }
    } else if iy + 1 >= i32_width(grid.height) {
        if iy + 1 == i32_width(grid.height) && frct_y < 10.0 * REL {
            iy -= 1;
            frct_y = 1.0;
        } else {
            return Err(TransformError::OutsideGrid);
        }
    }
    let ix = index(ix)?;
    let iy = index(iy)?;
    let s00 = node(grid, ix, iy, east_band, north_band)?;
    let s10 = node(grid, ix + 1, iy, east_band, north_band)?;
    let s01 = node(grid, ix, iy + 1, east_band, north_band)?;
    let s11 = node(grid, ix + 1, iy + 1, east_band, north_band)?;
    let m10 = frct_x * (1.0 - frct_y);
    let m11 = frct_x * frct_y;
    let m01 = (1.0 - frct_x) * frct_y;
    let m00 = (1.0 - frct_x) * (1.0 - frct_y);
    Ok((
        m00 * s00.0 + m10 * s10.0 + m01 * s01.0 + m11 * s11.0,
        m00 * s00.1 + m10 * s10.1 + m01 * s01.1 + m11 * s11.1,
    ))
}

fn node(
    grid: &Grid,
    x: usize,
    y: usize,
    east_band: usize,
    north_band: usize,
) -> Result<(f64, f64), TransformError> {
    Ok((
        f64::from(
            grid.value(x, y, east_band)
                .ok_or(TransformError::OutsideGrid)?,
        ),
        f64::from(
            grid.value(x, y, north_band)
                .ok_or(TransformError::OutsideGrid)?,
        ),
    ))
}

fn sample_vertical(grids: &[Grid], lam: f64, phi: f64) -> Result<f64, TransformError> {
    let grid = pick(grids, lam, phi)?;
    if !grid.geographic {
        return Err(TransformError::OutsideDomain);
    }
    let mut grid_x = (lam - grid.west) / grid.res_x;
    if lam < grid.west {
        grid_x = (lam + 2.0 * std::f64::consts::PI - grid.west) / grid.res_x;
    } else if lam > grid.east {
        grid_x = (lam - 2.0 * std::f64::consts::PI - grid.west) / grid.res_x;
    }
    let grid_y = (phi - grid.south) / grid.res_y;
    let ix = trunc_i32(grid_x.floor());
    let iy = trunc_i32(grid_y.floor());
    if ix < 0 || iy < 0 || index(ix)? >= grid.width || index(iy)? >= grid.height {
        return Err(TransformError::OutsideGrid);
    }
    let ix = index(ix)?;
    let iy = index(iy)?;
    let ix2 = if ix + 1 >= grid.width {
        grid.width - 1
    } else {
        ix + 1
    };
    let iy2 = if iy + 1 >= grid.height {
        grid.height - 1
    } else {
        iy + 1
    };
    let fx = grid_x - dim(ix);
    let fy = grid_y - dim(iy);
    let a = f64::from(grid.value(ix, iy, 0).ok_or(TransformError::OutsideGrid)?);
    let b = f64::from(grid.value(ix2, iy, 0).ok_or(TransformError::OutsideGrid)?);
    let c = f64::from(grid.value(ix, iy2, 0).ok_or(TransformError::OutsideGrid)?);
    let d = f64::from(grid.value(ix2, iy2, 0).ok_or(TransformError::OutsideGrid)?);
    if nodata(a) && nodata(b) && nodata(c) && nodata(d) {
        return Err(TransformError::OutsideGrid);
    }
    let w00 = (1.0 - fx) * (1.0 - fy);
    let w10 = fx * (1.0 - fy);
    let w01 = (1.0 - fx) * fy;
    let w11 = fx * fy;
    let mut value = 0.0;
    let mut weight = 0.0;
    for (valid, sample, sample_weight) in [
        (!nodata(a), a, w00),
        (!nodata(b), b, w10),
        (!nodata(c), c, w01),
        (!nodata(d), d, w11),
    ] {
        if valid {
            value += sample * sample_weight;
            weight += sample_weight;
        }
    }
    if weight == 0.0 {
        return Err(TransformError::OutsideGrid);
    }
    Ok(value / weight)
}

fn nodata(value: f64) -> bool {
    value > 1000.0 || value < -1000.0 || value == -88.888_80
}

fn pick(grids: &[Grid], lam: f64, phi: f64) -> Result<&Grid, TransformError> {
    grids
        .iter()
        .find(|grid| {
            let eps = (grid.res_x + grid.res_y) * REL;
            grid.contains(lam, phi, eps)
        })
        .ok_or(TransformError::OutsideGrid)
}

fn enu_to_geocentric(lam: f64, phi: f64, east: f64, north: f64, up: f64) -> (f64, f64, f64) {
    let sp = phi.sin();
    let cp = phi.cos();
    let sl = lam.sin();
    let cl = lam.cos();
    (
        -sp * cl * north - sl * east + cp * cl * up,
        -sp * sl * north + cl * east + cp * sl * up,
        cp * north + sp * up,
    )
}

fn index(value: i32) -> Result<usize, TransformError> {
    usize::try_from(value).map_err(|_| TransformError::OutsideGrid)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "grid indices are small; truncation toward zero matches PROJ's C cast"
)]
fn trunc_i32(value: f64) -> i32 {
    value as i32
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "a grid index fits in the f64 mantissa"
)]
fn dim(value: usize) -> f64 {
    value as f64
}

fn dim_i(value: i32) -> f64 {
    f64::from(value)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "grid width fits in i32 for the files this operation reads"
)]
fn i32_width(value: usize) -> i32 {
    value as i32
}
