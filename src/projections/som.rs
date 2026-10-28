//! Space Oblique Mercator, including the MISR and Landsat specializations.
//!
//! PROJ overwrites `lam0` with the ascending longitude. The kernel receives
//! longitude already reduced by `lon_0`, so it applies the remaining shift
//! `asc_lon - lon_0`.

#![allow(
    clippy::pedantic,
    reason = "the series and the iteration follow som.cpp, including its exit tests"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::aasin;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const TOL: f64 = 1e-7;
const DEG: f64 = 0.017_453_292_519_943_296;
const TWOPI: f64 = 6.283_185_307_179_586_476_93;
const PI_HALFPI: f64 = 4.712_388_980_384_689_857_69;
const TWOPI_HALFPI: f64 = 7.853_981_633_974_483_096_16;

struct Som {
    shift: f64,
    a2: f64,
    a4: f64,
    b: f64,
    c1: f64,
    c3: f64,
    q: f64,
    t: f64,
    u: f64,
    w: f64,
    p22: f64,
    sa: f64,
    ca: f64,
    xj: f64,
    rlm: f64,
    rlm2: f64,
    one_es: f64,
    es: f64,
    rone_es: f64,
}

struct Series {
    a2: f64,
    a4: f64,
    b: f64,
    c1: f64,
    c3: f64,
}

fn seraz0(
    series: &mut Series,
    lam_deg: f64,
    mult: f64,
    p22: f64,
    sa: f64,
    ca: f64,
    t: f64,
    w: f64,
    q: f64,
    xj: f64,
) {
    let lam = lam_deg * DEG;
    let sd = lam.sin();
    let sdsq = sd * sd;
    let s =
        p22 * sa * lam.cos() * ((1.0 + t * sdsq) / ((1.0 + w * sdsq) * (1.0 + q * sdsq))).sqrt();
    let d_1 = 1.0 + q * sdsq;
    let h =
        ((1.0 + q * sdsq) / (1.0 + w * sdsq)).sqrt() * ((1.0 + w * sdsq) / (d_1 * d_1) - p22 * ca);
    let sq = (xj * xj + s * s).sqrt();
    let mut fc = mult * (h * xj - s * s) / sq;
    series.b += fc;
    series.a2 += fc * (lam + lam).cos();
    series.a4 += fc * (lam * 4.0).cos();
    fc = mult * s * (h + xj) / sq;
    series.c1 += fc * lam.cos();
    series.c3 += fc * (lam * 3.0).cos();
}

impl Som {
    fn new(asc: f64, alf: f64, p22: f64, rlm: f64, frame: &Frame) -> Self {
        let mut ca = alf.cos();
        if ca.abs() < 1e-9 {
            ca = 1e-9;
        }
        let sa = alf.sin();
        let es = frame.earth.es();
        let one_es = frame.earth.one_es();
        let rone_es = 1.0 / one_es;
        let esc = es * ca * ca;
        let ess = es * sa * sa;
        let mut w = (1.0 - esc) * rone_es;
        w = w * w - 1.0;
        let q = ess * rone_es;
        let t = ess * (2.0 - es) * rone_es * rone_es;
        let u = esc * rone_es;
        let xj = one_es * one_es * one_es;
        let mut series = Series {
            a2: 0.0,
            a4: 0.0,
            b: 0.0,
            c1: 0.0,
            c3: 0.0,
        };
        seraz0(&mut series, 0.0, 1.0, p22, sa, ca, t, w, q, xj);
        let mut lam = 9.0;
        while lam <= 81.0 {
            seraz0(&mut series, lam, 4.0, p22, sa, ca, t, w, q, xj);
            lam += 18.0;
        }
        lam = 18.0;
        while lam <= 72.0 {
            seraz0(&mut series, lam, 2.0, p22, sa, ca, t, w, q, xj);
            lam += 18.0;
        }
        seraz0(&mut series, 90.0, 1.0, p22, sa, ca, t, w, q, xj);
        Self {
            shift: asc - frame.lam0,
            a2: series.a2 / 30.0,
            a4: series.a4 / 60.0,
            b: series.b / 30.0,
            c1: series.c1 / 15.0,
            c3: series.c3 / 45.0,
            q,
            t,
            u,
            w,
            p22,
            sa,
            ca,
            xj,
            rlm,
            rlm2: rlm + TWOPI,
            one_es,
            es,
            rone_es,
        }
    }
}

