//! Geocentric latitude.
//!
//! Port of PROJ 9.9 `src/conversions/geoc.cpp`. The spherification radii live
//! in `crate::engine`, beside `R_C`, because they rewrite the ellipsoid before
//! any kernel runs.

#![allow(
    clippy::pedantic,
    reason = "the pole cutoff is the one in pj_geocentric_latitude"
)]

use crate::angle::adjlon;
use crate::engine::{
    geocentric_to_geographic, geographic_to_geocentric, prepare_forward, Coord, CreateError, Frame,
    IoKind, Params, Step, TransformError,
};

/// `+proj=geoc` takes only the global ellipsoid and frame keys.
pub(crate) const KEYS: &[&str] = &[];

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "geoc" {
        Some(KEYS)
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    _params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "geoc" {
        return Ok(None);
    }
    Ok(Some(Box::new(Geoc {
        frame: frame.clone(),
    })))
}

struct Geoc {
    frame: Frame,
}

impl Step for Geoc {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(nan_plane(coord));
        }
        // `fwd_prepare` already turned a `+geoc` input into geographic latitude
        // and subtracted `lon_0`. The kernel then goes the other way. Output
        // longitude stays relative to `lon_0`; radians output does not add it back.
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let phi = geographic_to_geocentric(phi, self.frame.earth.one_es());
        Ok(Coord {
            x: lam,
            y: phi,
            z: self.frame.vfr_meter * (coord.z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if coord.x.is_nan() || coord.y.is_nan() {
            return Ok(nan_plane(coord));
        }
        let mut phi = geocentric_to_geographic(coord.y, self.frame.earth.one_es());
        // `inv_finalize` applies `+geoc` after the kernel, so both flags cancel.
        if self.frame.geoc {
            phi = geographic_to_geocentric(phi, self.frame.earth.one_es());
        }
        let mut lam = coord.x + self.frame.from_greenwich + self.frame.lam0;
        if !self.frame.over {
            lam = adjlon(lam);
        }
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
        IoKind::Angular
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

fn nan_plane(coord: Coord) -> Coord {
    Coord {
        x: f64::NAN,
        y: f64::NAN,
        z: coord.z,
        t: coord.t,
    }
}

#[cfg(test)]
mod tests {
    use super::{build, Frame};
    use crate::engine::{Coord, Params};
    use crate::Ellipsoid;

    fn grs80_frame() -> Frame {
        Frame {
            earth: Ellipsoid::grs80(),
            lam0: 0.0,
            phi0: 0.0,
            x0: 0.0,
            y0: 0.0,
            z0: 0.0,
            k0: 1.0,
            to_meter: 1.0,
            fr_meter: 1.0,
            vto_meter: 1.0,
            vfr_meter: 1.0,
            over: false,
            geoc: false,
            from_greenwich: 0.0,
            lon_wrap: None,
        }
    }

    fn step() -> Box<dyn crate::engine::Step> {
        let params = Params::parse("+proj=geoc +ellps=GRS80").expect("params");
        build("geoc", &params, &grs80_frame())
            .expect("build")
            .expect("geoc")
    }

    #[test]
    fn grs80_matches_more_builtins_gie() {
        let geoc = step();
        let input = Coord::xy(12.0_f64.to_radians(), 55.0_f64.to_radians());
        let got = geoc.forward(input).expect("forward");
        assert!((got.x.to_degrees() - 12.0).abs() < 1e-12);
        assert!((got.y.to_degrees() - 54.818_973_308_324_573).abs() < 1e-12);
        assert_eq!(got.z, 0.0);
        let back = geoc.inverse(got).expect("inverse");
        assert!((back.x - input.x).abs() < 1e-12);
        assert!((back.y - input.y).abs() < 1e-12);
        for latitude in [90.0_f64, -90.0, 89.999_999_999_99] {
            let pole = Coord::xy(12.0_f64.to_radians(), latitude.to_radians());
            let got = geoc.forward(pole).expect("pole");
            assert!((got.y - pole.y).abs() < 1e-15, "{latitude}");
        }
    }

    #[test]
    fn create_accepts_geoc_and_rejects_an_unknown_parameter() {
        let op = crate::Operation::create("+proj=geoc +ellps=GRS80").expect("geoc");
        let got = op
            .forward(Coord::xy(12.0_f64.to_radians(), 55.0_f64.to_radians()))
            .expect("forward");
        assert!((got.y.to_degrees() - 54.818_973_308_324_573).abs() < 1e-12);
        let err = crate::Operation::create("+proj=geoc +ellps=GRS80 +not_a_param");
        assert!(matches!(err, Err(crate::CreateError::UnknownParameter(_))));
    }
}
