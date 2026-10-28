//! Mercator sphérique (Web Mercator) et ellipsoïdal.
//!
//! Portage de `src/projections/merc.cpp` (PROJ 9.4). Voir `docs/PORTING-merc.md`.

use crate::angle::adjlon;
use crate::lcc::{phi2, tsfn};
use crate::{Ellipsoid, Geographic, Projected, ProjectionError};

/// Rayon de sphère du Web Mercator (EPSG:3857), le `a` de WGS 84.
const WEB_RADIUS: f64 = 6_378_137.0;

#[derive(Clone, Copy, Debug)]
enum Model {
    Sphere,
    Ellipsoid { e: f64 },
}

/// Mercator préparé.
#[derive(Clone, Copy, Debug)]
pub struct Mercator {
    a: f64,
    k0: f64,
    lam0: f64,
    x0: f64,
    y0: f64,
    model: Model,
}

impl Mercator {
    /// Web Mercator : sphère de rayon 6 378 137 m, origine nulle, `k0 = 1`.
    ///
    /// C'est `+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +k=1`.
    #[must_use]
    pub fn web() -> Self {
        Self {
            a: WEB_RADIUS,
            k0: 1.0,
            lam0: 0.0,
            x0: 0.0,
            y0: 0.0,
            model: Model::Sphere,
        }
    }

