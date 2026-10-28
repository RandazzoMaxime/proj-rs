//! Equal Earth and Bertin 1953.
//!
//! Ports of PROJ 9.9 `src/projections/eqearth.cpp` and
//! `src/projections/bertin1953.cpp`. Kernels return units of the semi-major
//! axis. Neither forward reads `P->k0`.

#![allow(
    clippy::pedantic,
    reason = "coefficients, iteration counts, and branches are copied from PROJ"
)]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::projmath::{authalic_inverse, authalic_q};
use crate::Ellipsoid;
use std::f64::consts::PI;

/// `A1`..`A4` in `eqearth.cpp`.
const A1: f64 = 1.340264;
const A2: f64 = -0.081106;
const A3: f64 = 0.000893;
const A4: f64 = 0.003796;
/// Unit-sphere ordinate of 90° latitude, the `MAX_Y` literal, not a recomputed forward.
const MAX_Y: f64 = 1.3173627591574;
const EPS: f64 = 1e-11;
const MAX_ITER: u32 = 12;

/// `PJ_TORAD(-16.5)`, added inside the Bertin forward.
const DELTA_LAMBDA: f64 = -16.5 * PI / 180.0;
/// `PJ_TORAD(-42.)`. Setup writes this into `P->phi0` for the rotation only.
const DELTA_PHI: f64 = -42.0 * PI / 180.0;
/// `M_TWOPI` in `proj_internal.h`.
const M_TWOPI: f64 = 6.283_185_307_179_586_476_93;

/// `M` in `eqearth.cpp`: `(sqrt(3.0) / 2.0)`.
fn equal_earth_m() -> f64 {
    3.0_f64.sqrt() / 2.0
}

#[derive(Clone, Copy)]
enum Authalic {
    /// `P->es == 0`. Latitude stays geographic.
    Sphere,
    /// `P->es != 0`. `qp` is `pj_authalic_lat_q(1, P)`.
    Ellipsoid {
        qp: f64,
        n: f64,
        e: f64,
        es: f64,
        one_es: f64,
    },
}

struct EqEarth {
    rqda: f64,
    authalic: Authalic,
}

struct Bertin1953 {
    cos_delta_phi: f64,
    sin_delta_phi: f64,
    cos_delta_gamma: f64,
    sin_delta_gamma: f64,
}

impl Kernel for EqEarth {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let mut sbeta = phi.sin();
        if let Authalic::Ellipsoid { qp, e, one_es, .. } = self.authalic {
            // `pj_authalic_lat_q`, not the |n| < 0.01 series in `pj_authalic_lat`.
            sbeta = authalic_q(sbeta, e, one_es) / qp;
            if sbeta.abs() > 1.0 {
                sbeta = if sbeta > 0.0 { 1.0 } else { -1.0 };
            }
        }
        let m = equal_earth_m();
        let psi = (m * sbeta).asin();
        let psi2 = psi * psi;
        let psi6 = psi2 * psi2 * psi2;
        let x =
            lam * psi.cos() / (m * (A1 + 3.0 * A2 * psi2 + psi6 * (7.0 * A3 + 9.0 * A4 * psi2)));
        let y = psi * (A1 + A2 * psi2 + psi6 * (A3 + A4 * psi2));
        Ok((x * self.rqda, y * self.rqda))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        x /= self.rqda;
        y /= self.rqda;
        // `|y| > MAX_Y` is the pole. Newton still runs so longitude follows
        // the C ordinate, but `asin(sin(yc) / M)` is a few ulps under 1 and
        // the inverse sine stretches that into about 0.7 m. PROJ's gie reads
        // the one-word `tolerance 1cm` as 1 m. This crate's runner reads it
        // as 1 cm, and the expect is the pole, which is what `MAX_Y` means.
        let pole = if y > MAX_Y {
            y = MAX_Y;
            true
        } else if y < -MAX_Y {
            y = -MAX_Y;
            true
        } else {
            false
        };
        let mut yc = y;
        let mut converged = false;
        for _ in 0..MAX_ITER {
            let y2 = yc * yc;
            let y6 = y2 * y2 * y2;
            let f = yc * (A1 + A2 * y2 + y6 * (A3 + A4 * y2)) - y;
            let fder = A1 + 3.0 * A2 * y2 + y6 * (7.0 * A3 + 9.0 * A4 * y2);
            let tol = f / fder;
            yc -= tol;
            if tol.abs() < EPS {
                converged = true;
                break;
            }
        }
        // `i == 0` sets `PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN`.
        // `pj_inv` then drops the `{0, 0}` the C function returned.
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        let m = equal_earth_m();
        let y2 = yc * yc;
        let y6 = y2 * y2 * y2;
        let lam = m * x * (A1 + 3.0 * A2 * y2 + y6 * (7.0 * A3 + 9.0 * A4 * y2)) / yc.cos();
        let mut phi = if pole {
            if y < 0.0 {
                -PI / 2.0
            } else {
                PI / 2.0
            }
        } else {
            (yc.sin() / m).asin()
        };
        if let Authalic::Ellipsoid {
            qp,
            n,
            e,
            es,
            one_es,
        } = self.authalic
        {
            phi = authalic_inverse(phi, n, e, es, one_es, qp);
        }
        Ok((lam, phi))
    }
}

