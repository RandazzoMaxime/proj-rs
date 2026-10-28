//! Equal-area cylindrical.
//!
//! Port of PROJ 9.9 `src/projections/cea.cpp`. `+lat_ts` replaces `k0`.

#![allow(clippy::pedantic, reason = "formulae follow PROJ cea.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_inverse, authalic_q};

const EPS: f64 = 1e-10;

struct Cea {
    k0: f64,
    ellips: bool,
    e: f64,
    n: f64,
    es: f64,
    one_es: f64,
    qp: f64,
}

impl Kernel for Cea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let x = self.k0 * lam;
        let y = if self.ellips {
            0.5 * authalic_q(phi.sin(), self.e, self.one_es) / self.k0
        } else {
            phi.sin() / self.k0
        };
        Ok((x, y))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let lam = x / self.k0;
        if self.ellips {
            let beta = (2.0 * y * self.k0 / self.qp).asin();
            let phi = authalic_inverse(beta, self.n, self.e, self.es, self.one_es, self.qp);
            return Ok((lam, phi));
        }
        y *= self.k0;
        let t = y.abs();
        if t - EPS > 1.0 {
            return Err(TransformError::OutsideDomain);
        }
        let phi = if t >= 1.0 {
            if y < 0.0 {
                -std::f64::consts::FRAC_PI_2
            } else {
                std::f64::consts::FRAC_PI_2
            }
        } else {
            y.asin()
        };
        Ok((lam, phi))
    }
}

const KEYS: &[&str] = &["lat_ts"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "cea" {
        Some(KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "cea" {
        return Ok(None);
    }
    let mut k0 = frame.k0;
    let ellips = frame.earth.es() != 0.0;
    if let Some(lat_ts) = params.radians("lat_ts")? {
        k0 = lat_ts.cos();
        if k0 < 0.0 {
            return Err(CreateError::IllegalValue("lat_ts"));
        }
        if ellips {
            let t = lat_ts.sin();
            k0 /= (1.0 - frame.earth.es() * t * t).sqrt();
        }
    }
    let e = frame.earth.e();
    let one_es = frame.earth.one_es();
    let qp = if ellips {
        authalic_q(1.0, e, one_es)
    } else {
        0.0
    };
    Ok(Some(classic_step(
        frame,
        Cea {
            k0,
            ellips,
            e,
            n: frame.earth.n(),
            es: frame.earth.es(),
            one_es,
            qp,
        },
    )))
}
