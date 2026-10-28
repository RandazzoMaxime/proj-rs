//! Krovak and Modified Krovak.
//!
//! Port of PROJ 9.9 `src/projections/krovak.cpp`. The setup always replaces the
//! ellipsoid with Bessel 1841 and, when `lat_0` or `lon_0` is absent, the
//! Czech/Slovak origin. The kernel then scales by `a_bessel / a_frame` so the
//! classic wrapper, which multiplies by the caller's semi-major axis, matches
//! PROJ's forced Bessel radius.

#![allow(clippy::pedantic, reason = "formulae follow PROJ krovak.cpp")]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const BESSEL_A: f64 = 6_377_397.155;
const BESSEL_ES: f64 = 0.006_674_372_230_614;
const UQ: f64 = 1.042_168_563_804_74;
const S0: f64 = 1.370_083_462_815_55;
const DEFAULT_PHI0: f64 = 0.863_937_979_737_193;
const DEFAULT_LAM0: f64 = 0.741_764_932_097_590_1 - 0.308_341_501_185_665;
const EPS: f64 = 1e-15;

const X0: f64 = 1_089_000.0;
const Y0: f64 = 654_000.0;
const C1: f64 = 2.946_529_277e-02;
const C2: f64 = 2.515_965_696e-02;
const C3: f64 = 1.193_845_912e-07;
const C4: f64 = -4.668_270_147e-07;
const C5: f64 = 9.233_980_362e-12;
const C6: f64 = 1.523_735_715e-12;
const C7: f64 = 1.696_780_024e-18;
const C8: f64 = 4.408_314_235e-18;
const C9: f64 = -8.331_083_518e-24;
const C10: f64 = -3.689_471_323e-24;

fn correction(xr: f64, yr: f64) -> (f64, f64) {
    let xr2 = xr * xr;
    let yr2 = yr * yr;
    let xr4 = xr2 * xr2;
    let yr4 = yr2 * yr2;
    let dx = C1 + C3 * xr - C4 * yr - 2.0 * C6 * xr * yr
        + C5 * (xr2 - yr2)
        + C7 * xr * (xr2 - 3.0 * yr2)
        - C8 * yr * (3.0 * xr2 - yr2)
        + 4.0 * C9 * xr * yr * (xr2 - yr2)
        + C10 * (xr4 + yr4 - 6.0 * xr2 * yr2);
    let dy = C2
        + C3 * yr
        + C4 * xr
        + 2.0 * C5 * xr * yr
        + C6 * (xr2 - yr2)
        + C8 * xr * (xr2 - 3.0 * yr2)
        + C7 * yr * (3.0 * xr2 - yr2)
        - 4.0 * C10 * xr * yr * (xr2 - yr2)
        + C9 * (xr4 + yr4 - 6.0 * xr2 * yr2);
    (dx, dy)
}

struct Krovak {
    alpha: f64,
    k: f64,
    n: f64,
    rho0: f64,
    ad: f64,
    easting_northing: bool,
    modified: bool,
    e: f64,
    x0: f64,
    y0: f64,
    shift: f64,
    over: bool,
    /// `a_bessel / a` of the caller's frame.
    scale: f64,
}

impl Krovak {
    fn relative(&self, lam: f64) -> f64 {
        let lam = lam - self.shift;
        if self.over {
            lam
        } else {
            adjlon(lam)
        }
    }
}

