//! Deformation model (`+proj=defmodel`).
//!
//! The evaluator follows `Evaluator::forward` and `Evaluator::inverse` in
//! `defmodel_impl.hpp`. Time functions, the degree-to-radian grid offsets, the
//! metre-to-longitude conversion and the geocentric bilinear cell are copied
//! from that file. Geographic input and output are radians, matching
//! `PJ_IO_UNITS_RADIANS`.

use std::f64::consts::PI;

use super::grid::Grid;
use super::json;
use crate::ellipsoid::Ellipsoid;
use crate::engine::{
    cartesian_to_geodetic, geodetic_to_cartesian, Coord, CreateError, Frame, IoKind, Params, Step,
    TransformError,
};

const DEG_TO_RAD: f64 = PI / 180.0;

enum TimeFn {
    Constant,
    Velocity {
        reference: f64,
    },
    Step {
        epoch: f64,
    },
    ReverseStep {
        epoch: f64,
    },
    Piecewise {
        before: Bound,
        after: Bound,
        samples: Vec<(f64, f64)>,
    },
    Exponential {
        reference: f64,
        end: Option<f64>,
        relaxation: f64,
        before: f64,
        initial: f64,
        final_scale: f64,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bound {
    Zero,
    Constant,
    Linear,
}

#[derive(Clone, Copy)]
enum Displacement {
    Horizontal,
    Vertical,
    Three,
}

struct Component {
    displacement: Displacement,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    bilinear: bool,
    grid: Grid,
    time: TimeFn,
}

struct Model {
    geographic: bool,
    degree: bool,
    addition: bool,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    t0: f64,
    t1: f64,
    components: Vec<Component>,
    a: f64,
    b: f64,
    es: f64,
    earth: Ellipsoid,
}

pub(super) fn keys() -> &'static [&'static str] {
    &["model"]
}

pub(super) fn build(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let Some(file) = params.text("model") else {
        return Err(CreateError::MissingArgument("model"));
    };
    if file.is_empty() {
        return Err(CreateError::MissingArgument("model"));
    }
    let bytes = super::read_resource(file)?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(CreateError::IllegalValue("model"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| CreateError::IllegalValue("model"))?;
    let model = parse_model(text, frame).ok_or(CreateError::IllegalValue("model"))?;
    Ok(Box::new(model))
}

fn parse_model(text: &str, frame: &Frame) -> Option<Model> {
    let root = json::parse(text)?.as_object()?.clone();
    if json::req_str(&root, "file_type")? != "deformation_model_master_file" {
        return None;
    }
    if json::req_str(&root, "format_version")? != "1.0" {
        return None;
    }
    let source = json::req_str(&root, "source_crs")?;
    let definition = json::req_str(&root, "definition_crs")?;
    if source != definition {
        return None;
    }
    let _ = json::req_str(&root, "target_crs")?;
    let geographic = is_geographic(definition);
    let horizontal_unit = json::opt_str(&root, "horizontal_offset_unit").unwrap_or("");
    if !horizontal_unit.is_empty() && horizontal_unit != "metre" && horizontal_unit != "degree" {
        return None;
    }
    let vertical_unit = json::opt_str(&root, "vertical_offset_unit").unwrap_or("");
    if !vertical_unit.is_empty() && vertical_unit != "metre" {
        return None;
    }
    let method = json::opt_str(&root, "horizontal_offset_method").unwrap_or("");
    if !method.is_empty() && method != "addition" && method != "geocentric" {
        return None;
    }
    let (minx, miny, maxx, maxy) = extent(json::obj(&root, "extent")?, geographic)?;
    let time = json::obj(&root, "time_extent")?;
    let t0 = decimal_year(json::req_str(time, "first")?)?;
    let t1 = decimal_year(json::req_str(time, "last")?)?;
    let mut components = Vec::new();
    for item in json::arr(&root, "components")? {
        let component = item.as_object()?;
        let displacement = match json::req_str(component, "displacement_type")? {
            "horizontal" => Displacement::Horizontal,
            "vertical" => Displacement::Vertical,
            "3d" => Displacement::Three,
            "none" => continue,
            _ => return None,
        };
        let _ = json::req_str(component, "uncertainty_type")?;
        if matches!(displacement, Displacement::Horizontal | Displacement::Three)
            && (horizontal_unit.is_empty() || method.is_empty())
        {
            return None;
        }
        if matches!(displacement, Displacement::Vertical | Displacement::Three)
            && vertical_unit.is_empty()
        {
            return None;
        }
        let spatial = json::obj(component, "spatial_model")?;
        if json::req_str(spatial, "type")? != "GeoTIFF" {
            return None;
        }
        let interpolation = json::req_str(spatial, "interpolation_method")?;
        if interpolation != "bilinear" && interpolation != "geocentric_bilinear" {
            return None;
        }
        if horizontal_unit == "degree" && interpolation != "bilinear" {
            return None;
        }
        if !geographic && interpolation != "bilinear" {
            return None;
        }
        let filename = json::req_str(spatial, "filename")?;
        let grid_bytes = super::read_resource(filename).ok()?;
        let grid = super::grid::open_named(filename, &grid_bytes)?;
        let (cminx, cminy, cmaxx, cmaxy) = extent(json::obj(component, "extent")?, geographic)?;
        let time_fn = parse_time(json::obj(component, "time_function")?)?;
        components.push(Component {
            displacement,
            minx: cminx,
            miny: cminy,
            maxx: cmaxx,
            maxy: cmaxy,
            bilinear: interpolation == "bilinear",
            grid,
            time: time_fn,
        });
    }
    if horizontal_unit == "degree" && method != "addition" {
        return None;
    }
    if !geographic && horizontal_unit == "degree" {
        return None;
    }
    if !geographic && method == "geocentric" {
        return None;
    }
    Some(Model {
        geographic,
        degree: horizontal_unit == "degree",
        addition: method != "geocentric",
        minx,
        miny,
        maxx,
        maxy,
        t0,
        t1,
        components,
        a: frame.earth.a(),
        b: frame.earth.b(),
        es: frame.earth.es(),
        earth: frame.earth,
    })
}

fn is_geographic(definition: &str) -> bool {
    let Some((auth, code)) = definition.split_once(':') else {
        return true;
    };
    match crate::projdb::with_db(|connection| {
        connection.query_row(
            "SELECT 1 FROM projected_crs WHERE auth_name = ?1 AND code = ?2",
            rusqlite::params![auth, code],
            |_| Ok(()),
        )
    }) {
        Ok(()) => false,
        Err(_) => true,
    }
}

fn extent(
    object: &std::collections::BTreeMap<String, json::Value>,
    geographic: bool,
) -> Option<(f64, f64, f64, f64)> {
    if json::req_str(object, "type")? != "bbox" {
        return None;
    }
    let bbox = json::arr(json::obj(object, "parameters")?, "bbox")?;
    if bbox.len() != 4 {
        return None;
    }
    let mut values = [0.0; 4];
    for (index, item) in bbox.iter().enumerate() {
        values[index] = item.as_f64()?;
    }
    if geographic {
        Some((
            values[0] * DEG_TO_RAD,
            values[1] * DEG_TO_RAD,
            values[2] * DEG_TO_RAD,
            values[3] * DEG_TO_RAD,
        ))
    } else {
        Some((values[0], values[1], values[2], values[3]))
    }
}

fn parse_time(object: &std::collections::BTreeMap<String, json::Value>) -> Option<TimeFn> {
    let kind = json::req_str(object, "type")?;
    if kind == "constant" {
        return Some(TimeFn::Constant);
    }
    let parameters = json::obj(object, "parameters")?;
    match kind {
        "velocity" => Some(TimeFn::Velocity {
            reference: decimal_year(json::req_str(parameters, "reference_epoch")?)?,
        }),
        "step" => Some(TimeFn::Step {
            epoch: decimal_year(json::req_str(parameters, "step_epoch")?)?,
        }),
        "reverse_step" => Some(TimeFn::ReverseStep {
            epoch: decimal_year(json::req_str(parameters, "step_epoch")?)?,
        }),
        "piecewise" => {
            let before = bound(json::req_str(parameters, "before_first")?)?;
            let after = bound(json::req_str(parameters, "after_last")?)?;
            let mut samples = Vec::new();
            for item in json::arr(parameters, "model")? {
                let row = item.as_object()?;
                samples.push((
                    decimal_year(json::req_str(row, "epoch")?)?,
                    json::req_f64(row, "scale_factor")?,
                ));
            }
            Some(TimeFn::Piecewise {
                before,
                after,
                samples,
            })
        }
        "exponential" => {
            let relaxation = json::req_f64(parameters, "relaxation_constant")?;
            if relaxation <= 0.0 {
                return None;
            }
            let end = match json::opt_str(parameters, "end_epoch") {
                Some(text) if !text.is_empty() => Some(decimal_year(text)?),
                _ => None,
            };
            Some(TimeFn::Exponential {
                reference: decimal_year(json::req_str(parameters, "reference_epoch")?)?,
                end,
                relaxation,
                before: json::req_f64(parameters, "before_scale_factor")?,
                initial: json::req_f64(parameters, "initial_scale_factor")?,
                final_scale: json::req_f64(parameters, "final_scale_factor")?,
            })
        }
        _ => None,
    }
}

fn bound(text: &str) -> Option<Bound> {
    match text {
        "zero" => Some(Bound::Zero),
        "constant" => Some(Bound::Constant),
        "linear" => Some(Bound::Linear),
        _ => None,
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` → decimal year, leap days included, leap seconds not.
fn decimal_year(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let year = std::str::from_utf8(&bytes[0..4])
        .ok()?
        .parse::<i32>()
        .ok()?;
    let month = std::str::from_utf8(&bytes[5..7])
        .ok()?
        .parse::<usize>()
        .ok()?;
    let day = std::str::from_utf8(&bytes[8..10])
        .ok()?
        .parse::<i32>()
        .ok()?;
    let hour = std::str::from_utf8(&bytes[11..13])
        .ok()?
        .parse::<i32>()
        .ok()?;
    let minute = std::str::from_utf8(&bytes[14..16])
        .ok()?
        .parse::<i32>()
        .ok()?;
    let second = std::str::from_utf8(&bytes[17..19])
        .ok()?
        .parse::<i32>()
        .ok()?;
    if year < 1582
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..61).contains(&second)
    {
        return None;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let table: [i32; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    if day > table[month - 1] {
        return None;
    }
    let mut day_in_year = day - 1;
    for previous in table.iter().take(month - 1) {
        day_in_year += previous;
    }
    let seconds = f64::from(day_in_year) * 86_400.0
        + f64::from(hour) * 3_600.0
        + f64::from(minute) * 60.0
        + f64::from(second);
    let length = if leap {
        86_400.0 * 366.0
    } else {
        86_400.0 * 365.0
    };
    Some(f64::from(year) + seconds / length)
}

impl TimeFn {
    fn evaluate(&self, time: f64) -> f64 {
        match self {
            Self::Constant => 1.0,
            Self::Velocity { reference } => time - reference,
            Self::Step { epoch } => {
                if time < *epoch {
                    0.0
                } else {
                    1.0
                }
            }
            Self::ReverseStep { epoch } => {
                if time < *epoch {
                    -1.0
                } else {
                    0.0
                }
            }
            Self::Piecewise {
                before,
                after,
                samples,
            } => piecewise(*before, *after, samples, time),
            Self::Exponential {
                reference,
                end,
                relaxation,
                before,
                initial,
                final_scale,
            } => {
                if time < *reference {
                    return *before;
                }
                let time = if let Some(end) = end {
                    time.min(*end)
                } else {
                    time
                };
                initial + (final_scale - initial) * (1.0 - (-(time - reference) / relaxation).exp())
            }
        }
    }
}

fn piecewise(before: Bound, after: Bound, samples: &[(f64, f64)], time: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let (t1, f1) = samples[0];
    if time < t1 {
        return match before {
            Bound::Zero => 0.0,
            Bound::Constant => f1,
            Bound::Linear => {
                if samples.len() == 1 {
                    f1
                } else {
                    let (t2, f2) = samples[1];
                    if t1 == t2 {
                        f1
                    } else {
                        (f1 * (t2 - time) + f2 * (time - t1)) / (t2 - t1)
                    }
                }
            }
        };
    }
    for index in 1..samples.len() {
        let (t_next, f_next) = samples[index];
        if time < t_next {
            let (t_prev, f_prev) = samples[index - 1];
            return (f_prev * (t_next - time) + f_next * (time - t_prev)) / (t_next - t_prev);
        }
    }
    let (t_last, f_last) = samples[samples.len() - 1];
    match after {
        Bound::Zero => 0.0,
        Bound::Constant => f_last,
        Bound::Linear => {
            if samples.len() == 1 {
                f_last
            } else {
                let (t_prev, f_prev) = samples[samples.len() - 2];
                if t_prev == t_last {
                    f_last
                } else {
                    (f_prev * (t_last - time) + f_last * (time - t_prev)) / (t_last - t_prev)
                }
            }
        }
    }
}

impl Step for Model {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.t.is_finite() {
            return Err(TransformError::MissingTime);
        }
        self.eval(coord, false)
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.t.is_finite() {
            return Err(TransformError::MissingTime);
        }
        let mut out = coord;
        for _ in 0..10 {
            let forward = self.eval(out, true)?;
            let dx = forward.x - coord.x;
            let dy = forward.y - coord.y;
            let dz = forward.z - coord.z;
            out.x -= dx;
            out.y -= dy;
            out.z -= dz;
            if dx.abs().max(dy.abs()) < 1e-12 && dz.abs() < 1e-3 {
                return Ok(out);
            }
        }
        Err(TransformError::OutsideDomain)
    }

    fn input_kind(&self) -> IoKind {
        if self.geographic {
            IoKind::Angular
        } else {
            IoKind::Projected
        }
    }

    fn output_kind(&self) -> IoKind {
        self.input_kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}

impl Model {
    fn eval(&self, coord: Coord, inverse: bool) -> Result<Coord, TransformError> {
        let eps = if self.geographic { 1e-10 } else { 1e-5 };
        let mut x = coord.x;
        let mut y = coord.y;
        let mut x_out = coord.x;
        let mut y_out = coord.y;
        let mut z_out = coord.z;
        if self.geographic {
            while x < self.minx - eps {
                x += 2.0 * PI;
            }
            while x > self.maxx + eps {
                x -= 2.0 * PI;
            }
        }
        let margin = if self.geographic {
            0.1 * DEG_TO_RAD
        } else {
            10_000.0
        };
        if !bbox(
            inverse, &mut x, &mut y, self.minx, self.miny, self.maxx, self.maxy, eps, margin,
        ) {
            return Err(TransformError::OutsideDomain);
        }
        if coord.t < self.t0 || coord.t > self.t1 {
            return Err(TransformError::OutsideDomain);
        }
        let mut dlam = 0.0;
        let mut dphi = 0.0;
        let mut de = 0.0;
        let mut dn = 0.0;
        let mut dz = 0.0;
        let mut sinphi = 0.0;
        let mut cosphi = 0.0;
        let mut have_phi = false;
        for component in &self.components {
            let mut gx = x;
            let mut gy = y;
            if !bbox(
                inverse,
                &mut gx,
                &mut gy,
                component.minx,
                component.miny,
                component.maxx,
                component.maxy,
                eps,
                0.0,
            ) {
                continue;
            }
            gx = gx.clamp(component.minx, component.maxx);
            gy = gy.clamp(component.miny, component.maxy);
            let factor = component.time.evaluate(coord.t);
            if factor == 0.0 {
                continue;
            }
            let grid = &component.grid;
            if grid.width < 2 || grid.height < 2 {
                return Err(TransformError::OutsideDomain);
            }
            let ix_d = (gx - grid.west) / grid.res_x;
            let iy_d = (gy - grid.south) / grid.res_y;
            if ix_d < -eps
                || iy_d < -eps
                || ix_d + 1.0 >= dim(grid.width) + eps
                || iy_d + 1.0 >= dim(grid.height) + eps
            {
                continue;
            }
            let ix0 = trunc_nonneg(ix_d).min(grid.width - 2);
            let iy0 = trunc_nonneg(iy_d).min(grid.height - 2);
            let ix1 = ix0 + 1;
            let iy1 = iy0 + 1;
            let frct_x = ix_d - dim(ix0);
            let frct_y = iy_d - dim(iy0);
            let m00 = (1.0 - frct_x) * (1.0 - frct_y);
            let m10 = frct_x * (1.0 - frct_y);
            let m01 = (1.0 - frct_x) * frct_y;
            let m11 = frct_x * frct_y;
            if matches!(component.displacement, Displacement::Vertical) {
                dz += factor * interp_z(grid, ix0, iy0, ix1, iy1, m00, m01, m10, m11)?;
            } else if self.degree {
                let (dx00, dy00, z00) = offsets(grid, ix0, iy0, true, component.displacement)?;
                let (dx10, dy10, z10) = offsets(grid, ix1, iy0, true, component.displacement)?;
                let (dx01, dy01, z01) = offsets(grid, ix0, iy1, true, component.displacement)?;
                let (dx11, dy11, z11) = offsets(grid, ix1, iy1, true, component.displacement)?;
                if matches!(component.displacement, Displacement::Three) {
                    dz += factor * (z00 * m00 + z01 * m01 + z10 * m10 + z11 * m11);
                }
                dlam += factor * (dx00 * m00 + dx01 * m01 + dx10 * m10 + dx11 * m11);
                dphi += factor * (dy00 * m00 + dy01 * m01 + dy10 * m10 + dy11 * m11);
            } else {
                let (de00, dn00, z00) = offsets(grid, ix0, iy0, false, component.displacement)?;
                let (de10, dn10, z10) = offsets(grid, ix1, iy0, false, component.displacement)?;
                let (de01, dn01, z01) = offsets(grid, ix0, iy1, false, component.displacement)?;
                let (de11, dn11, z11) = offsets(grid, ix1, iy1, false, component.displacement)?;
                if matches!(component.displacement, Displacement::Three) {
                    dz += factor * (z00 * m00 + z01 * m01 + z10 * m10 + z11 * m11);
                }
                if component.bilinear {
                    de += factor * (de00 * m00 + de01 * m01 + de10 * m10 + de11 * m11);
                    dn += factor * (dn00 * m00 + dn01 * m01 + dn10 * m10 + dn11 * m11);
                } else {
                    let (d_x, d_y, d_z) = geocentric_cell(
                        grid, iy0, de00, dn00, de01, dn01, de10, dn10, de11, dn11, m00, m01, m10,
                        m11,
                    );
                    if !have_phi {
                        sinphi = y.sin();
                        cosphi = y.cos();
                        have_phi = true;
                    }
                    let lam = (frct_x - 0.5) * grid.res_x;
                    let (sinlam, coslam) = if grid.res_x < DEG_TO_RAD {
                        (lam * (1.0 - (1.0 / 6.0) * lam * lam), 1.0 - 0.5 * lam * lam)
                    } else {
                        (lam.sin(), lam.cos())
                    };
                    de += factor * (-d_x * sinlam + d_y * coslam);
                    dn += factor * ((-d_x * coslam - d_y * sinlam) * sinphi + d_z * cosphi);
                }
            }
        }
        if self.degree {
            x_out += dlam;
            y_out += dphi;
        } else if self.addition && !self.geographic {
            x_out += de;
            y_out += dn;
        } else if self.addition {
            if !have_phi {
                cosphi = y.cos();
            }
            let (lam, phi) = east_north_to_lonlat(cosphi, de, dn, self.a, self.b, self.es);
            x_out += lam;
            y_out += phi;
        } else {
            if !have_phi {
                sinphi = y.sin();
                cosphi = y.cos();
            }
            let sinlam = x.sin();
            let coslam = x.cos();
            let north_sin = dn * sinphi;
            let d_x = -de * sinlam - north_sin * coslam;
            let d_y = de * coslam - north_sin * sinlam;
            let d_z = dn * cosphi;
            let (mut gx, mut gy, mut gz) = geodetic_to_cartesian(self.earth, x, y, 0.0);
            gx += d_x;
            gy += d_y;
            gz += d_z;
            let (lam, phi, _) = cartesian_to_geodetic(self.earth, gx, gy, gz);
            x_out = lam;
            y_out = phi;
        }
        z_out += dz;
        Ok(Coord {
            x: x_out,
            y: y_out,
            z: z_out,
            t: coord.t,
        })
    }
}

fn bbox(
    inverse: bool,
    x: &mut f64,
    y: &mut f64,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    eps: f64,
    margin: f64,
) -> bool {
    if *x >= minx - eps && *x <= maxx + eps && *y >= miny - eps && *y <= maxy + eps {
        return true;
    }
    if !inverse {
        return false;
    }
    let x_ok = if *x >= minx - eps && *x <= maxx + eps {
        true
    } else if *x > minx - margin && *x < minx {
        *x = minx;
        true
    } else if *x < maxx + margin && *x > maxx {
        *x = maxx;
        true
    } else {
        false
    };
    let y_ok = if *y >= miny - eps && *y <= maxy + eps {
        true
    } else if *y > miny - margin && *y < miny {
        *y = miny;
        true
    } else if *y < maxy + margin && *y > maxy {
        *y = maxy;
        true
    } else {
        false
    };
    x_ok && y_ok
}

fn offsets(
    grid: &Grid,
    x: usize,
    y: usize,
    degree: bool,
    displacement: Displacement,
) -> Result<(f64, f64, f64), TransformError> {
    let (sample_x, sample_y) = band_pair(grid)?;
    let east = f64::from(
        grid.value(x, y, sample_x)
            .ok_or(TransformError::OutsideDomain)?,
    );
    let north = f64::from(
        grid.value(x, y, sample_y)
            .ok_or(TransformError::OutsideDomain)?,
    );
    let (east, north) = if degree {
        (east * DEG_TO_RAD, north * DEG_TO_RAD)
    } else {
        (east, north)
    };
    let z = if matches!(displacement, Displacement::Three) {
        let sample = band_z(grid)?;
        f64::from(
            grid.value(x, y, sample)
                .ok_or(TransformError::OutsideDomain)?,
        )
    } else {
        0.0
    };
    Ok((east, north, z))
}

fn interp_z(
    grid: &Grid,
    ix0: usize,
    iy0: usize,
    ix1: usize,
    iy1: usize,
    m00: f64,
    m01: f64,
    m10: f64,
    m11: f64,
) -> Result<f64, TransformError> {
    let sample = band_z(grid)?;
    let z00 = f64::from(
        grid.value(ix0, iy0, sample)
            .ok_or(TransformError::OutsideDomain)?,
    );
    let z10 = f64::from(
        grid.value(ix1, iy0, sample)
            .ok_or(TransformError::OutsideDomain)?,
    );
    let z01 = f64::from(
        grid.value(ix0, iy1, sample)
            .ok_or(TransformError::OutsideDomain)?,
    );
    let z11 = f64::from(
        grid.value(ix1, iy1, sample)
            .ok_or(TransformError::OutsideDomain)?,
    );
    Ok(z00 * m00 + z01 * m01 + z10 * m10 + z11 * m11)
}

fn band_pair(grid: &Grid) -> Result<(usize, usize), TransformError> {
    if grid.samples < 2 {
        return Err(TransformError::OutsideDomain);
    }
    let mut x = 0usize;
    let mut y = 1usize;
    let mut found = false;
    let mut any = false;
    for sample in 0..grid.samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "east_offset" {
            x = sample;
            found = true;
        } else if desc == "north_offset" {
            y = sample;
        }
    }
    let have_y = (0..grid.samples).any(|sample| grid.description(sample) == "north_offset");
    if any && (!found || !have_y) {
        return Err(TransformError::OutsideDomain);
    }
    let unit = grid.unit(x);
    if !unit.is_empty() && unit != "degree" && unit != "metre" {
        return Err(TransformError::OutsideDomain);
    }
    Ok((x, y))
}

fn band_z(grid: &Grid) -> Result<usize, TransformError> {
    if grid.samples == 1 {
        return Ok(0);
    }
    if grid.samples < 3 && grid.description(0) != "vertical_offset" {
        return Err(TransformError::OutsideDomain);
    }
    let mut sample_z = if grid.samples == 1 { 0 } else { 2 };
    let mut any = false;
    let mut found = false;
    for sample in 0..grid.samples {
        let desc = grid.description(sample);
        if !desc.is_empty() {
            any = true;
        }
        if desc == "vertical_offset" {
            sample_z = sample;
            found = true;
        }
    }
    if any && !found {
        return Err(TransformError::OutsideDomain);
    }
    let unit = grid.unit(sample_z);
    if !unit.is_empty() && unit != "metre" {
        return Err(TransformError::OutsideDomain);
    }
    Ok(sample_z)
}

fn east_north_to_lonlat(cosphi: f64, de: f64, dn: f64, a: f64, b: f64, es: f64) -> (f64, f64) {
    let one_minus = es * (1.0 - cosphi * cosphi);
    let x = 1.0 - one_minus;
    let sqrt_x = x.sqrt();
    let dlam = de * sqrt_x / (a * cosphi);
    let dphi = dn * a * sqrt_x * x / (b * b);
    (dlam, dphi)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the eight cell offsets are the PROJ arguments"
)]
fn geocentric_cell(
    grid: &Grid,
    iy0: usize,
    de00: f64,
    dn00: f64,
    de01: f64,
    dn01: f64,
    de10: f64,
    dn10: f64,
    de11: f64,
    dn11: f64,
    m00: f64,
    m01: f64,
    m10: f64,
    m11: f64,
) -> (f64, f64, f64) {
    let y0 = grid.south + dim(iy0) * grid.res_y;
    let sinphi0 = y0.sin();
    let cosphi0 = y0.cos();
    let sin_res = grid.res_y.sin();
    let cos_res = grid.res_y.cos();
    let sinphi1 = sinphi0 * cos_res + cosphi0 * sin_res;
    let cosphi1 = cosphi0 * cos_res - sinphi0 * sin_res;
    let sin_half = (grid.res_x / 2.0).sin();
    let cos_half = (grid.res_x / 2.0).cos();
    let (x00, y00, z00) = enu(-sin_half, cos_half, sinphi0, cosphi0, de00, dn00);
    let (x01, y01, z01) = enu(-sin_half, cos_half, sinphi1, cosphi1, de01, dn01);
    let (x10, y10, z10) = enu(sin_half, cos_half, sinphi0, cosphi0, de10, dn10);
    let (x11, y11, z11) = enu(sin_half, cos_half, sinphi1, cosphi1, de11, dn11);
    (
        m00 * x00 + m01 * x01 + m10 * x10 + m11 * x11,
        m00 * y00 + m01 * y01 + m10 * y10 + m11 * y11,
        m00 * z00 + m01 * z01 + m10 * z10 + m11 * z11,
    )
}

fn enu(sinlam: f64, coslam: f64, sinphi: f64, cosphi: f64, de: f64, dn: f64) -> (f64, f64, f64) {
    let north_sin = dn * sinphi;
    (
        -de * sinlam - north_sin * coslam,
        de * coslam - north_sin * sinlam,
        dn * cosphi,
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions,
    reason = "ix_d was rejected when negative or past the grid, so the truncation matches PROJ's int cast"
)]
fn trunc_nonneg(value: f64) -> usize {
    let truncated = value as i32;
    if truncated <= 0 {
        0
    } else {
        truncated as usize
    }
}

#[expect(
    clippy::cast_precision_loss,
    clippy::as_conversions,
    reason = "grid dimensions used here are far below the f64 mantissa"
)]
fn dim(value: usize) -> f64 {
    value as f64
}
