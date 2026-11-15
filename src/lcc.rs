//! Lambert conforme conique ellipsoïdal.
//!
//! Portage de `src/projections/lcc.cpp` et de `pj_tsfn` / `pj_msfn` / `pj_phi2`
//! (PROJ 9.9). Lambert-93 est ce cône sur GRS80. Voir `docs/PORTING-lcc.md`.

mod batch;

use crate::angle::adjlon;
use crate::{Ellipsoid, Geographic, Projected, ProjectionError};

pub(crate) const EPS10: f64 = 1e-10;

/// Paramètres du cône, en degrés et en mètres. Un struct plutôt que huit
/// arguments positionnels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cone {
    /// Longitude de l'origine, degrés.
    pub lon0_deg: f64,
    /// Latitude de l'origine, degrés.
    pub lat0_deg: f64,
    /// Premier parallèle standard, degrés.
    pub lat1_deg: f64,
    /// Second parallèle standard, degrés. Égal au premier : cône tangent.
    pub lat2_deg: f64,
    /// Faux est, mètres.
    pub x0: f64,
    /// Faux nord, mètres.
    pub y0: f64,
    /// Facteur d'échelle. Lambert-93 utilise 1.
    pub k0: f64,
}

impl Cone {
    /// Parallèles, origine et faux origine de Lambert-93.
    #[must_use]
    pub fn lambert93() -> Self {
        Self {
            lon0_deg: 3.0,
            lat0_deg: 46.5,
            lat1_deg: 49.0,
            lat2_deg: 44.0,
            x0: 700_000.0,
            y0: 6_600_000.0,
            k0: 1.0,
        }
    }
}

/// Lambert conforme conique préparé.
#[derive(Clone, Debug)]
pub struct Lcc {
    a: f64,
    ra: f64,
    e: f64,
    lam0: f64,
    k0: f64,
    x0: f64,
    y0: f64,
    n: f64,
    rho0: f64,
    c: f64,
}

impl Lcc {
    /// Parallèles `lat1_deg` et `lat2_deg`. S'ils sont égaux, le cône est tangent.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::InvalidParallel`] si un parallèle est à ±90° ou si
    /// leur somme est nulle, [`ProjectionError::DegenerateCone`] si la
    /// constante du cône s'annule, [`ProjectionError::InvalidScale`] si `k0`
    /// est nul, [`ProjectionError::NonFinite`] si un paramètre ne l'est pas.
    pub fn try_new(ell: Ellipsoid, cone: Cone) -> Result<Self, ProjectionError> {
        let Cone {
            lon0_deg,
            lat0_deg,
            lat1_deg,
            lat2_deg,
            x0,
            y0,
            k0,
        } = cone;
        if ![lon0_deg, lat0_deg, lat1_deg, lat2_deg, x0, y0]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(ProjectionError::NonFinite);
        }
        if !k0.is_finite() || k0 == 0.0 {
            return Err(ProjectionError::InvalidScale);
        }
        // PROJ rejects an eccentricity this close to 1 inside the cone setup
        // (the oss-fuzz latitude that divides by zero). A real ellipsoid is far
        // from that.
        if ell.es() > 0.999 {
            return Err(ProjectionError::DegenerateCone);
        }
        let phi1 = lat1_deg.to_radians();
        let phi2 = lat2_deg.to_radians();
        let phi0 = lat0_deg.to_radians();
        let half = std::f64::consts::FRAC_PI_2;
        if (phi1 + phi2).abs() < EPS10 {
            return Err(ProjectionError::InvalidParallel);
        }
        let sin_phi = phi1.sin();
        let cos_phi = phi1.cos();
        if cos_phi.abs() < EPS10 || phi1.abs() >= half {
            return Err(ProjectionError::InvalidParallel);
        }
        if phi2.cos().abs() < EPS10 || phi2.abs() >= half {
            return Err(ProjectionError::InvalidParallel);
        }

