//! Oblique Mercator.
//!
//! Port of PROJ 9.9 `src/projections/omerc.cpp`. Setup replaces `lam0`; the
//! kernel applies that meridian instead of the caller's `+lon_0`, which PROJ
//! ignores.

#![allow(clippy::pedantic, reason = "formulae follow PROJ omerc.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aasin, phi2, tsfn};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const TOL: f64 = 1e-7;
const EPS: f64 = 1e-10;

struct Omerc {
    a: f64,
    b: f64,
    e_coef: f64,
    ar_b: f64,
    br_a: f64,
    r_b: f64,
    singam: f64,
    cosgam: f64,
    sinrot: f64,
    cosrot: f64,
    v_pole_n: f64,
    v_pole_s: f64,
    u_0: f64,
    no_rot: bool,
    ecc: f64,
    shift: f64,
    over: bool,
}

fn asin_arg(v: f64, name: &'static str) -> Result<f64, CreateError> {
    aasin(v).map_err(|()| CreateError::IllegalValue(name))
}

impl Omerc {
    fn relative(&self, lam: f64) -> f64 {
        let lam = lam - self.shift;
        if self.over {
            lam
        } else {
            adjlon(lam)
        }
    }
}

impl Kernel for Omerc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.relative(lam);
        let (u, v) = if (phi.abs() - FRAC_PI_2).abs() > EPS {
            let w = self.e_coef / tsfn(phi, phi.sin(), self.ecc).powf(self.b);
            let s = 0.5 * (w - 1.0 / w);
            let t = 0.5 * (w + 1.0 / w);
            let v_sin = (self.b * lam).sin();
            let u_arg = (s * self.singam - v_sin * self.cosgam) / t;
            if (u_arg.abs() - 1.0).abs() < EPS {
                return Err(TransformError::OutsideDomain);
            }
            let v = 0.5 * self.ar_b * ((1.0 - u_arg) / (1.0 + u_arg)).ln();
            let temp = (self.b * lam).cos();
            let u = if temp.abs() < TOL {
                self.a * lam
            } else {
                self.ar_b * (s * self.cosgam + v_sin * self.singam).atan2(temp)
            };
            (u, v)
        } else {
            let v = if phi > 0.0 {
                self.v_pole_n
            } else {
                self.v_pole_s
            };
            (self.ar_b * phi, v)
        };
        if self.no_rot {
            return Ok((u, v));
        }
        let u = u - self.u_0;
        Ok((
            v * self.cosrot + u * self.sinrot,
            u * self.cosrot - v * self.sinrot,
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let (u, v) = if self.no_rot {
            (x, y)
        } else {
            (
                y * self.cosrot + x * self.sinrot + self.u_0,
                x * self.cosrot - y * self.sinrot,
            )
        };
        let qp = (-self.br_a * v).exp();
        if qp == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let sp = 0.5 * (qp - 1.0 / qp);
        let tp = 0.5 * (qp + 1.0 / qp);
        let vp = (self.br_a * u).sin();
        let up = (vp * self.cosgam + sp * self.singam) / tp;
        let (lam, phi) = if (up.abs() - 1.0).abs() < EPS {
            (0.0, if up < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 })
        } else {
            let ratio = self.e_coef / ((1.0 + up) / (1.0 - up)).sqrt();
            let phi = phi2(ratio.powf(1.0 / self.b), self.ecc)
                .map_err(|()| TransformError::OutsideDomain)?;
            let lam =
                -self.r_b * (sp * self.cosgam - vp * self.singam).atan2((self.br_a * u).cos());
            (lam, phi)
        };
        Ok((lam + self.shift, phi))
    }
}

