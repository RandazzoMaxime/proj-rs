//! Roussilhe stereographic.
//!
//! Meridian distance is Evenden's `proj_mdist` series, not the rectifying
//! latitude used by the other projections. `k_0` is applied in the kernel.

#![allow(
    clippy::pedantic,
    reason = "the coefficient block and the meridian series follow rouss.cpp and proj_mdist.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};

struct Mdist {
    es: f64,
    big_e: f64,
    coeff: Vec<f64>,
}

struct Rouss {
    s0: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    a4: f64,
    a5: f64,
    a6: f64,
    b1: f64,
    b2: f64,
    b3: f64,
    b4: f64,
    b5: f64,
    b6: f64,
    b7: f64,
    b8: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
    c5: f64,
    c6: f64,
    c7: f64,
    c8: f64,
    d1: f64,
    d2: f64,
    d3: f64,
    d4: f64,
    d5: f64,
    d6: f64,
    d7: f64,
    d8: f64,
    d9: f64,
    d10: f64,
    d11: f64,
    es: f64,
    k0: f64,
    meridian: Mdist,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "rouss" {
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
    if name != "rouss" {
        return Ok(None);
    }
    let es = frame.earth.es();
    let meridian = Mdist::new(es);
    let sin0 = frame.phi0.sin();
    let s0 = meridian.forward(frame.phi0);
    let es2 = es * sin0 * sin0;
    let t = 1.0 - es2;
    let n0 = 1.0 / t.sqrt();
    let r2 = t * t / frame.earth.one_es();
    let r4 = r2 * r2;
    let tan0 = frame.phi0.tan();
    let t2 = tan0 * tan0;
    Ok(Some(classic_step(
        frame,
        Rouss {
            s0,
            a1: r2 / 4.0,
            a2: r2 * (2.0 * t2 - 1.0 - 2.0 * es2) / 12.0,
            a3: r2 * tan0 * (1.0 + 4.0 * t2) / (12.0 * n0),
            a4: r4 / 24.0,
            a5: r4 * (-1.0 + t2 * (11.0 + 12.0 * t2)) / 24.0,
            a6: r4 * (-2.0 + t2 * (11.0 - 2.0 * t2)) / 240.0,
            b1: tan0 / (2.0 * n0),
            b2: r2 / 12.0,
            b3: r2 * (1.0 + 2.0 * t2 - 2.0 * es2) / 4.0,
            b4: r2 * tan0 * (2.0 - t2) / (24.0 * n0),
            b5: r2 * tan0 * (5.0 + 4.0 * t2) / (8.0 * n0),
            b6: r4 * (-2.0 + t2 * (-5.0 + 6.0 * t2)) / 48.0,
            b7: r4 * (5.0 + t2 * (19.0 + 12.0 * t2)) / 24.0,
            b8: r4 / 120.0,
            c1: r2 / 4.0,
            c2: r2 * (2.0 * t2 - 1.0 - 2.0 * es2) / 12.0,
            c3: r2 * tan0 * (1.0 + t2) / (3.0 * n0),
            c4: r4 * (-3.0 + t2 * (34.0 + 22.0 * t2)) / 240.0,
            c5: r4 * (4.0 + t2 * (13.0 + 12.0 * t2)) / 24.0,
            c6: r4 / 16.0,
            c7: r4 * tan0 * (11.0 + t2 * (33.0 + t2 * 16.0)) / (48.0 * n0),
            c8: r4 * tan0 * (1.0 + t2 * 4.0) / (36.0 * n0),
            d1: tan0 / (2.0 * n0),
            d2: r2 / 12.0,
            d3: r2 * (2.0 * t2 + 1.0 - 2.0 * es2) / 4.0,
            d4: r2 * tan0 * (1.0 + t2) / (8.0 * n0),
            d5: r2 * tan0 * (1.0 + t2 * 2.0) / (4.0 * n0),
            d6: r4 * (1.0 + t2 * (6.0 + t2 * 6.0)) / 16.0,
            d7: r4 * t2 * (3.0 + t2 * 4.0) / 8.0,
            d8: r4 / 80.0,
            d9: r4 * tan0 * (-21.0 + t2 * (178.0 - t2 * 26.0)) / 720.0,
            d10: r4 * tan0 * (29.0 + t2 * (86.0 + t2 * 48.0)) / (96.0 * n0),
            d11: r4 * tan0 * (37.0 + t2 * 44.0) / (96.0 * n0),
            es,
            k0: frame.k0,
            meridian,
        },
    )))
}

impl Kernel for Rouss {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let (cp, sp) = (phi.cos(), phi.sin());
        let s = self.meridian.forward(phi) - self.s0;
        let s2 = s * s;
        let al = lam * cp / (1.0 - self.es * sp * sp).sqrt();
        let al2 = al * al;
        let x = self.k0
            * al
            * (1.0 + s2 * (self.a1 + s2 * self.a4)
                - al2 * (self.a2 + s * self.a3 + s2 * self.a5 + al2 * self.a6));
        let y = self.k0
            * (al2 * (self.b1 + al2 * self.b4)
                + s * (1.0
                    + al2 * (self.b3 - al2 * self.b6)
                    + s2 * (self.b2 + s2 * self.b8)
                    + s * al2 * (self.b5 + s * self.b7)));
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let x = x / self.k0;
        let y = y / self.k0;
        let x2 = x * x;
        let y2 = y * y;
        let al = x
            * (1.0 - self.c1 * y2
                + x2 * (self.c2 + self.c3 * y - self.c4 * x2 + self.c5 * y2 - self.c7 * x2 * y)
                + y2 * (self.c6 * y2 - self.c8 * x2 * y));
        let s = self.s0
            + y * (1.0 + y2 * (-self.d2 + self.d8 * y2))
            + x2 * (-self.d1
                + y * (-self.d3 + y * (-self.d5 + y * (-self.d7 + y * self.d11)))
                + x2 * (self.d4 + y * (self.d6 + y * self.d10) - x2 * self.d9));
        let phi = self.meridian.inverse(s)?;
        let sin_phi = phi.sin();
        let lam = al * (1.0 - self.es * sin_phi * sin_phi).sqrt() / phi.cos();
        Ok((lam, phi))
    }
}

