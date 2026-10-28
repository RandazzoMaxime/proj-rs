//! `+proj=unitconvert`, `+proj=noop`, `+proj=push`, `+proj=pop`, and `+proj=set`.
//!
//! Ports of PROJ 9.9 `src/conversions/unitconvert.cpp`, `noop.cpp`, `set.cpp`,
//! and the push/pop operations in `src/pipeline.cpp`. Unit names and factors
//! are `src/units.cpp` (`pj_units`, then `pj_angular_units`).

#![allow(
    clippy::pedantic,
    reason = "unit factors and the time-unit loops follow unitconvert.cpp"
)]

use crate::angle::adjlon;
use crate::engine::{
    proj_number, Coord, CreateError, Frame, IoKind, Params, PipelineStack, Step, TransformError,
};

const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;
const GRAD_TO_RAD: f64 = 0.015_707_963_267_948_967;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Horizontal {
    Whatever,
    Radians,
    Degrees,
}

impl Horizontal {
    fn kind(self) -> IoKind {
        match self {
            Self::Whatever => IoKind::Any,
            Self::Radians => IoKind::Angular,
            Self::Degrees => IoKind::Degrees,
        }
    }
}

struct Resolved {
    factor: f64,
    linear: Option<bool>,
    horizontal: Horizontal,
}

struct NamedUnit {
    id: &'static str,
    factor: f64,
    linear: bool,
    horizontal: Horizontal,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "unitconvert" => Some(&["xy_in", "xy_out", "z_in", "z_out", "t_in", "t_out"]),
        "noop" => Some(&[]),
        "push" | "pop" | "set" => Some(&["v_1", "v_2", "v_3", "v_4"]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    match name {
        "unitconvert" => Ok(Some(Box::new(UnitConvert::create(params, frame)?))),
        "noop" => Ok(Some(Box::new(Noop))),
        "set" => Ok(Some(Box::new(Set::create(params)?))),
        "push" => Ok(Some(Box::new(PushPop::create(params, Role::Push)?))),
        "pop" => Ok(Some(Box::new(PushPop::create(params, Role::Pop)?))),
        _ => Ok(None),
    }
}

fn named_units() -> &'static [NamedUnit] {
    const UNITS: &[NamedUnit] = &[
        NamedUnit {
            id: "km",
            factor: 1000.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "m",
            factor: 1.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "dm",
            factor: 0.1,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "cm",
            factor: 0.01,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "mm",
            factor: 0.001,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "kmi",
            factor: 1852.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "in",
            factor: 0.0254,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "ft",
            factor: 0.3048,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "yd",
            factor: 0.9144,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "mi",
            factor: 1609.344,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "fath",
            factor: 1.8288,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "ch",
            factor: 20.1168,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "link",
            factor: 0.201_168,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "us-in",
            factor: 100.0 / 3937.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "us-ft",
            factor: 1200.0 / 3937.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "us-yd",
            factor: 3600.0 / 3937.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "us-ch",
            factor: 79200.0 / 3937.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "us-mi",
            factor: 6_336_000.0 / 3937.0,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "ind-yd",
            factor: 0.914_395_23,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "ind-ft",
            factor: 0.304_798_41,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "ind-ch",
            factor: 20.116_695_06,
            linear: true,
            horizontal: Horizontal::Whatever,
        },
        NamedUnit {
            id: "rad",
            factor: 1.0,
            linear: false,
            horizontal: Horizontal::Radians,
        },
        NamedUnit {
            id: "deg",
            factor: DEG_TO_RAD,
            linear: false,
            horizontal: Horizontal::Degrees,
        },
        NamedUnit {
            id: "grad",
            factor: GRAD_TO_RAD,
            linear: false,
            horizontal: Horizontal::Whatever,
        },
    ];
    UNITS
}

