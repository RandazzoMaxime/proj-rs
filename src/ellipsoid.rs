//! Ellipsoïde de révolution, comme `+a` et `+rf` dans PROJ.

use crate::EllipsoidError;

/// Ellipsoïdes nommés de PROJ, dans l'ordre de `pj_ellps` (`ellps.cpp`) : identifiant,
/// demi-grand axe, puis `rf` quand le quatrième champ vaut `true`, `b` sinon.
/// `lookForProjWellKnownEllps` prend la première ligne qui correspond.
pub(crate) const PJ_ELLPS: &[(&str, f64, f64, bool)] = &[
    ("MERIT", 6_378_137.0, 298.257, true),
    ("SGS85", 6_378_136.0, 298.257, true),
    ("GRS80", 6_378_137.0, 298.257_222_101, true),
    ("IAU76", 6_378_140.0, 298.257, true),
    ("airy", 6_377_563.396, 299.324_964_6, true),
    ("APL4.9", 6_378_137.0, 298.25, true),
    ("NWL9D", 6_378_145.0, 298.25, true),
    ("mod_airy", 6_377_340.189, 6_356_034.446, false),
    ("andrae", 6_377_104.43, 300.0, true),
    ("danish", 6_377_019.256_3, 300.0, true),
    ("aust_SA", 6_378_160.0, 298.25, true),
    ("GRS67", 6_378_160.0, 298.247_167_427, true),
    ("GSK2011", 6_378_136.5, 298.256_415_1, true),
    ("bessel", 6_377_397.155, 299.152_812_8, true),
    ("bess_nam", 6_377_483.865, 299.152_812_8, true),
    ("clrk66", 6_378_206.4, 6_356_583.8, false),
    ("clrk80", 6_378_249.145, 293.4663, true),
    ("clrk80ign", 6_378_249.2, 293.466_021_293_626_9, true),
    ("CPM", 6_375_738.7, 334.29, true),
    ("delmbr", 6_376_428.0, 311.5, true),
    ("engelis", 6_378_136.05, 298.2566, true),
    ("evrst30", 6_377_276.345, 300.8017, true),
    ("evrst48", 6_377_304.063, 300.8017, true),
    ("evrst56", 6_377_301.243, 300.8017, true),
    ("evrst69", 6_377_295.664, 300.8017, true),
    ("evrstSS", 6_377_298.556, 300.8017, true),
    ("fschr60", 6_378_166.0, 298.3, true),
    ("fschr60m", 6_378_155.0, 298.3, true),
    ("fschr68", 6_378_150.0, 298.3, true),
    ("helmert", 6_378_200.0, 298.3, true),
    ("hough", 6_378_270.0, 297.0, true),
    ("intl", 6_378_388.0, 297.0, true),
    ("krass", 6_378_245.0, 298.3, true),
    ("kaula", 6_378_163.0, 298.24, true),
    ("lerch", 6_378_139.0, 298.257, true),
    ("mprts", 6_397_300.0, 191.0, true),
    ("new_intl", 6_378_157.5, 6_356_772.2, false),
    ("plessis", 6_376_523.0, 6_355_863.0, false),
    ("PZ90", 6_378_136.0, 298.257_84, true),
    ("SEasia", 6_378_155.0, 6_356_773.320_5, false),
    ("walbeck", 6_376_896.0, 6_355_834.846_7, false),
    ("WGS60", 6_378_165.0, 298.3, true),
    ("WGS66", 6_378_145.0, 298.25, true),
    ("WGS72", 6_378_135.0, 298.26, true),
    ("WGS84", 6_378_137.0, 298.257_223_563, true),
    ("sphere", 6_370_997.0, 6_370_997.0, false),
];

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

    /// `1 − e²`.
    #[must_use]
    pub fn one_es(self) -> f64 {
        1.0 - self.es
    }

    /// Demi-petit axe, mètres.
    #[must_use]
    pub fn b(self) -> f64 {
        self.a * (1.0 - self.f)
    }

    /// Carré de la seconde excentricité, `e'² = e² / (1 − e²)`.
    #[must_use]
    pub fn e2s(self) -> f64 {
        self.es / (1.0 - self.es)
    }

    /// Sphère de rayon `radius`, ou ellipsoïde défini par `a` et `e²`.
    ///
    /// `es == 0` est une sphère. C'est le cas `+R` et `+a` sans aplatissement
    /// de PROJ, que [`Self::try_new`] refuse parce qu'`rf` n'est pas fini.
    ///
    /// # Errors
    ///
    /// [`EllipsoidError::NonFiniteParameter`] si une entrée n'est pas finie,
    /// [`EllipsoidError::NonPositiveSemiMajorAxis`] si `a` n'est pas strictement
    /// positif, [`EllipsoidError::InvalidFlattening`] si `es` n'est pas dans
    /// `[0, 1)`.
    pub fn try_from_a_es(a: f64, es: f64) -> Result<Self, EllipsoidError> {
        if !a.is_finite() || !es.is_finite() {
            return Err(EllipsoidError::NonFiniteParameter);
        }
        if a <= 0.0 {
            return Err(EllipsoidError::NonPositiveSemiMajorAxis);
        }
        if !(0.0..1.0).contains(&es) {
            return Err(EllipsoidError::InvalidFlattening);
        }
        let f = if es == 0.0 {
            0.0
        } else {
            1.0 - (1.0 - es).sqrt()
        };
        let n = if f == 0.0 { 0.0 } else { f / (2.0 - f) };
        Ok(Self {
            a,
            f,
            es,
            e: es.sqrt(),
            n,
            ra: 1.0 / a,
        })
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
