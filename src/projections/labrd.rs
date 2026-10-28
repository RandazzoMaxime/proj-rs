//! Laborde projection, the Madagascar cylinder.
//!
//! `lat_0 = 0` is rejected. `k_0` is applied inside the kernel. The result
//! is in units of the semi-major axis.

#![allow(
    clippy::pedantic,
    reason = "the series and the complex rotation follow labrd.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_4;

const EPS: f64 = 1e-10;

struct Labrd {
    k_rg: f64,
    p0s: f64,
    big_a: f64,
    c_shift: f64,
    ca: f64,
    cb: f64,
    cc: f64,
    cd: f64,
    e: f64,
    k0: f64,
    phi0: f64,
    one_es: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "labrd" {
        Some(&["azi"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "labrd" {
        return Ok(None);
    }
    if frame.phi0 == 0.0 {
        return Err(CreateError::IllegalValue("lat_0"));
    }
    let azi = params.radians("azi")?.unwrap_or(0.0);
    let es = frame.earth.es();
    let e = frame.earth.e();
    let one_es = frame.earth.one_es();
    let sinp = frame.phi0.sin();
    let t = 1.0 - es * sinp * sinp;
    let n_radius = 1.0 / t.sqrt();
    let radius = one_es * n_radius / t;
    let k_rg = frame.k0 * (n_radius * radius).sqrt();
    let p0s = ((radius / n_radius).sqrt() * frame.phi0.tan()).atan();
    let big_a = sinp / p0s.sin();
    let e_sin = e * sinp;
    let c_shift = 0.5 * e * big_a * ((1.0 + e_sin) / (1.0 - e_sin)).ln()
        - big_a * (FRAC_PI_4 + 0.5 * frame.phi0).tan().ln()
        + (FRAC_PI_4 + 0.5 * p0s).tan().ln();
    let double = azi + azi;
    let mut cb = 1.0 / (12.0 * k_rg * k_rg);
    let ca = (1.0 - double.cos()) * cb;
    cb *= double.sin();
    let cc = 3.0 * (ca * ca - cb * cb);
    let cd = 6.0 * ca * cb;
    Ok(Some(classic_step(
        frame,
        Labrd {
            k_rg,
            p0s,
            big_a,
            c_shift,
            ca,
            cb,
            cc,
            cd,
            e,
            k0: frame.k0,
            phi0: frame.phi0,
            one_es,
        },
    )))
}

impl Kernel for Labrd {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let v1 = self.big_a * (FRAC_PI_4 + 0.5 * phi).tan().ln();
        let e_sin = self.e * phi.sin();
        let v2 = 0.5 * self.e * self.big_a * ((1.0 + e_sin) / (1.0 - e_sin)).ln();
        let ps = 2.0 * ((v1 - v2 + self.c_shift).exp().atan() - FRAC_PI_4);
        let i1 = ps - self.p0s;
        let cosps = ps.cos();
        let sinps = ps.sin();
        let cosps2 = cosps * cosps;
        let sinps2 = sinps * sinps;
        let i4 = self.big_a * cosps;
        let i2 = 0.5 * self.big_a * i4 * sinps;
        let i3 = i2 * self.big_a * self.big_a * (5.0 * cosps2 - sinps2) / 12.0;
        let mut i6 = i4 * self.big_a * self.big_a;
        let i5 = i6 * (cosps2 - sinps2) / 6.0;
        i6 *= self.big_a * self.big_a * (5.0 * cosps2 * cosps2 + sinps2 * (sinps2 - 18.0 * cosps2))
            / 120.0;
        let t = lam * lam;
        let mut x = self.k_rg * lam * (i4 + t * (i5 + t * i6));
        let mut y = self.k_rg * (i1 + t * (i2 + t * i3));
        let x2 = x * x;
        let y2 = y * y;
        let rot1 = 3.0 * x * y2 - x * x2;
        let rot2 = y * y2 - 3.0 * x2 * y;
        x += self.ca * rot1 + self.cb * rot2;
        y += self.ca * rot2 - self.cb * rot1;
        Ok((x, y))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        let x2 = x * x;
        let y2 = y * y;
        let v1 = 3.0 * x * y2 - x * x2;
        let v2 = y * y2 - 3.0 * x2 * y;
        let v3 = x * (5.0 * y2 * y2 + x2 * (-10.0 * y2 + x2));
        let v4 = y * (5.0 * x2 * x2 + y2 * (-10.0 * x2 + y2));
        x += -self.ca * v1 - self.cb * v2 + self.cc * v3 + self.cd * v4;
        y += self.cb * v1 - self.ca * v2 - self.cd * v3 + self.cc * v4;
        let ps = self.p0s + y / self.k_rg;
        let mut pe = ps + self.phi0 - self.p0s;
        for _ in 0..20 {
            let v1 = self.big_a * (FRAC_PI_4 + 0.5 * pe).tan().ln();
            let tpe = self.e * pe.sin();
            let v2 = 0.5 * self.e * self.big_a * ((1.0 + tpe) / (1.0 - tpe)).ln();
            let t = ps - 2.0 * ((v1 - v2 + self.c_shift).exp().atan() - FRAC_PI_4);
            pe += t;
            if t.abs() < EPS {
                break;
            }
        }
        let e_sin = self.e * pe.sin();
        let t = 1.0 - e_sin * e_sin;
        let rectified = self.one_es / (t * t.sqrt());
        let tan_ps = ps.tan();
        let t2 = tan_ps * tan_ps;
        let scale = self.k_rg * self.k_rg;
        let mut d = rectified * self.k0 * self.k_rg;
        let i7 = tan_ps / (2.0 * d);
        let i8 = tan_ps * (5.0 + 3.0 * t2) / (24.0 * d * scale);
        d = ps.cos() * self.k_rg * self.big_a;
        let i9 = 1.0 / d;
        d *= scale;
        let i10 = (1.0 + 2.0 * t2) / (6.0 * d);
        let i11 = (5.0 + t2 * (28.0 + 24.0 * t2)) / (120.0 * d * scale);
        let x2 = x * x;
        let phi = pe + x2 * (-i7 + i8 * x2);
        let lam = x * (i9 + x2 * (-i10 + x2 * i11));
        Ok((lam, phi))
    }
}
