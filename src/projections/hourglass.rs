//! Hourglass equal-area projection.
//!
//! Port of PROJ 9.9 `src/projections/hourglass.cpp`. Coordinates stay in units
//! of the semi-major axis. `k0` is not read.

#![allow(
    clippy::pedantic,
    reason = "the powers follow the left-associative expressions in hourglass.cpp"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_forward, authalic_inverse, authalic_q};
use std::f64::consts::{FRAC_PI_2, PI};

/// `|y|` at or below this, in units of the semi-major axis, is the origin.
/// PROJ special-cases that singularity before the inverse powers.
const ORIGIN_Y: f64 = 1e-8;
const LAM_SLOP: f64 = 1e-7;

struct Hourglass {
    /// Shape parameter `+n`. Not the third flattening.
    n: f64,
    np1: f64,
    sqrts: f64,
    l: f64,
    l_nm1_n: f64,
    l_1mn: f64,
    qp: f64,
    /// `0` disables the inverse longitude gate. The default in PROJ is `1`.
    outinvalid: i32,
    ellips: bool,
    /// Third flattening passed to the authalic series.
    en: f64,
    e: f64,
    es: f64,
    one_es: f64,
}

impl Kernel for Hourglass {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let beta = if self.ellips {
            authalic_forward(phi, phi.sin(), self.en, self.e, self.one_es, self.qp)
        } else {
            phi
        };
        let sign = if phi >= 0.0 { 1.0 } else { -1.0 };
        let q = self.qp * beta.sin().abs();
        let lead = FRAC_PI_2 * self.np1 / self.n;
        let x = lam / self.sqrts / PI * (lead * self.l.powf(self.n - 1.0) * q).powf(1.0 / self.np1);
        let y = sign
            * self.sqrts
            * (lead * self.l.powf((1.0 - self.n) / self.n) * q).powf(self.n / self.np1);
        Ok((x, y))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if y.abs() <= ORIGIN_Y {
            return Ok((0.0, 0.0));
        }
        let sign = if y >= 0.0 { 1.0 } else { -1.0 };
        let t = (y.abs() / self.sqrts).powf(self.np1 / self.n);
        let sinbeta = t * self.n / self.np1 / PI * 2.0 / self.qp * self.l_nm1_n;
        if sinbeta > 1.0 {
            return Err(TransformError::OutsideDomain);
        }
        let u = self.n / self.np1 / PI * 2.0 / self.qp * self.l_1mn / sinbeta;
        let lam = x * PI * self.sqrts * u.powf(1.0 / self.np1);
        let beta = sinbeta.asin();
        let mut phi = if self.ellips {
            authalic_inverse(beta, self.en, self.e, self.es, self.one_es, self.qp)
        } else {
            beta
        };
        phi *= sign;
        if self.outinvalid != 0 {
            let limit = (f64::from(self.outinvalid) * 2.0 - 1.0) * PI + LAM_SLOP;
            if lam.abs() > limit {
                return Err(TransformError::OutsideDomain);
            }
        }
        Ok((lam, phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "hourglass" {
        Some(&["n", "s", "outinvalid"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "hourglass" {
        return Ok(None);
    }
    let n = positive_param(params, "n")?;
    let s = positive_param(params, "s")?;
    let outinvalid = if params.flag("outinvalid") {
        params.integer("outinvalid")?.unwrap_or(0)
    } else {
        1
    };
    let np1 = n + 1.0;
    let earth = frame.earth;
    let e = earth.e();
    let (l, qp, ellips) = if e != 0.0 {
        let factor = 1.0 + earth.one_es() / e * e.atanh();
        let l = (FRAC_PI_2 * np1 / n * factor).sqrt();
        (l, authalic_q(1.0, e, earth.one_es()), true)
    } else {
        ((PI * np1 / n).sqrt(), 2.0, false)
    };
    Ok(Some(classic_step(
        frame,
        Hourglass {
            n,
            np1,
            sqrts: s.sqrt(),
            l,
            l_nm1_n: l.powf((n - 1.0) / n),
            l_1mn: l.powf(1.0 - n),
            qp,
            outinvalid,
            ellips,
            en: earth.n(),
            e,
            es: earth.es(),
            one_es: earth.one_es(),
        },
    )))
}

/// `+n` and `+s` default to 1. PROJ rejects a present value that is not `> 0`.
fn positive_param(params: &Params, key: &'static str) -> Result<f64, CreateError> {
    let value = if params.flag(key) {
        params.number(key)?.unwrap_or(0.0)
    } else {
        1.0
    };
    if value <= 0.0 {
        Err(CreateError::IllegalValue(key))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::CreateError;
    use crate::engine::{lpz_dist, Coord, Operation};

    fn near(got: f64, expect: f64) {
        assert!(
            (got - expect).abs() <= 1e-4,
            "{got} differs from {expect} by {}",
            (got - expect).abs()
        );
    }

    #[test]
    fn gie_defaults_and_scaled_shape() {
        let op = Operation::create("+proj=hourglass").expect("hourglass");
        let points: [(f64, f64, f64, f64); 4] = [
            (20.0, 63.0, 1_674_151.870_33, 15_067_366.832_97),
            (-20.0, 63.0, -1_674_151.870_33, 15_067_366.832_97),
            (20.0, -63.0, 1_674_151.870_33, -15_067_366.832_97),
            (-20.0, -63.0, -1_674_151.870_33, -15_067_366.832_97),
        ];
        for (lon, lat, east, north) in points {
            let fwd = op
                .forward(Coord::xy(lon.to_radians(), lat.to_radians()))
                .expect("forward");
            near(fwd.x, east);
            near(fwd.y, north);
            let back = op.inverse(fwd).expect("inverse");
            let (a, es) = op.distance_ellipsoid();
            let miss = lpz_dist(a, es, Coord::xy(lon.to_radians(), lat.to_radians()), back);
            assert!(miss <= 1e-4, "roundtrip {miss} m at {lon},{lat}");
        }
        let origin = op.forward(Coord::xy(0.0, 0.0)).expect("origin");
        near(origin.x, 0.0);
        near(origin.y, 0.0);
        let back = op.inverse(Coord::xy(0.0, 0.0)).expect("inverse origin");
        near(back.x, 0.0);
        near(back.y, 0.0);

        let shaped = Operation::create("+proj=hourglass +s=0.5 +n=2").expect("shaped");
        let fwd = shaped
            .forward(Coord::xy(20.0_f64.to_radians(), 63.0_f64.to_radians()))
            .expect("shaped forward");
        near(fwd.x, 2_090_550.698_57);
        near(fwd.y, 9_049_670.637_60);
    }

    #[test]
    fn scale_factor_is_ignored_and_non_positive_shape_is_rejected() {
        let plain = Operation::create("+proj=hourglass").expect("plain");
        let scaled = Operation::create("+proj=hourglass +k_0=2").expect("scaled");
        let point = Coord::xy(20.0_f64.to_radians(), 63.0_f64.to_radians());
        let left = plain.forward(point).expect("plain forward");
        let right = scaled.forward(point).expect("scaled forward");
        near(left.x, right.x);
        near(left.y, right.y);
        let bad = Operation::create("+proj=hourglass +s=0").expect_err("s");
        assert!(matches!(bad, CreateError::IllegalValue("s")));
        let bad_n = Operation::create("+proj=hourglass +n=-1").expect_err("n");
        assert!(matches!(bad_n, CreateError::IllegalValue("n")));
    }
}