impl Mdist {
    fn new(es: f64) -> Self {
        const MAX: usize = 20;
        let mut series = [0.0; MAX];
        series[0] = 1.0;
        let mut ens = es;
        let mut numf = 1.0;
        let mut twon1 = 1.0;
        let mut denfi = 1.0;
        let mut denf = 1.0;
        let mut twon = 4.0;
        let mut accumulated = 1.0;
        let mut previous = 1.0;
        let mut terms = 1usize;
        while terms < MAX {
            numf *= twon1 * twon1;
            let denominator = twon * denf * denf * twon1;
            series[terms] = numf / denominator * ens;
            accumulated -= series[terms];
            ens *= es;
            twon *= 4.0;
            denfi += 1.0;
            denf *= denfi;
            twon1 += 2.0;
            if accumulated == previous {
                break;
            }
            previous = accumulated;
            terms += 1;
        }
        let mut coeff = vec![0.0; terms];
        let mut running = 1.0 - accumulated;
        coeff[0] = running;
        let mut num = 1.0;
        let mut den = 1.0;
        let mut num_step = 2.0;
        let mut den_step = 3.0;
        for index in 1..terms {
            running -= series[index];
            num *= num_step;
            den *= den_step;
            coeff[index] = running * num / den;
            num_step += 2.0;
            den_step += 2.0;
        }
        Self {
            es,
            big_e: accumulated,
            coeff,
        }
    }

    fn forward(&self, phi: f64) -> f64 {
        let sine = phi.sin();
        let cosine = phi.cos();
        let product = sine * cosine;
        let sine2 = sine * sine;
        let direct = phi * self.big_e - self.es * product / (1.0 - self.es * sine2).sqrt();
        let mut index = self.coeff.len() - 1;
        let mut sum = self.coeff[index];
        while index > 0 {
            index -= 1;
            sum = self.coeff[index] + sine2 * sum;
        }
        direct + product * sum
    }

    fn inverse(&self, distance: f64) -> Result<f64, TransformError> {
        let scale = 1.0 / (1.0 - self.es);
        let mut phi = distance;
        for _ in 0..20 {
            let sine = phi.sin();
            let t = 1.0 - self.es * sine * sine;
            let delta = (self.forward(phi) - distance) * (t * t.sqrt()) * scale;
            phi -= delta;
            if delta.abs() < 1e-14 {
                return Ok(phi);
            }
        }
        Err(TransformError::OutsideDomain)
    }
}