impl Kernel for Krovak {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let lam = self.relative(lam);
        let gfi = ((1.0 + self.e * phi.sin()) / (1.0 - self.e * phi.sin()))
            .powf(self.alpha * self.e / 2.0);
        let u = 2.0
            * ((self.k * (phi / 2.0 + FRAC_PI_4).tan().powf(self.alpha) / gfi).atan() - FRAC_PI_4);
        let deltav = -lam * self.alpha;
        let s = (self.ad.cos() * u.sin() + self.ad.sin() * u.cos() * deltav.cos()).asin();
        if s.cos() < 1e-12 {
            return Ok((0.0, 0.0));
        }
        let d = (u.cos() * deltav.sin() / s.cos()).asin();
        let eps = self.n * d;
        let rho = self.rho0 * (S0 / 2.0 + FRAC_PI_4).tan().powf(self.n)
            / (s / 2.0 + FRAC_PI_4).tan().powf(self.n);
        let mut x = rho * eps.cos();
        let mut y = rho * eps.sin();
        if self.modified {
            let xr = x * BESSEL_A - X0;
            let yr = y * BESSEL_A - Y0;
            let (dx, dy) = correction(xr, yr);
            x -= dx / BESSEL_A;
            y -= dy / BESSEL_A;
        }
        std::mem::swap(&mut x, &mut y);
        if self.easting_northing {
            x = -x - 2.0 * self.x0 / BESSEL_A;
            y = -y - 2.0 * self.y0 / BESSEL_A;
        }
        Ok((x * self.scale, y * self.scale))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        // `inv.cpp` scales by the original `ra`, captured before this setup
        // replaces `a` with Bessel. The forward path multiplies by the new
        // `a`. The kernel therefore sees different units in the two directions.
        if self.easting_northing {
            y = -y - 2.0 * self.x0 / BESSEL_A;
            x = -x - 2.0 * self.y0 / BESSEL_A;
        }
        std::mem::swap(&mut x, &mut y);
        if self.modified {
            let xr = x * BESSEL_A - X0;
            let yr = y * BESSEL_A - Y0;
            let (dx, dy) = correction(xr, yr);
            x += dx / BESSEL_A;
            y += dy / BESSEL_A;
        }
        let rho = x.hypot(y);
        let eps = y.atan2(x);
        let d = eps / S0.sin();
        let s = if rho == 0.0 {
            FRAC_PI_2
        } else {
            2.0 * (((self.rho0 / rho).powf(1.0 / self.n) * (S0 / 2.0 + FRAC_PI_4).tan()).atan()
                - FRAC_PI_4)
        };
        let u = (self.ad.cos() * s.sin() - self.ad.sin() * s.cos() * d.cos()).asin();
        let deltav = (s.cos() * d.sin() / u.cos()).asin();
        let lam = -deltav / self.alpha;
        let num_base =
            self.k.powf(-1.0 / self.alpha) * (u / 2.0 + FRAC_PI_4).tan().powf(1.0 / self.alpha);
        let mut fi1 = u;
        let mut phi = u;
        let mut converged = false;
        for _ in 0..100 {
            phi = 2.0
                * ((num_base
                    * ((1.0 + self.e * fi1.sin()) / (1.0 - self.e * fi1.sin()))
                        .powf(self.e / 2.0))
                .atan()
                    - FRAC_PI_4);
            if (fi1 - phi).abs() < EPS {
                converged = true;
                break;
            }
            fi1 = phi;
        }
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam + self.shift, phi))
    }
}

/// `+alpha` is the EPSG colatitude of the cone axis. PROJ accepts it and does
/// not read it: the cone constant is computed from `lat_0`.
const KEYS: &[&str] = &["czech", "alpha"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "krovak" | "mod_krovak" => Some(KEYS),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let modified = match name {
        "krovak" => false,
        "mod_krovak" => true,
        _ => return Ok(None),
    };
    let e = BESSEL_ES.sqrt();
    let phi0 = if params.flag("lat_0") {
        frame.phi0
    } else {
        DEFAULT_PHI0
    };
    let lam0 = if params.flag("lon_0") {
        frame.lam0
    } else {
        DEFAULT_LAM0
    };
    let k0 = if params.flag("k") || params.flag("k_0") {
        frame.k0
    } else {
        0.9999
    };
    let alpha = (1.0 + BESSEL_ES * phi0.cos().powi(4) / (1.0 - BESSEL_ES)).sqrt();
    let u0 = (phi0.sin() / alpha).asin();
    let g = ((1.0 + e * phi0.sin()) / (1.0 - e * phi0.sin())).powf(alpha * e / 2.0);
    let tan_half = (phi0 / 2.0 + FRAC_PI_4).tan();
    if tan_half == 0.0 {
        return Err(CreateError::IllegalValue("lat_0"));
    }
    let k = (u0 / 2.0 + FRAC_PI_4).tan() / tan_half.powf(alpha) * g;
    let n0 = (1.0 - BESSEL_ES).sqrt() / (1.0 - BESSEL_ES * phi0.sin().powi(2));
    Ok(Some(classic_step(
        frame,
        Krovak {
            alpha,
            k,
            n: S0.sin(),
            rho0: k0 * n0 / S0.tan(),
            ad: FRAC_PI_2 - UQ,
            easting_northing: !params.flag("czech"),
            modified,
            e,
            x0: frame.x0,
            y0: frame.y0,
            shift: lam0 - frame.lam0,
            over: frame.over,
            scale: BESSEL_A / frame.earth.a(),
        },
    )))
}