const KEYS: &[&str] = &[
    "alpha", "gamma", "no_off", "no_uoff", "no_rot", "lonc", "lon_1", "lat_1", "lon_2", "lat_2",
];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "omerc" {
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
    if name != "omerc" {
        return Ok(None);
    }
    let no_rot = params.flag("no_rot");
    let has_alpha = params.flag("alpha");
    let has_gamma = params.flag("gamma");
    let alpha_c = params.radians("alpha")?.unwrap_or(0.0);
    let mut gamma = params.radians("gamma")?.unwrap_or(0.0);
    let (lam1, mut lam2, phi1, phi2, lamc, no_off) = if has_alpha || has_gamma {
        let no_off = params.flag("no_off") || params.flag("no_uoff");
        (
            0.0,
            0.0,
            0.0,
            0.0,
            params.radians("lonc")?.unwrap_or(0.0),
            no_off,
        )
    } else {
        let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
        let phi2 = params.radians("lat_2")?.unwrap_or(0.0);
        if phi1.abs() > FRAC_PI_2 - TOL {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        if phi2.abs() > FRAC_PI_2 - TOL {
            return Err(CreateError::IllegalValue("lat_2"));
        }
        if (phi1 - phi2).abs() <= TOL {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        if phi1.abs() <= TOL {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        if (frame.phi0.abs() - FRAC_PI_2).abs() <= TOL {
            return Err(CreateError::IllegalValue("lat_0"));
        }
        (
            params.radians("lon_1")?.unwrap_or(0.0),
            params.radians("lon_2")?.unwrap_or(0.0),
            phi1,
            phi2,
            0.0,
            false,
        )
    };

    let com = frame.earth.one_es().sqrt();
    let ecc = frame.earth.e();
    let (b_coef, a_coef, d, mut f_term, e_coef) = if frame.phi0.abs() > EPS {
        let sinph0 = frame.phi0.sin();
        let cosph0 = frame.phi0.cos();
        let con = 1.0 - frame.earth.es() * sinph0 * sinph0;
        let mut b_coef = cosph0 * cosph0;
        b_coef = (1.0 + frame.earth.es() * b_coef * b_coef / frame.earth.one_es()).sqrt();
        let a_coef = b_coef * frame.k0 * com / con;
        let d = b_coef * com / (cosph0 * con.sqrt());
        let mut f_term = d * d - 1.0;
        if f_term <= 0.0 {
            f_term = 0.0;
        } else {
            f_term = f_term.sqrt();
            if frame.phi0 < 0.0 {
                f_term = -f_term;
            }
        }
        f_term += d;
        let e_coef = f_term * tsfn(frame.phi0, sinph0, ecc).powf(b_coef);
        (b_coef, a_coef, d, f_term, e_coef)
    } else {
        (1.0 / com, frame.k0, 1.0, 1.0, 1.0)
    };

    let (gamma0, alpha_c, lam0) = if has_alpha || has_gamma {
        let gamma0 = if has_alpha {
            asin_arg(alpha_c.sin() / d, "alpha")?
        } else {
            gamma
        };
        let alpha_for_offset = if has_alpha {
            alpha_c
        } else {
            asin_arg(d * gamma0.sin(), "gamma")?
        };
        if has_alpha && !has_gamma {
            gamma = alpha_c;
        }
        if (frame.phi0.abs() - FRAC_PI_2).abs() <= TOL {
            return Err(CreateError::IllegalValue("lat_0"));
        }
        let lam0 = lamc - asin_arg(0.5 * (f_term - 1.0 / f_term) * gamma0.tan(), "lonc")? / b_coef;
        (gamma0, alpha_for_offset, lam0)
    } else {
        let h = tsfn(phi1, phi1.sin(), ecc).powf(b_coef);
        let l = tsfn(phi2, phi2.sin(), ecc).powf(b_coef);
        f_term = e_coef / h;
        let p = (l - h) / (l + h);
        if p == 0.0 {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        let mut j = e_coef * e_coef;
        j = (j - l * h) / (j + l * h);
        let span = lam1 - lam2;
        if span < -PI {
            lam2 -= 2.0 * PI;
        } else if span > PI {
            lam2 += 2.0 * PI;
        }
        let lam0 = adjlon(
            0.5 * (lam1 + lam2) - (j * (0.5 * b_coef * (lam1 - lam2)).tan() / p).atan() / b_coef,
        );
        let denom = f_term - 1.0 / f_term;
        if denom == 0.0 {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        let gamma0 = (2.0 * (b_coef * adjlon(lam1 - lam0)).sin() / denom).atan();
        let gamma_rot = asin_arg(d * gamma0.sin(), "lat_1")?;
        gamma = gamma_rot;
        (gamma0, gamma_rot, lam0)
    };

    let r_b = 1.0 / b_coef;
    let ar_b = a_coef * r_b;
    let u_0 = if no_off {
        0.0
    } else {
        let mut u_0 = (ar_b * ((d * d - 1.0).sqrt() / alpha_c.cos()).atan()).abs();
        if frame.phi0 < 0.0 {
            u_0 = -u_0;
        }
        u_0
    };
    let half = 0.5 * gamma0;
    Ok(Some(classic_step(
        frame,
        Omerc {
            a: a_coef,
            b: b_coef,
            e_coef,
            ar_b,
            br_a: 1.0 / ar_b,
            r_b,
            singam: gamma0.sin(),
            cosgam: gamma0.cos(),
            sinrot: gamma.sin(),
            cosrot: gamma.cos(),
            v_pole_n: ar_b * (FRAC_PI_4 - half).tan().ln(),
            v_pole_s: ar_b * (FRAC_PI_4 + half).tan().ln(),
            u_0,
            no_rot,
            ecc,
            shift: lam0 - frame.lam0,
            over: frame.over,
        },
    )))
}
