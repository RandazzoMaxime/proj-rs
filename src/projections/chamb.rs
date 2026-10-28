//! Chamberlin trimetric. Spherical, forward only.
//!
//! Missing control latitudes and longitudes are zero, as `pj_param` returns.
//! The three stations are stored relative to `lon_0`.

#![allow(
    clippy::pedantic,
    reason = "the intercept average follows chamb.cpp, including the initial offset"
)]

use crate::angle::adjlon;
use crate::engine::{classic_step, CreateError, Frame, Kernel, Params, TransformError};
use std::f64::consts::{FRAC_PI_2, PI};

const THIRD: f64 = 1.0 / 3.0;
const TOL: f64 = 1e-9;
const ONE_TOL: f64 = 1.000_000_000_000_01;

#[derive(Clone, Copy)]
struct Station {
    phi: f64,
    lam: f64,
    cosphi: f64,
    sinphi: f64,
    distance: f64,
    azimuth: f64,
    x: f64,
    y: f64,
}

struct Chamb {
    station: [Station; 3],
    x: f64,
    y: f64,
    beta_1: f64,
    beta_2: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "chamb" {
        Some(&["lat_1", "lon_1", "lat_2", "lon_2", "lat_3", "lon_3"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if name != "chamb" {
        return Ok(None);
    }
    let mut station = [Station {
        phi: 0.0,
        lam: 0.0,
        cosphi: 1.0,
        sinphi: 0.0,
        distance: 0.0,
        azimuth: 0.0,
        x: 0.0,
        y: 0.0,
    }; 3];
    for (index, place) in station.iter_mut().enumerate() {
        let lat_key = ["lat_1", "lat_2", "lat_3"][index];
        let lon_key = ["lon_1", "lon_2", "lon_3"][index];
        place.phi = params.radians(lat_key)?.unwrap_or(0.0);
        let lon = params.radians(lon_key)?.unwrap_or(0.0);
        place.lam = adjlon(lon - frame.lam0);
        place.cosphi = place.phi.cos();
        place.sinphi = place.phi.sin();
    }
    for index in 0..3 {
        let next = if index == 2 { 0 } else { index + 1 };
        let leg = vect(
            station[next].phi - station[index].phi,
            station[index].cosphi,
            station[index].sinphi,
            station[next].cosphi,
            station[next].sinphi,
            station[next].lam - station[index].lam,
        )
        .map_err(|_| CreateError::IllegalValue("lat_1"))?;
        if leg.0 == 0.0 {
            return Err(CreateError::IllegalValue("lat_1"));
        }
        station[index].distance = leg.0;
        station[index].azimuth = leg.1;
    }
    let beta_0 = lc(
        station[0].distance,
        station[2].distance,
        station[1].distance,
    )
    .map_err(|_| CreateError::IllegalValue("lat_1"))?;
    let beta_1 = lc(
        station[0].distance,
        station[1].distance,
        station[2].distance,
    )
    .map_err(|_| CreateError::IllegalValue("lat_1"))?;
    let beta_2 = PI - beta_0;
    station[0].y = station[2].distance * beta_0.sin();
    station[1].y = station[0].y;
    let y = 2.0 * station[0].y;
    station[2].y = 0.0;
    station[1].x = 0.5 * station[0].distance;
    station[0].x = -station[1].x;
    station[2].x = station[0].x + station[2].distance * beta_0.cos();
    let x = station[2].x;
    Ok(Some(classic_step(
        frame,
        Chamb {
            station,
            x,
            y,
            beta_1,
            beta_2,
        },
    )))
}

impl Kernel for Chamb {
    fn forward(&self, lam: f64, phi: f64) -> Result<(f64, f64), TransformError> {
        let (sinphi, cosphi) = (phi.sin(), phi.cos());
        let mut legs = [(0.0, 0.0); 3];
        for (index, station) in self.station.iter().enumerate() {
            let leg = vect(
                phi - station.phi,
                station.cosphi,
                station.sinphi,
                cosphi,
                sinphi,
                lam - station.lam,
            )?;
            if leg.0 == 0.0 {
                return Ok((station.x, station.y));
            }
            legs[index] = (leg.0, adjlon(leg.1 - station.azimuth));
        }
        let mut x = self.x;
        let mut y = self.y;
        for index in 0..3 {
            let next = if index == 2 { 0 } else { index + 1 };
            let mut angle = lc(self.station[index].distance, legs[index].0, legs[next].0)?;
            if legs[index].1 < 0.0 {
                angle = -angle;
            }
            let distance = legs[index].0;
            if index == 0 {
                x += distance * angle.cos();
                y -= distance * angle.sin();
            } else if index == 1 {
                angle = self.beta_1 - angle;
                x -= distance * angle.cos();
                y -= distance * angle.sin();
            } else {
                angle = self.beta_2 - angle;
                x += distance * angle.cos();
                y += distance * angle.sin();
            }
        }
        Ok((x * THIRD, y * THIRD))
    }

    fn inverse(&self, _x: f64, _y: f64) -> Result<(f64, f64), TransformError> {
        Err(TransformError::NoInverse)
    }
}

fn vect(
    dphi: f64,
    c1: f64,
    s1: f64,
    c2: f64,
    s2: f64,
    dlam: f64,
) -> Result<(f64, f64), TransformError> {
    let cdl = dlam.cos();
    let mut distance = if dphi.abs() > 1.0 || dlam.abs() > 1.0 {
        aacos(s1 * s2 + c1 * c2 * cdl)?
    } else {
        let dp = (0.5 * dphi).sin();
        let dl = (0.5 * dlam).sin();
        2.0 * aasin((dp * dp + c1 * c2 * dl * dl).sqrt())?
    };
    let azimuth = if distance.abs() > TOL {
        (c2 * dlam.sin()).atan2(c1 * s2 - s1 * c2 * cdl)
    } else {
        distance = 0.0;
        0.0
    };
    Ok((distance, azimuth))
}

fn lc(b: f64, c: f64, a: f64) -> Result<f64, TransformError> {
    aacos(0.5 * (b * b + c * c - a * a) / (b * c))
}

fn aacos(value: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > ONE_TOL {
            return Err(TransformError::OutsideDomain);
        }
        return Ok(if value < 0.0 { PI } else { 0.0 });
    }
    Ok(value.acos())
}

fn aasin(value: f64) -> Result<f64, TransformError> {
    let magnitude = value.abs();
    if magnitude >= 1.0 {
        if magnitude > ONE_TOL {
            return Err(TransformError::OutsideDomain);
        }
        return Ok(if value < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
    }
    Ok(value.asin())
}
