//! Colombia Urban projection. `h_0` is a height in metres; the kernel stores
//! it in units of the semi-major axis, and the classic wrapper multiplies
//! the result by that axis.

#![allow(
    clippy::pedantic,
    reason = "the height scale and the meridional radius follow col_urban.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};

struct Urban {
    phi0: f64,
    h0: f64,
    rho0: f64,
    a_coeff: f64,
    b_coeff: f64,
    c_coeff: f64,
    d_coeff: f64,
    es: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "col_urban" {
        Some(&["h_0"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "col_urban" {
        return Ok(None);
    }
    let es = frame.earth.es();
    let phi0 = frame.phi0;
    let h0 = params.number("h_0")?.unwrap_or(0.0) / frame.earth.a();
    let sinphi0 = phi0.sin();
    let nu0 = nu(es, sinphi0);
    let rho0 = rho(es, sinphi0);
    Ok(Some(classic_step(
        frame,
        Urban {
            phi0,
            h0,
            rho0,
            a_coeff: 1.0 + h0 / nu0,
            b_coeff: phi0.tan() / (2.0 * rho0 * nu0),
            c_coeff: 1.0 + h0,
            d_coeff: rho0 * (1.0 + h0 / (1.0 - es)),
            es,
        },
    )))
}

impl Kernel for Urban {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let sinphi = phi.sin();
        let scaled = lam * nu(self.es, sinphi) * phi.cos();
        let sin_mid = (0.5 * (phi + self.phi0)).sin();
        let radius = rho(self.es, sin_mid);
        let height = 1.0 + self.h0 / radius;
        Ok((
            self.a_coeff * scaled,
            height * self.rho0 * ((phi - self.phi0) + self.b_coeff * scaled * scaled),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let ratio = x / self.c_coeff;
        let phi = self.phi0 + y / self.d_coeff - self.b_coeff * ratio * ratio;
        let sinphi = phi.sin();
        let lam = x / (self.c_coeff * nu(self.es, sinphi) * phi.cos());
        Ok((lam, phi))
    }
}

fn nu(es: f64, sinphi: f64) -> f64 {
    1.0 / (1.0 - es * sinphi * sinphi).sqrt()
}

fn rho(es: f64, sinphi: f64) -> f64 {
    (1.0 - es) / (1.0 - es * sinphi * sinphi).powf(1.5)
}
