//! Azimuthal equidistant.
//!
//! Sphere, Guam, and the polar ellipsoid follow `aeqd.cpp` directly. Oblique
//! and equatorial ellipsoids are the unit-ellipsoid geodesic: distance and
//! azimuth from `lat_0`, then scaled by the real semi-major axis.

#![allow(clippy::pedantic, reason = "formulae follow PROJ aeqd.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::geodesic::{degrees, radians, Geodesic};
use crate::projmath::{aasin, Meridian};
use std::f64::consts::{FRAC_PI_2, PI};

const EPS10: f64 = 1e-10;
const TOL: f64 = 1e-14;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct Aeqd {
    mode: Mode,
    sinph0: f64,
    cosph0: f64,
    phi0: f64,
    es: f64,
    e: f64,
    mp: f64,
    m1: f64,
    meridian: Option<Meridian>,
    geodesic: Option<Geodesic>,
    guam: bool,
}

fn asin_or(v: f64) -> Result<f64, TransformError> {
    aasin(v).map_err(|()| TransformError::OutsideDomain)
}

impl Kernel for Aeqd {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if let Some(earth) = &self.geodesic {
            return self.forward_geodesic(earth, lam, phi);
        }
        if self.guam {
            let meridian = self.meridian.expect("guam uses the meridian");
            let cosphi = phi.cos();
            let sinphi = phi.sin();
            let t = 1.0 / (1.0 - self.es * sinphi * sinphi).sqrt();
            let x = lam * cosphi * t;
            let y = meridian.forward(phi) - self.m1 + 0.5 * lam * lam * cosphi * sinphi * t;
            return Ok((x, y));
        }
        if self.meridian.is_none() {
            return self.forward_sphere(lam, phi);
        }
        let meridian = self.meridian.expect("polar ellipsoid uses the meridian");
        let mut coslam = lam.cos();
        if self.mode == Mode::North {
            coslam = -coslam;
        }
        let rho = (self.mp - meridian.forward(phi)).abs();
        Ok((rho * lam.sin(), rho * coslam))
    }

    fn inverse(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        if let Some(earth) = &self.geodesic {
            return self.inverse_geodesic(earth, x, y);
        }
        if self.guam {
            let meridian = self.meridian.expect("guam uses the meridian");
            let x2 = 0.5 * x * x;
            let mut phi = self.phi0;
            let mut t = 0.0;
            for _ in 0..3 {
                t = self.e * phi.sin();
                t = (1.0 - t * t).sqrt();
                phi = meridian.inverse(self.m1 + y - x2 * phi.tan() * t);
            }
            return Ok((x * t / phi.cos(), phi));
        }
        if self.meridian.is_none() {
            return self.inverse_sphere(x, y);
        }
        let meridian = self.meridian.expect("polar ellipsoid uses the meridian");
        let distance = x.hypot(y);
        if distance < EPS10 {
            return Ok((0.0, self.phi0));
        }
        let along = if self.mode == Mode::North {
            self.mp - distance
        } else {
            self.mp + distance
        };
        let phi = meridian.inverse(along);
        let lam = x.atan2(if self.mode == Mode::North { -y } else { y });
        Ok((lam, phi))
    }
}

impl Aeqd {
    fn forward_sphere(&self, lam: f64, mut phi: f64) -> Result<(f64, f64), TransformError> {
        if matches!(self.mode, Mode::Equator | Mode::Oblique) {
            let cosphi = phi.cos();
            let sinphi = phi.sin();
            let coslam = lam.cos();
            let sinlam = lam.sin();
            let cosphi_coslam = cosphi * coslam;
            let cosc = self.sinph0 * sinphi + self.cosph0 * cosphi_coslam;
            let t = self.cosph0 * sinphi - self.sinph0 * cosphi_coslam;
            let sinc = (cosphi * sinlam).hypot(t);
            if cosc < TOL - 1.0 {
                return Err(TransformError::OutsideDomain);
            }
            let k = if sinc == 0.0 {
                1.0
            } else {
                sinc.atan2(cosc) / sinc
            };
            return Ok((k * cosphi * sinlam, k * t));
        }
        let mut coslam = lam.cos();
        let sinlam = lam.sin();
        if self.mode == Mode::North {
            phi = -phi;
            coslam = -coslam;
        }
        if (phi - FRAC_PI_2).abs() < EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        let y = FRAC_PI_2 + phi;
        Ok((y * sinlam, y * coslam))
    }

