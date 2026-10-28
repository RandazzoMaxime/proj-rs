//! Simple conics, Aitoff, and Winkel Tripel from PROJ 9.9.
//!
//! Each of these forces a sphere and keeps the caller's semi-major axis.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

fn step<K: Kernel + 'static>(frame: &Frame, kernel: K) -> Box<dyn crate::engine::Step> {
    classic_step(frame, kernel)
}

#[derive(Clone, Copy)]
enum ConicKind {
    Euler,
    Murd1,
    Murd2,
    Murd3,
    Pconic,
    Tissot,
    Vitk1,
}

struct Sconic {
    n: f64,
    rho_c: f64,
    rho_0: f64,
    sig: f64,
    c1: f64,
    c2: f64,
    kind: ConicKind,
}

impl Kernel for Sconic {
    fn forward(&self, mut lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let rho = match self.kind {
            ConicKind::Murd2 => self.rho_c + (self.sig - phi).tan(),
            ConicKind::Pconic => self.c2 * (self.c1 - (phi - self.sig).tan()),
            _ => self.rho_c - phi,
        };
        lam *= self.n;
        Ok((rho * lam.sin(), self.rho_0 - rho * lam.cos()))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y = self.rho_0 - y;
        let mut rho = x.hypot(y);
        if self.n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }
        let lam = x.atan2(y) / self.n;
        let phi = match self.kind {
            ConicKind::Pconic => (self.c1 - rho / self.c2).atan() + self.sig,
            ConicKind::Murd2 => self.sig - (rho - self.rho_c).atan(),
            _ => self.rho_c - rho,
        };
        Ok((lam, phi))
    }
}

fn standard_parallels(params: &Params) -> Result<(f64, f64), CreateError> {
    let Some(lat1) = params.radians("lat_1")? else {
        return Err(CreateError::IllegalValue("lat_1"));
    };
    let Some(lat2) = params.radians("lat_2")? else {
        return Err(CreateError::IllegalValue("lat_2"));
    };
    let del = 0.5 * (lat2 - lat1);
    let sig = 0.5 * (lat2 + lat1);
    if del.abs() < 1e-10 || sig.abs() < 1e-10 {
        return Err(CreateError::IllegalValue("lat_1"));
    }
    Ok((del, sig))
}

fn sconic(frame: &Frame, params: &Params, kind: ConicKind) -> Result<Sconic, CreateError> {
    let (mut del, sig) = standard_parallels(params)?;
    let phi0 = frame.phi0;
    let (n, rho_c, rho_0, c1, c2) = match kind {
        ConicKind::Tissot => {
            let n = sig.sin();
            let cs = del.cos();
            let rho_c = n / cs + cs / n;
            let rho_0 = ((rho_c - 2.0 * phi0.sin()) / n).sqrt();
            (n, rho_c, rho_0, 0.0, 0.0)
        }
        ConicKind::Murd1 => {
            let rho_c = del.sin() / (del * sig.tan()) + sig;
            (sig.sin(), rho_c, rho_c - phi0, 0.0, 0.0)
        }
        ConicKind::Murd2 => {
            let cs = del.cos().sqrt();
            let rho_c = cs / sig.tan();
            (sig.sin() * cs, rho_c, rho_c + (sig - phi0).tan(), 0.0, 0.0)
        }
        ConicKind::Murd3 => {
            let rho_c = del / (sig.tan() * del.tan()) + sig;
            let n = sig.sin() * del.sin() * del.tan() / (del * del);
            (n, rho_c, rho_c - phi0, 0.0, 0.0)
        }
        ConicKind::Euler => {
            let n = sig.sin() * del.sin() / del;
            del *= 0.5;
            let rho_c = del / (del.tan() * sig.tan()) + sig;
            (n, rho_c, rho_c - phi0, 0.0, 0.0)
        }
        ConicKind::Pconic => {
            let n = sig.sin();
            let c2 = del.cos();
            let c1 = 1.0 / sig.tan();
            let del = phi0 - sig;
            if del.abs() - 1e-10 >= FRAC_PI_2 {
                return Err(CreateError::IllegalValue("lat_0"));
            }
            (n, 0.0, c2 * (c1 - del.tan()), c1, c2)
        }
        ConicKind::Vitk1 => {
            let cs = del.tan();
            let n = cs * sig.sin() / del;
            let rho_c = del / (cs * sig.tan()) + sig;
            (n, rho_c, rho_c - phi0, 0.0, 0.0)
        }
    };
    Ok(Sconic {
        n,
        rho_c,
        rho_0,
        sig,
        c1,
        c2,
        kind,
    })
}

fn aitoff_forward(lam: f64, phi: f64) -> (f64, f64) {
    let sin_half_lam = (0.5 * lam).sin();
    let sin_phi = phi.sin();
    let cos_phi = phi.cos();
    let sin_alpha = (cos_phi * sin_half_lam).hypot(sin_phi);
    let scale = if sin_alpha == 0.0 {
        1.0
    } else {
        sin_alpha.atan2(cos_phi * (0.5 * lam).cos()) / sin_alpha
    };
    (2.0 * cos_phi * sin_half_lam * scale, sin_phi * scale)
}

