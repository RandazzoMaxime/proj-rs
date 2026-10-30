//! Bounds of a transformed box, as PROJ computes them in `trans_bounds.cpp`.
//!
//! The edges of the box are densified and interior rows are sampled, then the
//! extremes are taken. A point with no image is infinite on every axis, as
//! PROJ's `HUGE_VAL`. Extremes skip it, except that the minimum keeps it when
//! nothing else is finite.

use super::{Body, Direction, Proj, ProjError, Report};

const HUGE: f64 = f64::INFINITY;

/// Whether the box is walked in two or three dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dims {
    Two,
    Three,
}

/// The transform in one direction, as the box walk needs it.
#[derive(Clone, Copy, Debug)]
struct Sides {
    degree_input: bool,
    degree_output: bool,
    input_order: Order,
    output_order: Order,
    input_geocentric: bool,
}

/// First axis of a geographic output or input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    LonLat,
    LatLon,
}

type Map<'a> = dyn Fn(f64, f64, f64) -> (f64, f64, f64) + 'a;

const FORWARD_LON_LAT: &str = "+step +proj=unitconvert +xy_in=rad +xy_out=deg";
const FORWARD_LAT_LON: &str =
    "+step +proj=unitconvert +xy_in=rad +xy_out=deg +step +proj=axisswap +order=2,1";
const INVERSE_LON_LAT: &str = "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad";
const INVERSE_LAT_LON: &str =
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=unitconvert +xy_in=deg +xy_out=rad";

impl Proj {
    /// PROJ's `proj_trans_bounds` and `proj_trans_bounds_3D` for this object.
    pub(super) fn bounds(
        &self,
        direction: Direction,
        min: [f64; 3],
        max: [f64; 3],
        densify_pts: i32,
        dims: Dims,
    ) -> Result<([f64; 3], [f64; 3]), ProjError> {
        if !(0..=10_000).contains(&densify_pts) {
            return Err(ProjError::Conversion(
                "densify_pts must be between 0-10000.".to_owned(),
            ));
        }
        if matches!(self.body, Body::Single(_)) {
            return Err(ProjError::Conversion(
                "NULL transformation not allowed.".to_owned(),
            ));
        }
        if self.is_noop() {
            return Ok((min, max));
        }
        let densify = usize::try_from(densify_pts)
            .map_err(|_| ProjError::Conversion("densify_pts is negative".to_owned()))?;
        let opposite = direction.opposite();
        let degree_output = self.degree_output(direction);
        let degree_input = self.degree_output(opposite);
        if degree_output && densify < 2 {
            return Err(ProjError::Conversion(
                "densify_pts must be at least 2 if the output is geographic.".to_owned(),
            ));
        }
        let sides = Sides {
            degree_input,
            degree_output,
            input_order: if degree_input {
                self.axis_order(opposite)?
            } else {
                Order::LonLat
            },
            output_order: if degree_output {
                self.axis_order(direction)?
            } else {
                Order::LonLat
            },
            input_geocentric: dims == Dims::Three && self.input_is_geocentric(direction),
        };
        let forward = |x: f64, y: f64, z: f64| self.map_or_inf(x, y, z, direction);
        let backward = |x: f64, y: f64, z: f64| self.map_or_inf(x, y, z, opposite);
        walk(&forward, &backward, sides, min, max, densify, dims)
    }

    /// `true` when the transform does nothing. PROJ answers the input unchanged.
    fn is_noop(&self) -> bool {
        match &self.body {
            Body::Native(_) => self.definition.trim() == "+proj=noop",
            Body::Between { bridge, .. } => {
                bridge.is_none()
                    && self
                        .definition
                        .split_once(" -> ")
                        .is_some_and(|(from, to)| from.trim() == to.trim())
            }
            Body::Single(_) => false,
        }
    }

    /// Whether `direction` writes degrees. The output is the target of a
    /// forward transform and the source of an inverse one.
    fn degree_output(&self, direction: Direction) -> bool {
        match &self.body {
            Body::Native(op) => op.degree_output(direction == Direction::Inverse),
            Body::Between { source, target, .. } => match direction {
                Direction::Forward => target.is_geographic(),
                Direction::Inverse => source.is_geographic(),
            },
            Body::Single(_) => false,
        }
    }

