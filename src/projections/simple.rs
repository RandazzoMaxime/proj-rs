//! Sphere projections whose PROJ sources are a single forward and inverse.
//!
//! Each kernel follows the named function in PROJ 9.9 `src/projections/`.
//! Setting `es = 0` there keeps the semi-major axis, which the classic
//! wrapper already multiplies by. Scale `k0` is applied only where the C
//! function reads `P->k0`.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{aasin, asqrt};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const EPS10: f64 = 1e-10;
const ONEEPS: f64 = 1.000_000_1;

fn asin_proj(v: f64) -> Result<f64, TransformError> {
    aasin(v).map_err(|()| TransformError::OutsideDomain)
}

struct Mill;
struct Gall;
struct Eck1;
struct Eck2;
struct Eck5;
struct Cc;
struct Tcc;
struct Collg;
struct Crast;
struct Fahey;
struct Comill;

struct Tcea {
    k0: f64,
    phi0: f64,
}
struct FoucS {
    n: f64,
    n1: f64,
}
struct EckFamily {
    cx: f64,
    cy: f64,
    a: f64,
    b: f64,
}
struct Wag3 {
    cx: f64,
}
struct Wink1 {
    cosphi1: f64,
}
struct Wag2;
struct Eck4 {
    over: bool,
}

impl Kernel for Mill {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((lam, (FRAC_PI_4 + phi * 0.4).tan().ln() * 1.25))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        Ok((x, 2.5 * ((0.8 * y).exp().atan() - FRAC_PI_4)))
    }
}

impl Kernel for Gall {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const XF: f64 = 0.707_106_781_186_547_524_40;
        const YF: f64 = 1.707_106_781_186_547_524_40;
        Ok((XF * lam, YF * (0.5 * phi).tan()))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const RXF: f64 = 1.414_213_562_373_095_048_80;
        const RYF: f64 = 0.585_786_437_626_904_951_19;
        Ok((RXF * x, 2.0 * (y * RYF).atan()))
    }
}

impl Kernel for Eck1 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const FC: f64 = 0.921_317_731_923_561_278_02;
        const RP: f64 = 0.318_309_886_183_790_671_54;
        Ok((FC * lam * (1.0 - RP * phi.abs()), FC * phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const FC: f64 = 0.921_317_731_923_561_278_02;
        const RP: f64 = 0.318_309_886_183_790_671_54;
        let phi = y / FC;
        Ok((x / (FC * (1.0 - RP * phi.abs())), phi))
    }
}

impl Kernel for Eck2 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const FXC: f64 = 0.460_658_865_961_780_639_02;
        const FYC: f64 = 1.447_202_509_116_535_318_71;
        let width = (4.0 - 3.0 * phi.abs().sin()).sqrt();
        let mut y = FYC * (2.0 - width);
        if phi < 0.0 {
            y = -y;
        }
        Ok((FXC * lam * width, y))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const FXC: f64 = 0.460_658_865_961_780_639_02;
        const FYC: f64 = 1.447_202_509_116_535_318_71;
        const C13: f64 = 0.333_333_333_333_333_333_33;
        let width = 2.0 - y.abs() / FYC;
        let lam = x / (FXC * width);
        let mut sine = (4.0 - width * width) * C13;
        if sine.abs() >= 1.0 {
            if sine.abs() > ONEEPS {
                return Err(TransformError::OutsideDomain);
            }
            sine = if sine < 0.0 { -1.0 } else { 1.0 };
        }
        let mut phi = sine.asin();
        if y < 0.0 {
            phi = -phi;
        }
        Ok((lam, phi))
    }
}

impl Kernel for Eck5 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const XF: f64 = 0.441_012_771_724_551_482_19;
        const YF: f64 = 0.882_025_543_449_102_964_38;
        Ok((XF * (1.0 + phi.cos()) * lam, YF * phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const RXF: f64 = 2.267_508_027_238_226_391_37;
        const RYF: f64 = 1.133_754_013_619_113_195_68;
        let phi = RYF * y;
        Ok((RXF * x / (1.0 + phi.cos()), phi))
    }
}

