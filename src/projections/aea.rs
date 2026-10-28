#![allow(
    clippy::pedantic,
    reason = "Albers formulae are transcribed from PROJ aea.cpp"
)]

//! Albers equal area and Lambert equal-area conic.
//!
//! Port of PROJ 9.9 `src/projections/aea.cpp`.

use crate::engine::{
    authalic_lat_inverse, authalic_q, classic_step, proj_msfn, CreateError, Frame, Kernel, Params,
    TransformError,
};
use std::f64::consts::FRAC_PI_2;

const EPS10: f64 = 1e-10;
const TOL7: f64 = 1e-7;

pub(crate) const KEYS: &[&str] = &["lat_1", "lat_2", "south"];

struct Aea {
    ellips: bool,
    cone_n: f64,
    n2: f64,
    c: f64,
    dd: f64,
    rho0: f64,
    ec: f64,
    qp: f64,
    e: f64,
    es: f64,
    one_es: f64,
    third_n: f64,
}

impl Kernel for Aea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let rho_arg = self.c
            - if self.ellips {
                self.cone_n * authalic_q(phi.sin(), self.e, self.one_es)
            } else {
                self.n2 * phi.sin()
            };
        if rho_arg < 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let rho = self.dd * rho_arg.sqrt();
        let lam_n = lam * self.cone_n;
        Ok((rho * lam_n.sin(), self.rho0 - rho * lam_n.cos()))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y = self.rho0 - y;
        let mut rho = x.hypot(y);
        if rho == 0.0 {
            let phi = if self.cone_n > 0.0 {
                FRAC_PI_2
            } else {
                -FRAC_PI_2
            };
            return Ok((0.0, phi));
        }
        if self.cone_n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }
        let ratio = rho / self.dd;
        let phi = if self.ellips {
            let qs = (self.c - ratio * ratio) / self.cone_n;
            if (self.ec - qs.abs()).abs() > TOL7 {
                if qs.abs() > 2.0 {
                    return Err(TransformError::OutsideDomain);
                }
                let sine = (qs / self.qp).clamp(-1.0, 1.0);
                authalic_lat_inverse(
                    sine.asin(),
                    self.third_n,
                    self.e,
                    self.es,
                    self.one_es,
                    self.qp,
                )
            } else if qs < 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            }
        } else {
            let qs = (self.c - ratio * ratio) / self.n2;
            if qs.abs() <= 1.0 {
                qs.asin()
            } else if qs < 0.0 {
                -FRAC_PI_2
            } else {
                FRAC_PI_2
            }
        };
        Ok((x.atan2(y) / self.cone_n, phi))
    }
}

pub(crate) fn build(
    params: &Params,
    frame: &Frame,
    leac: bool,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let (phi1, phi2) = if leac {
        let phi2 = params.radians("lat_1")?.unwrap_or(0.0);
        let phi1 = if params.flag("south") {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        };
        (phi1, phi2)
    } else {
        let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
        let phi2 = params.radians("lat_2")?.unwrap_or(0.0);
        (phi1, phi2)
    };
    if phi1.abs() > FRAC_PI_2 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    if phi2.abs() > FRAC_PI_2 {
        return Err(CreateError::IllegalValue("lat_2"));
    }
    if (phi1 + phi2).abs() < EPS10 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let earth = &frame.earth;
    let mut cone_n = phi1.sin();
    let sinphi = phi1.sin();
    let cosphi = phi1.cos();
    let secant = (phi1 - phi2).abs() >= EPS10;
    let ellips = earth.es() > 0.0;
    let (n2, c, dd, rho0, ec, qp) = if ellips {
        let m1 = proj_msfn(sinphi, cosphi, earth.es());
        let ml1 = authalic_q(sinphi, earth.e(), earth.one_es());
        if secant {
            let sin2 = phi2.sin();
            let cos2 = phi2.cos();
            let m2 = proj_msfn(sin2, cos2, earth.es());
            let ml2 = authalic_q(sin2, earth.e(), earth.one_es());
            if ml2 == ml1 {
                return Err(CreateError::IllegalValue("lat_2"));
            }
            cone_n = (m1 * m1 - m2 * m2) / (ml2 - ml1);
            if cone_n == 0.0 {
                return Err(CreateError::IllegalValue("lat_1"));
            }
        }
        let ec =
            1.0 - 0.5 * earth.one_es() * ((1.0 - earth.e()) / (1.0 + earth.e())).ln() / earth.e();
        let c = m1 * m1 + cone_n * ml1;
        let dd = 1.0 / cone_n;
        let rho_arg = c - cone_n * authalic_q(frame.phi0.sin(), earth.e(), earth.one_es());
        if rho_arg < 0.0 {
            return Err(CreateError::IllegalValue("lat_0"));
        }
        let rho0 = dd * rho_arg.sqrt();
        (
            0.0,
            c,
            dd,
            rho0,
            ec,
            authalic_q(1.0, earth.e(), earth.one_es()),
        )
    } else {
        if secant {
            cone_n = 0.5 * (cone_n + phi2.sin());
        }
        if cone_n == 0.0 {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        let n2 = cone_n + cone_n;
        let c = cosphi * cosphi + n2 * sinphi;
        let dd = 1.0 / cone_n;
        let rho_arg = c - n2 * frame.phi0.sin();
        if rho_arg < 0.0 {
            return Err(CreateError::IllegalValue("lat_0"));
        }
        let rho0 = dd * rho_arg.sqrt();
        (n2, c, dd, rho0, 0.0, 0.0)
    };
    Ok(classic_step(
        frame,
        Aea {
            ellips,
            cone_n,
            n2,
            c,
            dd,
            rho0,
            ec,
            qp,
            e: earth.e(),
            es: earth.es(),
            one_es: earth.one_es(),
            third_n: earth.n(),
        },
    ))
}