    fn input_is_geocentric(&self, direction: Direction) -> bool {
        match &self.body {
            Body::Between { source, target, .. } => match direction {
                Direction::Forward => source.is_geocentric(),
                Direction::Inverse => target.is_geocentric(),
            },
            Body::Native(_) | Body::Single(_) => false,
        }
    }

    /// Whether the output of `direction` has longitude first.
    ///
    /// A CRS pair is always longitude, latitude here. A bare PROJ pipeline has
    /// no CRS, so its order comes from the last axis swap in its text, as
    /// PROJ reads it when no target CRS is attached.
    fn axis_order(&self, direction: Direction) -> Result<Order, ProjError> {
        match &self.body {
            Body::Between { .. } => Ok(Order::LonLat),
            Body::Native(_) => {
                let text = self
                    .definition
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let order = match direction {
                    Direction::Forward => {
                        if text.ends_with(FORWARD_LON_LAT) {
                            Some(Order::LonLat)
                        } else if text.ends_with(FORWARD_LAT_LON) {
                            Some(Order::LatLon)
                        } else {
                            None
                        }
                    }
                    Direction::Inverse => {
                        if text.starts_with(INVERSE_LON_LAT) {
                            Some(Order::LonLat)
                        } else if text.starts_with(INVERSE_LAT_LON) {
                            Some(Order::LatLon)
                        } else {
                            None
                        }
                    }
                };
                order.ok_or_else(|| {
                    ProjError::Conversion("Unable to retrieve target CRS".to_owned())
                })
            }
            Body::Single(_) => Err(ProjError::Conversion(
                "NULL transformation not allowed.".to_owned(),
            )),
        }
    }

    fn map_or_inf(&self, x: f64, y: f64, z: f64, direction: Direction) -> (f64, f64, f64) {
        match self.map_xyz(x, y, z, direction, Report::Conversion) {
            Ok(coord) => (coord.x, coord.y, coord.z),
            Err(_) => (HUGE, HUGE, HUGE),
        }
    }
}