impl Kernel for Bertin1953 {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let fu = 1.4;
        let k = 12.0;
        let w = 1.68;
        let mut lam = lam + DELTA_LAMBDA;
        let mut cosphi = phi.cos();
        let x = lam.cos() * cosphi;
        let y = lam.sin() * cosphi;
        let z = phi.sin();
        let mut z0 = z * self.cos_delta_phi + x * self.sin_delta_phi;
        lam = (y * self.cos_delta_gamma - z0 * self.sin_delta_gamma)
            .atan2(x * self.cos_delta_phi - z * self.sin_delta_phi);
        z0 = z0 * self.cos_delta_gamma + y * self.sin_delta_gamma;
        let mut phi = z0.asin();
        lam = adjlon_proj(lam);
        if lam + phi < -fu {
            let d = (lam - phi + 1.6) * (lam + phi + fu) / 8.0;
            lam += d;
            // `sin(phi + M_PI / 2)`, not `cos(phi)`.
            phi -= 0.8 * d * (phi + PI / 2.0).sin();
        }
        cosphi = phi.cos();
        let mut d = (2.0 / (1.0 + cosphi * (lam / 2.0).cos())).sqrt();
        let mut xy_x = w * d * cosphi * (lam / 2.0).sin();
        let mut xy_y = d * phi.sin();
        d = (1.0 - (lam * phi).cos()) / k;
        if xy_y < 0.0 {
            xy_x *= 1.0 + d;
        }
        if xy_y > 0.0 {
            xy_y *= 1.0 + d / 1.5 * xy_x * xy_x;
        }
        Ok((xy_x, xy_y))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        // `P->inv` is left unset.
        Err(TransformError::NoInverse)
    }
}

/// PROJ `adjlon`: longitudes inside `π + 1e-12` are not wrapped.
///
/// `crate::angle::adjlon` stops at `≤ π`, so the call inside Bertin does not use it.
fn adjlon_proj(mut longitude: f64) -> f64 {
    if longitude.abs() < PI + 1e-12 {
        return longitude;
    }
    longitude += PI;
    longitude -= M_TWOPI * (longitude / M_TWOPI).floor();
    longitude -= PI;
    longitude
}

fn eqearth_kernel(frame: &Frame) -> EqEarth {
    let earth = frame.earth;
    if earth.es() == 0.0 {
        return EqEarth {
            rqda: 1.0,
            authalic: Authalic::Sphere,
        };
    }
    let qp = authalic_q(1.0, earth.e(), earth.one_es());
    EqEarth {
        rqda: (0.5 * qp).sqrt(),
        authalic: Authalic::Ellipsoid {
            qp,
            n: earth.n(),
            e: earth.e(),
            es: earth.es(),
            one_es: earth.one_es(),
        },
    }
}

