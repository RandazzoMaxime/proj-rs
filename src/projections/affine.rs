//! Affine family: `+proj=affine`, `+proj=geogoffset`, `+proj=vertoffset`.
//!
//! `affine` and `geogoffset` follow PROJ 9.9 `src/transformations/affine.cpp`.
//! `vertoffset` follows `src/transformations/vertoffset.cpp`. None of them is a
//! classic projection: the result is not scaled by the semi-major axis, and
//! nothing reads `P->k0`.

#![allow(
    clippy::pedantic,
    reason = "cofactors and the arcsecond constant follow affine.cpp and vertoffset.cpp"
)]

use crate::angle::adjlon;
use crate::engine::{
    geographic_to_geocentric, prepare_forward, Coord, CreateError, Frame, IoKind, Params, Step,
    TransformError,
};

/// `DEG_TO_RAD` in `proj_internal.h` (`0.017453292519943296`), divided by 3600.
const ARCSEC_TO_RAD: f64 = 0.017453292519943296 / 3600.0;

const AFFINE_KEYS: &[&str] = &[
    "xoff", "yoff", "zoff", "toff", "s11", "s12", "s13", "s21", "s22", "s23", "s31", "s32", "s33",
    "tscale",
];
const GEOG_KEYS: &[&str] = &["dlon", "dlat", "dh"];
const VERT_KEYS: &[&str] = &["dh", "slope_lat", "slope_lon"];

#[derive(Clone, Copy)]
struct Coeffs {
    s11: f64,
    s12: f64,
    s13: f64,
    s21: f64,
    s22: f64,
    s23: f64,
    s31: f64,
    s32: f64,
    s33: f64,
    tscale: f64,
}