fn resolve_unit(text: &str, key: &'static str) -> Result<Resolved, CreateError> {
    if let Some(unit) = named_units().iter().find(|unit| unit.id == text) {
        return Ok(Resolved {
            factor: unit.factor,
            linear: Some(unit.linear),
            horizontal: unit.horizontal,
        });
    }
    let Some(factor) = proj_number(text) else {
        return Err(CreateError::IllegalValue(key));
    };
    // `1/f == 0` is how unitconvert.cpp rejects an infinite scale (`1e400`).
    if factor == 0.0 || 1.0 / factor == 0.0 {
        return Err(CreateError::IllegalValue(key));
    }
    Ok(Resolved {
        factor,
        linear: None,
        horizontal: Horizontal::Whatever,
    })
}

fn take_scale(params: &Params, key: &'static str) -> Result<Option<Resolved>, CreateError> {
    if !params.flag(key) {
        return Ok(None);
    }
    // `+xy_in` with no text is an empty string. `pj_atof("")` is 0, which is rejected.
    let text = params.text(key).unwrap_or("");
    Ok(Some(resolve_unit(text, key)?))
}

fn same_family(
    left: Option<bool>,
    right: Option<bool>,
    key: &'static str,
) -> Result<(), CreateError> {
    if let (Some(left), Some(right)) = (left, right) {
        if left != right {
            return Err(CreateError::IllegalValue(key));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeUnit {
    Mjd,
    DecimalYear,
    GpsWeek,
    Yyyymmdd,
}

impl TimeUnit {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "mjd" => Some(Self::Mjd),
            "decimalyear" => Some(Self::DecimalYear),
            "gps_week" => Some(Self::GpsWeek),
            "yyyymmdd" => Some(Self::Yyyymmdd),
            _ => None,
        }
    }

    fn to_mjd(self, value: f64) -> f64 {
        match self {
            Self::Mjd => value,
            Self::DecimalYear => decimalyear_to_mjd(value),
            Self::GpsWeek => 44244.0 + value * 7.0,
            Self::Yyyymmdd => yyyymmdd_to_mjd(value),
        }
    }

    fn leave_mjd(self, value: f64) -> f64 {
        match self {
            Self::Mjd => value,
            Self::DecimalYear => mjd_to_decimalyear(value),
            Self::GpsWeek => (value - 44244.0) / 7.0,
            Self::Yyyymmdd => mjd_to_yyyymmdd(value),
        }
    }
}

fn parse_time(params: &Params, key: &'static str) -> Result<Option<TimeUnit>, CreateError> {
    let Some(text) = params.text(key) else {
        if params.flag(key) {
            return Err(CreateError::IllegalValue(key));
        }
        return Ok(None);
    };
    TimeUnit::parse(text)
        .map(Some)
        .ok_or(CreateError::IllegalValue(key))
}

struct UnitConvert {
    xy_factor: f64,
    z_factor: f64,
    t_in: Option<TimeUnit>,
    t_out: Option<TimeUnit>,
    left: Horizontal,
    right: Horizontal,
    lam0: f64,
    from_greenwich: f64,
    z0: f64,
    vfr_meter: f64,
    over: bool,
    a: f64,
    es: f64,
}

impl UnitConvert {
    fn create(params: &Params, frame: &Frame) -> Result<Self, CreateError> {
        let mut xy_factor = 1.0;
        let mut z_factor = 1.0;
        let mut xy_in_linear = None;
        let mut xy_out_linear = None;
        let mut z_in_linear = None;
        let mut z_out_linear = None;
        let mut left = Horizontal::Whatever;
        let mut right = Horizontal::Whatever;
        if let Some(unit) = take_scale(params, "xy_in")? {
            xy_factor = unit.factor;
            xy_in_linear = unit.linear;
            left = unit.horizontal;
        }
        if let Some(unit) = take_scale(params, "xy_out")? {
            xy_factor /= unit.factor;
            xy_out_linear = unit.linear;
            right = unit.horizontal;
        }
        same_family(xy_in_linear, xy_out_linear, "xy_in")?;
        if let Some(unit) = take_scale(params, "z_in")? {
            z_factor = unit.factor;
            z_in_linear = unit.linear;
        }
        if let Some(unit) = take_scale(params, "z_out")? {
            z_factor /= unit.factor;
            z_out_linear = unit.linear;
        }
        same_family(z_in_linear, z_out_linear, "z_in")?;
        Ok(Self {
            xy_factor,
            z_factor,
            t_in: parse_time(params, "t_in")?,
            t_out: parse_time(params, "t_out")?,
            left,
            right,
            lam0: frame.lam0,
            from_greenwich: frame.from_greenwich,
            z0: frame.z0,
            vfr_meter: frame.vfr_meter,
            over: frame.over,
            a: frame.earth.a(),
            es: frame.earth.es(),
        })
    }

    fn convert(&self, mut coord: Coord, inverse: Direction) -> Coord {
        let (xy_factor, z_factor) = match inverse {
            Direction::Forward => (self.xy_factor, self.z_factor),
            Direction::Inverse => (1.0 / self.xy_factor, 1.0 / self.z_factor),
        };
        coord.x *= xy_factor;
        coord.y *= xy_factor;
        coord.z *= z_factor;
        coord.t = match inverse {
            Direction::Forward => convert_time(coord.t, self.t_in, self.t_out),
            Direction::Inverse => convert_time_inverse(coord.t, self.t_in, self.t_out),
        };
        coord
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Inverse,
}

fn convert_time(mut time: f64, t_in: Option<TimeUnit>, t_out: Option<TimeUnit>) -> f64 {
    if let Some(unit) = t_in {
        time = unit.to_mjd(time);
    }
    if let Some(unit) = t_out {
        time = unit.leave_mjd(time);
    }
    time
}

fn convert_time_inverse(mut time: f64, t_in: Option<TimeUnit>, t_out: Option<TimeUnit>) -> f64 {
    if let Some(unit) = t_out {
        time = unit.to_mjd(time);
    }
    if let Some(unit) = t_in {
        time = unit.leave_mjd(time);
    }
    time
}

impl Step for UnitConvert {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let mut coord = self.convert(coord, Direction::Forward);
        // `skip_fwd_prepare` is set, but radian output still runs `fwd_finalize`.
        if self.right == Horizontal::Radians {
            coord.z = self.vfr_meter * (coord.z + self.z0);
        }
        Ok(coord)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let mut coord = self.convert(coord, Direction::Inverse);
        // Prepare is skipped, so forward never subtracted `lam0`. Inverse
        // finalize still adds it when the inverse output is radians.
        if self.left == Horizontal::Radians {
            coord.x += self.from_greenwich + self.lam0;
            if !self.over {
                coord.x = adjlon(coord.x);
            }
        }
        Ok(coord)
    }

    fn input_kind(&self) -> IoKind {
        self.left.kind()
    }

    fn output_kind(&self) -> IoKind {
        self.right.kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

struct Noop;

impl Step for Noop {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(coord)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(coord)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (1.0, 0.0)
    }
}

struct Set {
    values: [Option<f64>; 4],
}

impl Set {
    fn create(params: &Params) -> Result<Self, CreateError> {
        Ok(Self {
            values: [
                optional_number(params, "v_1")?,
                optional_number(params, "v_2")?,
                optional_number(params, "v_3")?,
                optional_number(params, "v_4")?,
            ],
        })
    }

    fn apply(&self, coord: Coord) -> Coord {
        Coord {
            x: self.values[0].unwrap_or(coord.x),
            y: self.values[1].unwrap_or(coord.y),
            z: self.values[2].unwrap_or(coord.z),
            t: self.values[3].unwrap_or(coord.t),
        }
    }
}

fn optional_number(params: &Params, key: &'static str) -> Result<Option<f64>, CreateError> {
    if !params.flag(key) {
        return Ok(None);
    }
    // `+v_1` with no value: `pj_atof` of the empty tail is 0.
    Ok(Some(params.number(key)?.unwrap_or(0.0)))
}

impl Step for Set {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.apply(coord))
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.apply(coord))
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (1.0, 0.0)
    }
}

