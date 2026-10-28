//! S2 cube-face projection. Output is already in the projected unit, not
//! scaled by the semi-major axis. Longitude is not reduced by `lon_0`:
//! PROJ stores `from_greenwich = -lam0` so the two shifts cancel, and `lon_0`
//! only selects the cube face.

#![allow(
    clippy::as_conversions,
    clippy::pedantic,
    reason = "face axes and UV conversions follow s2.cpp; 2^53 is an exact f64"
)]

use crate::angle::adjlon;
use crate::engine::{Coord, CreateError, Frame, IoKind, Params, Step, TransformError};
use std::f64::consts::{FRAC_1_PI, FRAC_PI_2, FRAC_PI_4, FRAC_PI_8};

const EPS_LAT: f64 = 1e-12;
const TAN_EPS: f64 = 1.0 / (1u64 << 53) as f64;

#[derive(Clone, Copy)]
enum Uv {
    Linear,
    Quadratic,
    Tangent,
    None,
}

struct S2 {
    frame: Frame,
    face: u8,
    uv: Uv,
}

fn st_to_uv(s: f64, uv: Uv) -> f64 {
    match uv {
        Uv::Linear => 2.0 * s - 1.0,
        Uv::Quadratic => {
            if s >= 0.5 {
                (4.0 * s * s - 1.0) / 3.0
            } else {
                (1.0 - 4.0 * (1.0 - s) * (1.0 - s)) / 3.0
            }
        }
        Uv::Tangent => {
            let s = (FRAC_PI_2 * s - FRAC_PI_4).tan();
            s + TAN_EPS * s
        }
        Uv::None => s,
    }
}

fn uv_to_st(u: f64, uv: Uv) -> f64 {
    match uv {
        Uv::Linear => 0.5 * (u + 1.0),
        Uv::Quadratic => {
            if u >= 0.0 {
                0.5 * (1.0 + 3.0 * u).sqrt()
            } else {
                1.0 - 0.5 * (1.0 - 3.0 * u).sqrt()
            }
        }
        Uv::Tangent => 2.0 * FRAC_1_PI * (u.atan() + FRAC_PI_4),
        Uv::None => u,
    }
}

fn face_uv_to_xyz(face: u8, u: f64, v: f64) -> (f64, f64, f64) {
    let major = 1.0 / (1.0 + u * u + v * v).sqrt();
    let minor_u = u * major;
    let minor_v = v * major;
    match face {
        0 => (major, minor_u, minor_v),
        1 => (-minor_u, major, minor_v),
        2 => (-minor_u, -minor_v, major),
        3 => (-major, -minor_v, -minor_u),
        4 => (minor_v, -major, -minor_u),
        _ => (minor_v, minor_u, -major),
    }
}

fn xyz_to_face_uv(face: u8, x: f64, y: f64, z: f64) -> (f64, f64) {
    match face {
        0 => (y / x, z / x),
        1 => (-x / y, z / y),
        2 => (-x / z, -y / z),
        3 => (z / x, y / x),
        4 => (z / y, -x / y),
        _ => (-y / z, -x / z),
    }
}

fn face_from_origin(phi0: f64, lam0: f64) -> u8 {
    let polar = FRAC_PI_2 - FRAC_PI_8;
    if phi0 >= polar {
        2
    } else if phi0 <= -polar {
        5
    } else if lam0.abs() <= FRAC_PI_4 {
        0
    } else if lam0.abs() <= FRAC_PI_2 + FRAC_PI_4 {
        if lam0 > 0.0 {
            1
        } else {
            4
        }
    } else {
        3
    }
}

fn geocentric_latitude(phi: f64, one_es: f64) -> f64 {
    let limit = FRAC_PI_2 - 1e-9;
    if phi.abs() > limit || one_es == 1.0 {
        phi
    } else {
        (phi.tan() / one_es).atan()
    }
}

impl S2 {
    fn prepare(&self, mut lon: f64, mut lat: f64) -> Result<(f64, f64), TransformError> {
        if !lon.is_finite() || !lat.is_finite() {
            return Err(TransformError::NonFinite);
        }
        if lat.abs() > FRAC_PI_2 {
            if lat > 0.0 {
                if lat - FRAC_PI_2 > EPS_LAT {
                    return Err(TransformError::InvalidCoord);
                }
                lat = FRAC_PI_2;
            } else if lat - FRAC_PI_2 < -EPS_LAT {
                return Err(TransformError::InvalidCoord);
            } else {
                lat = -FRAC_PI_2;
            }
        }
        if self.frame.geoc {
            lat = geocentric_latitude(lat, self.frame.earth.one_es());
        }
        if !self.frame.over {
            lon = adjlon(lon);
        }
        Ok((lon, lat))
    }

    fn sphere_latitude(&self, phi: f64) -> f64 {
        if self.frame.earth.es() == 0.0 {
            phi
        } else {
            let ratio = self.frame.earth.b() / self.frame.earth.a();
            (ratio * ratio * phi.tan()).atan()
        }
    }

    fn geodetic_latitude(&self, phi: f64) -> f64 {
        if self.frame.earth.es() == 0.0 {
            return phi;
        }
        let negative = phi < 0.0;
        let tanphi = phi.tan();
        let ratio = self.frame.earth.b() / self.frame.earth.a();
        let xa = self.frame.earth.b() / (tanphi * tanphi + ratio * ratio).sqrt();
        let mut phi =
            ((self.frame.earth.a() * self.frame.earth.a() - xa * xa).sqrt() / (ratio * xa)).atan();
        if negative {
            phi = -phi;
        }
        phi
    }
}

impl Step for S2 {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lon, lat) = self.prepare(coord.x, coord.y)?;
        let lat = self.sphere_latitude(lat);
        let (u, v) = xyz_to_face_uv(
            self.face,
            lat.cos() * lon.cos(),
            lat.cos() * lon.sin(),
            lat.sin(),
        );
        let s = uv_to_st(u, self.uv);
        let t = uv_to_st(v, self.uv);
        Ok(Coord {
            x: self.frame.fr_meter * (s + self.frame.x0),
            y: self.frame.fr_meter * (t + self.frame.y0),
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
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
        let s = self.frame.to_meter * coord.x - self.frame.x0;
        let t = self.frame.to_meter * coord.y - self.frame.y0;
        let (x, y, z) = face_uv_to_xyz(self.face, st_to_uv(s, self.uv), st_to_uv(t, self.uv));
        let phi = self.geodetic_latitude((-z).acos() - FRAC_PI_2);
        Ok(Coord {
            x: y.atan2(x),
            y: phi,
            z: self.frame.vto_meter * coord.z - self.frame.z0,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "s2" {
        Some(&["UVtoST"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "s2" {
        return Ok(None);
    }
    let uv = match params.text("UVtoST") {
        None | Some("quadratic") => Uv::Quadratic,
        Some("linear") => Uv::Linear,
        Some("tangent") => Uv::Tangent,
        Some("none") => Uv::None,
        Some(_) => return Err(CreateError::IllegalValue("UVtoST")),
    };
    Ok(Some(Box::new(S2 {
        face: face_from_origin(frame.phi0, frame.lam0),
        frame: frame.clone(),
        uv,
    })))
}