impl Kernel for Cc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if (phi.abs() - FRAC_PI_2).abs() <= EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam, phi.tan()))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        Ok((x, y.atan()))
    }
}

impl Kernel for Tcc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let b = phi.cos() * lam.sin();
        let bt = 1.0 - b * b;
        if bt < EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        Ok((b / bt.sqrt(), phi.tan().atan2(lam.cos())))
    }
    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

impl Kernel for Tcea {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((
            phi.cos() * lam.sin() / self.k0,
            self.k0 * (phi.tan().atan2(lam.cos()) - self.phi0),
        ))
    }
    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        y = y / self.k0 + self.phi0;
        x *= self.k0;
        let t = (1.0 - x * x).sqrt();
        Ok((x.atan2(t * y.cos()), (t * y.sin()).asin()))
    }
}

impl Kernel for Collg {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const FXC: f64 = 1.128_379_167_095_512_573_90;
        const FYC: f64 = 1.772_453_850_905_516_027_29;
        let mut width = 1.0 - phi.sin();
        width = if width <= 0.0 { 0.0 } else { width.sqrt() };
        Ok((FXC * lam * width, FYC * (1.0 - width)))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const FXC: f64 = 1.128_379_167_095_512_573_90;
        const FYC: f64 = 1.772_453_850_905_516_027_29;
        let mut sine = y / FYC - 1.0;
        sine = 1.0 - sine * sine;
        let phi = if sine.abs() < 1.0 {
            sine.asin()
        } else if sine.abs() > ONEEPS {
            return Err(TransformError::OutsideDomain);
        } else if sine < 0.0 {
            -FRAC_PI_2
        } else {
            FRAC_PI_2
        };
        let remain = 1.0 - phi.sin();
        let lam = if remain <= 0.0 {
            0.0
        } else {
            x / (FXC * remain.sqrt())
        };
        Ok((lam, phi))
    }
}

impl Kernel for Crast {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const XM: f64 = 0.977_205_023_805_839_843_17;
        const YM: f64 = 3.069_980_123_839_465_465_42;
        const THIRD: f64 = 0.333_333_333_333_333_333;
        let reduced = phi * THIRD;
        Ok((
            XM * lam * (2.0 * (reduced + reduced).cos() - 1.0),
            YM * reduced.sin(),
        ))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const RXM: f64 = 1.023_326_707_946_488_488_47;
        const RYM: f64 = 0.325_735_007_935_279_947_72;
        const THIRD: f64 = 0.333_333_333_333_333_333;
        let phi = 3.0 * (y * RYM).asin();
        Ok((x * RXM / (2.0 * ((phi + phi) * THIRD).cos() - 1.0), phi))
    }
}

impl Kernel for Fahey {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let t = (0.5 * phi).tan();
        Ok((0.819_152 * lam * asqrt(1.0 - t * t), 1.819_152 * t))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const TOL: f64 = 1e-6;
        let t = y / 1.819_152;
        let phi = 2.0 * t.atan();
        let remain = 1.0 - t * t;
        let lam = if remain.abs() < TOL {
            0.0
        } else {
            x / (0.819_152 * remain.sqrt())
        };
        Ok((lam, phi))
    }
}

impl Kernel for FoucS {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let t = phi.cos();
        Ok((
            lam * t / (self.n + self.n1 * t),
            self.n * phi + self.n1 * phi.sin(),
        ))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = if self.n == 0.0 {
            asin_proj(y)?
        } else {
            let mut phi = y;
            let mut left = 10;
            let mut converged = false;
            while left > 0 {
                left -= 1;
                let step =
                    (self.n * phi + self.n1 * phi.sin() - y) / (self.n + self.n1 * phi.cos());
                phi -= step;
                if step.abs() < 1e-7 {
                    converged = true;
                    break;
                }
            }
            if !converged {
                if y < 0.0 {
                    -FRAC_PI_2
                } else {
                    FRAC_PI_2
                }
            } else {
                phi
            }
        };
        let v = phi.cos();
        Ok((x * (self.n + self.n1 * v) / v, phi))
    }
}

