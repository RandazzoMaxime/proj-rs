//! Molodensky, Molodensky-Badekas, Helmert (including `+exact`), and Web Mercator.
//!
//! Ports of PROJ 9.9 `src/transformations/molodensky.cpp`,
//! `src/transformations/helmert.cpp` (`helmert` and `molobadekas`), and the
//! `webmerc` entry in `src/projections/merc.cpp`. `+lon_wrap` itself lives on
//! the frame (`src/fwd.cpp` / `src/init.cpp`). No grid shift is implemented.

#![allow(
    clippy::pedantic,
    reason = "branch order and the linear rotation follow helmert.cpp and molodensky.cpp"
)]

use crate::engine::{
    classic_step, finish_angular, prepare_forward, proj_number, wrap_longitude, Coord, CreateError,
    Frame, IoKind, Kernel, Params, Step, TransformError,
};
use std::f64::consts::{FRAC_PI_2, PI};

/// Arcseconds to radians. PROJ `ARCSEC_TO_RAD` (`DEG_TO_RAD / 3600`).
const ARCSEC_TO_RAD: f64 = PI / (180.0 * 3600.0);

/// Keys `+proj=helmert` reads, including the ones the linear catalogue used to reject.
pub(crate) const HELMERT_KEYS: &[&str] = &[
    "x",
    "y",
    "z",
    "rx",
    "ry",
    "rz",
    "s",
    "convention",
    "exact",
    "theta",
    "dx",
    "dy",
    "dz",
    "drx",
    "dry",
    "drz",
    "ds",
    "dtheta",
    "t_epoch",
    "towgs84",
    "transpose",
];

const MOLODENSKY_KEYS: &[&str] = &["dx", "dy", "dz", "da", "df", "abridged"];

const MOLOBADEKAS_KEYS: &[&str] = &[
    "x",
    "y",
    "z",
    "rx",
    "ry",
    "rz",
    "s",
    "convention",
    "exact",
    "px",
    "py",
    "pz",
];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "helmert" => Some(HELMERT_KEYS),
        "molodensky" => Some(MOLODENSKY_KEYS),
        "molobadekas" => Some(MOLOBADEKAS_KEYS),
        "webmerc" => Some(&[]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    match name {
        "helmert" => Ok(Some(helmert(params, frame)?)),
        "molobadekas" => Ok(Some(molobadekas(params, frame)?)),
        "molodensky" => Ok(Some(molodensky(params, frame)?)),
        "webmerc" => Ok(Some(classic_step(frame, WebMerc))),
        _ => Ok(None),
    }
}

fn key_name(key: &str) -> &'static str {
    match key {
        "x" => "x",
        "y" => "y",
        "z" => "z",
        "rx" => "rx",
        "ry" => "ry",
        "rz" => "rz",
        "s" => "s",
        "convention" => "convention",
        "exact" => "exact",
        "theta" => "theta",
        "dx" => "dx",
        "dy" => "dy",
        "dz" => "dz",
        "drx" => "drx",
        "dry" => "dry",
        "drz" => "drz",
        "ds" => "ds",
        "dtheta" => "dtheta",
        "t_epoch" => "t_epoch",
        "towgs84" => "towgs84",
        "transpose" => "transpose",
        "px" => "px",
        "py" => "py",
        "pz" => "pz",
        "da" => "da",
        "df" => "df",
        "abridged" => "abridged",
        _ => "parameter",
    }
}

fn optional_number(params: &Params, key: &str) -> Result<f64, CreateError> {
    if !params.flag(key) {
        return Ok(0.0);
    }
    let value = params.number(key)?.unwrap_or(0.0);
    if !value.is_finite() {
        return Err(CreateError::IllegalValue(key_name(key)));
    }
    Ok(value)
}

fn required_number(params: &Params, key: &str) -> Result<f64, CreateError> {
    if !params.flag(key) {
        return Err(CreateError::MissingArgument(key_name(key)));
    }
    optional_number(params, key)
}

enum Switch {
    Off,
    On,
}

/// PROJ `pj_param` type `b`: absent, empty/`T`/`t` true, `F`/`f` false.
fn switch(params: &Params, key: &str) -> Result<Option<Switch>, CreateError> {
    if !params.flag(key) {
        return Ok(None);
    }
    let Some(text) = params.text(key) else {
        return Ok(Some(Switch::On));
    };
    match text.as_bytes().first() {
        None | Some(b'T' | b't') => Ok(Some(Switch::On)),
        Some(b'F' | b'f') => Ok(Some(Switch::Off)),
        _ => Err(CreateError::IllegalValue(key_name(key))),
    }
}

