//! Approximate transverse Mercator (Evenden / Snyder).
//!
//! Port of `approx_e_fwd` / `approx_e_inv` in PROJ 9.9 `src/projections/tmerc.cpp`.
//! Meridional distance is `pj_mlfn`, which in 9.9 is the order-6 rectifying series.

#![allow(
    clippy::pedantic,
    reason = "the polynomial is transcribed from PROJ approx_e_fwd and approx_e_inv"
)]

use crate::angle::adjlon;
use crate::projmath::Meridian;
use crate::{Ellipsoid, Geographic, Projected, ProjectionError};
use std::f64::consts::FRAC_PI_2;

const FC1: f64 = 1.0;
const FC2: f64 = 0.5;
const FC3: f64 = 0.166_666_666_666_666_666_66;
const FC4: f64 = 0.083_333_333_333_333_333_33;
const FC5: f64 = 0.05;
const FC6: f64 = 0.033_333_333_333_333_333_33;
const FC7: f64 = 0.023_809_523_809_523_809_52;
const FC8: f64 = 0.017_857_142_857_142_857_14;

/// Ellipsoidal Evenden/Snyder transverse Mercator, metres in and out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApproxTmerc {
    a: f64,
    ra: f64,
    k0: f64,
    lam0: f64,
    x0: f64,
    y0: f64,
    es: f64,
    esp: f64,
    ml0: f64,
    meridian: Meridian,
}

impl ApproxTmerc {
    /// `lon0_deg` and `lat0_deg` are the origin. `k0` is the central scale.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] when an origin is not finite, and
    /// [`ProjectionError::InvalidScale`] when `k0` is not strictly positive.
    pub(crate) fn try_new(
        ell: Ellipsoid,
        lon0_deg: f64,
        lat0_deg: f64,
        k0: f64,
        x0: f64,
        y0: f64,
    ) -> Result<Self, ProjectionError> {
        if !lon0_deg.is_finite() || !lat0_deg.is_finite() || !x0.is_finite() || !y0.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        if !k0.is_finite() || k0 <= 0.0 {
            return Err(ProjectionError::InvalidScale);
        }
        let phi0 = lat0_deg.to_radians();
        let meridian = Meridian::new(ell.n());
        Ok(Self {
            a: ell.a(),
            ra: ell.ra(),
            k0,
            lam0: lon0_deg.to_radians(),
            x0,
            y0,
            es: ell.es(),
            esp: ell.e2s(),
            ml0: meridian.forward(phi0),
            meridian,
        })
    }

    /// Absolute radians to metres.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::OutsideDomain`] when the longitude is more than 90°
    /// from the central meridian, matching `approx_e_fwd`.
    pub(crate) fn forward(&self, lon_rad: f64, lat_rad: f64) -> Result<Projected, ProjectionError> {
        if !lon_rad.is_finite() || !lat_rad.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        let phi = lat_rad.clamp(-FRAC_PI_2, FRAC_PI_2);
        let lam = adjlon(adjlon(lon_rad) - self.lam0);
        let (x, y) = self.fwd_unit(lam, phi)?;
        Ok(Projected {
            east: self.a * x + self.x0,
            north: self.a * y + self.y0,
        })
    }

    /// Metres to degrees.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] when an input is not finite.
    pub(crate) fn inverse(&self, xy: Projected) -> Result<Geographic, ProjectionError> {
        if !xy.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        let (lam, phi) = self.inv_unit(
            (xy.east - self.x0) * self.ra,
            (xy.north - self.y0) * self.ra,
        );
        Ok(Geographic {
            lon_deg: adjlon(lam + self.lam0).to_degrees(),
            lat_deg: phi.to_degrees(),
        })
    }

    fn fwd_unit(&self, lam: f64, phi: f64) -> Result<(f64, f64), ProjectionError> {
        if lam < -FRAC_PI_2 || lam > FRAC_PI_2 {
            return Err(ProjectionError::OutsideDomain);
        }
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let t = if cosphi.abs() > 1e-10 {
            sinphi / cosphi
        } else {
            0.0
        };
        let t2 = t * t;
        let als = {
            let al = cosphi * lam;
            al * al
        };
        let al = cosphi * lam / (1.0 - self.es * sinphi * sinphi).sqrt();
        let n = self.esp * cosphi * cosphi;
        let x_poly = 61.0 + t2 * (t2 * (179.0 - t2) - 479.0);
        let x_mid = 5.0 + t2 * (t2 - 18.0) + n * (14.0 - 58.0 * t2) + FC7 * als * x_poly;
        let x = self.k0 * al * (FC1 + FC3 * als * (1.0 - t2 + n + FC5 * als * x_mid));
        let y_poly = 1385.0 + t2 * (t2 * (543.0 - t2) - 3111.0);
        let y_mid = 61.0 + t2 * (t2 - 58.0) + n * (270.0 - 330.0 * t2) + FC8 * als * y_poly;
        let y_inner = 5.0 - t2 + n * (9.0 + 4.0 * n) + FC6 * als * y_mid;
        let y = self.k0
            * (self.meridian.forward(phi) - self.ml0
                + sinphi * al * lam * FC2 * (1.0 + FC4 * als * y_inner));
        Ok((x, y))
    }

    fn inv_unit(&self, x: f64, y: f64) -> (f64, f64) {
        let mut phi = self.meridian.inverse(self.ml0 + y / self.k0);
        if phi.abs() >= FRAC_PI_2 {
            return (0.0, if y < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 });
        }
        let sinphi = phi.sin();
        let cosphi = phi.cos();
        let t = if cosphi.abs() > 1e-10 {
            sinphi / cosphi
        } else {
            0.0
        };
        let t2 = t * t;
        let n = self.esp * cosphi * cosphi;
        let con = 1.0 - self.es * sinphi * sinphi;
        let d = x * con.sqrt() / self.k0;
        let ds = d * d;
        let phi_poly = 1385.0 + t2 * (3633.0 + t2 * (4095.0 + 1575.0 * t2));
        let phi_mid = 61.0 + t2 * (90.0 - 252.0 * n + 45.0 * t2) + 46.0 * n - ds * FC8 * phi_poly;
        let phi_inner = 5.0 + t2 * (3.0 - 9.0 * n) + n * (1.0 - 4.0 * n) - ds * FC6 * phi_mid;
        phi -= (con * t * ds / (1.0 - self.es)) * FC2 * (1.0 - ds * FC4 * phi_inner);
        let lam_poly = 61.0 + t2 * (662.0 + t2 * (1320.0 + 720.0 * t2));
        let lam_mid = 5.0 + t2 * (28.0 + 24.0 * t2 + 8.0 * n) + 6.0 * n - ds * FC7 * lam_poly;
        let lam = d * (FC1 - ds * FC3 * (1.0 + 2.0 * t2 + n - ds * FC5 * lam_mid)) / cosphi;
        (lam, phi)
    }
}
