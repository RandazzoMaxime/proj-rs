//! Oblique stereographic alternative.
//!
//! Port of PROJ 9.9 `src/projections/sterea.cpp` and `src/gauss.cpp`.

#![allow(
    clippy::pedantic,
    reason = "formulae follow PROJ sterea.cpp and gauss.cpp"
)]

use crate::engine::{classic_step_batch, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{auxlat_coeffs, auxlat_convert, AuxLat};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

// The vector path of `sterea`, four points at a time.
#[path = "sterea_batch.rs"]
mod sterea_batch;

#[cfg(simd_lanes)]
const DEL_TOL: f64 = 1e-14;

#[derive(Clone, Copy)]
struct Gauss {
    c: f64,
    k: f64,
    e: f64,
    ratexp: f64,
    inv_c: f64,
    cgb: [f64; 6],
}

fn srat(esinp: f64, ratexp: f64) -> f64 {
    ((1.0 - esinp) / (1.0 + esinp)).powf(ratexp)
}

fn gauss_ini(e: f64, n: f64, phi0: f64) -> Result<(Gauss, f64, f64), CreateError> {
    let es = e * e;
    let sphi = phi0.sin();
    let cphi = phi0.cos();
    // `gauss.cpp` squares cosine once, then uses `cphi * cphi`, so this is cos⁴.
    let cphi2 = cphi * cphi;
    let rc = (1.0 - es).sqrt() / (1.0 - es * sphi * sphi);
    let c = (1.0 + es * cphi2 * cphi2 / (1.0 - es)).sqrt();
    if c == 0.0 {
        return Err(gauss_failure());
    }
    let chi = (sphi / c).asin();
    let ratexp = 0.5 * c * e;
    let srat_val = srat(e * sphi, ratexp);
    if srat_val == 0.0 {
        return Err(gauss_failure());
    }
    let k = if 0.5 * phi0 + FRAC_PI_4 < 1e-10 {
        1.0 / srat_val
    } else {
        (0.5 * chi + FRAC_PI_4).tan() / ((0.5 * phi0 + FRAC_PI_4).tan().powf(c) * srat_val)
    };
    let cgb = auxlat_coeffs(n, AuxLat::Conformal, AuxLat::Geographic)
        .expect("conformal to geographic is generated");
    let inv_c = 1.0 / c;
    Ok((
        Gauss {
            c,
            k,
            e,
            ratexp,
            inv_c,
            cgb,
        },
        chi,
        rc,
    ))
}

impl Gauss {
    fn forward(self, lam: f64, phi: f64) -> (f64, f64) {
        let phi = 2.0
            * (self.k
                * (0.5 * phi + FRAC_PI_4).tan().powf(self.c)
                * srat(self.e * phi.sin(), self.ratexp))
            .atan()
            - FRAC_PI_2;
        (self.c * lam, phi)
    }

    /// The Gauss sphere is the conformal latitude raised to `c`, so the geodetic latitude
    /// comes from the conformal series, not from PROJ's fixed-point loop.
    fn inverse(self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let tan_conformal = ((0.5 * phi + FRAC_PI_4).tan() / self.k).powf(self.inv_c);
        let conformal = 2.0 * tan_conformal.atan() - FRAC_PI_2;
        let phi = auxlat_convert(conformal, &self.cgb);
        if !phi.is_finite() {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam / self.c, phi))
    }
}

#[derive(Clone, Copy)]
struct Sterea {
    gauss: Gauss,
    sinc0: f64,
    cosc0: f64,
    r2: f64,
    k0: f64,
}

impl Kernel for Sterea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let (lam, phi) = self.gauss.forward(lam, phi);
        let sinc = phi.sin();
        let cosc = phi.cos();
        let cosl = lam.cos();
        let denom = 1.0 + self.sinc0 * sinc + self.cosc0 * cosc * cosl;
        if denom == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let k = self.k0 * self.r2 / denom;
        Ok((
            k * cosc * lam.sin(),
            k * (self.cosc0 * sinc - self.sinc0 * cosc * cosl),
        ))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        x /= self.k0;
        y /= self.k0;
        let rho = x.hypot(y);
        let (lam, phi) = if rho != 0.0 {
            let c = 2.0 * rho.atan2(self.r2);
            let sinc = c.sin();
            let cosc = c.cos();
            let phi = (cosc * self.sinc0 + y * sinc * self.cosc0 / rho).asin();
            let lam = (x * sinc).atan2(rho * self.cosc0 * cosc - y * self.sinc0 * sinc);
            (lam, phi)
        } else {
            (0.0, self.sinc0.atan2(self.cosc0))
        };
        self.gauss.inverse(lam, phi)
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "sterea" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "sterea" {
        return Ok(None);
    }
    let (gauss, chi, radius) = gauss_ini(frame.earth.e(), frame.earth.n(), frame.phi0)?;
    let kernel = Sterea {
        gauss,
        sinc0: chi.sin(),
        cosc0: chi.cos(),
        r2: 2.0 * radius,
        k0: frame.k0,
    };
    Ok(Some(classic_step_batch(
        frame,
        kernel,
        crate::projections::unit_batch::UnitBatch(kernel, frame.earth),
    )))
}

/// `pj_gauss_ini` failed: `PJ_PROJECTION(sterea)` returns `PROJ_ERR_OTHER` and logs nothing.
fn gauss_failure() -> CreateError {
    CreateError::rejected(crate::errno::PROJ_ERR_OTHER, "")
}
