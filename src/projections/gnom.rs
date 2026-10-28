//! Gnomonic.
//!
//! The sphere is the closed form in `gnom.cpp`. The ellipsoid is Karney's
//! geodesic: `rho = m / M` on the unit ellipsoid, then the classic wrapper
//! multiplies by the real semi-major axis.

#![allow(clippy::pedantic, reason = "formulae follow PROJ gnom.cpp")]

use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use crate::geodesic::{degrees, radians, Geodesic, GeodesicLine};
use std::f64::consts::FRAC_PI_2;

const EPS10: f64 = 1e-10;

#[derive(Clone, Copy)]
enum Mode {
    North,
    South,
    Equator,
    Oblique,
}

struct Gnom {
    mode: Mode,
    sinph0: f64,
    cosph0: f64,
    phi0: f64,
    geodesic: Option<Geodesic>,
}

impl Kernel for Gnom {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        if let Some(earth) = &self.geodesic {
            return ellipsoid_forward(earth, self.phi0, lam, phi);
        }
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let coslam = lam.cos();
        let mut y = match self.mode {
            Mode::Equator => cosphi * coslam,
            Mode::Oblique => self.sinph0 * sinphi + self.cosph0 * cosphi * coslam,
            Mode::South => -sinphi,
            Mode::North => sinphi,
        };
        if y <= EPS10 {
            return Err(TransformError::OutsideDomain);
        }
        y = 1.0 / y;
        let x = y * cosphi * lam.sin();
        y = match self.mode {
            Mode::Equator => y * sinphi,
            Mode::Oblique => y * (self.cosph0 * sinphi - self.sinph0 * cosphi * coslam),
            Mode::North => y * cosphi * -coslam,
            Mode::South => y * cosphi * coslam,
        };
        Ok((x, y))
    }

    fn inverse(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), TransformError> {
        if let Some(earth) = &self.geodesic {
            return ellipsoid_inverse(earth, self.phi0, x, y);
        }
        let rh = x.hypot(y);
        let mut phi = rh.atan();
        let sinz = phi.sin();
        let cosz = (1.0 - sinz * sinz).sqrt();
        if rh.abs() <= EPS10 {
            return Ok((0.0, self.phi0));
        }
        match self.mode {
            Mode::Oblique => {
                let raw = cosz * self.sinph0 + y * sinz * self.cosph0 / rh;
                phi = if raw.abs() >= 1.0 {
                    if raw > 0.0 {
                        FRAC_PI_2
                    } else {
                        -FRAC_PI_2
                    }
                } else {
                    raw.asin()
                };
                y = (cosz - self.sinph0 * phi.sin()) * rh;
                x *= sinz * self.cosph0;
            }
            Mode::Equator => {
                let raw = y * sinz / rh;
                phi = if raw.abs() >= 1.0 {
                    if raw > 0.0 {
                        FRAC_PI_2
                    } else {
                        -FRAC_PI_2
                    }
                } else {
                    raw.asin()
                };
                y = cosz * rh;
                x *= sinz;
            }
            Mode::South => phi -= FRAC_PI_2,
            Mode::North => {
                phi = FRAC_PI_2 - phi;
                y = -y;
            }
        }
        Ok((x.atan2(y), phi))
    }
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "gnom" {
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
    if name != "gnom" {
        return Ok(None);
    }
    let geodesic = if frame.earth.es() == 0.0 {
        None
    } else {
        Some(Geodesic::unit(frame.earth.flattening()))
    };
    let mode = if (frame.phi0.abs() - FRAC_PI_2).abs() < EPS10 {
        if frame.phi0 < 0.0 {
            Mode::South
        } else {
            Mode::North
        }
    } else if frame.phi0.abs() < EPS10 {
        Mode::Equator
    } else {
        Mode::Oblique
    };
    Ok(Some(classic_step(
        frame,
        Gnom {
            mode,
            sinph0: frame.phi0.sin(),
            cosph0: frame.phi0.cos(),
            phi0: frame.phi0,
            geodesic,
        },
    )))
}

fn ellipsoid_forward(
    earth: &Geodesic,
    phi0: f64,
    lam: f64,
    phi: f64,
) -> Result<(f64, f64), TransformError> {
    let (azimuth, reduced, scale) =
        earth.inverse_scale(degrees(phi0), 0.0, degrees(phi), degrees(lam));
    if scale <= 0.0 {
        return Err(TransformError::OutsideDomain);
    }
    let rho = reduced / scale;
    let azimuth = radians(azimuth);
    Ok((rho * azimuth.sin(), rho * azimuth.cos()))
}

fn ellipsoid_inverse(
    earth: &Geodesic,
    phi0: f64,
    x: f64,
    y: f64,
) -> Result<(f64, f64), TransformError> {
    let eps = 0.01 * f64::EPSILON.sqrt();
    let azimuth = degrees(x.atan2(y));
    let mut rho = x.hypot(y);
    let mut distance = rho.atan();
    let little = rho <= 1.0;
    if !little {
        rho = 1.0 / rho;
    }
    let line = GeodesicLine::from_azimuth(earth, degrees(phi0), 0.0, azimuth);
    let mut count = 10;
    let mut trip = false;
    let mut lat = 0.0;
    let mut lon = 0.0;
    while count > 0 {
        count -= 1;
        let (lat1, lon1, _along, reduced, scale) = line.position(distance);
        lat = lat1;
        lon = lon1;
        if trip {
            break;
        }
        let step = if little {
            (reduced - rho * scale) * scale
        } else {
            (rho * reduced - scale) * reduced
        };
        distance -= step;
        if !(step.abs() >= eps) {
            trip = true;
        }
    }
    if trip {
        Ok((radians(lon), radians(lat)))
    } else {
        Err(TransformError::OutsideDomain)
    }
}
