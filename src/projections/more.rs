//! Van der Grinten I, Natural Earth, and the American polyconic.
//!
//! Sphere formulas ignore the ellipsoid flattening. The polyconic keeps it.

#![allow(
    clippy::pedantic,
    reason = "constants and branches are transcribed from the PROJ sources"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{msfn, Meridian};
use std::f64::consts::{FRAC_PI_2, PI};

fn step<K: Kernel + 'static>(frame: &Frame, kernel: K) -> Box<dyn crate::engine::Step> {
    classic_step(frame, kernel)
}

struct Vandg {
    over: bool,
}

const VANDG_TOL: f64 = 1e-10;
const VANDG_THIRD: f64 = 0.333_333_333_333_333_333_33;
const VANDG_C2_27: f64 = 0.074_074_074_074_074_074_07;
const VANDG_PI4_3: f64 = 4.188_790_204_786_390_984_58;
const VANDG_PISQ: f64 = 9.869_604_401_089_358_618_69;
const VANDG_TPISQ: f64 = 19.739_208_802_178_717_237_38;
const VANDG_HPISQ: f64 = 4.934_802_200_544_679_309_34;

impl Kernel for Vandg {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut p2 = (phi / FRAC_PI_2).abs();
        if p2 - VANDG_TOL > 1.0 {
            return Err(TransformError::OutsideDomain);
        }
        let mut sign = 1.0;
        if self.over && lam.abs() > PI {
            sign = -1.0;
        }
        if p2 > 1.0 {
            p2 = 1.0;
        }
        if phi.abs() <= VANDG_TOL {
            return Ok((lam, 0.0));
        }
        if lam.abs() <= VANDG_TOL || (p2 - 1.0).abs() < VANDG_TOL {
            let mut y = PI * (0.5 * p2.asin()).tan();
            if phi < 0.0 {
                y = -y;
            }
            return Ok((0.0, y));
        }
        let al = 0.5 * sign * (PI / lam - lam / PI).abs();
        let al2 = al * al;
        let mut g = (1.0 - p2 * p2).sqrt();
        g /= p2 + g - 1.0;
        let g2 = g * g;
        p2 = g * (2.0 / p2 - 1.0);
        p2 *= p2;
        let mut x = g - p2;
        g = p2 + al2;
        x = PI * (al * x + (al2 * x * x - g * (g2 - p2)).sqrt()).abs() / g;
        if lam < 0.0 {
            x = -x;
        }
        let mut y = (x / PI).abs();
        y = 1.0 - y * (y + 2.0 * al);
        if y < -VANDG_TOL {
            return Err(TransformError::OutsideDomain);
        }
        y = if y < 0.0 {
            0.0
        } else {
            y.sqrt() * if phi < 0.0 { -PI } else { PI }
        };
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let x2 = x * x;
        let ay = y.abs();
        if ay < VANDG_TOL {
            let t = x2 * x2 + VANDG_TPISQ * (x2 + VANDG_HPISQ);
            let lam = if x.abs() <= VANDG_TOL {
                0.0
            } else {
                0.5 * (x2 - VANDG_PISQ + t.sqrt()) / x
            };
            return Ok((lam, 0.0));
        }
        let y2 = y * y;
        let r = x2 + y2;
        let r2 = r * r;
        let c1 = -PI * ay * (r + VANDG_PISQ);
        let c3 = r2 + 2.0 * PI * (ay * r + PI * (y2 + PI * (ay + FRAC_PI_2)));
        let c2 = c1 + VANDG_PISQ * (r - 3.0 * y2);
        let c0 = PI * ay;
        let c2 = c2 / c3;
        let al = c1 / c3 - VANDG_THIRD * c2 * c2;
        let m = 2.0 * (-VANDG_THIRD * al).sqrt();
        let mut d = VANDG_C2_27 * c2 * c2 * c2 + (c0 * c0 - VANDG_THIRD * c2 * c1) / c3;
        let al_m = al * m;
        if al_m.abs() < 1e-16 {
            return Err(TransformError::OutsideDomain);
        }
        d = 3.0 * d / al_m;
        let t = d.abs();
        if t - VANDG_TOL > 1.0 {
            return Err(TransformError::OutsideDomain);
        }
        d = if t > 1.0 {
            if d > 0.0 {
                0.0
            } else {
                PI
            }
        } else {
            d.acos()
        };
        if r > VANDG_PISQ {
            d = 2.0 * PI - d;
        }
        let mut phi = PI * (m * (d * VANDG_THIRD + VANDG_PI4_3).cos() - VANDG_THIRD * c2);
        if y < 0.0 {
            phi = -phi;
        }
        let t = r2 + VANDG_TPISQ * (x2 - y2 + VANDG_HPISQ);
        let lam = if x.abs() <= VANDG_TOL {
            0.0
        } else {
            0.5 * (r - VANDG_PISQ + if t <= 0.0 { 0.0 } else { t.sqrt() }) / x
        };
        Ok((lam, phi))
    }
}