fn walk(
    forward: &Map<'_>,
    backward: &Map<'_>,
    sides: Sides,
    min: [f64; 3],
    max: [f64; 3],
    densify: usize,
    dims: Dims,
) -> Result<([f64; 3], [f64; 3]), ProjError> {
    let [xmin, ymin, zmin] = min;
    let [xmax, ymax, zmax] = max;
    let side = densify + 1;
    let side_f = index_f64(side);
    let delta_x = if sides.degree_input && xmax < xmin {
        if sides.input_order != Order::LonLat {
            return Err(latitude_error());
        }
        (xmax - xmin + 360.0) / side_f
    } else {
        (xmax - xmin) / side_f
    };
    let delta_y = if sides.degree_input && ymax < ymin {
        if sides.input_order == Order::LonLat {
            return Err(latitude_error());
        }
        (ymax - ymin + 360.0) / side_f
    } else {
        (ymax - ymin) / side_f
    };
    let north = sides.degree_output
        && pole_in_box(backward, sides.output_order, 90.0, [xmin, ymin, xmax, ymax]);
    let south = sides.degree_output
        && pole_in_box(
            backward,
            sides.output_order,
            -90.0,
            [xmin, ymin, xmax, ymax],
        );

    // PROJ starts a 2D box at infinity and a 3D box at the float limits.
    let (start_min, start_max) = match dims {
        Dims::Two => (HUGE, -HUGE),
        Dims::Three => (f64::MAX, f64::MIN),
    };
    let mut out_min = [start_min; 3];
    let mut out_max = [start_max; 3];
    let planes: &[f64] = match dims {
        Dims::Two => &[0.0],
        Dims::Three => &[zmin, zmax],
    };

    if sides.input_geocentric {
        let mut x = Vec::with_capacity(12 * side);
        let mut y = Vec::with_capacity(12 * side);
        let mut z = Vec::with_capacity(12 * side);
        for z0 in [zmin, zmax] {
            for i in 0..side {
                let fi = index_f64(i);
                x.push(xmin);
                y.push(ymax - fi * delta_y);
                z.push(z0);
            }
            for i in 0..side {
                let fi = index_f64(i);
                x.push(xmin + fi * delta_x);
                y.push(ymin);
                z.push(z0);
            }
            for i in 0..side {
                let fi = index_f64(i);
                x.push(xmax);
                y.push(ymin + fi * delta_y);
                z.push(z0);
            }
            for i in 0..side {
                let fi = index_f64(i);
                x.push(xmax - fi * delta_x);
                y.push(ymax);
                z.push(z0);
            }
        }
        let delta_z = (zmax - zmin) / side_f;
        for (xe, ye) in [(xmin, ymin), (xmin, ymax), (xmax, ymin), (xmax, ymax)] {
            for i in 0..side {
                x.push(xe);
                y.push(ye);
                z.push(zmin + index_f64(i) * delta_z);
            }
        }
        map_all(forward, &mut x, &mut y, &mut z);
        for (axis, values) in [&x, &y, &z].into_iter().enumerate() {
            out_min[axis] = cmin(out_min[axis], simple_min(values));
            out_max[axis] = cmax(out_max[axis], simple_max(values));
        }
        return Ok((out_min, out_max));
    }

    for &z0 in planes {
        let mut x = vec![0.0; 4 * side];
        let mut y = vec![0.0; 4 * side];
        let mut z = vec![z0; 4 * side];
        for i in 0..side {
            let fi = index_f64(i);
            y[i] = ymax - fi * delta_y;
            x[i] = xmin;
            y[i + side] = ymin;
            x[i + side] = xmin + fi * delta_x;
            y[i + 2 * side] = ymin + fi * delta_y;
            x[i + 2 * side] = xmax;
            y[i + 3 * side] = ymax;
            x[i + 3 * side] = xmax - fi * delta_x;
        }
        map_all(forward, &mut x, &mut y, &mut z);
        if sides.degree_output && sides.output_order == Order::LatLon {
            std::mem::swap(&mut x, &mut y);
        }

        let mut crossed_antimeridian = false;
        if !sides.degree_output {
            out_min[0] = cmin(out_min[0], simple_min(&x));
            out_max[0] = cmax(out_max[0], simple_max(&x));
            out_min[1] = cmin(out_min[1], simple_min(&y));
            out_max[1] = cmax(out_max[1], simple_max(&y));
        } else if north {
            out_min[0] = -180.0;
            out_max[0] = 180.0;
            out_min[1] = cmin(out_min[1], simple_min(&y));
            out_max[1] = 90.0;
        } else if south {
            out_min[0] = -180.0;
            out_max[0] = 180.0;
            out_min[1] = -90.0;
            out_max[1] = cmax(out_max[1], simple_max(&y));
        } else {
            out_min[0] = cmin(out_min[0], antimeridian_min(&x));
            out_max[0] = cmax(out_max[0], antimeridian_max(&x));
            crossed_antimeridian = out_min[0] > out_max[0];
            out_min[1] = cmin(out_min[1], simple_min(&y));
            out_max[1] = cmax(out_max[1], simple_max(&y));
        }
        out_min[2] = cmin(out_min[2], simple_min(&z));
        out_max[2] = cmax(out_max[2], simple_max(&z));

        if sides.degree_output && sides.output_order == Order::LatLon {
            out_min.swap(0, 1);
            out_max.swap(0, 1);
        }

        if !crossed_antimeridian {
            for j in 1..side - 1 {
                let fj = index_f64(j);
                let mut xs: Vec<f64> = (0..side).map(|i| xmin + index_f64(i) * delta_x).collect();
                let mut ys = vec![ymin + fj * delta_y; side];
                let mut zs = vec![z0; side];
                map_all(forward, &mut xs, &mut ys, &mut zs);
                for i in 0..side {
                    if xs[i].is_finite() && ys[i].is_finite() {
                        out_min[0] = cmin(out_min[0], xs[i]);
                        out_max[0] = cmax(out_max[0], xs[i]);
                        out_min[1] = cmin(out_min[1], ys[i]);
                        out_max[1] = cmax(out_max[1], ys[i]);
                        out_min[2] = cmin(out_min[2], zs[i]);
                        out_max[2] = cmax(out_max[2], zs[i]);
                    }
                }
            }
        }
    }
    Ok((out_min, out_max))
}

fn latitude_error() -> ProjError {
    ProjError::Conversion("latitude max < latitude min.".to_owned())
}

/// Whether the box, in the input, contains the pole `latitude` of the output.
fn pole_in_box(
    backward: &Map<'_>,
    order: Order,
    latitude: f64,
    [xmin, ymin, xmax, ymax]: [f64; 4],
) -> bool {
    let (pole_x, pole_y) = if order == Order::LonLat {
        (0.0, latitude)
    } else {
        (latitude, 0.0)
    };
    let (pole_x, pole_y, _) = backward(pole_x, pole_y, 0.0);
    xmin < pole_x && pole_x < xmax && ymax > pole_y && pole_y > ymin
}

