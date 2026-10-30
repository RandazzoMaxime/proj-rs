//! Spherical cross-track height (SCH) around a peg point.
//!
//! PROJ runs SCH as `fwd3d` and `inv3d` on a geodetic input. The geocentric
//! point is shifted to the peg, rotated into the peg frame, and read as
//! longitude and latitude on a sphere of radius `rcurv`, the peg's radius of
//! curvature. The output is in metres on that sphere: `x = rcurv · λ`,
//! `y = rcurv · φ`, and `z` the height above it.

use crate::ellipsoid::Ellipsoid;
use crate::engine::{
    cartesian_to_geodetic, finish_angular, geodetic_to_cartesian, prepare_forward, Coord,
    CreateError, Frame, IoKind, Params, Step, TransformError,
};

struct Sch {
    frame: Frame,
    sphere: Ellipsoid,
    rcurv: f64,
    offset: [f64; 3],
    /// Row-major rotation from geocentric to the peg frame.
    rot: [f64; 9],
}

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if name == "sch" {
        Some(&["plat_0", "plon_0", "phdg_0", "h_0"])
    } else {
        None
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn Step>>, CreateError> {
    if name != "sch" {
        return Ok(None);
    }
    let plat = params
        .radians("plat_0")?
        .ok_or(CreateError::MissingArgument("plat_0"))?;
    let plon = params
        .radians("plon_0")?
        .ok_or(CreateError::MissingArgument("plon_0"))?;
    let phdg = params
        .radians("phdg_0")?
        .ok_or(CreateError::MissingArgument("phdg_0"))?;
    let h0 = params.number("h_0")?.unwrap_or(0.0);

    let earth = frame.earth;
    let (clt, slt) = (plat.cos(), plat.sin());
    let (clo, slo) = (plon.cos(), plon.sin());
    let (chdg, shdg) = (phdg.cos(), phdg.sin());

    let temp = (1.0 - earth.es() * slt * slt).sqrt();
    let reast = earth.a() / temp;
    let rnorth = earth.a() * (1.0 - earth.es()) / temp.powi(3);
    let rcurv = h0 + (reast * rnorth) / (reast * chdg * chdg + rnorth * shdg * shdg);
    let sphere =
        Ellipsoid::try_from_a_es(rcurv, 0.0).map_err(|_| CreateError::IllegalValue("h_0"))?;

    let rot = [
        clt * clo,
        -shdg * slo - slt * clo * chdg,
        slo * chdg - slt * clo * shdg,
        clt * slo,
        clo * shdg - slt * slo * chdg,
        -clo * chdg - slt * slo * shdg,
        slt,
        clt * chdg,
        clt * shdg,
    ];
    let (ox, oy, oz) = geodetic_to_cartesian(earth, plon, plat, h0);
    let offset = [
        ox - rcurv * clt * clo,
        oy - rcurv * clt * slo,
        oz - rcurv * slt,
    ];
    Ok(Some(Box::new(Sch {
        frame: frame.clone(),
        sphere,
        rcurv,
        offset,
        rot,
    })))
}

impl Step for Sch {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        let (lam, phi) = prepare_forward(&self.frame, coord.x, coord.y)?;
        let (x, y, z) = geodetic_to_cartesian(self.frame.earth, lam, phi, coord.z);
        let [ox, oy, oz] = self.offset;
        let (dx, dy, dz) = (x - ox, y - oy, z - oz);
        let r = &self.rot;
        let px = r[0] * dx + r[3] * dy + r[6] * dz;
        let py = r[1] * dx + r[4] * dy + r[7] * dz;
        let pz = r[2] * dx + r[5] * dy + r[8] * dz;
        let (slam, sphi, height) = cartesian_to_geodetic(self.sphere, px, py, pz);
        let frame = &self.frame;
        Ok(Coord {
            x: frame.fr_meter * (self.rcurv * slam + frame.x0),
            y: frame.fr_meter * (self.rcurv * sphi + frame.y0),
            z: frame.vfr_meter * (height + frame.z0),
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        let frame = &self.frame;
        let x = frame.to_meter * coord.x - frame.x0;
        let y = frame.to_meter * coord.y - frame.y0;
        let z = frame.vto_meter * coord.z - frame.z0;
        let (sx, sy, sz) = geodetic_to_cartesian(self.sphere, x / self.rcurv, y / self.rcurv, z);
        let r = &self.rot;
        let [ox, oy, oz] = self.offset;
        let gx = r[0] * sx + r[1] * sy + r[2] * sz + ox;
        let gy = r[3] * sx + r[4] * sy + r[5] * sz + oy;
        let gz = r[6] * sx + r[7] * sy + r[8] * sz + oz;
        let (lam, phi, height) = cartesian_to_geodetic(self.frame.earth, gx, gy, gz);
        let (lam, phi) = finish_angular(&self.frame, lam, phi);
        Ok(Coord {
            x: lam,
            y: phi,
            z: height,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> IoKind {
        IoKind::Angular
    }

    fn output_kind(&self) -> IoKind {
        IoKind::Cartesian
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.frame.earth.a(), self.frame.earth.es())
    }
}