struct NatEarth;

impl Kernel for NatEarth {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const A0: f64 = 0.8707;
        const A1: f64 = -0.131_979;
        const A2: f64 = -0.013_791;
        const A3: f64 = 0.003_971;
        const A4: f64 = -0.001_529;
        const B0: f64 = 1.007_226;
        const B1: f64 = 0.015_085;
        const B2: f64 = -0.044_475;
        const B3: f64 = 0.028_874;
        const B4: f64 = -0.005_916;
        let phi2 = phi * phi;
        let phi4 = phi2 * phi2;
        Ok((
            lam * (A0 + phi2 * (A1 + phi2 * (A2 + phi4 * phi2 * (A3 + phi2 * A4)))),
            phi * (B0 + phi2 * (B1 + phi4 * (B2 + B3 * phi2 + B4 * phi4))),
        ))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        const A0: f64 = 0.8707;
        const A1: f64 = -0.131_979;
        const A2: f64 = -0.013_791;
        const A3: f64 = 0.003_971;
        const A4: f64 = -0.001_529;
        const B0: f64 = 1.007_226;
        const B1: f64 = 0.015_085;
        const B2: f64 = -0.044_475;
        const B3: f64 = 0.028_874;
        const B4: f64 = -0.005_916;
        const C1: f64 = 3.0 * B1;
        const C2: f64 = 7.0 * B2;
        const C3: f64 = 9.0 * B3;
        const C4: f64 = 11.0 * B4;
        let max_y = 0.8707 * 0.52 * PI;
        y = y.clamp(-max_y, max_y);
        let mut yc = y;
        let mut guard = 100;
        while guard != 0 {
            let y2 = yc * yc;
            let y4 = y2 * y2;
            let f = yc * (B0 + y2 * (B1 + y4 * (B2 + B3 * y2 + B4 * y4))) - y;
            let slope = B0 + y2 * (C1 + y4 * (C2 + C3 * y2 + C4 * y4));
            let tol = f / slope;
            yc -= tol;
            if tol.abs() < 1e-11 {
                break;
            }
            guard -= 1;
        }
        if guard == 0 {
            return Err(TransformError::OutsideDomain);
        }
        let y2 = yc * yc;
        let lam = x / (A0 + y2 * (A1 + y2 * (A2 + y2 * y2 * y2 * (A3 + y2 * A4))));
        Ok((lam, yc))
    }
}

struct NatEarth2;