#[derive(Clone, Copy)]
enum Role {
    Push,
    Pop,
}

enum StackAct {
    Push,
    Pop,
}

struct PushPop {
    axes: [bool; 4],
    role: Role,
}

impl PushPop {
    fn create(params: &Params, role: Role) -> Result<Self, CreateError> {
        Ok(Self {
            axes: [
                params.flag("v_1"),
                params.flag("v_2"),
                params.flag("v_3"),
                params.flag("v_4"),
            ],
            role,
        })
    }

    fn act(role: Role, direction: Direction) -> StackAct {
        match (role, direction) {
            (Role::Push, Direction::Forward) | (Role::Pop, Direction::Inverse) => StackAct::Push,
            (Role::Push, Direction::Inverse) | (Role::Pop, Direction::Forward) => StackAct::Pop,
        }
    }
}

fn apply_stack(axes: [bool; 4], coord: Coord, stack: &mut PipelineStack, act: StackAct) -> Coord {
    let mut values = [coord.x, coord.y, coord.z, coord.t];
    for (index, selected) in axes.iter().enumerate() {
        if !selected {
            continue;
        }
        match act {
            StackAct::Push => stack.push_axis(index, values[index]),
            StackAct::Pop => {
                if let Some(value) = stack.pop_axis(index) {
                    values[index] = value;
                }
            }
        }
    }
    Coord {
        x: values[0],
        y: values[1],
        z: values[2],
        t: values[3],
    }
}

