//! New Zealand Map Grid.
//!
//! PROJ replaces the ellipsoid with the International 1924 major axis,
//! forces the origin to 173° E, 41° S, and applies false offsets of
//! 2 510 000 m and 6 023 150 m. The series output is multiplied by that
//! fixed axis, not by the ellipsoid the caller named.

#![allow(
    clippy::pedantic,
    reason = "the fixed-Earth series and the complex polynomial follow nzmg.cpp"
)]

use crate::angle::adjlon;
use crate::engine::{Coord, CreateError, Frame, IoKind, Params, Step, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS_LAT: f64 = 1e-12;
const EPSLN: f64 = 1e-10;
const SEC5_TO_RAD: f64 = 0.484_813_681_109_535_993_589_914_102_3;
const RAD_TO_SEC5: f64 = 2.062_648_062_470_963_551_564_733_573;
const AXIS: f64 = 6_378_388.0;
const FALSE_EASTING: f64 = 2_510_000.0;
const FALSE_NORTHING: f64 = 6_023_150.0;

#[derive(Clone, Copy)]
struct Complex {
    re: f64,
    im: f64,
}

struct Nzmg {
    frame: Frame,
    lam0: f64,
    phi0: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "nzmg" {
        Some(&[])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "nzmg" {
        return Ok(None);
    }
    Ok(Some(Box::new(Nzmg {
        frame: frame.clone(),
        lam0: 173.0_f64.to_radians(),
        phi0: -41.0_f64.to_radians(),
    })))
}

impl Step for Nzmg {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(Coord {
                x: f64::NAN,
                y: f64::NAN,
                z: coord.z,
                t: coord.t,
            });
        }
        let (lam, phi) = geographic(&self.frame, coord.x, coord.y, self.lam0)?;
        let scaled = (phi - self.phi0) * RAD_TO_SEC5;
        let point = zpoly1(
            Complex {
                re: horner(scaled, &TPSI),
                im: lam,
            },
            &BF,
        );
        Ok(Coord {
            x: self.frame.fr_meter * (AXIS * point.im + FALSE_EASTING),
            y: self.frame.fr_meter * (AXIS * point.re + FALSE_NORTHING),
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
        let x = (self.frame.to_meter * coord.x - FALSE_EASTING) / AXIS;
        let y = (self.frame.to_meter * coord.y - FALSE_NORTHING) / AXIS;
        let mut point = Complex { re: y, im: x };
        let mut converged = false;
        for _ in 0..20 {
            let (mut value, derivative) = zpolyd1(point, &BF);
            value.re -= y;
            value.im -= x;
            let den = derivative.re * derivative.re + derivative.im * derivative.im;
            if den == 0.0 {
                return Err(TransformError::OutsideDomain);
            }
            let d_re = -(value.re * derivative.re + value.im * derivative.im) / den;
            let d_im = -(value.im * derivative.re - value.re * derivative.im) / den;
            point.re += d_re;
            point.im += d_im;
            if d_re.abs() + d_im.abs() <= EPSLN {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(TransformError::OutsideDomain);
        }
        let mut lam = point.im + self.frame.from_greenwich + self.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
        let phi = self.phi0 + horner(point.re, &TPHI) * SEC5_TO_RAD;
        Ok(Coord {
            x: lam,
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
        (AXIS, self.frame.earth.es())
    }
}

fn geographic(
    frame: &Frame,
    mut lon: f64,
    mut lat: f64,
    lam0: f64,
) -> Result<(f64, f64), TransformError> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err(TransformError::NonFinite);
    }
    if lat.abs() > FRAC_PI_2 {
        if lat > 0.0 {
            if lat - FRAC_PI_2 > EPS_LAT {
                return Err(TransformError::InvalidCoord);
            }
            lat = FRAC_PI_2;
        } else if lat - FRAC_PI_2 < -EPS_LAT {
            return Err(TransformError::InvalidCoord);
        } else {
            lat = -FRAC_PI_2;
        }
    }
    if lon.abs() > PI && lon.abs() > 10.0 {
        return Err(TransformError::InvalidCoord);
    }
    if frame.geoc {
        let limit = FRAC_PI_2 - 1e-9;
        if lat.abs() <= limit && frame.earth.one_es() != 1.0 {
            lat = (lat.tan() / frame.earth.one_es()).atan();
        }
    }
    lon = (lon - frame.from_greenwich) - lam0;
    if !frame.over {
        lon = adjlon(lon);
    }
    Ok((lon, lat))
}

fn horner(value: f64, coeff: &[f64]) -> f64 {
    let mut acc = coeff[coeff.len() - 1];
    for term in coeff.iter().rev().skip(1) {
        acc = term + value * acc;
    }
    acc * value
}

fn zpoly1(z: Complex, coeff: &[Complex]) -> Complex {
    let mut a = coeff[coeff.len() - 1];
    for term in coeff.iter().rev().skip(1) {
        let t = a.re;
        a.re = term.re + z.re * t - z.im * a.im;
        a.im = term.im + z.re * a.im + z.im * t;
    }
    let t = a.re;
    a.re = z.re * t - z.im * a.im;
    a.im = z.re * a.im + z.im * t;
    a
}

fn zpolyd1(z: Complex, coeff: &[Complex]) -> (Complex, Complex) {
    let mut index = coeff.len() - 1;
    let mut a = coeff[index];
    let mut b = a;
    let mut first = true;
    while index > 0 {
        if first {
            first = false;
        } else {
            let t = b.re;
            b.re = a.re + z.re * t - z.im * b.im;
            b.im = a.im + z.re * b.im + z.im * t;
        }
        index -= 1;
        let t = a.re;
        a.re = coeff[index].re + z.re * t - z.im * a.im;
        a.im = coeff[index].im + z.re * a.im + z.im * t;
    }
    let t = b.re;
    b.re = a.re + z.re * t - z.im * b.im;
    b.im = a.im + z.re * b.im + z.im * t;
    let t = a.re;
    a.re = z.re * t - z.im * a.im;
    a.im = z.re * a.im + z.im * t;
    (a, b)
}

const BF: [Complex; 6] = [
    Complex {
        re: 0.755_785_322_8,
        im: 0.0,
    },
    Complex {
        re: 0.249_204_646,
        im: 0.003_371_507,
    },
    Complex {
        re: -0.001_541_739,
        im: 0.041_058_560,
    },
    Complex {
        re: -0.101_629_07,
        im: 0.017_276_09,
    },
    Complex {
        re: -0.266_234_89,
        im: -0.362_492_18,
    },
    Complex {
        re: -0.687_098_3,
        im: -1.165_196_7,
    },
];

const TPSI: [f64; 10] = [
    0.639_917_507_3,
    -0.135_879_761_3,
    0.063_294_409,
    -0.025_268_53,
    0.011_787_9,
    -0.005_516_1,
    0.002_690_6,
    -0.001_333,
    0.000_67,
    -0.000_34,
];

const TPHI: [f64; 9] = [
    1.562_701_424_3,
    0.518_540_639_8,
    -0.033_330_98,
    -0.105_290_6,
    -0.036_859_4,
    0.007_317,
    0.012_20,
    0.003_94,
    -0.001_3,
];