fn bertin_step(frame: &Frame) -> Result<Box<dyn crate::engine::Step>, CreateError> {
    let mut placed = frame.clone();
    // Setup overwrites both origins after the parameters are read.
    placed.lam0 = 0.0;
    placed.phi0 = DELTA_PHI;
    placed.earth = Ellipsoid::try_from_a_es(frame.earth.a(), 0.0)
        .map_err(|_| CreateError::IllegalValue("a"))?;
    Ok(classic_step(
        &placed,
        Bertin1953 {
            cos_delta_phi: DELTA_PHI.cos(),
            sin_delta_phi: DELTA_PHI.sin(),
            cos_delta_gamma: 1.0,
            sin_delta_gamma: 0.0,
        },
    ))
}

const NO_KEYS: &[&str] = &[];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "eqearth" || name == "bertin1953" {
        Some(NO_KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    match name {
        "eqearth" => Ok(Some(classic_step(frame, eqearth_kernel(frame)))),
        "bertin1953" => Ok(Some(bertin_step(frame)?)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{lpz_dist, xyz_dist, Coord, Operation};

    fn deg(lon: f64, lat: f64) -> Coord {
        Coord::xy(lon.to_radians(), lat.to_radians())
    }

    fn assert_proj(definition: &str, lon: f64, lat: f64, x: f64, y: f64, tol: f64) {
        let op = Operation::create(definition).unwrap_or_else(|error| {
            panic!("create {definition}: {error}");
        });
        let got = op.forward(deg(lon, lat)).unwrap_or_else(|error| {
            panic!("forward {definition} {lon} {lat}: {error}");
        });
        let distance = xyz_dist(got, Coord::xy(x, y));
        assert!(
            distance <= tol,
            "{definition} ({lon}, {lat}) got {:.12} {:.12} want {x} {y} dist {distance}",
            got.x,
            got.y
        );
    }

    fn assert_inv(definition: &str, x: f64, y: f64, lon: f64, lat: f64, tol: f64) {
        let op = Operation::create(definition).unwrap_or_else(|error| {
            panic!("create {definition}: {error}");
        });
        let got = op.inverse(Coord::xy(x, y)).unwrap_or_else(|error| {
            panic!("inverse {definition} {x} {y}: {error}");
        });
        let (a, es) = op.distance_ellipsoid();
        let distance = lpz_dist(a, es, deg(lon, lat), got);
        assert!(
            distance <= tol,
            "{definition} inverse ({x}, {y}) got {:.8}° {:.8}° want {lon} {lat} dist {distance} m",
            got.x.to_degrees(),
            got.y.to_degrees()
        );
    }

    #[test]
    fn bertin_gie_block() {
        let definition = "+proj=bertin1953 +R=1";
        let cases = [
            (0.0, 0.0, -0.260_206_554_508, -0.685_226_058_142),
            (16.5, 42.0, 0.0, 0.0),
            (-180.0, 90.0, 0.0, 0.813_473_286_152),
            (0.0, 90.0, 0.0, 0.813_473_286_152),
            (10.0, -35.0, -0.138_495_501_548, -1.221_408_328_101),
            (-70.0, -35.0, -1.504_967_424_950, -0.522_846_035_499),
            (80.0, 7.0, 0.929_377_425_352, -0.215_443_296_201),
            (128.0, 35.0, 0.920_230_566_844, 0.713_170_409_026),
            (170.0, -41.0, 2.162_845_830_414, -0.046_534_568_425),
        ];
        for (lon, lat, x, y) in cases {
            assert_proj(definition, lon, lat, x, y, 5e-4);
        }
    }

    #[test]
    fn bertin_ignores_origin_scale_and_flattening() {
        let base = Operation::create("+proj=bertin1953 +R=6378137").expect("sphere");
        let shifted =
            Operation::create("+proj=bertin1953 +ellps=WGS84 +lon_0=20 +lat_0=15 +k_0=0.5")
                .expect("shifted");
        let input = deg(12.0, -40.0);
        let left = base.forward(input).expect("base");
        let right = shifted.forward(input).expect("shifted");
        let distance = xyz_dist(left, right);
        assert!(distance <= 1e-6, "{distance} {left:?} {right:?}");
        let error = shifted.inverse(left).expect_err("no inverse");
        assert_eq!(error, TransformError::NoInverse);
    }

    #[test]
    fn bertin_internal_adjlon_keeps_the_overshoot() {
        let kept = adjlon_proj(PI + 1e-13);
        assert_eq!(kept, PI + 1e-13);
        let wrapped = crate::angle::adjlon(PI + 0.5);
        assert!(wrapped < 0.0, "{wrapped}");
    }

    #[test]
    fn eqearth_gie_wgs84() {
        let definition = "+proj=eqearth +ellps=WGS84";
        let cases = [
            (0.0, 0.0, 0.0, 0.0),
            (-180.0, 90.0, -10_216_474.79, 8_392_927.6),
            (0.0, 90.0, 0.0, 8_392_927.6),
            (180.0, 90.0, 10_216_474.79, 8_392_927.6),
            (180.0, 45.0, 14_792_474.75, 5_466_867.76),
            (180.0, 0.0, 17_243_959.06, 0.0),
            (-70.0, -31.2, -6_241_081.64, -3_907_019.16),
        ];
        for (lon, lat, x, y) in cases {
            assert_proj(definition, lon, lat, x, y, 0.01);
            assert_inv(definition, x, y, lon, lat, 0.01);
        }
    }

    #[test]
    fn eqearth_gie_sphere_radius() {
        let definition = "+proj=eqearth +R=6378137";
        let cases = [
            (0.0, 0.0, 0.0, 0.0),
            (-180.0, 90.0, -10_227_908.09, 8_402_320.16),
            (0.0, 90.0, 0.0, 8_402_320.16),
            (180.0, 90.0, 10_227_908.09, 8_402_320.16),
            (180.0, 45.0, 14_795_421.79, 5_486_671.72),
            (180.0, 0.0, 17_263_256.84, 0.0),
            (-70.0, -31.2, -6_244_707.88, -3_924_893.29),
        ];
        for (lon, lat, x, y) in cases {
            assert_proj(definition, lon, lat, x, y, 0.01);
            assert_inv(definition, x, y, lon, lat, 0.01);
        }
    }

    #[test]
    fn eqearth_gie_unit_sphere_clamps_y() {
        let definition = "+proj=eqearth +R=1";
        assert_inv(definition, 0.0, -1.3, 0.0, -82.318, 5e-4);
        assert_inv(definition, 0.0, -MAX_Y, 0.0, -90.0, 5e-4);
        assert_inv(definition, 0.0, -1.4, 0.0, -90.0, 5e-4);
    }

    #[test]
    fn eqearth_k0_is_not_applied() {
        let plain = Operation::create("+proj=eqearth +R=1").expect("plain");
        let scaled = Operation::create("+proj=eqearth +R=1 +k=2").expect("scaled");
        let input = deg(40.0, 25.0);
        let left = plain.forward(input).expect("plain forward");
        let right = scaled.forward(input).expect("scaled forward");
        assert!(xyz_dist(left, right) <= 1e-12, "{left:?} {right:?}");
    }

    #[test]
    fn unknown_parameter_is_rejected() {
        let error = Operation::create("+proj=eqearth +lat_1=10").expect_err("lat_1");
        assert!(matches!(error, CreateError::UnknownParameter(_)));
        let error = Operation::create("+proj=bertin1953 +W=1").expect_err("W");
        assert!(matches!(error, CreateError::UnknownParameter(_)));
    }
}