impl Kernel for EckFamily {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((
            self.cx * lam * (self.a + asqrt(1.0 - self.b * phi * phi)),
            self.cy * phi,
        ))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let phi = y / self.cy;
        let denom = self.cx * (self.a + asqrt(1.0 - self.b * phi * phi));
        let lam = if denom == 0.0 {
            f64::INFINITY
        } else {
            x / denom
        };
        Ok((lam, phi))
    }
}

impl Kernel for Wag3 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const TWOTHIRD: f64 = 0.666_666_666_666_666_666_666_7;
        Ok((self.cx * lam * (TWOTHIRD * phi).cos(), phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const TWOTHIRD: f64 = 0.666_666_666_666_666_666_666_7;
        Ok((x / (self.cx * (TWOTHIRD * y).cos()), y))
    }
}

impl Kernel for Wink1 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        Ok((0.5 * lam * (self.cosphi1 + phi.cos()), phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        Ok((2.0 * x / (self.cosphi1 + y.cos()), y))
    }
}

impl Kernel for Wag2 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 0.924_83;
        const C_Y: f64 = 1.387_25;
        const C_P1: f64 = 0.880_22;
        const C_P2: f64 = 0.885_50;
        let phi = asin_proj(C_P1 * (C_P2 * phi).sin())?;
        Ok((C_X * lam * phi.cos(), C_Y * phi))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 0.924_83;
        const C_Y: f64 = 1.387_25;
        const C_P1: f64 = 0.880_22;
        const C_P2: f64 = 0.885_50;
        let phi = y / C_Y;
        let lam = x / (C_X * phi.cos());
        Ok((lam, asin_proj(phi.sin() / C_P1)? / C_P2))
    }
}

impl Kernel for Eck4 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 0.422_238_200_315_771_201_49;
        const C_Y: f64 = 1.326_500_428_177_002_322_18;
        const C_P: f64 = 3.570_796_326_794_896_619_22;
        let p = C_P * phi.sin();
        let mut vv = phi * phi;
        let mut theta = phi * (0.895_168 + vv * (0.021_884_9 + vv * 0.008_268_09));
        let mut converged = false;
        for _ in 0..6 {
            let c = theta.cos();
            let s = theta.sin();
            vv = (theta + s * (c + 2.0) - p) / (1.0 + c * (c + 2.0) - s * s);
            theta -= vv;
            if vv.abs() < 1e-7 {
                converged = true;
                break;
            }
        }
        if converged {
            Ok((C_X * lam * (1.0 + theta.cos()), C_Y * theta.sin()))
        } else {
            Ok((C_X * lam, if theta < 0.0 { -C_Y } else { C_Y }))
        }
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const C_X: f64 = 0.422_238_200_315_771_201_49;
        const RC_Y: f64 = 0.753_863_307_360_021_782_05;
        const RC_P: f64 = 0.280_049_576_755_778_687_95;
        let sin_theta = y * RC_Y;
        let near = 1.0 - sin_theta.abs();
        let (mut lam, phi) = if (0.0..=1e-12).contains(&near) {
            (
                x / C_X,
                if sin_theta > 0.0 {
                    FRAC_PI_2
                } else {
                    -FRAC_PI_2
                },
            )
        } else {
            let theta = asin_proj(sin_theta)?;
            let cos_theta = theta.cos();
            let sin_phi = (theta + sin_theta * (cos_theta + 2.0)) * RC_P;
            (x / (C_X * (1.0 + cos_theta)), asin_proj(sin_phi)?)
        };
        if !self.over {
            let excess = lam.abs() - PI;
            if excess > 0.0 {
                if excess > 1e-10 {
                    return Err(TransformError::OutsideDomain);
                }
                lam = if lam > 0.0 { PI } else { -PI };
            }
        }
        Ok((lam, phi))
    }
}