impl Step for PushPop {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(coord)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(coord)
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (1.0, 0.0)
    }

    fn needs_pipeline_stack(&self) -> bool {
        self.axes.iter().any(|selected| *selected)
    }

    fn forward_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        Ok(apply_stack(
            self.axes,
            coord,
            stack,
            PushPop::act(self.role, Direction::Forward),
        ))
    }

    fn inverse_in_pipeline(
        &self,
        coord: Coord,
        stack: &mut PipelineStack,
    ) -> Result<Coord, TransformError> {
        Ok(apply_stack(
            self.axes,
            coord,
            stack,
            PushPop::act(self.role, Direction::Inverse),
        ))
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_year(year: i32) -> i32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

fn clamped_month(month: i32) -> i32 {
    if month < 0 || month > 12 {
        12
    } else if month == 0 {
        1
    } else {
        month
    }
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match clamped_month(month) {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn daynumber_in_year(year: i32, month: i32, day: i32) -> i32 {
    let month = clamped_month(month);
    let dim = days_in_month(year, month);
    let day = if day < 0 || day > dim { dim } else { day };
    let mut daynumber = 0i32;
    let mut cursor = 1i32;
    while cursor < month {
        daynumber += days_in_month(year, cursor);
        cursor += 1;
    }
    daynumber + day
}

fn floor_i32(value: f64) -> i32 {
    let floored = value.floor();
    if !floored.is_finite() || floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "finite value was range-checked against i32"
    )]
    {
        floored as i32
    }
}

fn lround_i32(value: f64) -> i32 {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "lround result was range-checked against i32, matching the (int) cast"
    )]
    {
        rounded as i32
    }
}

fn decimalyear_to_mjd(decimalyear: f64) -> f64 {
    // NaN fails the comparison and returns 0, as in unitconvert.cpp.
    if !(decimalyear >= -10000.0 && decimalyear <= 10000.0) {
        return 0.0;
    }
    let mut year = floor_i32(decimalyear);
    let fractional = decimalyear - f64::from(year);
    let mut mjd = f64::from(year - 1859) * 365.0 + 14.0 + 31.0;
    mjd += fractional * f64::from(days_in_year(year));
    year -= 1;
    while year > 1858 {
        if is_leap_year(year) {
            mjd += 1.0;
        }
        year -= 1;
    }
    mjd
}

