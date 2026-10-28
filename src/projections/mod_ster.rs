//! Modified stereographic projections: `alsk`, `gs50`, `gs48`, `lee_os`, `mil_os`.
//!
//! Each one overwrites the central meridian and latitude of origin. Alaska and
//! the 50-state map also replace the eccentricity with `sqrt(0.00676866)` on
//! an ellipsoid, and pick a separate coefficient series on a sphere. The
//! complex series is `pj_zpoly1`: no constant term, then one multiply by `z`
//! per stored coefficient.

#![allow(
    clippy::pedantic,
    reason = "complex Horner steps and the coefficient tables follow mod_ster.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_2;

const EPSLN: f64 = 1e-12;
const ONE_TOL: f64 = 1.000_000_000_000_01;
const ALASKA_ES: f64 = 0.006_768_66;

#[derive(Clone, Copy)]
struct Complex {
    re: f64,
    im: f64,
}

struct ModSter {
    coeff: &'static [Complex],
    phi0: f64,
    schio: f64,
    cchio: f64,
    e: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "alsk" | "gs50" | "gs48" | "lee_os" | "mil_os" => Some(&[]),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let ellipsoid = frame.earth.es() != 0.0;
    let (lam0_deg, phi0_deg, coeff, e): (f64, f64, &[Complex], f64) = match name {
        "mil_os" => (20.0, 18.0, MIL_OS.as_slice(), 0.0),
        "lee_os" => (-165.0, -10.0, LEE_OS.as_slice(), 0.0),
        "gs48" => (-96.0, 39.0, GS48.as_slice(), 0.0),
        "alsk" => (
            -152.0,
            64.0,
            if ellipsoid {
                ALSK_E.as_slice()
            } else {
                ALSK_S.as_slice()
            },
            if ellipsoid { ALASKA_ES.sqrt() } else { 0.0 },
        ),
        "gs50" => (
            -120.0,
            45.0,
            if ellipsoid {
                GS50_E.as_slice()
            } else {
                GS50_S.as_slice()
            },
            if ellipsoid { ALASKA_ES.sqrt() } else { 0.0 },
        ),
        _ => return Ok(None),
    };
    let phi0 = phi0_deg.to_radians();
    let chio = conformal(phi0, e);
    let mut placed = frame.clone();
    placed.lam0 = lam0_deg.to_radians();
    placed.phi0 = phi0;
    Ok(Some(classic_step(
        &placed,
        ModSter {
            coeff,
            phi0,
            schio: chio.sin(),
            cchio: chio.cos(),
            e,
        },
    )))
}