impl Kernel for Som {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = lam - self.shift;
        if phi > FRAC_PI_2 {
            phi = FRAC_PI_2;
        } else if phi < -FRAC_PI_2 {
            phi = -FRAC_PI_2;
        }
        let mut lampp = if phi >= 0.0 { FRAC_PI_2 } else { PI_HALFPI };
        let tanphi = phi.tan();
        let mut lamt;
        let mut lamdp;
        let mut l;
        let mut nn = 0;
        loop {
            let lamtp = lam + self.p22 * lampp;
            let cl = lamtp.cos();
            let fac = if cl < 0.0 {
                lampp + lampp.sin() * FRAC_PI_2
            } else {
                lampp - lampp.sin() * FRAC_PI_2
            };
            let mut sav = lampp;
            l = 50;
            loop {
                lamt = lam + self.p22 * sav;
                let c = lamt.cos();
                let lamt_sin = if c.abs() < TOL { lamt - TOL } else { lamt };
                let xlam = (self.one_es * tanphi * self.sa + lamt_sin.sin() * self.ca) / c;
                lamdp = xlam.atan() + fac;
                if (sav.abs() - lamdp.abs()).abs() < TOL {
                    break;
                }
                sav = lamdp;
                l -= 1;
                if l < 0 {
                    break;
                }
            }
            nn += 1;
            if l == 0 || nn >= 3 || (lamdp > self.rlm && lamdp < self.rlm2) {
                break;
            }
            if lamdp <= self.rlm {
                lampp = TWOPI_HALFPI;
            } else if lamdp >= self.rlm2 {
                lampp = FRAC_PI_2;
            }
        }
        if l == 0 {
            return Err(TransformError::OutsideDomain);
        }
        let sp = phi.sin();
        let phidp = aasin(
            (self.one_es * self.ca * sp - self.sa * phi.cos() * lamt.sin())
                / (1.0 - self.es * sp * sp).sqrt(),
        )
        .map_err(|()| TransformError::OutsideDomain)?;
        let tanph = (FRAC_PI_4 + 0.5 * phidp).tan().ln();
        let sd = lamdp.sin();
        let sdsq = sd * sd;
        let s = self.p22
            * self.sa
            * lamdp.cos()
            * ((1.0 + self.t * sdsq) / ((1.0 + self.w * sdsq) * (1.0 + self.q * sdsq))).sqrt();
        let d = (self.xj * self.xj + s * s).sqrt();
        let x = self.b * lamdp + self.a2 * (2.0 * lamdp).sin() + self.a4 * (lamdp * 4.0).sin()
            - tanph * s / d;
        let y = self.c1 * sd + self.c3 * (lamdp * 3.0).sin() + tanph * self.xj / d;
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut lamdp = x / self.b;
        let mut s;
        let mut nn = 50;
        loop {
            let sav = lamdp;
            let sd = lamdp.sin();
            let sdsq = sd * sd;
            s = self.p22
                * self.sa
                * lamdp.cos()
                * ((1.0 + self.t * sdsq) / ((1.0 + self.w * sdsq) * (1.0 + self.q * sdsq))).sqrt();
            lamdp = x + y * s / self.xj
                - self.a2 * (2.0 * lamdp).sin()
                - self.a4 * (lamdp * 4.0).sin()
                - s / self.xj * (self.c1 * lamdp.sin() + self.c3 * (lamdp * 3.0).sin());
            lamdp /= self.b;
            nn -= 1;
            if (lamdp - sav).abs() < TOL || nn == 0 {
                break;
            }
        }
        let sl = lamdp.sin();
        let fac = ((1.0 + s * s / self.xj / self.xj).sqrt()
            * (y - self.c1 * sl - self.c3 * (lamdp * 3.0).sin()))
        .exp();
        let phidp = 2.0 * (fac.atan() - FRAC_PI_4);
        let dd = sl * sl;
        if lamdp.cos().abs() < TOL {
            lamdp -= TOL;
        }
        let spp = phidp.sin();
        let sppsq = spp * spp;
        let denom = 1.0 - sppsq * (1.0 + self.u);
        if denom == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let mut lamt = (((1.0 - sppsq * self.rone_es) * lamdp.tan() * self.ca
            - spp * self.sa * ((1.0 + self.q * dd) * (1.0 - sppsq) - sppsq * self.u).sqrt()
                / lamdp.cos())
            / denom)
            .atan();
        let sign = if lamt >= 0.0 { 1.0 } else { -1.0 };
        let scl = if lamdp.cos() >= 0.0 { 1.0 } else { -1.0 };
        lamt -= FRAC_PI_2 * (1.0 - scl) * sign;
        let lam = lamt - self.p22 * lamdp + self.shift;
        let phi = if self.sa.abs() < TOL {
            aasin(spp / (self.one_es * self.one_es + self.es * sppsq).sqrt())
                .map_err(|()| TransformError::OutsideDomain)?
        } else {
            ((lamdp.tan() * lamt.cos() - self.ca * lamt.sin()) / (self.one_es * self.sa)).atan()
        };
        Ok((lam, phi))
    }
}