fn mjd_to_decimalyear(mjd: f64) -> f64 {
    // The C++ walk is one year per step. Refuse a non-finite or huge value
    // instead of looping until `year` overflows.
    if !mjd.is_finite() || !(-1.0e8..=1.0e8).contains(&mjd) {
        return 0.0;
    }
    let mut year: i32 = 1859;
    let mut mjd_iter = f64::from(14 + 31);
    while mjd >= mjd_iter {
        if year >= 300_000 {
            return 0.0;
        }
        mjd_iter += f64::from(days_in_year(year));
        year += 1;
    }
    year -= 1;
    mjd_iter -= f64::from(days_in_year(year));
    f64::from(year) + (mjd - mjd_iter) / f64::from(days_in_year(year))
}

fn yyyymmdd_to_mjd(yyyymmdd: f64) -> f64 {
    if !yyyymmdd.is_finite() {
        return 0.0;
    }
    let year = floor_i32(yyyymmdd / 10000.0);
    let month = floor_i32((yyyymmdd - f64::from(year) * 10000.0) / 100.0);
    let day = floor_i32(yyyymmdd - f64::from(year) * 10000.0 - f64::from(month) * 100.0);
    if !(-10_000..=1_000_000).contains(&year) {
        return 0.0;
    }
    let mut mjd = f64::from(daynumber_in_year(year, month, day));
    let mut walking = year - 1;
    let mut guard = 0i32;
    while walking > 1858 {
        mjd += f64::from(days_in_year(walking));
        walking -= 1;
        guard += 1;
        if guard > 200_000 {
            return 0.0;
        }
    }
    mjd + 13.0 + 31.0
}

fn mjd_to_yyyymmdd(mjd: f64) -> f64 {
    if !mjd.is_finite() {
        return 0.0;
    }
    let date = lround_i32(mjd);
    // PROJ casts through `unsigned int`. Negative MJD is outside the oracle.
    if date < 0 {
        return 0.0;
    }
    let mut date_iter: i32 = 14 + 31;
    let mut year: i32 = 1859;
    while date >= date_iter {
        if year >= 300_000 {
            return 0.0;
        }
        date_iter += days_in_year(year);
        year += 1;
    }
    year -= 1;
    date_iter -= days_in_year(year);
    let mut month = 1i32;
    while date_iter + days_in_month(year, month) <= date {
        date_iter += days_in_month(year, month);
        month += 1;
    }
    let day = date - date_iter + 1;
    f64::from(year) * 10000.0 + f64::from(month) * 100.0 + f64::from(day)
}

#[cfg(test)]
mod tests {
    use super::DEG_TO_RAD;
    use crate::engine::Coord;
    use crate::{CreateError, Operation};

    fn xyzt(x: f64, y: f64, z: f64, t: f64) -> Coord {
        Coord { x, y, z, t }
    }

    #[test]
    fn unitconvert_lengths_and_angles() {
        let scaled = Operation::create("+proj=unitconvert +xy_in=m +xy_out=dm +z_in=cm +z_out=mm")
            .expect("lengths");
        let got = scaled
            .forward(xyzt(55.25, 23.23, 45.5, 0.0))
            .expect("forward");
        assert!((got.x - 552.5).abs() < 1e-9);
        assert!((got.y - 232.3).abs() < 1e-9);
        assert!((got.z - 455.0).abs() < 1e-9);

        let ratio = Operation::create("+proj=unitconvert +xy_in=2.0 +xy_out=4.0").expect("ratio");
        let got = ratio.forward(xyzt(1.0, 1.0, 1.0, 1.0)).expect("forward");
        assert!((got.x - 0.5).abs() < 1e-15);
        assert!((got.z - 1.0).abs() < 1e-15);

        let angular =
            Operation::create("+proj=unitconvert +xy_in=deg +xy_out=rad").expect("deg to rad");
        assert!(!angular.angular_input(false));
        assert!(angular.angular_output(false));
        let got = angular.forward(xyzt(1.0, 1.0, 1.0, 1.0)).expect("forward");
        assert!((got.x - DEG_TO_RAD).abs() < 1e-18);
        assert!((got.y - DEG_TO_RAD).abs() < 1e-18);
        assert!((got.z - 1.0).abs() < 1e-15);

        let grad = Operation::create("+proj=unitconvert +xy_in=grad +xy_out=deg").expect("grad");
        assert!(!grad.angular_output(false));
        let got = grad.forward(xyzt(50.0, 50.0, 1.0, 1.0)).expect("forward");
        assert!((got.x - 45.0).abs() < 1e-12);
        assert!((got.y - 45.0).abs() < 1e-12);
    }

