//! Ellipsoïde de révolution, comme `+a` et `+rf` dans PROJ.

use crate::EllipsoidError;

/// Ellipsoïde. Les champs dérivés (`es`, `e`, `n`, `ra`) restent privés :
/// les modifier à la main casserait l'invariant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoid {
    a: f64,
    f: f64,
    es: f64,
    e: f64,
    n: f64,
    ra: f64,
}

impl Ellipsoid {
    /// Construit un ellipsoïde.
    ///
    /// `inv_f` est l'inverse de l'aplatissement, le paramètre `+rf` de PROJ.
    ///
    /// # Errors
    ///
    /// Retourne [`EllipsoidError`] si `a` n'est pas fini et strictement
    /// positif, ou si `inv_f` n'est pas fini et strictement supérieur à 1.
    pub fn try_new(a: f64, inv_f: f64) -> Result<Self, EllipsoidError> {
        if !a.is_finite() || !inv_f.is_finite() {
            return Err(EllipsoidError::NonFiniteParameter);
        }
        if a <= 0.0 {
            return Err(EllipsoidError::NonPositiveSemiMajorAxis);
        }
        if inv_f <= 1.0 {
            return Err(EllipsoidError::InvalidFlattening);
        }
        let f = 1.0 / inv_f;
        let es = f * (2.0 - f);
        Ok(Self {
            a,
            f,
            es,
            e: es.sqrt(),
            n: f / (2.0 - f),
            ra: 1.0 / a,
        })
    }

    /// WGS 84, `a = 6378137`, `rf = 298.257223563`.
    ///
    /// # Panics
    ///
    /// Panique si ces constantes ne passent plus [`Self::try_new`]. Ce sont
    /// des littéraux valides : l'échec serait un bug du crate.
    #[must_use]
    pub fn wgs84() -> Self {
        Self::try_new(6_378_137.0, 298.257_223_563)
            .expect("WGS84 semi-major axis and inverse flattening are valid")
    }

    /// GRS 80, `a = 6378137`, `rf = 298.257222101`.
    ///
    /// # Panics
    ///
    /// Panique si ces constantes ne passent plus [`Self::try_new`].
    #[must_use]
    pub fn grs80() -> Self {
        Self::try_new(6_378_137.0, 298.257_222_101)
            .expect("GRS80 semi-major axis and inverse flattening are valid")
    }

    /// Demi-grand axe, mètres.
    #[must_use]
    pub fn a(self) -> f64 {
        self.a
    }

    /// Aplatissement `f = 1 / rf`.
    #[must_use]
    pub fn flattening(self) -> f64 {
        self.f
    }

    /// Carré de l'excentricité, `e² = f (2 − f)`.
    #[must_use]
    pub fn es(self) -> f64 {
        self.es
    }

    /// Excentricité.
    #[must_use]
    pub fn e(self) -> f64 {
        self.e
    }

    /// Aplatissement troisième, `n = f / (2 − f)`. Ce n'est pas `e²`.
    #[must_use]
    pub fn n(self) -> f64 {
        self.n
    }

    /// Inverse du demi-grand axe.
    #[must_use]
    pub fn ra(self) -> f64 {
        self.ra
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgs84_matches_proj_parameters() {
        let e = Ellipsoid::wgs84();
        assert!((e.a() - 6_378_137.0).abs() < 1e-9);
        let f = 1.0 / 298.257_223_563;
        assert!((e.flattening() - f).abs() < 1e-18);
        assert!((e.es() - f * (2.0 - f)).abs() < 1e-18);
        assert!((e.n() - f / (2.0 - f)).abs() < 1e-18);
        assert!((e.ra() * e.a() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn grs80_differs_from_wgs84_in_flattening() {
        let w = Ellipsoid::wgs84();
        let g = Ellipsoid::grs80();
        assert!((w.a() - g.a()).abs() < 1e-9);
        assert!((w.flattening() - g.flattening()).abs() > 1e-15);
    }

    #[test]
    fn rejects_bad_parameters() {
        assert!(Ellipsoid::try_new(0.0, 298.0).is_err());
        assert!(Ellipsoid::try_new(-1.0, 298.0).is_err());
        assert!(Ellipsoid::try_new(f64::NAN, 298.0).is_err());
        assert!(Ellipsoid::try_new(6_378_137.0, 1.0).is_err());
        assert!(Ellipsoid::try_new(6_378_137.0, f64::INFINITY).is_err());
    }
}