fn towgs84_values(text: &str) -> Result<[f64; 7], CreateError> {
    let mut values = [0.0; 7];
    if text.is_empty() {
        return Ok(values);
    }
    for (index, piece) in text.split(',').enumerate() {
        if index >= 7 {
            break;
        }
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let Some(value) = proj_number(piece) else {
            return Err(CreateError::IllegalValue("towgs84"));
        };
        if !value.is_finite() {
            return Err(CreateError::IllegalValue("towgs84"));
        }
        values[index] = value;
    }
    Ok(values)
}

#[derive(Clone, Copy)]
enum Convention {
    PositionVector,
    CoordinateFrame,
}

#[derive(Clone, Copy)]
enum AngleLaw {
    Exact,
    Linear,
}

#[derive(Clone, Copy)]
enum Sense {
    Forward,
    Inverse,
}

#[derive(Clone, Copy)]
enum Law {
    Spatial {
        angles: AngleLaw,
        convention: Convention,
        ppm: f64,
        dppm: f64,
        rx: f64,
        ry: f64,
        rz: f64,
        drx: f64,
        dry: f64,
        drz: f64,
        px: f64,
        py: f64,
        pz: f64,
    },
    /// Four-parameter Helmert. `scale` is a raw factor, not parts per million.
    Planar {
        scale: f64,
        dscale: f64,
        theta: f64,
        dtheta: f64,
    },
}

struct EpochShift {
    frame: Frame,
    tx: f64,
    ty: f64,
    tz: f64,
    dtx: f64,
    dty: f64,
    dtz: f64,
    t_epoch: f64,
    law: Law,
}

#[derive(Clone, Copy)]
struct SpatialNow {
    tx: f64,
    ty: f64,
    tz: f64,
    ppm: f64,
    rx: f64,
    ry: f64,
    rz: f64,
    px: f64,
    py: f64,
    pz: f64,
    angles: AngleLaw,
    convention: Convention,
}

fn helmert(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    if matches!(switch(params, "transpose")?, Some(Switch::On)) {
        return Err(CreateError::IllegalValue("transpose"));
    }
    let mut tx = optional_number(params, "x")?;
    let mut ty = optional_number(params, "y")?;
    let mut tz = optional_number(params, "z")?;
    let mut rx_arc = optional_number(params, "rx")?;
    let mut ry_arc = optional_number(params, "ry")?;
    let mut rz_arc = optional_number(params, "rz")?;
    let mut scale = 0.0;
    if params.flag("towgs84") {
        let packed = towgs84_values(params.text("towgs84").unwrap_or(""))?;
        tx = packed[0];
        ty = packed[1];
        tz = packed[2];
        rx_arc = packed[3];
        ry_arc = packed[4];
        rz_arc = packed[5];
        scale = packed[6];
    }
    let planar = params.flag("theta");
    if planar {
        // `helmert.cpp` resets the scale to 1 before an explicit `+s`.
        scale = 1.0;
    }
    if params.flag("s") {
        scale = optional_number(params, "s")?;
    }
    if scale <= -1.0e6 {
        return Err(CreateError::IllegalValue("s"));
    }
    if planar && scale == 0.0 {
        return Err(CreateError::IllegalValue("s"));
    }
    let dtx = optional_number(params, "dx")?;
    let dty = optional_number(params, "dy")?;
    let dtz = optional_number(params, "dz")?;
    let drx_arc = optional_number(params, "drx")?;
    let dry_arc = optional_number(params, "dry")?;
    let drz_arc = optional_number(params, "drz")?;
    let ds = optional_number(params, "ds")?;
    let dtheta_arc = optional_number(params, "dtheta")?;
    let t_epoch = optional_number(params, "t_epoch")?;
    let theta_arc = if planar {
        optional_number(params, "theta")?
    } else {
        0.0
    };
    // `no_rotation` ignores `theta`. A zero 3-D rotation does not require `convention`.
    let spin = rx_arc != 0.0
        || ry_arc != 0.0
        || rz_arc != 0.0
        || drx_arc != 0.0
        || dry_arc != 0.0
        || drz_arc != 0.0;
    let convention = if spin {
        convention_of(params, true)?
    } else {
        Convention::PositionVector
    };
    let angles = match switch(params, "exact")? {
        Some(Switch::On) => AngleLaw::Exact,
        _ => AngleLaw::Linear,
    };
    let law = if planar {
        Law::Planar {
            scale,
            dscale: ds,
            theta: theta_arc * ARCSEC_TO_RAD,
            dtheta: dtheta_arc * ARCSEC_TO_RAD,
        }
    } else {
        Law::Spatial {
            angles,
            convention,
            ppm: scale,
            dppm: ds,
            rx: rx_arc * ARCSEC_TO_RAD,
            ry: ry_arc * ARCSEC_TO_RAD,
            rz: rz_arc * ARCSEC_TO_RAD,
            drx: drx_arc * ARCSEC_TO_RAD,
            dry: dry_arc * ARCSEC_TO_RAD,
            drz: drz_arc * ARCSEC_TO_RAD,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        }
    };
    Ok(Box::new(EpochShift {
        frame: frame.clone(),
        tx,
        ty,
        tz,
        dtx,
        dty,
        dtz,
        t_epoch,
        law,
    }))
}