        let secant = (phi1 - phi2).abs() >= EPS10;
        let mut n = sin_phi;
        let m1 = msfn(sin_phi, cos_phi, ell.es());
        let ml1 = tsfn(phi1, sin_phi, ell.e());
        if secant {
            let sin_phi2 = phi2.sin();
            n = (m1 / msfn(sin_phi2, phi2.cos(), ell.es())).ln();
            if n == 0.0 {
                return Err(ProjectionError::DegenerateCone);
            }
            let ml2 = tsfn(phi2, sin_phi2, ell.e());
            let denom = (ml1 / ml2).ln();
            if denom == 0.0 {
                return Err(ProjectionError::DegenerateCone);
            }
            n /= denom;
        }
        let mut rho0 = m1 * ml1.powf(-n) / n;
        let c = rho0;
        if (phi0.abs() - half).abs() < EPS10 {
            rho0 = 0.0;
        } else {
            rho0 *= tsfn(phi0, phi0.sin(), ell.e()).powf(n);
        }
        if !n.is_finite() || n == 0.0 || !rho0.is_finite() || !c.is_finite() {
            return Err(ProjectionError::DegenerateCone);
        }
        Ok(Self {
            a: ell.a(),
            ra: ell.ra(),
            e: ell.e(),
            lam0: lon0_deg.to_radians(),
            k0,
            x0,
            y0,
            n,
            rho0,
            c,
        })
    }

    /// RGF93 / Lambert-93. GRS80, parallèles 44° et 49°, origine 46,5°N 3°E.
    ///
    /// # Panics
    ///
    /// Panique si ces paramètres officiels ne passent plus [`Self::try_new`].
    /// Ce serait un bug du crate, pas une entrée utilisateur.
    #[must_use]
    pub fn lambert93() -> Self {
        Self::try_new(Ellipsoid::grs80(), Cone::lambert93())
            .expect("Lambert-93 standard parallels and origin are valid")
    }

    /// Degrés vers mètres.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`], [`ProjectionError::InvalidLatitude`],
    /// ou [`ProjectionError::OutsideDomain`] au pôle opposé au cône.
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
        let (x, y) = self.fwd_unit(lam, phi)?;
        Ok(Projected {
            east: self.a * x + self.x0,
            north: self.a * y + self.y0,
        })
    }

    /// Convertit un lot de degrés en mètres. Un point refusé par [`Self::forward`] est écrit
    /// `NaN`.
    ///
    /// Avec AVX2, quatre points sont calculés à la fois ; l'écart avec [`Self::forward`] vient
    /// des fonctions mathématiques vectorielles.
    ///
    /// # Panics
    ///
    /// Panique si les quatre tranches n'ont pas la même longueur.
    pub fn forward_batch(
        &self,
        lon_deg: &[f64],
        lat_deg: &[f64],
        east: &mut [f64],
        north: &mut [f64],
    ) {
        crate::simd::run(&batch::Forward(self), lon_deg, lat_deg, east, north);
    }

    /// Convertit un lot de mètres en degrés (longitude, latitude). Un point refusé par
    /// [`Self::inverse`] est écrit `NaN`.
    ///
    /// Même contrat que [`Self::forward_batch`] pour l'écart avec [`Self::inverse`].
    ///
    /// # Panics
    ///
    /// Panique si les quatre tranches n'ont pas la même longueur.
    pub fn inverse_batch(
        &self,
        east: &[f64],
        north: &[f64],
        lon_deg: &mut [f64],
        lat_deg: &mut [f64],
    ) {
        crate::simd::run(&batch::Inverse(self), east, north, lon_deg, lat_deg);
    }

    /// Mètres vers degrés.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] ou [`ProjectionError::OutsideDomain`].
    pub fn inverse(&self, xy: Projected) -> Result<Geographic, ProjectionError> {
        if !xy.is_finite() {
            return Err(ProjectionError::NonFinite);
        }
        let (lam, phi) = self.inv_unit(
            (xy.east - self.x0) * self.ra,
            (xy.north - self.y0) * self.ra,
        )?;
        Ok(Geographic {
            lon_deg: adjlon(lam + self.lam0).to_degrees(),
            lat_deg: phi.to_degrees(),
        })
    }

    fn fwd_unit(&self, lam: f64, phi: f64) -> Result<(f64, f64), ProjectionError> {
        let half = std::f64::consts::FRAC_PI_2;
        let rho = if (phi.abs() - half).abs() < EPS10 {
            if phi * self.n <= 0.0 {
                return Err(ProjectionError::OutsideDomain);
            }
            0.0
        } else {
            let sin_phi = phi.sin();
            self.c * tsfn(phi, sin_phi, self.e).powf(self.n)
        };
        let lam_n = lam * self.n;
        Ok((
            self.k0 * (rho * lam_n.sin()),
            self.k0 * (self.rho0 - rho * lam_n.cos()),
        ))
    }

    fn inv_unit(&self, mut x: f64, mut y: f64) -> Result<(f64, f64), ProjectionError> {
        x /= self.k0;
        y /= self.k0;
        y = self.rho0 - y;
        let mut rho = x.hypot(y);
        if rho == 0.0 {
            let phi = if self.n > 0.0 {
                std::f64::consts::FRAC_PI_2
            } else {
                -std::f64::consts::FRAC_PI_2
            };
            return Ok((0.0, phi));
        }
        if self.n < 0.0 {
            rho = -rho;
            x = -x;
            y = -y;
        }
        let phi = phi2((rho / self.c).powf(1.0 / self.n), self.e);
        if !phi.is_finite() {
            return Err(ProjectionError::OutsideDomain);
        }
        Ok((x.atan2(y) / self.n, phi))
    }
}

fn msfn(sin_phi: f64, cos_phi: f64, es: f64) -> f64 {
    cos_phi / (1.0 - es * sin_phi * sin_phi).sqrt()
}

pub(crate) fn tsfn(phi: f64, sin_phi: f64, e: f64) -> f64 {
    let cos_phi = phi.cos();
    (e * (e * sin_phi).atanh()).exp()
        * if sin_phi > 0.0 {
            cos_phi / (1.0 + sin_phi)
        } else {
            (1.0 - sin_phi) / cos_phi
        }
}