impl Kernel for NatEarth2 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const A0: f64 = 0.847_19;
        const A1: f64 = -0.130_63;
        const A2: f64 = -0.045_15;
        const A3: f64 = 0.054_94;
        const A4: f64 = -0.023_26;
        const A5: f64 = 0.003_31;
        const B0: f64 = 1.011_83;
        const B1: f64 = -0.026_25;
        const B2: f64 = 0.019_26;
        const B3: f64 = -0.003_96;
        let phi2 = phi * phi;
        let phi4 = phi2 * phi2;
        let phi6 = phi2 * phi4;
        Ok((
            lam * (A0 + A1 * phi2 + phi6 * phi6 * (A2 + A3 * phi2 + A4 * phi4 + A5 * phi6)),
            phi * (B0 + phi4 * phi4 * (B1 + B2 * phi2 + B3 * phi4)),
        ))
    }

    fn inverse(&self, x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        const A0: f64 = 0.847_19;
        const A1: f64 = -0.130_63;
        const A2: f64 = -0.045_15;
        const A3: f64 = 0.054_94;
        const A4: f64 = -0.023_26;
        const A5: f64 = 0.003_31;
        const B0: f64 = 1.011_83;
        const B1: f64 = -0.026_25;
        const B2: f64 = 0.019_26;
        const B3: f64 = -0.003_96;
        const C1: f64 = 9.0 * B1;
        const C2: f64 = 11.0 * B2;
        const C3: f64 = 13.0 * B3;
        let max_y = 0.847_19 * 0.535_117_535_153_096 * PI;
        y = y.clamp(-max_y, max_y);
        let mut yc = y;
        let mut guard = 100;
        while guard != 0 {
            let y2 = yc * yc;
            let y4 = y2 * y2;
            let f = yc * (B0 + y4 * y4 * (B1 + B2 * y2 + B3 * y4)) - y;
            let slope = B0 + y4 * y4 * (C1 + C2 * y2 + C3 * y4);
            let tol = f / slope;
            yc -= tol;
            if tol.abs() < 1e-11 {
                break;
            }
            guard -= 1;
        }
        if guard == 0 {
            return Err(TransformError::OutsideDomain);
        }
        let y2 = yc * yc;
        let y4 = y2 * y2;
        let y6 = y2 * y4;
        let lam = x / (A0 + A1 * y2 + y6 * y6 * (A2 + A3 * y2 + A4 * y4 + A5 * y6));
        Ok((lam, yc))
    }
}

struct Poly {
    ml0: f64,
    phi0: f64,
    es: f64,
    one_es: f64,
    meridian: Option<Meridian>,
}

impl Kernel for Poly {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if let Some(meridian) = self.meridian {
            if phi.abs() <= 1e-10 {
                return Ok((lam, -self.ml0));
            }
            let sp = phi.sin();
            let cp = phi.cos();
            let ms = if cp.abs() > 1e-10 {
                msfn(sp, cp, self.es) / sp
            } else {
                0.0
            };
            let lam = lam * sp;
            Ok((
                ms * lam.sin(),
                meridian.forward(phi) - self.ml0 + ms * (1.0 - lam.cos()),
            ))
        } else if phi.abs() <= 1e-10 {
            Ok((lam, self.ml0))
        } else {
            let cot = 1.0 / phi.tan();
            let e = lam * phi.sin();
            Ok((e.sin() * cot, phi - self.phi0 + cot * (1.0 - e.cos())))
        }
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if let Some(meridian) = self.meridian {
            let y = y + self.ml0;
            if y.abs() <= 1e-10 {
                return Ok((x, 0.0));
            }
            let r = y * y + x * x;
            let mut phi = y;
            let mut guard = 20;
            while guard != 0 {
                let sp = phi.sin();
                let cp = phi.cos();
                let s2ph = sp * cp;
                if cp.abs() < 1e-12 {
                    return Err(TransformError::OutsideDomain);
                }
                let mut mlp = (1.0 - self.es * sp * sp).sqrt();
                let c = sp * mlp / cp;
                let ml = meridian.forward(phi);
                let mlb = ml * ml + r;
                mlp = self.one_es / (mlp * mlp * mlp);
                let d_phi = (ml + ml + c * mlb - 2.0 * y * (c * ml + 1.0))
                    / (self.es * s2ph * (mlb - 2.0 * y * ml) / c
                        + 2.0 * (y - ml) * (c * mlp - 1.0 / s2ph)
                        - mlp
                        - mlp);
                phi += d_phi;
                if d_phi.abs() <= 1e-12 {
                    break;
                }
                guard -= 1;
            }
            if guard == 0 {
                return Err(TransformError::OutsideDomain);
            }
            let c = phi.sin();
            let lam = (x * phi.tan() * (1.0 - self.es * c * c).sqrt()).asin() / c;
            Ok((lam, phi))
        } else {
            let y = self.phi0 + y;
            if y.abs() <= 1e-10 {
                return Ok((x, 0.0));
            }
            let b = x * x + y * y;
            let mut phi = y;
            let mut guard = 10;
            loop {
                let tp = phi.tan();
                let dphi = (y * (phi * tp + 1.0) - phi - 0.5 * (phi * phi + b) * tp)
                    / ((phi - y) / tp - 1.0);
                phi -= dphi;
                if dphi.abs() <= 1e-10 {
                    break;
                }
                guard -= 1;
                if guard == 0 {
                    return Err(TransformError::OutsideDomain);
                }
            }
            Ok(((x * phi.tan()).asin() / phi.sin(), phi))
        }
    }
}

