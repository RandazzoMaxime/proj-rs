//! Robinson projection. Coefficients stay `f32`, as in PROJ `robin.cpp`,
//! and are promoted to `f64` only inside the polynomial.

#![allow(
    clippy::as_conversions,
    clippy::pedantic,
    reason = "the coefficient table is transcribed from robin.cpp and indexed by node"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::FRAC_PI_2;

const DEG: f64 = 0.017_453_292_519_943_296;
const RAD_TO_DEG: f64 = 57.295_779_513_082_321;
const FXC: f64 = 0.8487;
const FYC: f64 = 1.3523;
const C1: f64 = 11.459_155_902_616_464_175_44;
const RC1: f64 = 0.087_266_462_599_716_478_84;
const NODES: i64 = 18;
const ONEEPS: f64 = 1.000_001;

#[derive(Clone, Copy)]
struct Coefs {
    c0: f32,
    c1: f32,
    c2: f32,
    c3: f32,
}

fn poly(coef: Coefs, z: f64) -> f64 {
    let c0 = f64::from(coef.c0);
    let c1 = f64::from(coef.c1);
    let c2 = f64::from(coef.c2);
    let c3 = f64::from(coef.c3);
    c0 + z * (c1 + z * (c2 + z * c3))
}

fn slope(coef: Coefs, z: f64) -> f64 {
    let c1 = f64::from(coef.c1);
    let c2 = f64::from(coef.c2);
    let c3 = f64::from(coef.c3);
    c1 + 2.0 * z * c2 + z * z * 3.0 * c3
}

const X: [Coefs; 19] = [
    Coefs {
        c0: 1.0,
        c1: 2.2199e-17,
        c2: -7.15515e-05,
        c3: 3.1103e-06,
    },
    Coefs {
        c0: 0.9986,
        c1: -0.000_482_243,
        c2: -2.4897e-05,
        c3: -1.3309e-06,
    },
    Coefs {
        c0: 0.9954,
        c1: -0.000_831_03,
        c2: -4.48605e-05,
        c3: -9.86701e-07,
    },
    Coefs {
        c0: 0.99,
        c1: -0.001_353_64,
        c2: -5.9661e-05,
        c3: 3.6777e-06,
    },
    Coefs {
        c0: 0.9822,
        c1: -0.001_674_42,
        c2: -4.49547e-06,
        c3: -5.72411e-06,
    },
    Coefs {
        c0: 0.973,
        c1: -0.002_148_68,
        c2: -9.03571e-05,
        c3: 1.8736e-08,
    },
    Coefs {
        c0: 0.96,
        c1: -0.003_050_85,
        c2: -9.00761e-05,
        c3: 1.64917e-06,
    },
    Coefs {
        c0: 0.9427,
        c1: -0.003_827_92,
        c2: -6.53386e-05,
        c3: -2.6154e-06,
    },
    Coefs {
        c0: 0.9216,
        c1: -0.004_677_46,
        c2: -0.000_104_57,
        c3: 4.81243e-06,
    },
    Coefs {
        c0: 0.8962,
        c1: -0.005_362_23,
        c2: -3.23831e-05,
        c3: -5.43432e-06,
    },
    Coefs {
        c0: 0.8679,
        c1: -0.006_093_63,
        c2: -0.000_113_898,
        c3: 3.32484e-06,
    },
    Coefs {
        c0: 0.835,
        c1: -0.006_983_25,
        c2: -6.40253e-05,
        c3: 9.34959e-07,
    },
    Coefs {
        c0: 0.7986,
        c1: -0.007_553_38,
        c2: -5.00009e-05,
        c3: 9.35324e-07,
    },
    Coefs {
        c0: 0.7597,
        c1: -0.007_983_24,
        c2: -3.5971e-05,
        c3: -2.27626e-06,
    },
    Coefs {
        c0: 0.7186,
        c1: -0.008_513_67,
        c2: -7.01149e-05,
        c3: -8.6303e-06,
    },
    Coefs {
        c0: 0.6732,
        c1: -0.009_862_09,
        c2: -0.000_199_569,
        c3: 1.91974e-05,
    },
    Coefs {
        c0: 0.6213,
        c1: -0.010_418,
        c2: 8.83923e-05,
        c3: 6.24051e-06,
    },
    Coefs {
        c0: 0.5722,
        c1: -0.009_066_01,
        c2: 0.000_182,
        c3: 6.24051e-06,
    },
    Coefs {
        c0: 0.5322,
        c1: -0.006_777_97,
        c2: 0.000_275_608,
        c3: 6.24051e-06,
    },
];

