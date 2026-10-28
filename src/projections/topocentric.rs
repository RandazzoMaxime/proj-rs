//! Geocentric to topocentric (east, north, up).
//!
//! The origin is either geocentric `X_0 Y_0 Z_0` or geographic `lon_0 lat_0 h_0`.
//! The two forms are mutually exclusive. Input and output are Cartesian metres.

#![allow(
    clippy::pedantic,
    reason = "the east-north-up rotation follows topocentric.cpp"
)]

use crate::engine::{
    cartesian_to_geodetic, geodetic_to_cartesian, Coord, CreateError, Frame, IoKind, Params, Step,
    TransformError,
};

struct Topo {
    x0: f64,
    y0: f64,
    z0: f64,
    sinphi: f64,
    cosphi: f64,
    sinlam: f64,
    coslam: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "topocentric" {
        Some(&["X_0", "Y_0", "Z_0", "h_0"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "topocentric" {
        return Ok(None);
    }
    let has_x = params.flag("X_0");
    let has_y = params.flag("Y_0");
    let has_z = params.flag("Z_0");
    let has_lon = params.flag("lon_0");
    let has_lat = params.flag("lat_0");
    let has_h = params.flag("h_0");
    if !has_x && !has_lon {
        return Err(CreateError::MissingArgument("X_0"));
    }
    if (has_x || has_y || has_z) && (has_lon || has_lat || has_h) {
        return Err(CreateError::IllegalValue("X_0"));
    }
    if has_x && (!has_y || !has_z) {
        return Err(CreateError::MissingArgument("Z_0"));
    }
    if has_lon && !has_lat {
        return Err(CreateError::MissingArgument("lat_0"));
    }
    let (x0, y0, z0, lam, phi) = if has_x {
        let x0 = params.number("X_0")?.unwrap_or(0.0);
        let y0 = params.number("Y_0")?.unwrap_or(0.0);
        let z0 = params.number("Z_0")?.unwrap_or(0.0);
        let (lam, phi, _) = cartesian_to_geodetic(frame.earth, x0, y0, z0);
        (x0, y0, z0, lam, phi)
    } else {
        let height = params.number("h_0")?.unwrap_or(0.0);
        let (x0, y0, z0) = geodetic_to_cartesian(frame.earth, frame.lam0, frame.phi0, height);
        (x0, y0, z0, frame.lam0, frame.phi0)
    };
    Ok(Some(Box::new(Topo {
        x0,
        y0,
        z0,
        sinphi: phi.sin(),
        cosphi: phi.cos(),
        sinlam: lam.sin(),
        coslam: lam.cos(),
    })))
}

impl Step for Topo {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() || coord.z.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: f64::NAN,
                t: coord.t,
            });
        }
        let dx = coord.x - self.x0;
        let dy = coord.y - self.y0;
        let dz = coord.z - self.z0;
        Ok(Coord {
            x: -dx * self.sinlam + dy * self.coslam,
            y: -dx * self.sinphi * self.coslam - dy * self.sinphi * self.sinlam + dz * self.cosphi,
            z: dx * self.cosphi * self.coslam + dy * self.cosphi * self.sinlam + dz * self.sinphi,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() || coord.z.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: f64::NAN,
                t: coord.t,
            });
        }
        let (x, y, z) = (coord.x, coord.y, coord.z);
        Ok(Coord {
            x: self.x0 - x * self.sinlam - y * self.sinphi * self.coslam
                + z * self.cosphi * self.coslam,
            y: self.y0 + x * self.coslam - y * self.sinphi * self.sinlam
                + z * self.cosphi * self.sinlam,
            z: self.z0 + y * self.cosphi + z * self.sinphi,
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
        (1.0, 0.0)
    }
}
