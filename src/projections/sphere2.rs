//! Sphere projections transcribed from PROJ 9.9 `src/projections/`.
//!
//! Those that assign `P->es = 0` keep the caller's semi-major axis. The
//! classic wrapper multiplies by that axis. `k0` is applied only where the
//! C forward reads `P->k0`.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aasin, aatan2, Meridian};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

fn asin_or(value: f64) -> Result<f64, TransformError> {
    aasin(value).map_err(|()| TransformError::OutsideDomain)
}

fn clamp_asin(value: f64, tol: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > tol {
            Err(TransformError::OutsideDomain)
        } else {
            Ok(if value < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 })
        }
    } else {
        Ok(value.asin())
    }
}

fn step<K: Kernel + 'static>(frame: &Frame, kernel: K) -> Box<dyn crate::engine::Step> {
    classic_step(frame, kernel)
}

fn generic_inverse(
    x: f64,
    y: f64,
    mut lam: f64,
    mut phi: f64,
    tolerance: f64,
    mut forward: impl FnMut(f64, f64) -> (f64, f64),
) -> Result<(f64, f64), TransformError> {
    let mut dlam_x = 0.0;
    let mut dlam_y = 0.0;
    let mut dphi_x = 0.0;
    let mut dphi_y = 0.0;
    for i in 0..15 {
        let (approx_x, approx_y) = forward(lam, phi);
        let dx = approx_x - x;
        let dy = approx_y - y;
        if dx.abs() < tolerance && dy.abs() < tolerance {
            return Ok((lam, phi));
        }
        if i == 0 || dx.abs() > 1e-6 || dy.abs() > 1e-6 {
            let step_lam = if lam > 0.0 { -1e-6 } else { 1e-6 };
            let (x_lam, y_lam) = forward(lam + step_lam, phi);
            let dx_lam = (x_lam - approx_x) / step_lam;
            let dy_lam = (y_lam - approx_y) / step_lam;
            let step_phi = if phi > 0.0 { -1e-6 } else { 1e-6 };
            let (x_phi, y_phi) = forward(lam, phi + step_phi);
            let dx_phi = (x_phi - approx_x) / step_phi;
            let dy_phi = (y_phi - approx_y) / step_phi;
            let det = dx_lam * dy_phi - dx_phi * dy_lam;
            if det != 0.0 {
                dlam_x = dy_phi / det;
                dlam_y = -dx_phi / det;
                dphi_x = -dy_lam / det;
                dphi_y = dx_lam / det;
            }
        }
        lam -= (dx * dlam_x + dy * dlam_y).clamp(-0.3, 0.3);
        lam = lam.clamp(-PI, PI);
        phi -= (dx * dphi_x + dy * dphi_y).clamp(-0.3, 0.3);
        phi = phi.clamp(-FRAC_PI_2, FRAC_PI_2);
    }
    Err(TransformError::OutsideDomain)
}

struct Sts {
    c_x: f64,
    c_y: f64,
    c_p: f64,
    tan_mode: bool,
}

impl Kernel for Sts {
    fn forward(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        let mut x = self.c_x * lam * phi.cos();
        let mut y = self.c_y;
        phi *= self.c_p;
        let c = phi.cos();
        if self.tan_mode {
            x *= c * c;
            y *= phi.tan();
        } else {
            x /= c;
            y *= phi.sin();
        }
        Ok((x, y))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y /= self.c_y;
        let mut phi = if self.tan_mode { y.atan() } else { asin_or(y)? };
        let c = phi.cos();
        phi /= self.c_p;
        let mut lam = x / (self.c_x * phi.cos());
        if self.tan_mode {
            lam /= c * c;
        } else {
            lam *= c;
        }
        Ok((lam, phi))
    }
}

fn sts(frame: &Frame, p: f64, q: f64, tan_mode: bool) -> Box<dyn crate::engine::Step> {
    step(
        frame,
        Sts {
            c_x: q / p,
            c_y: p,
            c_p: 1.0 / q,
            tan_mode,
        },
    )
}