    #[test]
    fn unitconvert_rejects_bad_scales() {
        for definition in [
            "+proj=unitconvert +xy_in=m +xy_out=rad",
            "+proj=unitconvert +z_in=rad +z_out=m",
            "+proj=unitconvert +xy_in=0",
            "+proj=unitconvert +xy_out=0",
            "+proj=unitconvert +xy_in=1e400",
            "+proj=unitconvert +xy_out=1e400",
            "+proj=unitconvert +z_in=0",
            "+proj=unitconvert +z_out=0",
            "+proj=unitconvert +z_in=1e400",
            "+proj=unitconvert +z_out=1e400",
            "+proj=unitconvert +t_in=fortnight",
        ] {
            assert!(
                matches!(
                    Operation::create(definition),
                    Err(CreateError::IllegalValue(_))
                ),
                "{definition}"
            );
        }
        assert!(matches!(
            Operation::create("+proj=unitconvert +xy_in=m +not_a_key=1"),
            Err(CreateError::UnknownParameter(_))
        ));
    }

    #[test]
    fn time_units_use_modified_julian_date() {
        let op = Operation::create("+proj=unitconvert +t_in=yyyymmdd +t_out=mjd").expect("time");
        let got = op
            .forward(xyzt(1.0, 2.0, 3.0, 19_800_106.0))
            .expect("forward");
        assert!((got.t - 44244.0).abs() < 1e-9);
        assert!((got.x - 1.0).abs() < 1e-15);
        let back = op.inverse(got).expect("inverse");
        assert!((back.t - 19_800_106.0).abs() < 1e-6);

        let gps = Operation::create("+proj=unitconvert +t_in=gps_week +t_out=mjd").expect("gps");
        let got = gps.forward(xyzt(0.0, 0.0, 0.0, 0.0)).expect("forward");
        assert!((got.t - 44244.0).abs() < 1e-9);

        let year = Operation::create("+proj=unitconvert +t_in=decimalyear +t_out=decimalyear")
            .expect("year");
        let got = year.forward(xyzt(0.0, 0.0, 0.0, 2020.5)).expect("forward");
        assert!((got.t - 2020.5).abs() < 1e-9);
    }

    #[test]
    fn noop_set_and_standalone_stack_are_identities() {
        let noop = Operation::create("+proj=noop").expect("noop");
        let got = noop.forward(xyzt(25.0, 25.0, 25.0, 25.0)).expect("forward");
        assert!((got.x - 25.0).abs() < 1e-15 && (got.t - 25.0).abs() < 1e-15);

        let set = Operation::create("+proj=set +v_1=10 +v_2=20 +v_3=30 +v_4=40").expect("set");
        let got = set.inverse(xyzt(1.0, 2.0, 3.0, 4.0)).expect("inverse");
        assert!((got.x - 10.0).abs() < 1e-15);
        assert!((got.y - 20.0).abs() < 1e-15);
        assert!((got.z - 30.0).abs() < 1e-15);
        assert!((got.t - 40.0).abs() < 1e-15);

        let bare = Operation::create("+proj=set +v_1").expect("bare");
        let got = bare.forward(xyzt(1.0, 2.0, 3.0, 4.0)).expect("forward");
        assert!(got.x.abs() < 1e-15);
        assert!((got.y - 2.0).abs() < 1e-15);

        let push = Operation::create("+proj=push +v_3").expect("push");
        let pop = Operation::create("+proj=pop +v_3").expect("pop");
        let point = xyzt(12.0, 56.0, 0.0, 0.0);
        let pushed = push.forward(point).expect("push");
        let popped = pop.forward(point).expect("pop");
        assert!((pushed.z - point.z).abs() < 1e-15);
        assert!((popped.x - point.x).abs() < 1e-15);
    }