    fn inverse_sphere(&self, x: f64, y: f64) -> Result<(f64, f64), TransformError> {
        let mut distance = x.hypot(y);
        if distance > PI {
            if distance - EPS10 > PI {
                return Err(TransformError::OutsideDomain);
            }
            distance = PI;
        } else if distance < EPS10 {
            return Ok((0.0, self.phi0));
        }
        if matches!(self.mode, Mode::Oblique | Mode::Equator) {
            let sinc = distance.sin();
            let cosc = distance.cos();
            let (phi, x_at, y_at) = if self.mode == Mode::Equator {
                let phi = asin_or(y * sinc / distance)?;
                (phi, x * sinc, cosc * distance)
            } else {
                let phi = asin_or(cosc * self.sinph0 + y * sinc * self.cosph0 / distance)?;
                let y_at = (cosc - self.sinph0 * phi.sin()) * distance;
                (phi, x * sinc * self.cosph0, y_at)
            };
            let lam = if y_at == 0.0 { 0.0 } else { x_at.atan2(y_at) };
            return Ok((lam, phi));
        }
        if self.mode == Mode::North {
            Ok((x.atan2(-y), FRAC_PI_2 - distance))
        } else {
            Ok((x.atan2(y), distance - FRAC_PI_2))
        }
    }

    fn forward_geodesic(
        &self,
        earth: &Geodesic,
        lam: f64,
        phi: f64,
    ) -> Result<(f64, f64), TransformError> {
        if lam.abs() < EPS10 && (phi - self.phi0).abs() < EPS10 {
            return Ok((0.0, 0.0));
        }
        let (distance, azimuth) =
            earth.inverse(degrees(self.phi0), 0.0, degrees(phi), degrees(lam));
        let azimuth = radians(azimuth);
        Ok((distance * azimuth.sin(), distance * azimuth.cos()))
    }

    fn inverse_geodesic(
        &self,
        earth: &Geodesic,
        x: f64,
        y: f64,
    ) -> Result<(f64, f64), TransformError> {
        let distance = x.hypot(y);
        if distance < EPS10 {
            return Ok((0.0, self.phi0));
        }
        let azimuth = degrees(x.atan2(y));
        let (lat, lon) = earth.direct(degrees(self.phi0), 0.0, azimuth, distance);
        Ok((radians(lon), radians(lat)))
    }
}

const KEYS: &[&str] = &["guam"];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "aeqd" {
        Some(KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "aeqd" {
        return Ok(None);
    }
    let (mode, sinph0, cosph0) = if (frame.phi0.abs() - FRAC_PI_2).abs() < EPS10 {
        if frame.phi0 < 0.0 {
            (Mode::South, -1.0, 0.0)
        } else {
            (Mode::North, 1.0, 0.0)
        }
    } else if frame.phi0.abs() < EPS10 {
        (Mode::Equator, 0.0, 1.0)
    } else {
        (Mode::Oblique, frame.phi0.sin(), frame.phi0.cos())
    };
    let guam = params.flag("guam");
    let geodesic =
        if frame.earth.es() != 0.0 && !guam && matches!(mode, Mode::Equator | Mode::Oblique) {
            Some(Geodesic::unit(frame.earth.flattening()))
        } else {
            None
        };
    let meridian = if frame.earth.es() == 0.0 {
        None
    } else {
        Some(Meridian::new(frame.earth.n()))
    };
    let m1 = meridian.map_or(0.0, |m| m.forward(frame.phi0));
    let mp = match (meridian, mode) {
        (Some(m), Mode::North) => m.forward(FRAC_PI_2),
        (Some(m), Mode::South) => m.forward(-FRAC_PI_2),
        _ => 0.0,
    };
    Ok(Some(classic_step(
        frame,
        Aeqd {
            mode,
            sinph0,
            cosph0,
            phi0: frame.phi0,
            es: frame.earth.es(),
            e: frame.earth.e(),
            mp,
            m1,
            meridian,
            geodesic,
            guam,
        },
    )))
}