struct Aitoff {
    wintri: bool,
    cosphi1: f64,
}

impl Kernel for Aitoff {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let (mut x, mut y) = aitoff_forward(lam, phi);
        if self.wintri {
            x = (x + lam * self.cosphi1) * 0.5;
            y = (y + phi) * 0.5;
        }
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if !self.wintri {
            let half_x = 0.5 * x;
            let alpha = half_x.hypot(y);
            let sin_alpha = alpha.sin();
            if alpha > PI || sin_alpha < 0.0 {
                return Err(TransformError::OutsideDomain);
            }
            let phi = if alpha == 0.0 {
                y
            } else {
                ((y / alpha) * sin_alpha).asin()
            };
            let lam = 2.0 * (half_x * sin_alpha).atan2(alpha * alpha.cos());
            return Ok((lam, phi));
        }
        const EPSILON: f64 = 1e-12;
        if x.abs() < EPSILON && y.abs() < EPSILON {
            return Ok((0.0, 0.0));
        }
        let cosphi1 = self.cosphi1;
        let mut phi = y;
        let mut lam = x;
        for _round in 0..21 {
            for _iter in 0..11 {
                let sl = (lam * 0.5).sin();
                let cl = (lam * 0.5).cos();
                let sp = phi.sin();
                let cp = phi.cos();
                let mut d = cp * cl;
                let c = 1.0 - d * d;
                let denom = c.powf(1.5);
                if denom == 0.0 {
                    return Err(TransformError::OutsideDomain);
                }
                d = d.acos() / denom;
                let mut f1 = 2.0 * d * c * cp * sl;
                let mut f2 = d * c * sp;
                let mut f1p = 2.0 * (sl * cl * sp * cp / c - d * sp * sl);
                let mut f1l = cp * cp * sl * sl / c + d * cp * cl * sp * sp;
                let mut f2p = sp * sp * cl / c + d * sl * sl * cp;
                let mut f2l = 0.5 * (sp * cp * sl / c - d * sp * cp * cp * sl * cl);
                f1 = 0.5 * (f1 + lam * cosphi1);
                f2 = 0.5 * (f2 + phi);
                f1p *= 0.5;
                f1l = 0.5 * (f1l + cosphi1);
                f2p = 0.5 * (f2p + 1.0);
                f2l *= 0.5;
                f1 -= x;
                f2 -= y;
                let det = f1p * f2l - f2p * f1l;
                let dl = ((f2 * f1p - f1 * f2p) / det) % PI;
                let dp = (f1 * f2l - f2 * f1l) / det;
                phi -= dp;
                lam -= dl;
                if dp.abs() <= EPSILON && dl.abs() <= EPSILON {
                    break;
                }
            }
            if phi > FRAC_PI_2 {
                phi -= 2.0 * (phi - FRAC_PI_2);
            }
            if phi < -FRAC_PI_2 {
                phi -= 2.0 * (phi + FRAC_PI_2);
            }
            let (px, py) = aitoff_forward(lam, phi);
            let px = (px + lam * cosphi1) * 0.5;
            let py = (py + phi) * 0.5;
            if (x - px).abs() <= EPSILON && (y - py).abs() <= EPSILON {
                break;
            }
        }
        Ok((lam, phi))
    }
}

const LAT_12: &[&str] = &["lat_1", "lat_2"];
const LAT_1: &[&str] = &["lat_1"];
const NONE: &[&str] = &[];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "euler" | "murd1" | "murd2" | "murd3" | "pconic" | "tissot" | "vitk1" => LAT_12,
        "wintri" => LAT_1,
        "aitoff" => NONE,
        _ => return None,
    })
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let kind = match name {
        "euler" => ConicKind::Euler,
        "murd1" => ConicKind::Murd1,
        "murd2" => ConicKind::Murd2,
        "murd3" => ConicKind::Murd3,
        "pconic" => ConicKind::Pconic,
        "tissot" => ConicKind::Tissot,
        "vitk1" => ConicKind::Vitk1,
        "aitoff" => {
            return Ok(Some(step(
                frame,
                Aitoff {
                    wintri: false,
                    cosphi1: 1.0,
                },
            )));
        }
        "wintri" => {
            let cosphi1 = if params.flag("lat_1") {
                let lat1 = params.radians("lat_1")?.unwrap_or(0.0);
                let cosphi1 = lat1.cos();
                if cosphi1 == 0.0 {
                    return Err(CreateError::IllegalValue("lat_1"));
                }
                cosphi1
            } else {
                0.636_619_772_367_581_343
            };
            return Ok(Some(step(
                frame,
                Aitoff {
                    wintri: true,
                    cosphi1,
                },
            )));
        }
        _ => return Ok(None),
    };
    Ok(Some(step(frame, sconic(frame, params, kind)?)))
}