fn molobadekas(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    // `no_rotation` stays 0 in `PJ_TRANSFORMATION(molobadekas)`, so `convention` is mandatory.
    let convention = convention_of(params, false)?;
    let scale = optional_number(params, "s")?;
    if !scale.is_finite() {
        return Err(CreateError::IllegalValue("s"));
    }
    let px = optional_number(params, "px")?;
    let py = optional_number(params, "py")?;
    let pz = optional_number(params, "pz")?;
    let angles = match switch(params, "exact")? {
        Some(Switch::On) => AngleLaw::Exact,
        _ => AngleLaw::Linear,
    };
    // The reference point is folded into the translation, as in helmert.cpp.
    Ok(Box::new(EpochShift {
        frame: frame.clone(),
        tx: optional_number(params, "x")? + px,
        ty: optional_number(params, "y")? + py,
        tz: optional_number(params, "z")? + pz,
        dtx: 0.0,
        dty: 0.0,
        dtz: 0.0,
        t_epoch: 0.0,
        law: Law::Spatial {
            angles,
            convention,
            ppm: scale,
            dppm: 0.0,
            rx: optional_number(params, "rx")? * ARCSEC_TO_RAD,
            ry: optional_number(params, "ry")? * ARCSEC_TO_RAD,
            rz: optional_number(params, "rz")? * ARCSEC_TO_RAD,
            drx: 0.0,
            dry: 0.0,
            drz: 0.0,
            px,
            py,
            pz,
        },
    }))
}

/// `require_towgs84_position` is false for Molodensky-Badekas, which never reads `towgs84`.
fn convention_of(params: &Params, helmert_towgs84: bool) -> Result<Convention, CreateError> {
    let convention = match params.text("convention") {
        None if params.flag("convention") => {
            return Err(CreateError::IllegalValue("convention"));
        }
        None => return Err(CreateError::MissingArgument("convention")),
        Some("position_vector") => Convention::PositionVector,
        Some("coordinate_frame") => Convention::CoordinateFrame,
        Some(_) => return Err(CreateError::IllegalValue("convention")),
    };
    if helmert_towgs84
        && params.flag("towgs84")
        && matches!(convention, Convention::CoordinateFrame)
    {
        return Err(CreateError::IllegalValue("towgs84"));
    }
    Ok(convention)
}

impl EpochShift {
    fn observed_dt(&self, t: f64) -> f64 {
        // `helmert_forward_4d`: a non-finite time (PROJ `HUGE_VAL`) means the epoch.
        let observed = if t.is_finite() { t } else { self.t_epoch };
        observed - self.t_epoch
    }