    #[test]
    fn pipeline_push_restores_longitude_across_utm() {
        let restored = Operation::create(
            "+proj=pipeline +step +proj=push +v_1 +step +proj=utm +zone=32 +step +proj=utm +zone=33 +inv +step +proj=pop +v_1",
        )
        .expect("restore");
        let input = xyzt(12.0_f64.to_radians(), 56.0_f64.to_radians(), 0.0, 2020.0);
        let got = restored.forward(input).expect("forward");
        assert!(
            (got.x.to_degrees() - 12.0).abs() < 1e-8,
            "{}",
            got.x.to_degrees()
        );
        assert!(
            (got.y.to_degrees() - 56.0).abs() < 1e-8,
            "{}",
            got.y.to_degrees()
        );
        assert!((got.t - 2020.0).abs() < 1e-9);
        let again = restored.forward(input).expect("second");
        assert!((again.x - got.x).abs() < 1e-9);

        let shifted = Operation::create(
            "+proj=pipeline +step +proj=push +v_1 +step +proj=utm +zone=32 +step +proj=utm +zone=33 +inv",
        )
        .expect("shift");
        let got = shifted.forward(input).expect("forward");
        assert!(
            (got.x.to_degrees() - 18.0).abs() < 1e-6,
            "{}",
            got.x.to_degrees()
        );
        assert!((got.y.to_degrees() - 56.0).abs() < 1e-8);

        let empty = Operation::create(
            "+proj=pipeline +step +proj=utm +zone=32 +step +proj=utm +zone=33 +inv +step +proj=pop +v_1",
        )
        .expect("empty");
        let got = empty.forward(input).expect("forward");
        assert!(
            (got.x.to_degrees() - 18.0).abs() < 1e-6,
            "{}",
            got.x.to_degrees()
        );

        let nested = Operation::create(
            "+proj=pipeline +step +proj=push +v_1 +step +proj=utm +zone=32 +step +proj=push +v_1 +step +proj=utm +zone=33 +inv +step +proj=utm +zone=34 +step +proj=pop +v_1 +step +proj=utm +zone=32 +inv +step +proj=pop +v_1",
        )
        .expect("nested");
        let got = nested.forward(input).expect("forward");
        assert!(
            (got.x.to_degrees() - 12.0).abs() < 1e-7,
            "{}",
            got.x.to_degrees()
        );
        assert!(
            (got.y.to_degrees() - 56.0).abs() < 1e-8,
            "{}",
            got.y.to_degrees()
        );
    }

    #[test]
    fn merc_pipeline_converts_metres_to_kilometres() {
        let op = Operation::create(
            "+proj=pipeline +step +proj=merc +ellps=WGS84 +step +proj=unitconvert +xy_in=m +xy_out=km",
        )
        .expect("merc");
        assert!(op.angular_input(false));
        assert!(!op.angular_output(false));
        let got = op
            .forward(xyzt(12.0_f64.to_radians(), 56.0_f64.to_radians(), 0.0, 0.0))
            .expect("forward");
        let dx = got.x - 1335.8339;
        let dy = got.y - 7522.963;
        assert!(dx.hypot(dy) < 5e-4, "{got:?}");
    }
}