impl Kernel for Comill {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const K1: f64 = 0.9902;
        const K2: f64 = 0.1604;
        const K3: f64 = -0.03054;
        let lat_sq = phi * phi;
        Ok((lam, phi * (K1 + lat_sq * (K2 + K3 * lat_sq))))
    }
    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        const K1: f64 = 0.9902;
        const K2: f64 = 0.1604;
        const K3: f64 = -0.03054;
        const C1: f64 = K1;
        const C2: f64 = 3.0 * K2;
        const C3: f64 = 5.0 * K3;
        const MAX_Y: f64 = 0.600_020_766_986_265_5 * PI;
        let y = y.clamp(-MAX_Y, MAX_Y);
        let mut yc = y;
        let mut converged = false;
        for _ in 0..100 {
            let y2 = yc * yc;
            let f = yc * (K1 + y2 * (K2 + K3 * y2)) - y;
            let slope = C1 + y2 * (C2 + C3 * y2);
            let step = f / slope;
            yc -= step;
            if step.abs() < 1e-11 {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        Ok((x, yc))
    }
}

const NONE: &[&str] = &[];
const LAT_TS: &[&str] = &["lat_ts"];
const N_KEY: &[&str] = &["n"];
const LAT_PAIR: &[&str] = &["lat_1", "lat_2"];

fn unit<K: Kernel + 'static>(
    frame: &Frame,
    kernel: K,
) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    Ok(classic_step(frame, kernel))
}

/// Parameters accepted on top of the global PROJ keys.
pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "fouc_s" => N_KEY,
        "wag3" | "wink1" => LAT_TS,
        "mill" | "gall" | "eck1" | "eck2" | "eck3" | "eck4" | "eck5" | "putp1" | "wag6"
        | "kav7" | "cc" | "tcc" | "tcea" | "crast" | "fahey" | "comill" | "wag2" => NONE,
        "collg" => LAT_PAIR,
        _ => return None,
    })
}

/// Build one of the projections in this module.
///
/// # Errors
///
/// [`CreateError::IllegalValue`] when a parameter is outside the PROJ range.
pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let step = match name {
        "mill" => unit(frame, Mill)?,
        "gall" => unit(frame, Gall)?,
        "eck1" => unit(frame, Eck1)?,
        "eck2" => unit(frame, Eck2)?,
        "eck5" => unit(frame, Eck5)?,
        "cc" => unit(frame, Cc)?,
        "tcc" => unit(frame, Tcc)?,
        "collg" => unit(frame, Collg)?,
        "crast" => unit(frame, Crast)?,
        "fahey" => unit(frame, Fahey)?,
        "comill" => unit(frame, Comill)?,
        "wag2" => unit(frame, Wag2)?,
        "eck4" => unit(frame, Eck4 { over: frame.over })?,
        "tcea" => unit(
            frame,
            Tcea {
                k0: frame.k0,
                phi0: frame.phi0,
            },
        )?,
        "fouc_s" => {
            let n = params.number("n")?.unwrap_or(0.0);
            if !(0.0..=1.0).contains(&n) {
                return Err(CreateError::IllegalValue("n"));
            }
            unit(frame, FoucS { n, n1: 1.0 - n })?
        }
        "eck3" => unit(
            frame,
            EckFamily {
                cx: 0.422_238_200_315_771_201_49,
                cy: 0.844_476_400_631_542_402_98,
                a: 1.0,
                b: 0.405_284_734_569_351_085_775_5,
            },
        )?,
        "kav7" => unit(
            frame,
            EckFamily {
                cx: 0.866_025_403_784_4,
                cy: 1.0,
                a: 0.0,
                b: 0.303_963_550_927_013_314_33,
            },
        )?,
        "wag6" => unit(
            frame,
            EckFamily {
                cx: 1.0,
                cy: 1.0,
                a: 0.0,
                b: 0.303_963_550_927_013_314_33,
            },
        )?,
        "putp1" => unit(
            frame,
            EckFamily {
                cx: 1.894_90,
                cy: 0.947_45,
                a: -0.5,
                b: 0.303_963_550_927_013_314_33,
            },
        )?,
        "wag3" => {
            let ts = params.radians("lat_ts")?.unwrap_or(0.0);
            unit(
                frame,
                Wag3 {
                    cx: ts.cos() / (2.0 * ts / 3.0).cos(),
                },
            )?
        }
        "wink1" => {
            let ts = params.radians("lat_ts")?.unwrap_or(0.0);
            unit(frame, Wink1 { cosphi1: ts.cos() })?
        }
        _ => return Ok(None),
    };
    Ok(Some(step))
}