    fn apply(&self, coord: Coord, sense: Sense) -> Coord {
        let dt = self.observed_dt(coord.t);
        let (x, y, z) = match self.law {
            Law::Planar {
                scale,
                dscale,
                theta,
                dtheta,
            } => {
                // Translation rates are stored on `xyz` but the 2-D kernel reads `xyz_0`.
                let now = PlanarNow {
                    tx: self.tx,
                    ty: self.ty,
                    scale: scale + dscale * dt,
                    theta: theta + dtheta * dt,
                };
                match sense {
                    Sense::Forward => {
                        let (x, y, z) = planar_kernel(now, coord.x, coord.y, coord.z, sense);
                        let frame = &self.frame;
                        (
                            frame.fr_meter * (x + frame.x0),
                            frame.fr_meter * (y + frame.y0),
                            frame.vfr_meter * (z + frame.z0),
                        )
                    }
                    Sense::Inverse => {
                        let frame = &self.frame;
                        planar_kernel(
                            now,
                            frame.to_meter * coord.x - frame.x0,
                            frame.to_meter * coord.y - frame.y0,
                            frame.vto_meter * coord.z - frame.z0,
                            sense,
                        )
                    }
                }
            }
            Law::Spatial {
                angles,
                convention,
                ppm,
                dppm,
                rx,
                ry,
                rz,
                drx,
                dry,
                drz,
                px,
                py,
                pz,
            } => {
                let now = SpatialNow {
                    tx: self.tx + self.dtx * dt,
                    ty: self.ty + self.dty * dt,
                    tz: self.tz + self.dtz * dt,
                    ppm: ppm + dppm * dt,
                    rx: rx + drx * dt,
                    ry: ry + dry * dt,
                    rz: rz + drz * dt,
                    px,
                    py,
                    pz,
                    angles,
                    convention,
                };
                let meter = match sense {
                    Sense::Forward => 1.0,
                    Sense::Inverse => self.frame.to_meter,
                };
                let (x, y, z) = spatial_kernel(
                    now,
                    meter * coord.x,
                    meter * coord.y,
                    meter * coord.z,
                    sense,
                );
                match sense {
                    Sense::Forward => {
                        let out = self.frame.fr_meter;
                        (out * x, out * y, out * z)
                    }
                    Sense::Inverse => (x, y, z),
                }
            }
        };
        Coord {
            x,
            y,
            z,
            t: coord.t,
        }
    }
}

