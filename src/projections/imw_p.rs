//! International Map of the World polyconic.
//!
//! `lat_1` and `lat_2` are required and must not sum or differ by zero.
//! Meridian distance is the same rectifying series as `pj_mlfn`. The kernel
//! returns units of the semi-major axis.

#![allow(
    clippy::pedantic,
    reason = "the sphere intersection and the inverse iteration follow imw_p.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::Meridian;

const TOL: f64 = 1e-10;
const EPS: f64 = 1e-10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    NeitherZero,
    Phi1Zero,
    Phi2Zero,
}

struct Imw {
    p_coeff: f64,
    pp: f64,
    q_coeff: f64,
    qp: f64,
    r1: f64,
    r2: f64,
    sin1: f64,
    sin2: f64,
    c2: f64,
    phi1: f64,
    phi2: f64,
    mode: Mode,
    es: f64,
    meridian: Meridian,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "imw_p" {
        Some(&["lat_1", "lat_2", "lon_1"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "imw_p" {
        return Ok(None);
    }
    if !params.flag("lat_1") || !params.flag("lat_2") {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    let mut phi1 = params.radians("lat_1")?.unwrap_or(0.0);
    let mut phi2 = params.radians("lat_2")?.unwrap_or(0.0);
    let del = 0.5 * (phi2 - phi1);
    let sig = 0.5 * (phi2 + phi1);
    if del.abs() < EPS || sig.abs() < EPS {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    if phi2 < phi1 {
        std::mem::swap(&mut phi1, &mut phi2);
    }
    let lam1 = if params.flag("lon_1") {
        params.radians("lon_1")?.unwrap_or(0.0)
    } else {
        let degrees = sig.abs().to_degrees();
        let width = if degrees <= 60.0 {
            2.0_f64
        } else if degrees <= 76.0 {
            4.0
        } else {
            8.0
        };
        width.to_radians()
    };
    let es = frame.earth.es();
    let (mode, x1, y1, sin1, r1) = if phi1 != 0.0 {
        let (x, y, sine, radius) = standard_parallel(es, phi1, lam1);
        (Mode::NeitherZero, x, y, sine, radius)
    } else {
        (Mode::Phi1Zero, lam1, 0.0, 0.0, 0.0)
    };
    let (mode, x2, t2, sin2, r2) = if phi2 != 0.0 {
        let (x, y, sine, radius) = standard_parallel(es, phi2, lam1);
        (mode, x, y, sine, radius)
    } else {
        (Mode::Phi2Zero, lam1, 0.0, 0.0, 0.0)
    };
    let meridian = Meridian::new(frame.earth.n());
    let m1 = meridian.forward(phi1);
    let m2 = meridian.forward(phi2);
    let dt = m2 - m1;
    let ds = x2 - x1;
    let y2 = (dt * dt - ds * ds).sqrt() + y1;
    let inv = 1.0 / dt;
    Ok(Some(classic_step(
        frame,
        Imw {
            p_coeff: (m2 * y1 - m1 * y2) * inv,
            pp: (m2 * x1 - m1 * x2) * inv,
            q_coeff: (y2 - y1) * inv,
            qp: (x2 - x1) * inv,
            r1,
            r2,
            sin1,
            sin2,
            c2: y2 - t2,
            phi1,
            phi2,
            mode,
            es,
            meridian,
        },
    )))
}

impl Kernel for Imw {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let (x, y, _) = self.locate(lam, phi, 0.0);
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut phi = self.phi2;
        let mut lam = x / phi.cos();
        let mut yc = 0.0;
        let mut tx;
        let mut ty;
        let mut i = 0;
        loop {
            let located = self.locate(lam, phi, yc);
            tx = located.0;
            ty = located.1;
            yc = located.2;
            let denom = ty - yc;
            if denom != 0.0 || (ty - y).abs() > TOL {
                if denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                phi = ((phi - self.phi1) * (y - yc) / denom) + self.phi1;
            }
            if tx != 0.0 && (tx - x).abs() > TOL {
                lam = lam * x / tx;
            }
            i += 1;
            if i >= 1000 || ((tx - x).abs() <= TOL && (ty - y).abs() <= TOL) {
                break;
            }
        }
        if i == 1000 {
            Err(TransformError::OutsideDomain)
        } else {
            Ok((lam, phi))
        }
    }
}

impl Imw {
    /// `(x, y, yc)`. When `phi` is zero, `yc` is left unchanged, as in `loc_for`.
    fn locate(&self, lam: f64, phi: f64, yc_in: f64) -> (f64, f64, f64) {
        if phi == 0.0 {
            return (lam, 0.0, yc_in);
        }
        let sine = phi.sin();
        let distance = self.meridian.forward(phi);
        let xa = self.pp + self.qp * distance;
        let ya = self.p_coeff + self.q_coeff * distance;
        let radius = 1.0 / (phi.tan() * (1.0 - self.es * sine * sine).sqrt());
        let mut c = (radius * radius - xa * xa).sqrt();
        if phi < 0.0 {
            c = -c;
        }
        c += ya - radius;
        let (xb, yb) = if self.mode == Mode::Phi2Zero {
            (lam, self.c2)
        } else {
            let t = lam * self.sin2;
            (self.r2 * t.sin(), self.c2 + self.r2 * (1.0 - t.cos()))
        };
        let (xc, yc) = if self.mode == Mode::Phi1Zero {
            (lam, 0.0)
        } else {
            let t = lam * self.sin1;
            (self.r1 * t.sin(), self.r1 * (1.0 - t.cos()))
        };
        let slope = (xb - xc) / (yb - yc);
        let intercept = xc + slope * (c + radius - yc);
        let mut x =
            slope * (radius * radius * (1.0 + slope * slope) - intercept * intercept).sqrt();
        if phi > 0.0 {
            x = -x;
        }
        x = (intercept + x) / (1.0 + slope * slope);
        let mut y = (radius * radius - x * x).sqrt();
        if phi > 0.0 {
            y = -y;
        }
        y += c + radius;
        (x, y, yc)
    }
}

fn standard_parallel(es: f64, phi: f64, lam1: f64) -> (f64, f64, f64, f64) {
    let sine = phi.sin();
    let radius = 1.0 / (phi.tan() * (1.0 - es * sine * sine).sqrt());
    let angle = lam1 * sine;
    let y = radius * (1.0 - angle.cos());
    let x = radius * angle.sin();
    (x, y, sine, radius)
}