fn require_range(value: f64, lo: f64, hi: f64, name: &'static str) -> Result<f64, CreateError> {
    if value < lo || value > hi {
        Err(CreateError::IllegalValue(name))
    } else {
        Ok(value)
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "som" => &["inc_angle", "ps_rev", "asc_lon"],
        "misrsom" => &["path"],
        "lsat" => &["lsat", "path"],
        _ => return None,
    })
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let (asc, alf, p22, rlm) = match name {
        "som" => {
            let asc = params.radians("asc_lon")?.unwrap_or(0.0);
            let alf = params.radians("inc_angle")?.unwrap_or(0.0);
            let p22 = params.number("ps_rev")?.unwrap_or(0.0);
            require_range(asc, -TWOPI, TWOPI, "asc_lon")?;
            require_range(alf, 0.0, PI, "inc_angle")?;
            if p22 < 0.0 {
                return Err(CreateError::IllegalValue("ps_rev"));
            }
            (asc, alf, p22, 0.0)
        }
        "misrsom" => {
            let path = params.integer("path")?.unwrap_or(0);
            if !(1..=233).contains(&path) {
                return Err(CreateError::IllegalValue("path"));
            }
            let path = f64::from(path);
            (
                DEG * 129.3056 - TWOPI / 233.0 * path,
                98.30382 * DEG,
                98.88 / 1440.0,
                0.0,
            )
        }
        "lsat" => {
            let land = params.integer("lsat")?.unwrap_or(0);
            if !(1..=5).contains(&land) {
                return Err(CreateError::IllegalValue("lsat"));
            }
            let max_path = if land <= 3 { 251 } else { 233 };
            let path = params.integer("path")?.unwrap_or(0);
            if path <= 0 || path > max_path {
                return Err(CreateError::IllegalValue("path"));
            }
            let path = f64::from(path);
            let rlm = PI * (1.0 / 248.0 + 0.516_129_032_258_064_5);
            if land <= 3 {
                (
                    DEG * 128.87 - TWOPI / 251.0 * path,
                    DEG * 99.092,
                    103.266_932_3 / 1440.0,
                    rlm,
                )
            } else {
                (
                    DEG * 129.3 - TWOPI / 233.0 * path,
                    DEG * 98.2,
                    98.884_120_2 / 1440.0,
                    rlm,
                )
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(classic_step(
        frame,
        Som::new(asc, alf, p22, rlm, frame),
    )))
}