struct Vandg23 {
    third: bool,
}

impl Kernel for Vandg23 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let two_d_pi = 2.0 / PI;
        let bt = (two_d_pi * phi).abs();
        let mut ct = 1.0 - bt * bt;
        ct = if ct < 0.0 { 0.0 } else { ct.sqrt() };
        if lam.abs() < 1e-10 {
            let y = PI * if phi < 0.0 { -bt } else { bt } / (1.0 + ct);
            return Ok((0.0, y));
        }
        let at = 0.5 * (PI / lam - lam / PI).abs();
        let (mut x, mut y) = if self.third {
            let x1 = bt / (1.0 + ct);
            (PI * ((at * at + 1.0 - x1 * x1).sqrt() - at), PI * x1)
        } else {
            let x1 = (ct * (1.0 + at * at).sqrt() - at * ct * ct) / (1.0 + at * at * bt * bt);
            (PI * x1, PI * (1.0 - x1 * (x1 + 2.0 * at) + 1e-10).sqrt())
        };
        if lam < 0.0 {
            x = -x;
        }
        if phi < 0.0 {
            y = -y;
        }
        Ok((x, y))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

struct Vandg4;

impl Kernel for Vandg4 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let two_d_pi = 2.0 / PI;
        if phi.abs() < 1e-10 {
            return Ok((lam, 0.0));
        }
        if lam.abs() < 1e-10 || (phi.abs() - FRAC_PI_2).abs() < 1e-10 {
            return Ok((0.0, phi));
        }
        let bt = (two_d_pi * phi).abs();
        let bt2 = bt * bt;
        let ct = 0.5 * (bt * (8.0 - bt * (2.0 + bt2)) - 5.0) / (bt2 * (bt - 1.0));
        let ct2 = ct * ct;
        let mut dt = two_d_pi * lam;
        dt += 1.0 / dt;
        dt = (dt * dt - 4.0).sqrt();
        if lam.abs() - FRAC_PI_2 < 0.0 {
            dt = -dt;
        }
        let dt2 = dt * dt;
        let mut x1 = bt + ct;
        x1 *= x1;
        let t = bt + 3.0 * ct;
        let ft = x1 * (bt2 + ct2 * dt2 - 1.0)
            + (1.0 - bt2) * (bt2 * (t * t + 4.0 * ct2) + ct2 * (12.0 * bt * ct + 4.0 * ct2));
        x1 = (dt * (x1 + ct2 - 1.0) + 2.0 * ft.sqrt()) / (4.0 * x1 + dt2);
        let mut x = FRAC_PI_2 * x1;
        let mut y = FRAC_PI_2 * (1.0 + dt * x1.abs() - x1 * x1).sqrt();
        if lam < 0.0 {
            x = -x;
        }
        if phi < 0.0 {
            y = -y;
        }
        Ok((x, y))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

struct Nicol;