pub(crate) fn phi2(ts: f64, e: f64) -> f64 {
    sinhpsi2tanphi((1.0 / ts - ts) / 2.0, e).atan()
}

/// Newton de `GeographicLib`, recopié de `pj_sinhpsi2tanphi` (PROJ 9.9).
#[expect(
    clippy::similar_names,
    reason = "tau, taup and tau1 are the names in PROJ pj_sinhpsi2tanphi"
)]
#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "PROJ negates the comparisons so a NaN falls through unchanged"
)]
fn sinhpsi2tanphi(taup: f64, e: f64) -> f64 {
    const NUMIT: i32 = 5;
    let rooteps = f64::EPSILON.sqrt();
    let tol = rooteps / 10.0;
    let tmax = 2.0 / rooteps;
    let e2m = 1.0 - e * e;
    let stol = tol * 1.0_f64.max(taup.abs());
    let mut tau = if taup.abs() > 70.0 {
        taup * (e * e.atanh()).exp()
    } else {
        taup / e2m
    };
    if !(tau.abs() < tmax) {
        return tau;
    }
    let mut i = NUMIT;
    while i > 0 {
        i -= 1;
        let tau1 = (1.0 + tau * tau).sqrt();
        let sig = (e * (e * tau / tau1).atanh()).sinh();
        let taupa = (1.0 + sig * sig).sqrt() * tau - sig * tau1;
        let dtau = ((taup - taupa) * (1.0 + e2m * (tau * tau)))
            / (e2m * tau1 * (1.0 + taupa * taupa).sqrt());
        tau += dtau;
        if !(dtau.abs() >= stol) {
            break;
        }
    }
    tau
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_meters(got: Projected, east: f64, north: f64) {
        assert!((got.east - east).abs() < 1e-6, "east {}", got.east);
        assert!((got.north - north).abs() < 1e-6, "north {}", got.north);
    }

    #[test]
    fn ossfuzz_flat_ellipsoid_is_rejected() {
        let a = 6_378_137.0;
        let b = 0.2;
        let f = (a - b) / a;
        let ell = Ellipsoid::try_from_a_es(a, f * (2.0 - f)).expect("axes");
        let cone = Cone {
            lon0_deg: 0.0,
            lat0_deg: 0.0,
            lat1_deg: 2.0 + 32.0 / 60.0,
            lat2_deg: 0.0,
            x0: 0.0,
            y0: 0.0,
            k0: 1.0,
        };
        let built = Lcc::try_new(ell, cone);
        assert!(built.is_err(), "{built:?}");
    }

    #[test]
    fn proj_origin_is_false_origin() {
        let xy = Lcc::lambert93().forward(3.0, 46.5).expect("origin");
        assert_meters(xy, 700_000.0, 6_600_000.0);
    }

    #[test]
    fn proj_eiffel_and_samples() {
        let l = Lcc::lambert93();
        let cases = [
            (2.2945, 48.858_222, 648_237.124_710_83, 6_862_251.889_600_65),
            (-4.0, 48.0, 178_626.396_944_32, 6_789_748.646_358_65),
            (7.5, 43.5, 1_063_897.404_187_55, 6_277_140.292_306_08),
            (1.5, 50.5, 593_420.223_660_56, 7_045_752.197_208_37),
        ];
        for (lon, lat, east, north) in cases {
            assert_meters(l.forward(lon, lat).expect("forward"), east, north);
        }
    }

    #[test]
    fn roundtrip_france() {
        let l = Lcc::lambert93();
        for (lon, lat) in [(2.2945, 48.858_222), (-4.0, 48.0), (7.5, 43.5), (3.0, 46.5)] {
            let back = l.inverse(l.forward(lon, lat).expect("fwd")).expect("inv");
            assert!((back.lon_deg - lon).abs() < 1e-11, "{back:?}");
            assert!((back.lat_deg - lat).abs() < 1e-11, "{back:?}");
        }
    }

    #[test]
    fn batch_matches_scalar_within_ten_nanometres() {
        let l = Lcc::lambert93();
        let lon = [2.2945, 3.0, -4.0, 7.5, 1.5, 8.0, 0.0];
        let lat = [48.858_222, 46.5, 48.0, 43.5, 50.5, 42.0, 44.0];
        let mut east = [0.0; 7];
        let mut north = [0.0; 7];
        l.forward_batch(&lon, &lat, &mut east, &mut north);
        for i in 0..lon.len() {
            let xy = l.forward(lon[i], lat[i]).expect("scalar");
            assert!((east[i] - xy.east).abs() < 1e-8, "{i} east");
            assert!((north[i] - xy.north).abs() < 1e-8, "{i} north");
        }
    }

    #[test]
    fn rejects_parallel_sum_zero() {
        let err = Lcc::try_new(
            Ellipsoid::grs80(),
            Cone {
                lon0_deg: 3.0,
                lat0_deg: 0.0,
                lat1_deg: 45.0,
                lat2_deg: -45.0,
                x0: 0.0,
                y0: 0.0,
                k0: 1.0,
            },
        )
        .expect_err("sum");
        assert_eq!(err, ProjectionError::InvalidParallel);
    }
}