    /// Mercator sphérique de rayon `radius`.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] si un paramètre ne l'est pas,
    /// [`ProjectionError::InvalidScale`] si `k0` est nul,
    /// [`ProjectionError::NonFinite`] si `radius` n'est pas strictement positif.
    pub fn try_sphere(
        radius: f64,
        lon0_deg: f64,
        k0: f64,
        x0: f64,
        y0: f64,
    ) -> Result<Self, ProjectionError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ProjectionError::NonFinite);
        }
        if ![lon0_deg, x0, y0].iter().all(|value| value.is_finite()) {
            return Err(ProjectionError::NonFinite);
        }
        if !k0.is_finite() || k0 == 0.0 {
            return Err(ProjectionError::InvalidScale);
        }
        Ok(Self {
            a: radius,
            k0,
            lam0: lon0_deg.to_radians(),
            x0,
            y0,
            model: Model::Sphere,
        })
    }

    /// Mercator ellipsoïdal, `+proj=merc +ellps=…`.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] si un paramètre ne l'est pas,
    /// [`ProjectionError::InvalidScale`] si `k0` est nul.
    pub fn try_ellipsoid(
        ell: Ellipsoid,
        lon0_deg: f64,
        k0: f64,
        x0: f64,
        y0: f64,
    ) -> Result<Self, ProjectionError> {
        if ![lon0_deg, x0, y0].iter().all(|v| v.is_finite()) {
            return Err(ProjectionError::NonFinite);
        }
        if !k0.is_finite() || k0 == 0.0 {
            return Err(ProjectionError::InvalidScale);
        }
        Ok(Self {
            a: ell.a(),
            k0,
            lam0: lon0_deg.to_radians(),
            x0,
            y0,
            model: Model::Ellipsoid { e: ell.e() },
        })
    }

    /// Degrés vers mètres.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`], [`ProjectionError::InvalidLatitude`],
    /// ou [`ProjectionError::OutsideDomain`] à moins de `1e-10` rad d'un pôle.
    pub fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<Projected, ProjectionError> {
        self.forward_rad(lon_deg.to_radians(), lat_deg.to_radians())
    }

    /// Radians absolus vers mètres.
    ///
    /// # Errors
    ///
    /// Mêmes cas que [`Self::forward`].
    pub fn forward_rad(&self, lon_rad: f64, lat_rad: f64) -> Result<Projected, ProjectionError> {
        if !lon_rad.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        let half = std::f64::consts::FRAC_PI_2;
        if !lat_rad.is_finite() || lat_rad.abs() > half + 1e-12 {
            return Err(if lat_rad.is_finite() {
                ProjectionError::InvalidLatitude
            } else {
                ProjectionError::NonFinite
            });
        }
        let phi = lat_rad.clamp(-half, half);
        let lam = adjlon(adjlon(lon_rad) - self.lam0);
        let (x, y) = self.fwd_unit(lam, phi);
        if !x.is_finite() || !y.is_finite() {
            return Err(ProjectionError::OutsideDomain);
        }
        Ok(Projected {
            east: self.a * x + self.x0,
            north: self.a * y + self.y0,
        })
    }

    /// Mètres vers degrés.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] si une coordonnée ne l'est pas.
    pub fn inverse(&self, xy: Projected) -> Result<Geographic, ProjectionError> {
        if !xy.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        let (lam, phi) = self.inv_unit((xy.east - self.x0) / self.a, (xy.north - self.y0) / self.a);
        Ok(Geographic {
            lon_deg: adjlon(lam + self.lam0).to_degrees(),
            lat_deg: phi.to_degrees(),
        })
    }

    fn fwd_unit(&self, lam: f64, phi: f64) -> (f64, f64) {
        let y = match self.model {
            Model::Sphere => phi.tan().asinh(),
            Model::Ellipsoid { e } => -tsfn(phi, phi.sin(), e).ln(),
        };
        (self.k0 * lam, self.k0 * y)
    }

    fn inv_unit(&self, x: f64, y: f64) -> (f64, f64) {
        let lam = x / self.k0;
        let north = y / self.k0;
        let phi = match self.model {
            Model::Sphere => north.sinh().atan(),
            Model::Ellipsoid { e } => phi2((-north).exp(), e),
        };
        (lam, phi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_meters(got: Projected, east: f64, north: f64) {
        assert!((got.east - east).abs() < 1e-4, "east {}", got.east);
        assert!((got.north - north).abs() < 1e-4, "north {}", got.north);
    }

    #[test]
    fn proj_web_origin_is_zero() {
        let xy = Mercator::web().forward(0.0, 0.0).expect("origin");
        assert_meters(xy, 0.0, 0.0);
    }

    #[test]
    fn proj_web_samples() {
        let m = Mercator::web();
        let cases = [
            (2.2945, 48.858_222, 255_422.571_625, 6_250_838.784_189),
            (3.0, 46.5, 333_958.472_380, 5_860_839.829_948),
            (-4.0, 48.0, -445_277.963_173, 6_106_854.834_885),
        ];
        for (lon, lat, east, north) in cases {
            assert_meters(m.forward(lon, lat).expect("fwd"), east, north);
        }
    }

    #[test]
    fn proj_ellipsoid_differs_in_northing() {
        let m = Mercator::try_ellipsoid(Ellipsoid::wgs84(), 0.0, 1.0, 0.0, 0.0).expect("merc");
        let xy = m.forward(2.2945, 48.858_222).expect("fwd");
        assert_meters(xy, 255_422.571_625, 6_218_643.069_815);
        let high = m.forward(10.0, 60.0).expect("60");
        assert_meters(high, 1_113_194.907_933, 8_362_698.548_501);
    }

    #[test]
    fn roundtrip() {
        for merc in [
            Mercator::web(),
            Mercator::try_ellipsoid(Ellipsoid::wgs84(), 0.0, 1.0, 0.0, 0.0).expect("ell"),
        ] {
            for (lon, lat) in [(2.2945, 48.858_222), (-4.0, 48.0), (10.0, 60.0)] {
                let back = merc
                    .inverse(merc.forward(lon, lat).expect("fwd"))
                    .expect("inv");
                assert!((back.lon_deg - lon).abs() < 1e-9, "{back:?}");
                assert!((back.lat_deg - lat).abs() < 1e-9, "{back:?}");
            }
        }
    }

    #[test]
    fn pole_latitude_is_a_large_finite_northing() {
        let xy = Mercator::web()
            .forward(0.0, 90.0)
            .expect("PROJ 9.9 merc keeps tan(pi/2) finite");
        assert!(xy.north.abs() > 1.0e8, "{}", xy.north);
        assert!(xy.north.is_finite());
    }
}