impl Kernel for Nicol {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        const EPS: f64 = 1e-10;
        if lam.abs() < EPS {
            return Ok((0.0, phi));
        }
        if phi.abs() < EPS {
            return Ok((lam, 0.0));
        }
        if (lam.abs() - FRAC_PI_2).abs() < EPS {
            return Ok((lam * phi.cos(), FRAC_PI_2 * phi.sin()));
        }
        if (phi.abs() - FRAC_PI_2).abs() < EPS {
            return Ok((0.0, phi));
        }
        let tb = FRAC_PI_2 / lam - lam / FRAC_PI_2;
        let c = phi / FRAC_PI_2;
        let sp = phi.sin();
        let d = (1.0 - c * c) / (sp - c);
        let mut r2 = tb / d;
        r2 *= r2;
        let m = (tb * sp / d - 0.5 * tb) / (1.0 + r2);
        let n = (sp / r2 + 0.5 * d) / (1.0 + 1.0 / r2);
        let mut x = (m * m + phi.cos().powi(2) / (1.0 + r2)).sqrt();
        x = FRAC_PI_2 * (m + if lam < 0.0 { -x } else { x });
        let mut y = (n * n - (sp * sp / r2 + d * sp - 1.0) / (1.0 + 1.0 / r2)).sqrt();
        y = FRAC_PI_2 * (n + if phi < 0.0 { y } else { -y });
        Ok((x, y))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

struct Urm5 {
    m: f64,
    rmn: f64,
    q3: f64,
    n: f64,
}

impl Kernel for Urm5 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let phi = crate::projmath::aasin(self.n * phi.sin())
            .map_err(|()| TransformError::OutsideDomain)?;
        let t = phi * phi;
        Ok((
            self.m * lam * phi.cos(),
            phi * (1.0 + t * self.q3) * self.rmn,
        ))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

struct Rpoly {
    phi0: f64,
    mode: bool,
    fxa: f64,
    fxb: f64,
}

impl Kernel for Rpoly {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let fa = if self.mode {
            (lam * self.fxb).tan() * self.fxa
        } else {
            0.5 * lam
        };
        if phi.abs() < 1e-9 {
            return Ok((fa + fa, -self.phi0));
        }
        let cot = 1.0 / phi.tan();
        let fa = 2.0 * (fa * phi.sin()).atan();
        Ok((fa.sin() * cot, phi - self.phi0 + (1.0 - fa.cos()) * cot))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "urm5" => &["n", "q", "alpha"],
        "rpoly" => &["lat_ts"],
        "vandg" | "natearth" | "natearth2" | "poly" | "vandg2" | "vandg3" | "vandg4" | "nicol" => {
            &[]
        }
        _ => return None,
    })
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    let built = match name {
        "vandg" => step(frame, Vandg { over: frame.over }),
        "natearth" => step(frame, NatEarth),
        "natearth2" => step(frame, NatEarth2),
        "vandg2" => step(frame, Vandg23 { third: false }),
        "vandg3" => step(frame, Vandg23 { third: true }),
        "vandg4" => step(frame, Vandg4),
        "nicol" => step(frame, Nicol),
        "urm5" => {
            if !params.flag("n") {
                return Err(CreateError::MissingArgument("n"));
            }
            let n = params.number("n")?.unwrap_or(0.0);
            if n <= 0.0 || n > 1.0 {
                return Err(CreateError::IllegalValue("n"));
            }
            let q3 = params.number("q")?.unwrap_or(0.0) / 3.0;
            let alpha = params.radians("alpha")?.unwrap_or(0.0);
            let t = n * alpha.sin();
            let denom = (1.0 - t * t).sqrt();
            if denom == 0.0 {
                return Err(CreateError::IllegalValue("alpha"));
            }
            let m = alpha.cos() / denom;
            step(
                frame,
                Urm5 {
                    m,
                    rmn: 1.0 / (m * n),
                    q3,
                    n,
                },
            )
        }
        "rpoly" => {
            let phi1 = params.radians("lat_ts")?.unwrap_or(0.0).abs();
            let mode = phi1 > 1e-9;
            let (fxa, fxb) = if mode {
                let fxb = 0.5 * phi1.sin();
                (0.5 / fxb, fxb)
            } else {
                (0.0, 0.0)
            };
            step(
                frame,
                Rpoly {
                    phi0: frame.phi0,
                    mode,
                    fxa,
                    fxb,
                },
            )
        }
        "poly" => {
            let es = frame.earth.es();
            if es == 0.0 {
                step(
                    frame,
                    Poly {
                        ml0: -frame.phi0,
                        phi0: frame.phi0,
                        es,
                        one_es: 1.0,
                        meridian: None,
                    },
                )
            } else {
                let meridian = Meridian::new(frame.earth.n());
                step(
                    frame,
                    Poly {
                        ml0: meridian.forward(frame.phi0),
                        phi0: frame.phi0,
                        es,
                        one_es: frame.earth.one_es(),
                        meridian: Some(meridian),
                    },
                )
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(built))
}