fn map_all(forward: &Map<'_>, x: &mut [f64], y: &mut [f64], z: &mut [f64]) {
    for ((xi, yi), zi) in x.iter_mut().zip(y.iter_mut()).zip(z.iter_mut()) {
        (*xi, *yi, *zi) = forward(*xi, *yi, *zi);
    }
}

/// `std::min(a, b)`: `b` only when it is smaller.
fn cmin(a: f64, b: f64) -> f64 {
    if b < a {
        b
    } else {
        a
    }
}

/// `std::max(a, b)`: `b` only when it is larger.
fn cmax(a: f64, b: f64) -> f64 {
    if a < b {
        b
    } else {
        a
    }
}

fn simple_min(data: &[f64]) -> f64 {
    let mut min_value = data[0];
    for &value in &data[1..] {
        if value < min_value {
            min_value = value;
        }
    }
    min_value
}

fn simple_max(data: &[f64]) -> f64 {
    let mut max_value = data[0];
    for &value in &data[1..] {
        if (value > max_value || max_value == HUGE) && value != HUGE {
            max_value = value;
        }
    }
    max_value
}

/// Nearest index before `index` whose value is not infinite, wrapping around.
fn find_previous_index(index: usize, data: &[f64]) -> usize {
    let last = data.len() - 1;
    let mut previous = if index == 0 { last } else { index - 1 };
    while data[previous] == HUGE && previous != index {
        previous = if previous == 0 { last } else { previous - 1 };
    }
    previous
}

/// Minimum of a closed ring of longitudes, taking the antimeridian into account.
///
/// Two neighbours more than 200° apart mean the ring crossed the antimeridian.
/// Four such jumps mean the ring wraps the globe. See PROJ `antimeridian_min`.
fn antimeridian_min(data: &[f64]) -> f64 {
    let mut positive_min = HUGE;
    let mut min_value = HUGE;
    let mut crossed = 0;
    let mut positive_meridian = false;
    for index in 0..data.len() {
        if data[index] == HUGE {
            continue;
        }
        let previous = find_previous_index(index, data);
        let delta = data[previous] - data[index];
        if delta >= 200.0 && delta != HUGE {
            if crossed == 0 {
                positive_min = min_value;
            }
            crossed += 1;
            positive_meridian = false;
        } else if delta <= -200.0 && delta != HUGE {
            if crossed == 0 {
                positive_min = data[index];
            }
            crossed += 1;
            positive_meridian = true;
        }
        if positive_meridian && data[index] < positive_min {
            positive_min = data[index];
        }
        if data[index] < min_value {
            min_value = data[index];
        }
    }
    if crossed == 2 {
        positive_min
    } else if crossed == 4 {
        -180.0
    } else {
        min_value
    }
}

/// Maximum of a closed ring of longitudes, taking the antimeridian into account.
/// See [`antimeridian_min`]; PROJ `antimeridian_max`.
fn antimeridian_max(data: &[f64]) -> f64 {
    let mut negative_max = -HUGE;
    let mut max_value = -HUGE;
    let mut negative_meridian = false;
    let mut crossed = 0;
    for index in 0..data.len() {
        if data[index] == HUGE {
            continue;
        }
        let previous = find_previous_index(index, data);
        let delta = data[previous] - data[index];
        if delta >= 200.0 && delta != HUGE {
            if crossed == 0 {
                negative_max = data[index];
            }
            crossed += 1;
            negative_meridian = true;
        } else if delta <= -200.0 && delta != HUGE {
            if crossed == 0 {
                negative_max = max_value;
            }
            negative_meridian = false;
            crossed += 1;
        }
        if negative_meridian
            && (data[index] > negative_max || negative_max == HUGE)
            && data[index] != HUGE
        {
            negative_max = data[index];
        }
        if (data[index] > max_value || max_value == HUGE) && data[index] != HUGE {
            max_value = data[index];
        }
    }
    if crossed == 2 {
        negative_max
    } else if crossed == 4 {
        180.0
    } else {
        max_value
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "indices are at most a few tens of thousands, exact in f64"
)]
fn index_f64(index: usize) -> f64 {
    index as f64
}
