//! Equidistant cylindrical (plate carrée).
//!
//! Port of PROJ 9.9 `src/projections/eqc.cpp`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ eqc.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;

pub(crate) const KEYS: &[&str] = &["lat_ts"];

struct Eqc {
    rc: f64,
    phi0: f64,
    meridian: Option<Meridian>,
    origin: f64,
}

impl Kernel for Eqc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let y = if let Some(meridian) = self.meridian {
            meridian.forward(phi) - self.origin
        } else {
            phi - self.phi0
        };
        Ok((self.rc * lam, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if self.rc == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let phi = if let Some(meridian) = self.meridian {
            meridian.inverse(y + self.origin)
        } else {
            y + self.phi0
        };
        Ok((x / self.rc, phi))
    }
}

pub(crate) fn build(
    params: &Params,
    frame: &Frame,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let lat_ts = params.radians("lat_ts")?.unwrap_or(0.0);
    let cos_ts = lat_ts.cos();
    if cos_ts <= 0.0 {
        return Err(CreateError::IllegalValue("lat_ts"));
    }
    let (rc, meridian, origin) = if frame.earth.es() == 0.0 {
        (cos_ts, None, 0.0)
    } else {
        let sin_ts = lat_ts.sin();
        let nu = 1.0 / (1.0 - frame.earth.es() * sin_ts * sin_ts).sqrt();
        let meridian = Meridian::new(frame.earth.n());
        let origin = meridian.forward(frame.phi0);
        (nu * cos_ts, Some(meridian), origin)
    };
    Ok(classic_step(
        frame,
        Eqc {
            rc,
            phi0: frame.phi0,
            meridian,
            origin,
        },
    ))
}