struct Moll {
    c_x: f64,
    c_y: f64,
    c_p: f64,
}

impl Kernel for Moll {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let k = self.c_p * phi.sin();
        let mut phi = phi;
        let mut guard = 30;
        while guard != 0 {
            let v = (phi + phi.sin() - k) / (1.0 + phi.cos());
            phi -= v;
            if v.abs() < 1e-7 {
                break;
            }
            guard -= 1;
        }
        if guard == 0 {
            phi = if phi < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
        } else {
            phi *= 0.5;
        }
        Ok((self.c_x * lam * phi.cos(), self.c_y * phi.sin()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut phi = asin_or(y / self.c_y)?;
        let lam = x / (self.c_x * phi.cos());
        if lam.abs() < PI {
            phi += phi;
            Ok((lam, asin_or((phi + phi.sin()) / self.c_p)?))
        } else {
            Err(TransformError::OutsideDomain)
        }
    }
}

pub(crate) fn mollweide_forward(lam: f64, phi: f64) -> (f64, f64) {
    let p = FRAC_PI_2;
    let p2 = p + p;
    let sp = p.sin();
    let r = (2.0 * PI * sp / (p2 + p2.sin())).sqrt();
    let kernel = Moll {
        c_x: 2.0 * r / PI,
        c_y: r / sp,
        c_p: p2 + p2.sin(),
    };
    kernel.forward(lam, phi).unwrap_or((f64::NAN, f64::NAN))
}

pub(crate) fn mollweide_inverse(x: f64, y: f64) -> Result<(f64, f64), TransformError> {
    let p = FRAC_PI_2;
    let p2 = p + p;
    let sp = p.sin();
    let r = (2.0 * PI * sp / (p2 + p2.sin())).sqrt();
    let kernel = Moll {
        c_x: 2.0 * r / PI,
        c_y: r / sp,
        c_p: p2 + p2.sin(),
    };
    kernel.inverse(x, y)
}

fn moll_from_p(frame: &Frame, p: f64) -> Box<dyn crate::engine::Step> {
    let p2 = p + p;
    let sp = p.sin();
    let r = (2.0 * PI * sp / (p2 + p2.sin())).sqrt();
    step(
        frame,
        Moll {
            c_x: 2.0 * r / PI,
            c_y: r / sp,
            c_p: p2 + p2.sin(),
        },
    )
}

struct Tobmerc {
    k0: f64,
}

impl Kernel for Tobmerc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if phi.abs() >= FRAC_PI_2 {
            return Err(TransformError::OutsideDomain);
        }
        let cosphi = phi.cos();
        Ok((self.k0 * lam * cosphi * cosphi, self.k0 * phi.tan().asinh()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = (y / self.k0).sinh().atan();
        let cosphi = phi.cos();
        Ok((x / self.k0 / (cosphi * cosphi), phi))
    }
}

struct Hammer {
    w: f64,
    m: f64,
    rm: f64,
}

impl Kernel for Hammer {
    fn forward(&self, mut lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let cosphi = phi.cos();
        lam *= self.w;
        let denom = 1.0 + cosphi * lam.cos();
        if denom == 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let d = (2.0 / denom).sqrt();
        Ok((self.m * d * cosphi * lam.sin(), self.rm * d * phi.sin()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let z = (1.0 - 0.25 * self.w * self.w * x * x - 0.25 * y * y).sqrt();
        if (2.0 * z * z - 1.0).abs() < 1e-10 {
            return Err(TransformError::OutsideDomain);
        }
        Ok((
            aatan2(self.w * x * z, 2.0 * z * z - 1.0) / self.w,
            asin_or(z * y)?,
        ))
    }
}

struct Wink2 {
    cosphi1: f64,
}

fn wink2_xy(cosphi1: f64, lam: f64, phi: f64) -> (f64, f64) {
    let y_lin = phi * (2.0 / PI);
    let k = PI * phi.sin();
    let mut phi = phi * 1.8;
    let mut guard = 10;
    while guard != 0 {
        let v = (phi + phi.sin() - k) / (1.0 + phi.cos());
        phi -= v;
        if v.abs() < 1e-7 {
            break;
        }
        guard -= 1;
    }
    if guard == 0 {
        phi = if phi < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
    } else {
        phi *= 0.5;
    }
    (
        0.5 * lam * (phi.cos() + cosphi1),
        FRAC_PI_4 * (phi.sin() + y_lin),
    )
}

impl Kernel for Wink2 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok(wink2_xy(self.cosphi1, lam, phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let cosphi1 = self.cosphi1;
        generic_inverse(x, y, x, y, 1e-10, |lam, phi| wink2_xy(cosphi1, lam, phi))
    }
}

struct Loxim {
    phi1: f64,
    cosphi1: f64,
    tanphi1: f64,
}

const LOXIM_EPS: f64 = 1e-8;

impl Kernel for Loxim {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let y = phi - self.phi1;
        let x = if y.abs() < LOXIM_EPS {
            lam * self.cosphi1
        } else {
            let t = FRAC_PI_4 + 0.5 * phi;
            if t.abs() < LOXIM_EPS || (t.abs() - FRAC_PI_2).abs() < LOXIM_EPS {
                0.0
            } else {
                lam * y / (t.tan() / self.tanphi1).ln()
            }
        };
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y + self.phi1;
        let lam = if y.abs() < LOXIM_EPS {
            x / self.cosphi1
        } else {
            let t = FRAC_PI_4 + 0.5 * phi;
            if t.abs() < LOXIM_EPS || (t.abs() - FRAC_PI_2).abs() < LOXIM_EPS {
                0.0
            } else {
                x * (t.tan() / self.tanphi1).ln() / y
            }
        };
        Ok((lam, phi))
    }
}

struct Putp3 {
    a: f64,
}

const PUTP3_C: f64 = 0.797_884_56;

impl Kernel for Putp3 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((PUTP3_C * lam * (1.0 - self.a * phi * phi), PUTP3_C * phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y / PUTP3_C;
        Ok((x / (PUTP3_C * (1.0 - self.a * phi * phi)), phi))
    }
}

struct Putp4 {
    c_x: f64,
    c_y: f64,
}

impl Kernel for Putp4 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut phi = asin_or(0.883_883_476 * phi.sin())?;
        let mut x = self.c_x * lam * phi.cos();
        phi *= 0.333_333_333_333_333;
        x /= phi.cos();
        Ok((x, self.c_y * phi.sin()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut phi = asin_or(y / self.c_y)?;
        let mut lam = x * phi.cos() / self.c_x;
        phi *= 3.0;
        lam /= phi.cos();
        Ok((lam, asin_or(1.131_370_85 * phi.sin())?))
    }
}

struct Putp5 {
    a: f64,
    b: f64,
}

const PUTP5_C: f64 = 1.013_46;
const PUTP5_D: f64 = 1.215_854_2;

impl Kernel for Putp5 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((
            PUTP5_C * lam * (self.a - self.b * (1.0 + PUTP5_D * phi * phi).sqrt()),
            PUTP5_C * phi,
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y / PUTP5_C;
        Ok((
            x / (PUTP5_C * (self.a - self.b * (1.0 + PUTP5_D * phi * phi).sqrt())),
            phi,
        ))
    }
}

struct Putp6 {
    c_x: f64,
    c_y: f64,
    a: f64,
    b: f64,
    d: f64,
}

const PUTP6_POLE: f64 = 1.732_050_807_568_877;

impl Kernel for Putp6 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let p = self.b * phi.sin();
        let mut phi = phi * 1.102_657_79;
        let mut guard = 10;
        while guard != 0 {
            let r = (1.0 + phi * phi).sqrt();
            let v = ((self.a - r) * phi - (phi + r).ln() - p) / (self.a - 2.0 * r);
            phi -= v;
            if v.abs() < 1e-10 {
                break;
            }
            guard -= 1;
        }
        let root = if guard == 0 {
            phi = if p < 0.0 { -PUTP6_POLE } else { PUTP6_POLE };
            2.0
        } else {
            (1.0 + phi * phi).sqrt()
        };
        Ok((self.c_x * lam * (self.d - root), self.c_y * phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y / self.c_y;
        let r = (1.0 + phi * phi).sqrt();
        let lam = x / (self.c_x * (self.d - r));
        Ok((
            lam,
            asin_or(((self.a - r) * phi - (phi + r).ln()) / self.b)?,
        ))
    }
}

enum GnKind {
    Sphere { m: f64, n: f64, c_x: f64, c_y: f64 },
    Ellipsoid { es: f64, meridian: Meridian },
}

struct GnSinu {
    kind: GnKind,
}

impl Kernel for GnSinu {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        match self.kind {
            GnKind::Sphere { m, n, c_x, c_y } => {
                let mut phi = phi;
                if m == 0.0 {
                    if n != 1.0 {
                        phi = asin_or(n * phi.sin())?;
                    }
                } else {
                    let k = n * phi.sin();
                    let mut guard = 8;
                    while guard != 0 {
                        let v = (m * phi + phi.sin() - k) / (m + phi.cos());
                        phi -= v;
                        if v.abs() < 1e-7 {
                            break;
                        }
                        guard -= 1;
                    }
                    if guard == 0 {
                        return Err(TransformError::OutsideDomain);
                    }
                }
                Ok((c_x * lam * (m + phi.cos()), c_y * phi))
            }
            GnKind::Ellipsoid { es, meridian } => {
                let s = phi.sin();
                Ok((
                    lam * phi.cos() / (1.0 - es * s * s).sqrt(),
                    meridian.forward(phi),
                ))
            }
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        match self.kind {
            GnKind::Sphere { m, n, c_x, c_y } => {
                let y = y / c_y;
                let phi = if m != 0.0 {
                    asin_or((m * y + y.sin()) / n)?
                } else if n != 1.0 {
                    asin_or(y.sin() / n)?
                } else {
                    y
                };
                Ok((x / (c_x * (m + y.cos())), phi))
            }
            GnKind::Ellipsoid { es, meridian } => {
                let phi = meridian.inverse(y);
                let magnitude = phi.abs();
                let lam = if magnitude < FRAC_PI_2 {
                    let s = phi.sin();
                    x * (1.0 - es * s * s).sqrt() / phi.cos()
                } else if magnitude - 1e-10 < FRAC_PI_2 {
                    0.0
                } else {
                    return Err(TransformError::OutsideDomain);
                };
                Ok((lam, phi))
            }
        }
    }
}

fn gn_sphere(m: f64, n: f64) -> GnKind {
    let c_y = ((m + 1.0) / n).sqrt();
    GnKind::Sphere {
        m,
        n,
        c_x: c_y / (m + 1.0),
        c_y,
    }
}

struct Hatano;

impl Kernel for Hatano {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const CN: f64 = 2.675_95;
        const CSZ: f64 = 2.437_63;
        const FYCN: f64 = 1.758_59;
        const FYCS: f64 = 1.930_52;
        const FXC: f64 = 0.85;
        let c = phi.sin() * if phi < 0.0 { CSZ } else { CN };
        let mut phi = phi;
        let mut guard = 20;
        while guard != 0 {
            let th1 = (phi + phi.sin() - c) / (1.0 + phi.cos());
            phi -= th1;
            if th1.abs() < 1e-7 {
                break;
            }
            guard -= 1;
        }
        phi *= 0.5;
        Ok((
            FXC * lam * phi.cos(),
            phi.sin() * if phi < 0.0 { FYCS } else { FYCN },
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const RCN: f64 = 0.373_699_060_146_863_730_63;
        const RCS: f64 = 0.410_234_531_081_419_247_38;
        const RYCN: f64 = 0.568_637_374_260_060_616_74;
        const RYCS: f64 = 0.517_995_151_565_381_348_03;
        const RXC: f64 = 1.176_470_588_235_294_117_64;
        const ONETOL: f64 = 1.000_001;
        let mut th = y * if y < 0.0 { RYCS } else { RYCN };
        if th.abs() > 1.0 {
            if th.abs() > ONETOL {
                return Err(TransformError::OutsideDomain);
            }
            th = if th > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
        } else {
            th = th.asin();
        }
        let lam = RXC * x / th.cos();
        th += th;
        let mut phi = (th + th.sin()) * if y < 0.0 { RCS } else { RCN };
        if phi.abs() > 1.0 {
            if phi.abs() > ONETOL {
                return Err(TransformError::OutsideDomain);
            }
            phi = if phi > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
        } else {
            phi = phi.asin();
        }
        Ok((lam, phi))
    }
}

struct MbtFps;

impl Kernel for MbtFps {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const C1: f64 = 0.455_03;
        const C2: f64 = 1.365_09;
        const C3: f64 = 1.415_46;
        const C_X: f64 = 0.222_48;
        const C_Y: f64 = 1.444_92;
        const C1_2: f64 = 0.333_333_333_333_333_333_333_333_33;
        let k = C3 * phi.sin();
        let mut phi = phi;
        let mut guard = 10;
        while guard != 0 {
            let t = phi / C2;
            let v = (C1 * t.sin() + phi.sin() - k) / (C1_2 * t.cos() + phi.cos());
            phi -= v;
            if v.abs() < 1e-7 {
                break;
            }
            guard -= 1;
        }
        let t = phi / C2;
        Ok((C_X * lam * (1.0 + 3.0 * phi.cos() / t.cos()), C_Y * t.sin()))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const C1: f64 = 0.455_03;
        const C2: f64 = 1.365_09;
        const C3: f64 = 1.415_46;
        const C_X: f64 = 0.222_48;
        const C_Y: f64 = 1.444_92;
        let t = asin_or(y / C_Y)?;
        let mut phi = C2 * t;
        let lam = x / (C_X * (1.0 + 3.0 * phi.cos() / t.cos()));
        phi = asin_or((C1 * t.sin() + phi.sin()) / C3)?;
        Ok((lam, phi))
    }
}

struct NellH;

impl Kernel for NellH {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((
            0.5 * lam * (1.0 + phi.cos()),
            2.0 * (phi - (0.5 * phi).tan()),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let p = 0.5 * y;
        let mut phi: f64 = 0.0;
        let mut guard = 9;
        while guard != 0 {
            let c = (0.5 * phi).cos();
            let v = (phi - (phi / 2.0).tan() - p) / (1.0 - 0.5 / (c * c));
            phi -= v;
            if v.abs() < 1e-7 {
                break;
            }
            guard -= 1;
        }
        let lam = if guard == 0 {
            phi = if p < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
            2.0 * x
        } else {
            2.0 * x / (1.0 + phi.cos())
        };
        Ok((lam, phi))
    }
}

struct Mbtfpp;

impl Kernel for Mbtfpp {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const CSY: f64 = 0.952_579_344_415_680_371_52;
        const FXC: f64 = 0.925_820_099_772_551_461_56;
        const FYC: f64 = 3.401_680_257_083_045_044_93;
        let phi = (CSY * phi.sin()).asin();
        Ok((
            FXC * lam * (2.0 * (2.0 / 3.0 * phi).cos() - 1.0),
            FYC * (phi / 3.0).sin(),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const CSY: f64 = 0.952_579_344_415_680_371_52;
        const FXC: f64 = 0.925_820_099_772_551_461_56;
        const FYC: f64 = 3.401_680_257_083_045_044_93;
        const ONEEPS: f64 = 1.000_000_1;
        let mut phi = y / FYC;
        phi = clamp_asin(phi, ONEEPS)?;
        phi *= 3.0;
        let lam = x / (FXC * (2.0 * (2.0 / 3.0 * phi).cos() - 1.0));
        phi = phi.sin() / CSY;
        Ok((lam, clamp_asin(phi, ONEEPS)?))
    }
}

struct Mbtfpq;

impl Kernel for Mbtfpq {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const C: f64 = 1.707_106_781_186_547_524_40;
        const FYC: f64 = 1.874_758_284_622_694_955_05;
        const FXC: f64 = 0.312_459_714_103_782_492_50;
        let c = C * phi.sin();
        let mut phi = phi;
        let mut guard = 20;
        while guard != 0 {
            let th1 = ((0.5 * phi).sin() + phi.sin() - c) / (0.5 * (0.5 * phi).cos() + phi.cos());
            phi -= th1;
            if th1.abs() < 1e-7 {
                break;
            }
            guard -= 1;
        }
        Ok((
            FXC * lam * (1.0 + 2.0 * phi.cos() / (0.5 * phi).cos()),
            FYC * (0.5 * phi).sin(),
        ))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const RC: f64 = 0.585_786_437_626_904_951_19;
        const RYC: f64 = 0.533_402_096_794_177_016_85;
        const RXC: f64 = 3.200_412_580_765_062_101_22;
        const ONETOL: f64 = 1.000_001;
        let mut phi = RYC * y;
        let t = if phi.abs() > 1.0 {
            if phi.abs() > ONETOL {
                return Err(TransformError::OutsideDomain);
            } else if phi < 0.0 {
                phi = -PI;
                -1.0
            } else {
                phi = PI;
                1.0
            }
        } else {
            let t = phi;
            phi = 2.0 * phi.asin();
            t
        };
        let lam = RXC * x / (1.0 + 2.0 * phi.cos() / (0.5 * phi).cos());
        phi = RC * (t + phi.sin());
        if phi.abs() > 1.0 {
            if phi.abs() > ONETOL {
                return Err(TransformError::OutsideDomain);
            }
            phi = if phi < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
        } else {
            phi = phi.asin();
        }
        Ok((lam, phi))
    }
}

struct Bacon {
    bacn: bool,
    ortl: bool,
}

impl Kernel for Bacon {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const HLFPI2: f64 = 2.467_401_100_272_339_654_67;
        const EPS: f64 = 1e-10;
        let y = if self.bacn {
            FRAC_PI_2 * phi.sin()
        } else {
            phi
        };
        let ax = lam.abs();
        let mut x = if ax >= EPS {
            if self.ortl && ax >= FRAC_PI_2 {
                (HLFPI2 - phi * phi + EPS).sqrt() + ax - FRAC_PI_2
            } else {
                let f = 0.5 * (HLFPI2 / ax + ax);
                ax - f + (f * f - y * y).sqrt()
            }
        } else {
            0.0
        };
        if lam < 0.0 {
            x = -x;
        }
        Ok((x, y))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

struct Times;

impl Kernel for Times {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let t = (phi / 2.0).tan();
        let s2 = (FRAC_PI_4 * t).sin().powi(2);
        Ok((lam * (0.744_82 - 0.345_88 * s2), 1.707_11 * t))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let t = y / 1.707_11;
        let s2 = (FRAC_PI_4 * t).sin().powi(2);
        Ok((x / (0.744_82 - 0.345_88 * s2), 2.0 * t.atan()))
    }
}

struct Urmfps {
    n: f64,
    c_y: f64,
}

const URMFPS_CX: f64 = 0.877_382_675_3;
const URMFPS_CY: f64 = 1.139_753_528_477;

impl Kernel for Urmfps {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let phi = asin_or(self.n * phi.sin())?;
        Ok((URMFPS_CX * lam * phi.cos(), self.c_y * phi))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let y = y / self.c_y;
        Ok((x / (URMFPS_CX * y.cos()), asin_or(y.sin() / self.n)?))
    }
}

struct Goode {
    sinu: GnSinu,
    moll: Moll,
}

const GOODE_Y: f64 = 0.052_80;
const GOODE_LIM: f64 = 0.710_930_781_979_023_580_62;

impl Kernel for Goode {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if phi.abs() <= GOODE_LIM {
            self.sinu.forward(lam, phi)
        } else {
            let (x, mut y) = self.moll.forward(lam, phi)?;
            y -= if phi >= 0.0 { GOODE_Y } else { -GOODE_Y };
            Ok((x, y))
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if y.abs() <= GOODE_LIM {
            self.sinu.inverse(x, y)
        } else {
            let y = y + if y >= 0.0 { GOODE_Y } else { -GOODE_Y };
            self.moll.inverse(x, y)
        }
    }
}

struct Gins8;

impl Kernel for Gins8 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const CL: f64 = 0.000_952_426;
        const CP: f64 = 0.162_388;
        const C12: f64 = 0.083_333_333_333_333_33;
        let t = phi * phi;
        let mut x = lam * (1.0 - CP * t);
        let lam2 = lam * lam;
        x *= 0.87 - CL * lam2 * lam2;
        Ok((x, phi * (1.0 + t * C12)))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

const NONE: &[&str] = &[];
const LAT_1: &[&str] = &["lat_1"];
const LAT_PAIR: &[&str] = &["lat_1", "lat_2"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "hammer" => &["W", "M"],
        "wink2" => LAT_1,
        "loxim" => LAT_PAIR,
        "gn_sinu" => &["m", "n"],
        "urmfps" => &["n"],
        "kav5" | "qua_aut" | "fouc" | "mbt_s" | "moll" | "wag4" | "wag5" | "tobmerc" | "putp3"
        | "putp3p" | "putp4p" | "weren" | "putp5" | "putp5p" | "putp6" | "putp6p" | "sinu"
        | "eck6" | "mbtfps" | "hatano" | "mbt_fps" | "nell_h" | "mbtfpp" | "mbtfpq" | "bacon"
        | "apian" | "ortel" | "times" | "wag1" | "gins8" | "goode" => NONE,
        _ => return None,
    })
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let built = match name {
        "kav5" => sts(frame, 1.504_88, 1.354_39, false),
        "qua_aut" => sts(frame, 2.0, 2.0, false),
        "fouc" => sts(frame, 2.0, 2.0, true),
        "mbt_s" => sts(frame, 1.488_75, 1.365_09, false),
        "moll" => moll_from_p(frame, FRAC_PI_2),
        "wag4" => moll_from_p(frame, PI / 3.0),
        "wag5" => step(
            frame,
            Moll {
                c_x: 0.909_77,
                c_y: 1.650_14,
                c_p: 3.008_96,
            },
        ),
        "tobmerc" => step(frame, Tobmerc { k0: frame.k0 }),
        "hammer" => {
            let w = if params.flag("W") {
                let w = params.number("W")?.unwrap_or(0.0).abs();
                if w <= 0.0 {
                    return Err(CreateError::IllegalValue("W"));
                }
                w
            } else {
                0.5
            };
            let mut m = if params.flag("M") {
                let m = params.number("M")?.unwrap_or(0.0).abs();
                if m <= 0.0 {
                    return Err(CreateError::IllegalValue("M"));
                }
                m
            } else {
                1.0
            };
            let rm = 1.0 / m;
            m /= w;
            step(frame, Hammer { w, m, rm })
        }
        "wink2" => {
            let lat1 = params.radians("lat_1")?.unwrap_or(0.0);
            step(
                frame,
                Wink2 {
                    cosphi1: lat1.cos(),
                },
            )
        }
        "loxim" => {
            let phi1 = params.radians("lat_1")?.unwrap_or(0.0);
            let cosphi1 = phi1.cos();
            if cosphi1 < LOXIM_EPS {
                return Err(CreateError::IllegalValue("lat_1"));
            }
            step(
                frame,
                Loxim {
                    phi1,
                    cosphi1,
                    tanphi1: (FRAC_PI_4 + 0.5 * phi1).tan(),
                },
            )
        }
        "putp3" => step(
            frame,
            Putp3 {
                a: 4.0 * 0.101_321_183_6,
            },
        ),
        "putp3p" => step(
            frame,
            Putp3 {
                a: 2.0 * 0.101_321_183_6,
            },
        ),
        "putp4p" => step(
            frame,
            Putp4 {
                c_x: 0.874_038_744,
                c_y: 3.883_251_825,
            },
        ),
        "weren" => step(
            frame,
            Putp4 {
                c_x: 1.0,
                c_y: 4.442_882_938,
            },
        ),
        "putp5" => step(frame, Putp5 { a: 2.0, b: 1.0 }),
        "putp5p" => step(frame, Putp5 { a: 1.5, b: 0.5 }),
        "putp6" => step(
            frame,
            Putp6 {
                c_x: 1.013_46,
                c_y: 0.919_10,
                a: 4.0,
                b: 2.147_143_718_212_937_878_4,
                d: 2.0,
            },
        ),
        "putp6p" => step(
            frame,
            Putp6 {
                c_x: 0.443_29,
                c_y: 0.804_04,
                a: 6.0,
                b: 5.611_25,
                d: 3.0,
            },
        ),
        "sinu" => step(
            frame,
            GnSinu {
                kind: if frame.earth.es() == 0.0 {
                    gn_sphere(0.0, 1.0)
                } else {
                    GnKind::Ellipsoid {
                        es: frame.earth.es(),
                        meridian: Meridian::new(frame.earth.n()),
                    }
                },
            },
        ),
        "eck6" => step(
            frame,
            GnSinu {
                kind: gn_sphere(1.0, 2.570_796_326_794_896_619_231_321_691),
            },
        ),
        "mbtfps" => step(
            frame,
            GnSinu {
                kind: gn_sphere(0.5, 1.785_398_163_397_448_309_615_660_845),
            },
        ),
        "gn_sinu" => {
            if !params.flag("n") {
                return Err(CreateError::MissingArgument("n"));
            }
            if !params.flag("m") {
                return Err(CreateError::MissingArgument("m"));
            }
            let n = params.number("n")?.unwrap_or(0.0);
            let m = params.number("m")?.unwrap_or(0.0);
            if n <= 0.0 {
                return Err(CreateError::IllegalValue("n"));
            }
            if m < 0.0 {
                return Err(CreateError::IllegalValue("m"));
            }
            step(
                frame,
                GnSinu {
                    kind: gn_sphere(m, n),
                },
            )
        }
        "hatano" => step(frame, Hatano),
        "mbt_fps" => step(frame, MbtFps),
        "nell_h" => step(frame, NellH),
        "mbtfpp" => step(frame, Mbtfpp),
        "mbtfpq" => step(frame, Mbtfpq),
        "bacon" => step(
            frame,
            Bacon {
                bacn: true,
                ortl: false,
            },
        ),
        "apian" => step(
            frame,
            Bacon {
                bacn: false,
                ortl: false,
            },
        ),
        "ortel" => step(
            frame,
            Bacon {
                bacn: false,
                ortl: true,
            },
        ),
        "times" => step(frame, Times),
        "wag1" => step(
            frame,
            Urmfps {
                n: 0.866_025_403_784_438_646_763_723_170_7,
                c_y: URMFPS_CY / 0.866_025_403_784_438_646_763_723_170_7,
            },
        ),
        "urmfps" => {
            if !params.flag("n") {
                return Err(CreateError::MissingArgument("n"));
            }
            let n = params.number("n")?.unwrap_or(0.0);
            if n <= 0.0 || n > 1.0 {
                return Err(CreateError::IllegalValue("n"));
            }
            step(
                frame,
                Urmfps {
                    n,
                    c_y: URMFPS_CY / n,
                },
            )
        }
        "gins8" => step(frame, Gins8),
        "goode" => {
            let p2 = PI;
            let sp = 1.0;
            let r = (2.0 * PI * sp / (p2 + p2.sin())).sqrt();
            step(
                frame,
                Goode {
                    sinu: GnSinu {
                        kind: gn_sphere(0.0, 1.0),
                    },
                    moll: Moll {
                        c_x: 2.0 * r / PI,
                        c_y: r / sp,
                        c_p: p2 + p2.sin(),
                    },
                },
            )
        }
        _ => return Ok(None),
    };
    Ok(Some(built))
}
