//! CalCOFI line/station numbers.
//!
//! PROJ forces `a = ra = 1`, `lam0 = x0 = y0 = 0` and `+over` after the
//! ellipsoid is chosen, so the classic scaling never touches the result and
//! `lon_0` / false offsets are ignored. The numbers are already the output.

#![allow(
    clippy::pedantic,
    reason = "the line and station reduction follows calcofi.cpp"
)]

use crate::engine::{Coord, CreateError, Frame, IoKind, Params, Step, TransformError};
use crate::projmath::{phi2, tsfn};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const EPS10: f64 = 1e-10;
const EPS_LAT: f64 = 1e-12;
const RAD_TO_DEG: f64 = 57.295_779_513_082_321;
const LINE_TO_RAD: f64 = 0.003_490_658_503_988_659_2;
const STATION_TO_RAD: f64 = 0.001_163_552_834_662_886_3;
const PT_O_LINE: f64 = 80.0;
const PT_O_STATION: f64 = 60.0;
const PT_O_LAMBDA: f64 = -2.114_466_388_791_130_1;
const PT_O_PHI: f64 = 0.596_029_939_556_063_54;
const ROTATION: f64 = 0.523_598_775_598_298_82;

struct Calcofi {
    frame: Frame,
}

fn mercator(phi: f64, e: f64, sphere: bool) -> f64 {
    if sphere {
        (FRAC_PI_4 + 0.5 * phi).tan().ln()
    } else {
        -tsfn(phi, phi.sin(), e).ln()
    }
}

impl Step for Calcofi {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        if !coord.x.is_finite() || !coord.y.is_finite() {
            return Err(TransformError::NonFinite);
        }
        let mut phi = coord.y;
        if phi.abs() > FRAC_PI_2 {
            if phi > 0.0 {
                if phi - FRAC_PI_2 > EPS_LAT {
                    return Err(TransformError::InvalidCoord);
                }
                phi = FRAC_PI_2;
            } else if phi - FRAC_PI_2 < -EPS_LAT {
                return Err(TransformError::InvalidCoord);
            } else {
                phi = -FRAC_PI_2;
            }
        }
        let earth = self.frame.earth;
        if self.frame.geoc {
            let limit = FRAC_PI_2 - 1e-9;
            if phi.abs() <= limit && earth.one_es() != 1.0 {
                phi = (phi.tan() / earth.one_es()).atan();
            }
        }
        let lam = coord.x - self.frame.from_greenwich;
        if (phi.abs() - FRAC_PI_2).abs() <= EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        let sphere = earth.es() == 0.0;
        let e = earth.e();
        let y = mercator(phi, e, sphere);
        let oy = mercator(PT_O_PHI, e, sphere);
        let l1 = (y - oy) * ROTATION.tan();
        let l2 = -lam - l1 + PT_O_LAMBDA;
        let mut ry = l2 * ROTATION.cos() * ROTATION.sin() + y;
        ry = if sphere {
            FRAC_PI_2 - 2.0 * (-ry).exp().atan()
        } else {
            phi2((-ry).exp(), e).map_err(|()| TransformError::OutsideDomain)?
        };
        let line = PT_O_LINE - RAD_TO_DEG * (ry - PT_O_PHI) * 5.0 / ROTATION.cos();
        let station = PT_O_STATION + RAD_TO_DEG * (ry - phi) * 15.0 / ROTATION.sin();
        Ok(Coord {
            x: line,
            y: station,
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let earth = self.frame.earth;
        let sphere = earth.es() == 0.0;
        let e = earth.e();
        let ry = PT_O_PHI - LINE_TO_RAD * (coord.x - PT_O_LINE) * ROTATION.cos();
        let phi = ry - STATION_TO_RAD * (coord.y - PT_O_STATION) * ROTATION.sin();
        let oy = mercator(PT_O_PHI, e, sphere);
        let rym = mercator(ry, e, sphere);
        let xym = mercator(phi, e, sphere);
        let l1 = (xym - oy) * ROTATION.tan();
        let l2 = (rym - xym) / (ROTATION.cos() * ROTATION.sin());
        Ok(Coord {
            x: PT_O_LAMBDA - (l1 + l2) + self.frame.from_greenwich,
            y: phi,
            z: self.frame.vto_meter * coord.z - self.frame.z0,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Projected
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (1.0, self.frame.earth.es())
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "calcofi" {
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
    if name == "calcofi" {
        Ok(Some(Box::new(Calcofi {
            frame: frame.clone(),
        })))
    } else {
        Ok(None)
    }
}