const Y: [Coefs; 19] = [
    Coefs {
        c0: -5.20417e-18,
        c1: 0.0124,
        c2: 1.21431e-18,
        c3: -8.45284e-11,
    },
    Coefs {
        c0: 0.062,
        c1: 0.0124,
        c2: -1.26793e-09,
        c3: 4.22642e-10,
    },
    Coefs {
        c0: 0.124,
        c1: 0.0124,
        c2: 5.07171e-09,
        c3: -1.60604e-09,
    },
    Coefs {
        c0: 0.186,
        c1: 0.012_399_9,
        c2: -1.90189e-08,
        c3: 6.00152e-09,
    },
    Coefs {
        c0: 0.248,
        c1: 0.012_400_2,
        c2: 7.10039e-08,
        c3: -2.24e-08,
    },
    Coefs {
        c0: 0.31,
        c1: 0.012_399_2,
        c2: -2.64997e-07,
        c3: 8.35986e-08,
    },
    Coefs {
        c0: 0.372,
        c1: 0.012_402_9,
        c2: 9.88983e-07,
        c3: -3.11994e-07,
    },
    Coefs {
        c0: 0.434,
        c1: 0.012_389_3,
        c2: -3.69093e-06,
        c3: -4.35621e-07,
    },
    Coefs {
        c0: 0.4958,
        c1: 0.012_319_8,
        c2: -1.02252e-05,
        c3: -3.45523e-07,
    },
    Coefs {
        c0: 0.5571,
        c1: 0.012_191_6,
        c2: -1.54081e-05,
        c3: -5.82288e-07,
    },
    Coefs {
        c0: 0.6176,
        c1: 0.011_993_8,
        c2: -2.41424e-05,
        c3: -5.25327e-07,
    },
    Coefs {
        c0: 0.6769,
        c1: 0.011_713,
        c2: -3.20223e-05,
        c3: -5.16405e-07,
    },
    Coefs {
        c0: 0.7346,
        c1: 0.011_354_1,
        c2: -3.97684e-05,
        c3: -6.09052e-07,
    },
    Coefs {
        c0: 0.7903,
        c1: 0.010_910_7,
        c2: -4.89042e-05,
        c3: -1.04739e-06,
    },
    Coefs {
        c0: 0.8435,
        c1: 0.010_343_1,
        c2: -6.4615e-05,
        c3: -1.40374e-09,
    },
    Coefs {
        c0: 0.8936,
        c1: 0.009_696_86,
        c2: -6.4636e-05,
        c3: -8.547e-06,
    },
    Coefs {
        c0: 0.9394,
        c1: 0.008_409_47,
        c2: -0.000_192_841,
        c3: -4.2106e-06,
    },
    Coefs {
        c0: 0.9761,
        c1: 0.006_165_27,
        c2: -0.000_256,
        c3: -4.2106e-06,
    },
    Coefs {
        c0: 1.0,
        c1: 0.003_289_47,
        c2: -0.000_319_159,
        c3: -4.2106e-06,
    },
];

struct Robin;

impl Kernel for Robin {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut width = phi.abs();
        if width.is_nan() {
            return Err(TransformError::OutsideDomain);
        }
        let mut index = (width * C1 + 1e-15).floor() as i64;
        if index < 0 {
            return Err(TransformError::OutsideDomain);
        }
        if index >= NODES {
            index = NODES;
        }
        width = RAD_TO_DEG * (width - RC1 * index as f64);
        let x = poly(X[index as usize], width) * FXC * lam;
        let mut y = poly(Y[index as usize], width) * FYC;
        if phi < 0.0 {
            y = -y;
        }
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut lam = x / FXC;
        let mut phi = (y / FYC).abs();
        if phi >= 1.0 {
            if phi > ONEEPS {
                return Err(TransformError::OutsideDomain);
            }
            phi = if y < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 };
            lam /= f64::from(X[NODES as usize].c0);
            return Ok((lam, phi));
        }
        let mut index = (phi * NODES as f64).floor() as i64;
        if index < 0 || index >= NODES {
            return Err(TransformError::OutsideDomain);
        }
        loop {
            if f64::from(Y[index as usize].c0) > phi {
                index -= 1;
            } else if f64::from(Y[(index + 1) as usize].c0) <= phi {
                index += 1;
            } else {
                break;
            }
            if !(0..NODES).contains(&index) {
                return Err(TransformError::OutsideDomain);
            }
        }
        let row = Y[index as usize];
        let mut t = 5.0 * (phi - f64::from(row.c0))
            / (f64::from(Y[(index + 1) as usize].c0) - f64::from(row.c0));
        let mut guard = 100;
        while guard != 0 {
            let step = (poly(row, t) - phi) / slope(row, t);
            t -= step;
            if step.abs() < 1e-10 {
                break;
            }
            guard -= 1;
        }
        if guard == 0 {
            return Err(TransformError::OutsideDomain);
        }
        phi = (5.0 * index as f64 + t) * DEG;
        if y < 0.0 {
            phi = -phi;
        }
        lam /= poly(X[index as usize], t);
        if lam.abs() > std::f64::consts::PI {
            return Err(TransformError::OutsideDomain);
        }
        Ok((lam, phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "robin" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name == "robin" {
        Ok(Some(classic_step(frame, Robin)))
    } else {
        Ok(None)
    }
}
