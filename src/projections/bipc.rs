//! Bipolar conic of the western hemisphere, PROJ 9.9 `bipc.cpp`.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from bipc.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS: f64 = 1e-10;
const ONEEPS: f64 = 1.000_000_001;
const LAM_B: f64 = -0.348_949_767_262_506_815_39;
const N: f64 = 0.630_558_448_812_746_871_80;
const F: f64 = 1.897_247_425_674_610_305_82;
const AZ_AB: f64 = 0.816_500_436_746_863_631_66;
const AZ_BA: f64 = 1.822_618_438_561_859_251_33;
const T: f64 = 1.272_465_782_670_890_122_70;
const RHO_C: f64 = 1.207_091_215_215_687_219_27;
const C_AZC: f64 = 0.696_915_230_386_783_755_19;
const S_AZC: f64 = 0.717_153_513_311_436_075_55;
const C45: f64 = 0.707_106_781_186_547_524_69;
const S45: f64 = 0.707_106_781_186_547_524_10;
const C20: f64 = 0.939_692_620_785_908_384_11;
const S20: f64 = -0.342_020_143_325_668_732_87;
const R110: f64 = 1.919_862_177_193_762_533_60;
const R104: f64 = 1.815_142_422_074_102_759_04;

fn clamp_acos(mut z: f64) -> Result<f64, TransformError> {
    if z.abs() > 1.0 {
        if z.abs() > ONEEPS {
            return Err(TransformError::OutsideDomain);
        }
        z = if z < 0.0 { -1.0 } else { 1.0 };
        Ok(z)
    } else {
        Ok(z.acos())
    }
}

struct Bipc {
    noskew: bool,
}

impl Kernel for Bipc {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let cphi = phi.cos();
        let sphi = phi.sin();
        let mut cdlam = (LAM_B - lam).cos();
        let mut sdlam = (LAM_B - lam).sin();
        let pole = (phi.abs() - FRAC_PI_2).abs() < EPS;
        let tphi = sphi / cphi;
        let mut az = if pole {
            if phi < 0.0 {
                PI
            } else {
                0.0
            }
        } else {
            sdlam.atan2(C45 * (tphi - cdlam))
        };
        let tag = az > AZ_BA;
        let (z, av, mut y) = if tag {
            sdlam = lam + R110;
            cdlam = sdlam.cos();
            sdlam = sdlam.sin();
            let z = clamp_acos(S20 * sphi + C20 * cphi * cdlam)?;
            if !pole {
                az = sdlam.atan2(C20 * tphi - S20 * cdlam);
            }
            (z, AZ_AB, RHO_C)
        } else {
            let z = clamp_acos(S45 * (sphi + cphi * cdlam))?;
            (z, AZ_BA, -RHO_C)
        };
        if z < 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let t_pow = (0.5 * z).tan().powf(N);
        let mut r = F * t_pow;
        let al = 0.5 * (R104 - z);
        if al < 0.0 {
            return Err(TransformError::OutsideDomain);
        }
        let al = clamp_acos((t_pow + al.powf(N)) / T)?;
        let t = N * (av - az);
        if t.abs() < al {
            r /= (al + if tag { t } else { -t }).cos();
        }
        let mut x = r * t.sin();
        y += (if tag { -r } else { r }) * t.cos();
        if self.noskew {
            let saved = x;
            x = -x * C_AZC - y * S_AZC;
            y = -y * C_AZC + saved * S_AZC;
        }
        Ok((x, y))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        if self.noskew {
            let saved = x;
            x = -x * C_AZC + y * S_AZC;
            y = -y * C_AZC - saved * S_AZC;
        }
        let neg = x < 0.0;
        let (s, c, av) = if neg {
            y = RHO_C - y;
            (S20, C20, AZ_AB)
        } else {
            y += RHO_C;
            (S45, C45, AZ_BA)
        };
        let rp = x.hypot(y);
        let mut az = x.atan2(y);
        let faz = az.abs();
        let mut r = rp;
        let mut previous = rp;
        let mut z = 0.0;
        let mut guard = 10;
        while guard != 0 {
            z = 2.0 * (r / F).powf(1.0 / N).atan();
            let al = (((0.5 * z).tan().powf(N) + (0.5 * (R104 - z)).tan().powf(N)) / T).acos();
            if faz < al {
                r = rp * (al + if neg { az } else { -az }).cos();
            }
            if (previous - r).abs() < EPS {
                break;
            }
            previous = r;
            guard -= 1;
        }
        if guard == 0 {
            return Err(TransformError::OutsideDomain);
        }
        az = av - az / N;
        let phi = (s * z.cos() + c * z.sin() * az.cos()).asin();
        let mut lam = az.sin().atan2(c / z.tan() - s * az.cos());
        if neg {
            lam -= R110;
        } else {
            lam = LAM_B - lam;
        }
        Ok((lam, phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "bipc" {
        Some(&["ns"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name == "bipc" {
        Ok(Some(classic_step(
            frame,
            Bipc {
                noskew: params.flag("ns"),
            },
        )))
    } else {
        Ok(None)
    }
}