impl Step for EpochShift {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.apply(coord, Sense::Forward))
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        Ok(self.apply(coord, Sense::Inverse))
    }

    fn input_kind(&self) -> IoKind {
        match self.law {
            Law::Planar { .. } => IoKind::Projected,
            Law::Spatial { .. } => IoKind::Cartesian,
        }
    }

    fn output_kind(&self) -> IoKind {
        self.input_kind()
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

#[derive(Clone, Copy)]
struct PlanarNow {
    tx: f64,
    ty: f64,
    scale: f64,
    theta: f64,
}

fn planar_kernel(now: PlanarNow, x: f64, y: f64, z: f64, sense: Sense) -> (f64, f64, f64) {
    match sense {
        Sense::Forward => {
            let cr = now.theta.cos() * now.scale;
            let sr = now.theta.sin() * now.scale;
            (cr * x + sr * y + now.tx, -sr * x + cr * y + now.ty, z)
        }
        Sense::Inverse => {
            let cr = now.theta.cos() / now.scale;
            let sr = now.theta.sin() / now.scale;
            let x = x - now.tx;
            let y = y - now.ty;
            (x * cr - y * sr, x * sr + y * cr, z)
        }
    }
}

fn rotation_matrix(
    rx: f64,
    ry: f64,
    rz: f64,
    angles: AngleLaw,
    convention: Convention,
) -> [[f64; 3]; 3] {
    // Built in the coordinate-frame sign, then transposed for position vector.
    let (f, t, p) = (rx, ry, rz);
    let mut matrix = match angles {
        AngleLaw::Exact => {
            let (sf, cf) = (f.sin(), f.cos());
            let (st, ct) = (t.sin(), t.cos());
            let (sp, cp) = (p.sin(), p.cos());
            [
                [ct * cp, cf * sp + sf * st * cp, sf * sp - cf * st * cp],
                [-ct * sp, cf * cp - sf * st * sp, sf * cp + cf * st * sp],
                [st, -sf * ct, cf * ct],
            ]
        }
        AngleLaw::Linear => [[1.0, p, -t], [-p, 1.0, f], [t, -f, 1.0]],
    };
    if matches!(convention, Convention::PositionVector) {
        matrix = [
            [matrix[0][0], matrix[1][0], matrix[2][0]],
            [matrix[0][1], matrix[1][1], matrix[2][1]],
            [matrix[0][2], matrix[1][2], matrix[2][2]],
        ];
    }
    matrix
}

fn matvec(matrix: [[f64; 3]; 3], x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    (
        matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z,
        matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z,
        matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z,
    )
}

fn spatial_kernel(now: SpatialNow, x: f64, y: f64, z: f64, sense: Sense) -> (f64, f64, f64) {
    let matrix = rotation_matrix(now.rx, now.ry, now.rz, now.angles, now.convention);
    let factor = 1.0 + now.ppm * 1e-6;
    match sense {
        Sense::Forward => {
            let (x, y, z) = matvec(matrix, x - now.px, y - now.py, z - now.pz);
            (
                factor * x + now.tx,
                factor * y + now.ty,
                factor * z + now.tz,
            )
        }
        Sense::Inverse => {
            let x = (x - now.tx) / factor;
            let y = (y - now.ty) / factor;
            let z = (z - now.tz) / factor;
            // Transpose multiply: PROJ does not invert the linearised matrix for real.
            (
                matrix[0][0] * x + matrix[1][0] * y + matrix[2][0] * z + now.px,
                matrix[0][1] * x + matrix[1][1] * y + matrix[2][1] * z + now.py,
                matrix[0][2] * x + matrix[1][2] * y + matrix[2][2] * z + now.pz,
            )
        }
    }
}

#[derive(Clone, Copy)]
enum MolodenskyForm {
    Standard,
    Abridged,
}

struct Molodensky {
    frame: Frame,
    dx: f64,
    dy: f64,
    dz: f64,
    da: f64,
    df: f64,
    form: MolodenskyForm,
}

fn molodensky(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let form = match switch(params, "abridged")? {
        Some(Switch::On) => MolodenskyForm::Abridged,
        _ => MolodenskyForm::Standard,
    };
    Ok(Box::new(Molodensky {
        frame: frame.clone(),
        dx: required_number(params, "dx")?,
        dy: required_number(params, "dy")?,
        dz: required_number(params, "dz")?,
        da: required_number(params, "da")?,
        df: required_number(params, "df")?,
        form,
    }))
}

fn prime_vertical(a: f64, es: f64, phi: f64) -> f64 {
    if es == 0.0 {
        return a;
    }
    let s = phi.sin();
    a / (1.0 - es * s * s).sqrt()
}

fn meridian(a: f64, es: f64, phi: f64) -> f64 {
    if es == 0.0 {
        return a;
    }
    if phi == 0.0 {
        return a * (1.0 - es);
    }
    if phi.abs() == FRAC_PI_2 {
        return a / (1.0 - es).sqrt();
    }
    let s = phi.sin();
    let denom = 1.0 - es * s * s;
    a * (1.0 - es) / denom.powf(1.5)
}

impl Molodensky {
    fn deltas(&self, lam: f64, phi: f64, height: f64) -> Result<(f64, f64, f64), TransformError> {
        let earth = self.frame.earth;
        let a = earth.a();
        let f = earth.flattening();
        let es = earth.es();
        let slam = lam.sin();
        let clam = lam.cos();
        let sphi = phi.sin();
        let cphi = phi.cos();
        let (dphi, dlam, dh) = match self.form {
            MolodenskyForm::Standard => {
                let rho = meridian(a, es, phi);
                let nu = prime_vertical(a, es, phi);
                let mut dphi = (-self.dx * sphi * clam) - (self.dy * sphi * slam)
                    + (self.dz * cphi)
                    + ((nu * es * sphi * cphi * self.da) / a)
                    + (sphi * cphi * (rho / (1.0 - f) + nu * (1.0 - f)) * self.df);
                let dphi_denom = rho + height;
                if dphi_denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                dphi /= dphi_denom;
                let dlam_denom = (nu + height) * cphi;
                if dlam_denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                let dlam = (-self.dx * slam + self.dy * clam) / dlam_denom;
                let dh = self.dx * cphi * clam + self.dy * cphi * slam + self.dz * sphi
                    - (a / nu) * self.da
                    + nu * (1.0 - f) * sphi * sphi * self.df;
                (dphi, dlam, dh)
            }
            MolodenskyForm::Abridged => {
                let adffda = a * self.df + f * self.da;
                let dphi = (-self.dx * sphi * clam - self.dy * sphi * slam
                    + self.dz * cphi
                    + adffda * (2.0 * phi).sin())
                    / meridian(a, es, phi);
                let dlam_denom = prime_vertical(a, es, phi) * cphi;
                if dlam_denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                let dlam = (-self.dx * slam + self.dy * clam) / dlam_denom;
                let dh = self.dx * cphi * clam + self.dy * cphi * slam + self.dz * sphi - self.da
                    + adffda * sphi * sphi;
                (dphi, dlam, dh)
            }
        };
        Ok((dlam, dphi, dh))
    }
}

impl Step for Molodensky {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let (dlam, dphi, dh) = self.deltas(lam, phi, coord.z)?;
        Ok(Coord {
            x: wrap_longitude(self.frame.lon_wrap, lam + dlam),
            y: phi + dphi,
            z: self.frame.vfr_meter * (coord.z + dh + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let height = self.frame.vto_meter * coord.z - self.frame.z0;
        let (dlam, dphi, dh) = self.deltas(coord.x, coord.y, height)?;
        let (lam, phi) = finish_angular(&self.frame, coord.x - dlam, coord.y - dphi);
        Ok(Coord {
            x: lam,
            y: phi,
            z: height - dh,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

/// Spherical Mercator with `k0` forced to 1. Flattening is ignored; `a` still scales.
struct WebMerc;

impl Kernel for WebMerc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let y = phi.tan().asinh();
        if !lam.is_finite() || !y.is_finite() {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y.sinh().atan();
        if !x.is_finite() || !phi.is_finite() {
            return Err(TransformError::OutsideDomain);
        }
        Ok((x, phi))
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{lpz_dist, xyz_dist, Coord, CreateError, TransformError};
    use crate::Operation;

    fn xyz(x: f64, y: f64, z: f64, t: f64) -> Coord {
        Coord { x, y, z, t }
    }

    fn deg(lon: f64, lat: f64, z: f64, t: f64) -> Coord {
        Coord {
            x: lon.to_radians(),
            y: lat.to_radians(),
            z,
            t,
        }
    }

    fn assert_metres(got: Coord, x: f64, y: f64, z: f64, tol: f64) {
        let dist = xyz_dist(got, xyz(x, y, z, 0.0));
        assert!(
            dist <= tol,
            "dist {dist} m, got {} {} {}",
            got.x,
            got.y,
            got.z
        );
    }

    fn assert_ang(op: &Operation, got: Coord, lon: f64, lat: f64, z: f64, tol: f64) {
        let (a, es) = op.distance_ellipsoid();
        let dist = lpz_dist(a, es, deg(lon, lat, z, 0.0), got);
        assert!(
            dist <= tol,
            "dist {dist} m, got {} {} {}",
            got.x.to_degrees(),
            got.y.to_degrees(),
            got.z
        );
    }

    #[test]
    fn molodensky_abridged_and_standard_match_gie() {
        let common =
            "proj=molodensky a=6378160 rf=298.25 da=-23 df=-8.120449e-8 dx=-134 dy=-48 dz=149";
        let abridged = Operation::create(&format!("{common} abridged")).expect("abridged");
        let standard = Operation::create(common).expect("standard");
        let input = deg(144.9667, -37.8, 50.0, 0.0);
        let abr = abridged.forward(input).expect("forward");
        let std = standard.forward(input).expect("forward");
        assert_ang(&abridged, abr, 144.968, -37.79848, 46.378, 2.0);
        assert_ang(&standard, std, 144.968, -37.79848, 46.378, 2.0);
        let mut round = input;
        for _ in 0..100 {
            round = abridged
                .inverse(abridged.forward(round).expect("fwd"))
                .expect("inv");
        }
        let (a, es) = abridged.distance_ellipsoid();
        assert!(lpz_dist(a, es, input, round) <= 1.0);
    }

    #[test]
    fn molodensky_zeros_are_identity() {
        let op = Operation::create("proj=molodensky a=6378160 rf=298.25 da=0 df=0 dx=0 dy=0 dz=0")
            .expect("zero");
        let got = op.forward(deg(144.9667, -37.8, 50.0, 0.0)).expect("fwd");
        assert_ang(&op, got, 144.9667, -37.8, 50.0, 0.001);
    }

    #[test]
    fn molodensky_and_molobadekas_reject_missing_arguments() {
        assert!(matches!(
            Operation::create("proj=molodensky a=6378160 rf=298.25"),
            Err(CreateError::MissingArgument("dx"))
        ));
        assert!(matches!(
            Operation::create("proj=molodensky a=6378160 rf=298.25 dx=0"),
            Err(CreateError::MissingArgument(_))
        ));
        assert!(matches!(
            Operation::create("proj=molobadekas"),
            Err(CreateError::MissingArgument("convention"))
        ));
    }

    #[test]
    fn molobadekas_la_canoa_matches_gie() {
        let op = Operation::create(
            "proj=molobadekas convention=coordinate_frame x=-270.933 y=115.599 z=-360.226 rx=-5.266 ry=-1.238 rz=2.381 s=-5.109 px=2464351.59 py=-5783466.61 pz=974809.81",
        )
        .expect("molobadekas");
        let input = xyz(2_550_408.96, -5_749_912.26, 1_054_891.11, 0.0);
        let got = op.forward(input).expect("fwd");
        assert_metres(got, 2_550_138.45, -5_749_799.87, 1_054_530.82, 0.01);
        let back = op.inverse(got).expect("inv");
        assert!(xyz_dist(input, back) <= 0.01);
    }

    #[test]
    fn helmert_linear_position_vector_matches_the_old_kernel() {
        let op = Operation::create("+proj=helmert +rx=1 +convention=position_vector").expect("h");
        let got = op.forward(xyz(100.0, 200.0, 300.0, 0.0)).expect("fwd");
        assert_metres(got, 100.0, 199.998_545_56, 300.000_969_63, 1e-6);
        let op = Operation::create("+proj=helmert +rx=1 +convention=coordinate_frame").expect("h");
        let got = op.forward(xyz(100.0, 200.0, 300.0, 0.0)).expect("fwd");
        assert_metres(got, 100.0, 200.001_454_44, 299.999_030_37, 1e-6);
    }

    #[test]
    fn helmert_gie_blocks_and_gda() {
        let jivall = Operation::create(
            "proj=helmert convention=coordinate_frame x=0.67678 y=0.65495 z=-0.52827 rx=-0.022742 ry=0.012667 rz=0.022704 s=-0.01070",
        )
        .expect("jivall");
        assert_metres(
            jivall
                .forward(xyz(3_565_285.0, 855_949.0, 5_201_383.0, 0.0))
                .expect("fwd"),
            3_565_285.413_423_51,
            855_948.679_867_59,
            5_201_382.729_397_91,
            1e-6,
        );

        let exact = Operation::create(
            "proj=helmert exact convention=coordinate_frame x=-081.0703 rx=-0.48488 y=-089.3603 ry=-0.02436 z=-115.7526 rz=-0.41321 s=-0.540645",
        )
        .expect("exact");
        assert_metres(
            exact
                .forward(xyz(3_494_994.301_2, 1_056_601.972_5, 5_212_382.166_6, 0.0))
                .expect("fwd"),
            3_494_909.840_263_68,
            1_056_506.789_386_33,
            5_212_265.666_997_61,
            1e-6,
        );

        let timed = Operation::create(
            "proj=helmert convention=position_vector x=0.0127 dx=-0.0029 rx=-0.00039 drx=-0.00011 y=0.0065 dy=-0.0002 ry=0.00080 dry=-0.00019 z=-0.0209 dz=-0.0006 rz=-0.00114 drz=0.00007 s=0.00195 ds=0.00001 t_epoch=1988.0",
        )
        .expect("timed");
        let src = xyz(3_370_658.378, 711_877.314, 5_349_787.086, 2017.0);
        assert_metres(
            timed.forward(src).expect("2017"),
            3_370_658.188_90,
            711_877.423_70,
            5_349_787.124_30,
            3e-5,
        );
        assert_metres(
            timed
                .forward(xyz(src.x, src.y, src.z, 2018.0))
                .expect("2018"),
            3_370_658.180_87,
            711_877.427_50,
            5_349_787.126_48,
            1e-4,
        );

        let planar =
            Operation::create("proj=helmert x=-9597.3572 y=.6112 s=0.304794780637 theta=-1.244048")
                .expect("planar");
        assert_metres(
            planar
                .forward(xyz(2_546_506.957, 542_256.609, 0.0, 0.0))
                .expect("fwd"),
            766_563.675,
            165_282.277,
            0.0,
            1e-3,
        );

        let rates = Operation::create(
            "+proj=helmert +x=-0.0019 +y=-0.0017 +z=-0.0105 +s=0.00134 +dx=0.0001 +dy=0.0001 +dz=-0.0018 +ds=0.00008 +t_epoch=2000.0 +convention=position_vector",
        )
        .expect("2333");
        assert_metres(
            rates
                .forward(xyz(
                    3_513_638.193_8,
                    778_956.452_5,
                    5_248_216.469_0,
                    2008.75,
                ))
                .expect("fwd"),
            3_513_638.199_9,
            778_956.453_3,
            5_248_216.453_5,
            1e-4,
        );

        let gda = Operation::create(
            "proj=helmert convention=coordinate_frame x=0.06155 rx=-0.0394924 y=-0.01087 ry=-0.0327221 z=-0.04019 rz=-0.0328979 s=-0.009994",
        )
        .expect("gda");
        assert_metres(
            gda.forward(xyz(
                -4_052_051.764_3,
                4_212_836.201_7,
                -2_545_106.024_5,
                0.0,
            ))
            .expect("fwd"),
            -4_052_052.737_9,
            4_212_835.989_7,
            -2_545_104.589_8,
            7.5e-5,
        );

        let pipe = Operation::create(
            "proj=pipeline ellps=GRS80 step proj=cart step proj=helmert convention=coordinate_frame x=0.06155 rx=-0.0394924 y=-0.01087 ry=-0.0327221 z=-0.04019 rz=-0.0328979 s=-0.009994 step proj=cart inv",
        )
        .expect("pipeline");
        assert_ang(
            &pipe,
            pipe.forward(deg(133.885_513_29, -23.670_123_89, 603.3466, 0.0))
                .expect("fwd"),
            133.885_521_6,
            -23.670_110_14,
            603.2489,
            0.002,
        );

        let kinematic = Operation::create(
            "proj=helmert exact convention=coordinate_frame x=0 rx=0 dx=0 drx=0.00150379 y=0 ry=0 dy=0 dry=0.00118346 z=0 rz=0 dz=0 drz=0.00120716 s=0 ds=0 t_epoch=2020.0",
        )
        .expect("itrs");
        assert_metres(
            kinematic
                .forward(xyz(
                    -4_052_052.658_8,
                    4_212_835.993_8,
                    -2_545_104.694_6,
                    2018.0,
                ))
                .expect("fwd"),
            -4_052_052.737_3,
            4_212_835.983_5,
            -2_545_104.586_7,
            4e-5,
        );
    }

    #[test]
    fn helmert_errors_match_gie() {
        assert!(matches!(
            Operation::create("proj=helmert rx=1"),
            Err(CreateError::MissingArgument("convention"))
        ));
        assert!(matches!(
            Operation::create("proj=helmert rx=1 convention=foo"),
            Err(CreateError::IllegalValue("convention"))
        ));
        assert!(matches!(
            Operation::create("proj=helmert rx=1 convention=1"),
            Err(CreateError::IllegalValue("convention"))
        ));
        assert!(matches!(
            Operation::create("proj=helmert towgs84=1,2,3,4,5,6,7 convention=coordinate_frame"),
            Err(CreateError::IllegalValue("towgs84"))
        ));
        assert!(matches!(
            Operation::create("proj=helmert transpose"),
            Err(CreateError::IllegalValue("transpose"))
        ));
        let oblique = Operation::create("+proj=ob_tran +o_proj=helmert +o_lat_p=0").expect("ob");
        assert_eq!(
            oblique.inverse(Coord::xy(0.0, 0.0)).expect_err("no inv"),
            TransformError::NoInverse
        );
    }

    #[test]
    fn webmerc_is_spherical_and_forces_k0() {
        let op = Operation::create("+proj=webmerc +ellps=WGS84").expect("web");
        let first = op
            .forward(deg(-100.333_333_33, 24.463_580_28, 0.0, 0.0))
            .expect("fwd");
        assert_metres(first, -11_169_055.58, 2_810_000.00, 0.0, 0.01);
        let second = op
            .forward(deg(-100.333_333_33, 24.381_786_94, 0.0, 0.0))
            .expect("fwd");
        assert_metres(second, -11_169_055.58, 2_800_000.00, 0.0, 0.01);
        let scaled = Operation::create("+proj=webmerc +ellps=WGS84 +k=0.5").expect("k");
        let other = scaled
            .forward(deg(-100.333_333_33, 24.463_580_28, 0.0, 0.0))
            .expect("fwd");
        assert!((first.x - other.x).abs() < 1e-6);
        assert!((first.y - other.y).abs() < 1e-6);
        let back = op.inverse(first).expect("inv");
        assert!((back.x.to_degrees() + 100.333_333_33).abs() < 1e-9);
        assert!((back.y.to_degrees() - 24.463_580_28).abs() < 1e-9);
    }

    #[test]
    fn lon_wrap_recenters_longlat_forward_only() {
        let op = Operation::create("+proj=longlat +ellps=WGS84 +lon_wrap=180").expect("wrap");
        let got = op.forward(deg(-1.0, 10.0, 0.0, 0.0)).expect("fwd");
        assert!(
            (got.x.to_degrees() - 359.0).abs() < 1e-9,
            "{}",
            got.x.to_degrees()
        );
        assert!((got.y.to_degrees() - 10.0).abs() < 1e-12);
        assert!(matches!(
            Operation::create("+proj=longlat +lon_wrap=4000"),
            Err(CreateError::IllegalValue("lon_wrap"))
        ));
    }
}