impl Kernel for ModSter {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let chi = conformal(phi, self.e);
        let (schi, cchi) = (chi.sin(), chi.cos());
        let (sinlon, coslon) = (lam.sin(), lam.cos());
        let denom = 1.0 + self.schio * schi + self.cchio * cchi * coslon;
        if denom == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let scale = 2.0 / denom;
        let point = zpoly1(
            Complex {
                re: scale * cchi * sinlon,
                im: scale * (self.cchio * schi - self.schio * cchi * coslon),
            },
            self.coeff,
        );
        Ok((point.re, point.im))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut point = Complex { re: x, im: y };
        let mut converged = false;
        for _ in 0..20 {
            let (mut value, derivative) = zpolyd1(point, self.coeff);
            value.re -= x;
            value.im -= y;
            let den = derivative.re * derivative.re + derivative.im * derivative.im;
            if den == 0.0 {
                return Err(TransformError::OutsideDomain);
            }
            let d_re = -(value.re * derivative.re + value.im * derivative.im) / den;
            let d_im = -(value.im * derivative.re - value.re * derivative.im) / den;
            point.re += d_re;
            point.im += d_im;
            if d_re.abs() + d_im.abs() <= EPSLN {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        let rh = point.re.hypot(point.im);
        if rh <= EPSLN {
            return Ok((0.0, self.phi0));
        }
        let z = 2.0 * (0.5 * rh).atan();
        let (sinz, cosz) = (z.sin(), z.cos());
        let chi = aasin(cosz * self.schio + point.im * sinz * self.cchio / rh)?;
        let mut phi = chi;
        let mut phi_ok = false;
        for _ in 0..20 {
            let esphi = self.e * phi.sin();
            let dphi = conformal_inverse_step(chi, phi, esphi, self.e);
            phi += dphi;
            if dphi.abs() <= EPSLN {
                phi_ok = true;
                break;
            }
        }
        if !phi_ok {
            return Err(TransformError::OutsideDomain);
        }
        let lam = (point.re * sinz).atan2(rh * self.cchio * cosz - point.im * self.schio * sinz);
        Ok((lam, phi))
    }
}

fn conformal(phi: f64, e: f64) -> f64 {
    let esphi = e * phi.sin();
    let ratio = ((1.0 - esphi) / (1.0 + esphi)).powf(0.5 * e);
    2.0 * (((FRAC_PI_2 + phi) * 0.5).tan() * ratio).atan() - FRAC_PI_2
}

fn conformal_inverse_step(chi: f64, phi: f64, esphi: f64, e: f64) -> f64 {
    let ratio = ((1.0 + esphi) / (1.0 - esphi)).powf(0.5 * e);
    2.0 * (((FRAC_PI_2 + chi) * 0.5).tan() * ratio).atan() - FRAC_PI_2 - phi
}

fn aasin(value: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > ONE_TOL {
            return Err(TransformError::OutsideDomain);
        }
        return Ok(if value < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
    }
    Ok(value.asin())
}

fn zpoly1(z: Complex, coeff: &[Complex]) -> Complex {
    let mut a = coeff[coeff.len() - 1];
    for term in coeff.iter().rev().skip(1) {
        let t = a.re;
        a.re = term.re + z.re * t - z.im * a.im;
        a.im = term.im + z.re * a.im + z.im * t;
    }
    let t = a.re;
    a.re = z.re * t - z.im * a.im;
    a.im = z.re * a.im + z.im * t;
    a
}

fn zpolyd1(z: Complex, coeff: &[Complex]) -> (Complex, Complex) {
    let mut index = coeff.len() - 1;
    let mut a = coeff[index];
    let mut b = a;
    let mut first = true;
    while index > 0 {
        if first {
            first = false;
        } else {
            let t = b.re;
            b.re = a.re + z.re * t - z.im * b.im;
            b.im = a.im + z.re * b.im + z.im * t;
        }
        index -= 1;
        let t = a.re;
        a.re = coeff[index].re + z.re * t - z.im * a.im;
        a.im = coeff[index].im + z.re * a.im + z.im * t;
    }
    let t = b.re;
    b.re = a.re + z.re * t - z.im * b.im;
    b.im = a.im + z.re * b.im + z.im * t;
    let t = a.re;
    a.re = z.re * t - z.im * a.im;
    a.im = z.re * a.im + z.im * t;
    (a, b)
}

const fn c(re: f64, im: f64) -> Complex {
    Complex { re, im }
}

const MIL_OS: [Complex; 3] = [c(0.924_500, 0.0), c(0.0, 0.0), c(0.019_430, 0.0)];
const LEE_OS: [Complex; 3] = [
    c(0.721_316, 0.0),
    c(0.0, 0.0),
    c(-0.008_816_2, -0.006_173_25),
];
const GS48: [Complex; 5] = [
    c(0.988_79, 0.0),
    c(0.0, 0.0),
    c(-0.050_909, 0.0),
    c(0.0, 0.0),
    c(0.075_528, 0.0),
];
const ALSK_E: [Complex; 6] = [
    c(0.994_530_3, 0.0),
    c(0.005_208_3, -0.002_740_4),
    c(0.007_272_1, 0.004_818_1),
    c(-0.015_108_9, -0.193_252_6),
    c(0.064_267_5, -0.138_122_6),
    c(0.358_280_2, -0.288_458_6),
];
const ALSK_S: [Complex; 6] = [
    c(0.997_252_3, 0.0),
    c(0.005_251_3, -0.004_117_5),
    c(0.007_460_6, 0.004_812_5),
    c(-0.015_378_3, -0.196_825_3),
    c(0.063_687_1, -0.140_802_7),
    c(0.366_097_6, -0.293_738_2),
];
const GS50_E: [Complex; 10] = [
    c(0.982_749_7, 0.0),
    c(0.021_066_9, 0.005_380_4),
    c(-0.103_141_5, -0.057_166_4),
    c(-0.032_333_7, -0.032_284_7),
    c(0.050_230_3, 0.121_198_3),
    c(0.025_180_5, 0.089_567_8),
    c(-0.001_231_5, -0.141_612_1),
    c(0.007_220_2, -0.131_709_1),
    c(-0.019_402_9, 0.075_967_7),
    c(-0.021_007_2, 0.083_403_7),
];
const GS50_S: [Complex; 10] = [
    c(0.984_299_0, 0.0),
    c(0.021_164_2, 0.003_760_8),
    c(-0.103_601_8, -0.057_510_2),
    c(-0.032_909_5, -0.032_011_9),
    c(0.049_947_1, 0.122_333_5),
    c(0.026_046_0, 0.089_980_5),
    c(0.000_738_8, -0.143_579_2),
    c(0.007_584_8, -0.133_410_8),
    c(-0.021_647_3, 0.077_664_5),
    c(-0.022_516_1, 0.085_367_3),
];