impl Coeffs {
    fn identity() -> Self {
        Self {
            s11: 1.0,
            s12: 0.0,
            s13: 0.0,
            s21: 0.0,
            s22: 1.0,
            s23: 0.0,
            s31: 0.0,
            s32: 0.0,
            s33: 1.0,
            tscale: 1.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Offsets {
    x: f64,
    y: f64,
    z: f64,
    t: f64,
}

/// `+proj=affine` (raw units) or `+proj=geogoffset` (radians in and out).
struct Affine {
    off: Offsets,
    forward: Coeffs,
    reverse: Option<Coeffs>,
    frame: Frame,
    angular: bool,
}

struct VertOffset {
    frame: Frame,
    slope_lon: f64,
    slope_lat: f64,
    zoff: f64,
    rho0: f64,
    nu0: f64,
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "affine" => Some(AFFINE_KEYS),
        "geogoffset" => Some(GEOG_KEYS),
        "vertoffset" => Some(VERT_KEYS),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    match name {
        "affine" => Ok(Some(Box::new(affine_from(params, frame)?))),
        "geogoffset" => Ok(Some(Box::new(geogoffset_from(params, frame)?))),
        "vertoffset" => Ok(Some(Box::new(vertoffset_from(params, frame)?))),
        _ => Ok(None),
    }
}

fn affine_from(params: &Params, frame: &Frame) -> Result<Affine, CreateError> {
    let mut forward = Coeffs::identity();
    // Absent `s11` / `s22` / `s33` / `tscale` stay 1. A present 0 replaces them.
    forward.s11 = diagonal(params, "s11", forward.s11)?;
    forward.s12 = term(params, "s12")?;
    forward.s13 = term(params, "s13")?;
    forward.s21 = term(params, "s21")?;
    forward.s22 = diagonal(params, "s22", forward.s22)?;
    forward.s23 = term(params, "s23")?;
    forward.s31 = term(params, "s31")?;
    forward.s32 = term(params, "s32")?;
    forward.s33 = diagonal(params, "s33", forward.s33)?;
    forward.tscale = diagonal(params, "tscale", forward.tscale)?;
    let reverse = inverse_coeffs(&forward);
    Ok(Affine {
        off: Offsets {
            x: term(params, "xoff")?,
            y: term(params, "yoff")?,
            z: term(params, "zoff")?,
            t: term(params, "toff")?,
        },
        forward,
        reverse,
        frame: frame.clone(),
        angular: false,
    })
}

fn geogoffset_from(params: &Params, frame: &Frame) -> Result<Affine, CreateError> {
    // `affine.cpp` does not recompute the inverse: the matrix stays identity.
    Ok(Affine {
        off: Offsets {
            x: term(params, "dlon")? * ARCSEC_TO_RAD,
            y: term(params, "dlat")? * ARCSEC_TO_RAD,
            z: term(params, "dh")?,
            t: 0.0,
        },
        forward: Coeffs::identity(),
        reverse: Some(Coeffs::identity()),
        frame: frame.clone(),
        angular: true,
    })
}

fn vertoffset_from(params: &Params, frame: &Frame) -> Result<VertOffset, CreateError> {
    let sinlat0 = frame.phi0.sin();
    let denom = 1.0 - frame.earth.es() * sinlat0 * sinlat0;
    let root = denom.sqrt();
    Ok(VertOffset {
        frame: frame.clone(),
        slope_lon: term(params, "slope_lon")? * ARCSEC_TO_RAD,
        slope_lat: term(params, "slope_lat")? * ARCSEC_TO_RAD,
        zoff: term(params, "dh")?,
        rho0: frame.earth.a() * frame.earth.one_es() / (denom * root),
        nu0: frame.earth.a() / root,
    })
}

/// A missing key keeps `default`. A present key, including 0, replaces it.
fn diagonal(params: &Params, key: &str, default: f64) -> Result<f64, CreateError> {
    if params.flag(key) {
        term(params, key)
    } else {
        Ok(default)
    }
}

fn term(params: &Params, key: &str) -> Result<f64, CreateError> {
    Ok(params.number(key)?.unwrap_or(0.0))
}

/// Cofactors from `computeReverseParameters`. `None` when `det` or `tscale` is 0.
fn inverse_coeffs(forward: &Coeffs) -> Option<Coeffs> {
    let a = forward.s11;
    let b = forward.s12;
    let c = forward.s13;
    let d = forward.s21;
    let e = forward.s22;
    let f = forward.s23;
    let g = forward.s31;
    let h = forward.s32;
    let i = forward.s33;
    let big_a = e * i - f * h;
    let big_b = -(d * i - f * g);
    let big_c = d * h - e * g;
    let big_d = -(b * i - c * h);
    let big_e = a * i - c * g;
    let big_f = -(a * h - b * g);
    let big_g = b * f - c * e;
    let big_h = -(a * f - c * d);
    let big_i = a * e - b * d;
    let det = a * big_a + b * big_b + c * big_c;
    if det == 0.0 || forward.tscale == 0.0 {
        return None;
    }
    Some(Coeffs {
        s11: big_a / det,
        s12: big_d / det,
        s13: big_g / det,
        s21: big_b / det,
        s22: big_e / det,
        s23: big_h / det,
        s31: big_c / det,
        s32: big_f / det,
        s33: big_i / det,
        tscale: 1.0 / forward.tscale,
    })
}

fn apply_forward(coeffs: &Coeffs, off: Offsets, coord: Coord) -> Coord {
    Coord {
        x: off.x + coeffs.s11 * coord.x + coeffs.s12 * coord.y + coeffs.s13 * coord.z,
        y: off.y + coeffs.s21 * coord.x + coeffs.s22 * coord.y + coeffs.s23 * coord.z,
        z: off.z + coeffs.s31 * coord.x + coeffs.s32 * coord.y + coeffs.s33 * coord.z,
        t: off.t + coeffs.tscale * coord.t,
    }
}

fn apply_reverse(coeffs: &Coeffs, off: Offsets, coord: Coord) -> Coord {
    let x = coord.x - off.x;
    let y = coord.y - off.y;
    let z = coord.z - off.z;
    Coord {
        x: coeffs.s11 * x + coeffs.s12 * y + coeffs.s13 * z,
        y: coeffs.s21 * x + coeffs.s22 * y + coeffs.s23 * z,
        z: coeffs.s31 * x + coeffs.s32 * y + coeffs.s33 * z,
        t: coeffs.tscale * (coord.t - off.t),
    }
}

impl Affine {
    fn forward_raw(&self, coord: Coord) -> Coord {
        apply_forward(&self.forward, self.off, coord)
    }

    fn inverse_raw(&self, coord: Coord) -> Result<Coord, TransformError> {
        let Some(reverse) = &self.reverse else {
            return Err(TransformError::NoInverse);
        };
        Ok(apply_reverse(reverse, self.off, coord))
    }

    fn forward_angular(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let shifted = self.forward_raw(Coord {
            x: lam,
            y: phi,
            z: coord.z,
            t: coord.t,
        });
        Ok(Coord {
            x: shifted.x,
            y: shifted.y,
            z: self.frame.vfr_meter * (shifted.z + self.frame.z0),
            t: shifted.t,
        })
    }

    fn inverse_angular(&self, coord: Coord) -> Result<Coord, TransformError> {
        let z = self.frame.vto_meter * coord.z - self.frame.z0;
        let shifted = self.inverse_raw(Coord {
            x: coord.x,
            y: coord.y,
            z,
            t: coord.t,
        })?;
        let (lam, phi) = finish_angular(&self.frame, shifted.x, shifted.y);
        Ok(Coord {
            x: lam,
            y: phi,
            z: shifted.z,
            t: shifted.t,
        })
    }
}

impl Step for Affine {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if self.angular {
            self.forward_angular(coord)
        } else {
            Ok(self.forward_raw(coord))
        }
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if self.angular {
            self.inverse_angular(coord)
        } else {
            self.inverse_raw(coord)
        }
    }

    fn input_kind(&self) -> IoKind {
        if self.angular {
            IoKind::Angular
        } else {
            IoKind::Any
        }
    }

    fn output_kind(&self) -> IoKind {
        if self.angular {
            IoKind::Angular
        } else {
            IoKind::Any
        }
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}

impl VertOffset {
    /// `get_forward_offset`. `lam` is already relative to `lon_0`.
    fn offset(&self, phi: f64, lam: f64) -> f64 {
        self.zoff
            + self.slope_lat * self.rho0 * (phi - self.frame.phi0)
            + self.slope_lon * self.nu0 * lam * phi.cos()
    }
}

impl Step for VertOffset {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let z = coord.z + self.offset(phi, lam);
        // `fwd_prepare` subtracted `lam0`. The kernel adds it back and leaves
        // `from_greenwich` subtracted. `fwd_finalize` then scales Z.
        Ok(Coord {
            x: lam + self.frame.lam0,
            y: phi,
            z: self.frame.vfr_meter * (z + self.frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let z = self.frame.vto_meter * coord.z - self.frame.z0;
        let lam = coord.x - self.frame.lam0;
        let phi = coord.y;
        let z = z - self.offset(phi, lam);
        let (lam, phi) = finish_angular(&self.frame, lam, phi);
        Ok(Coord {
            x: lam,
            y: phi,
            z,
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

/// `inv_finalize` for a radians output: restore `lam0` and the prime meridian.
fn finish_angular(frame: &Frame, mut lam: f64, mut phi: f64) -> (f64, f64) {
    lam = lam + frame.from_greenwich + frame.lam0;
    if !frame.over {
        lam = adjlon(lam);
    }
    if frame.geoc {
        phi = geographic_to_geocentric(phi, frame.earth.one_es());
    }
    (lam, phi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Operation;

    fn near(got: f64, expect: f64) -> bool {
        (got - expect).abs() < 1e-6
    }

    #[test]
    fn affine_matrix_matches_more_builtins() {
        let op = Operation::create(
            "+proj=affine +xoff=1 +yoff=2 +zoff=3 +toff=4 +s11=11 +s12=12 +s13=13 +s21=21 +s22=22 +s23=23 +s31=-31 +s32=32 +s33=33 +tscale=34",
        )
        .expect("affine");
        let got = op
            .forward(Coord {
                x: 2.0,
                y: 49.0,
                z: 10.0,
                t: 100.0,
            })
            .expect("forward");
        assert!(near(got.x, 741.0), "{}", got.x);
        assert!(near(got.y, 1352.0), "{}", got.y);
        assert!(near(got.z, 1839.0), "{}", got.z);
        assert!(near(got.t, 3404.0), "{}", got.t);
        let back = op.inverse(got).expect("inverse");
        assert!(near(back.x, 2.0) && near(back.y, 49.0) && near(back.z, 10.0));
        assert!(near(back.t, 100.0), "{}", back.t);

        let flat = op.forward(Coord::xy(2.0, 49.0)).expect("2d");
        assert!(near(flat.x, 611.0) && near(flat.y, 1122.0));
        let restored = op.inverse(flat).expect("2d inverse");
        assert!(near(restored.x, 2.0) && near(restored.y, 49.0) && near(restored.z, 0.0));
    }

    #[test]
    fn singular_affine_has_no_inverse() {
        let matrix = Operation::create("+proj=affine +s11=0 +s22=0 +s23=0").expect("det");
        assert_eq!(
            matrix.inverse(Coord::xy(0.0, 0.0)).expect_err("no inv"),
            TransformError::NoInverse
        );
        let time = Operation::create("+proj=affine +tscale=0").expect("tscale");
        assert_eq!(
            time.inverse(Coord::xy(0.0, 0.0)).expect_err("no inv"),
            TransformError::NoInverse
        );
        let forward =
            Operation::create("+proj=affine +s11=1 +s12=1 +s13=1 +s22=0 +s33=0").expect("row");
        let got = forward
            .forward(Coord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                t: 0.0,
            })
            .expect("forward");
        assert!(near(got.x, 6.0) && near(got.y, 0.0) && near(got.z, 0.0));
    }

    #[test]
    fn geogoffset_and_vertoffset_match_more_builtins() {
        let shift =
            Operation::create("+proj=geogoffset +dlon=3600 +dlat=-3600 +dh=3").expect("geogoffset");
        let got = shift
            .forward(Coord {
                x: 10.0_f64.to_radians(),
                y: 20.0_f64.to_radians(),
                z: 30.0,
                t: 40.0,
            })
            .expect("forward");
        assert!(near(got.x.to_degrees(), 11.0), "{}", got.x.to_degrees());
        assert!(near(got.y.to_degrees(), 19.0), "{}", got.y.to_degrees());
        assert!(near(got.z, 33.0) && near(got.t, 40.0));
        let back = shift.inverse(got).expect("inverse");
        assert!(near(back.x.to_degrees(), 10.0) && near(back.y.to_degrees(), 20.0));
        assert!(near(back.z, 30.0));

        let vert = Operation::create(
            "+proj=vertoffset +lat_0=46.9166666666666666 +lon_0=8.183333333333334 +dh=-0.245 +slope_lat=-0.210 +slope_lon=-0.032 +ellps=GRS80",
        )
        .expect("vertoffset");
        let lon = 9.666_666_666_666_666_f64.to_radians();
        let lat = 47.333_333_333_333_336_f64.to_radians();
        let got = vert
            .forward(Coord {
                x: lon,
                y: lat,
                z: 473.0,
                t: 0.0,
            })
            .expect("forward");
        assert!(near(got.x, lon), "{}", got.x);
        assert!(near(got.y, lat), "{}", got.y);
        assert!((got.z - 472.690).abs() < 1e-3, "{}", got.z);
        let back = vert.inverse(got).expect("inverse");
        assert!(near(back.x, lon) && near(back.y, lat) && near(back.z, 473.0));
    }

    #[test]
    fn pipeline_omit_with_affine_matches_the_gie_table() {
        let plain =
            Operation::create("+proj=pipeline +step +proj=affine +xoff=1 +yoff=1 +omit_inv")
                .expect("omit_inv");
        let got = plain.forward(Coord::xy(2.0, 49.0)).expect("fwd");
        assert!(near(got.x, 3.0) && near(got.y, 50.0));
        let held = plain.inverse(Coord::xy(2.0, 49.0)).expect("inv");
        assert!(near(held.x, 2.0) && near(held.y, 49.0));

        let flipped =
            Operation::create("+proj=pipeline +step +inv +proj=affine +xoff=1 +yoff=1 +omit_inv")
                .expect("inv omit_inv");
        let got = flipped.forward(Coord::xy(2.0, 49.0)).expect("fwd");
        assert!(near(got.x, 1.0) && near(got.y, 48.0));
        let held = flipped.inverse(Coord::xy(2.0, 49.0)).expect("inv");
        assert!(near(held.x, 2.0) && near(held.y, 49.0));

        let skip_fwd =
            Operation::create("+proj=pipeline +step +proj=affine +xoff=1 +yoff=1 +omit_fwd")
                .expect("omit_fwd");
        let held = skip_fwd.forward(Coord::xy(2.0, 49.0)).expect("fwd");
        assert!(near(held.x, 2.0) && near(held.y, 49.0));
        let got = skip_fwd.inverse(Coord::xy(2.0, 49.0)).expect("inv");
        assert!(near(got.x, 1.0) && near(got.y, 48.0));

        let skip_fwd_inv =
            Operation::create("+proj=pipeline +step +inv +proj=affine +xoff=1 +yoff=1 +omit_fwd")
                .expect("inv omit_fwd");
        let held = skip_fwd_inv.forward(Coord::xy(2.0, 49.0)).expect("fwd");
        assert!(near(held.x, 2.0) && near(held.y, 49.0));
        let got = skip_fwd_inv.inverse(Coord::xy(2.0, 49.0)).expect("inv");
        assert!(near(got.x, 3.0) && near(got.y, 50.0));

        let scale = Operation::create("proj=pipeline step proj=affine s11=2 omit_fwd")
            .expect("s11 omit_fwd");
        let held = scale
            .forward(Coord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                t: 0.0,
            })
            .expect("fwd");
        assert!(near(held.x, 1.0));
        let got = scale
            .inverse(Coord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                t: 0.0,
            })
            .expect("inv");
        assert!(near(got.x, 0.5) && near(got.y, 2.0) && near(got.z, 3.0));

        let singular = Operation::create(
            "proj=pipeline step proj=affine s11=1 s12=1 s13=1 s22=0 s33=0 omit_inv inv",
        )
        .expect("creation still succeeds");
        assert_eq!(
            singular
                .forward(Coord {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                    t: 0.0,
                })
                .expect_err("forward needs the missing inverse"),
            TransformError::NoInverse
        );
        let runnable = Operation::create(
            "proj=pipeline step proj=affine s11=1 s12=1 s13=1 s22=0 s33=0 omit_fwd inv",
        )
        .expect("omit_fwd inv");
        let got = runnable
            .inverse(Coord {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                t: 0.0,
            })
            .expect("inverse runs forward");
        assert!(near(got.x, 6.0) && near(got.y, 0.0) && near(got.z, 0.0));
    }

    #[test]
    fn unknown_affine_parameter_is_rejected() {
        let error = Operation::create("+proj=affine +not_a_param=1").expect_err("unknown");
        assert!(matches!(error, CreateError::UnknownParameter(_)));
    }
}
