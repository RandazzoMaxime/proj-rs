//! Transverse Mercator exact (Poder / Engsager), ordre 6.
//!
//! Portage du comportement de `src/projections/tmerc.cpp` (PROJ 9.9), algorithme
//! `PODER_ENGSAGER`. C'est le défaut `tmerc_default_algo` et donc le chemin UTM.
//! La correspondance avec le C est dans `docs/PORTING-tmerc.md`.
//!
//! L'algorithme approché d'Evenden / Snyder (`+approx`) est dans `tmerc_approx`.

mod batch;

use crate::angle::adjlon;
use crate::projmath::{
    auxlat_coeffs, auxlat_convert, auxlat_convert_sincos, rectifying_radius, AuxLat,
};
use crate::{Ellipsoid, Geographic, Projected, ProjectionError};

mod portable;

const ORDER: usize = 6;
/// Seuil `|Ce|` de PROJ, au-delà duquel le point est hors domaine (~150°).
const CE_LIMIT: f64 = 2.623_395_162_778;

/// Hémisphère UTM. Le sud pose le faux nord à 10 000 000 m.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hemisphere {
    /// Faux nord nul.
    North,
    /// Faux nord de 10 000 000 m.
    South,
}

/// Zone UTM de 1 à 60.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtmZone(u8);

impl UtmZone {
    /// Construit une zone.
    ///
    /// # Errors
    ///
    /// Retourne [`ProjectionError::InvalidZone`] si `zone` n'est pas dans `1..=60`.
    pub fn try_new(zone: u32) -> Result<Self, ProjectionError> {
        let Ok(zone) = u8::try_from(zone) else {
            return Err(ProjectionError::InvalidZone);
        };
        if !(1..=60).contains(&zone) {
            return Err(ProjectionError::InvalidZone);
        }
        Ok(Self(zone))
    }

    /// Numéro de zone, de 1 à 60.
    #[must_use]
    pub fn number(self) -> u32 {
        u32::from(self.0)
    }
}

/// Méridien central de la zone, en radians, comme `PJ_PROJECTION(utm)` de PROJ :
/// `(zone + 0,5) π / 30 − π` avec la zone à partir de 0. Passer par des degrés changerait les bits.
pub(crate) fn utm_central_meridian(zone: UtmZone) -> f64 {
    (f64::from(zone.0) - 0.5) * std::f64::consts::PI / 30.0 - std::f64::consts::PI
}

/// Transverse Mercator préparé (coefficients de la série déjà calculés).
#[derive(Clone, Debug)]
pub struct Tmerc {
    a: f64,
    ra: f64,
    lam0: f64,
    x0: f64,
    y0: f64,
    qn: f64,
    zb: f64,
    cgb: [f64; ORDER],
    cbg: [f64; ORDER],
    utg: [f64; ORDER],
    gtu: [f64; ORDER],
}

impl Tmerc {
    /// Projection générale.
    ///
    /// `lon0_deg` et `lat0_deg` sont l'origine. `k0` est le facteur d'échelle
    /// au méridien central. `x0` et `y0` sont le faux est et le faux nord, en mètres.
    ///
    /// # Errors
    ///
    /// Retourne [`ProjectionError::NonFinite`] si un angle ou un décalage n'est
    /// pas fini, et [`ProjectionError::InvalidScale`] si `k0` est nul ou non fini.
    pub fn try_new(
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
        if !k0.is_finite() || k0 == 0.0 {
            return Err(ProjectionError::InvalidScale);
        }
        Ok(Self::from_radians(
            ell,
            lon0_deg.to_radians(),
            lat0_deg.to_radians(),
            k0,
            x0,
            y0,
        ))
    }

    /// Même projection, origine en radians. Le cadre de `engine` (`fwd_prepare`,
    /// `fwd_finalize`) porte le méridien central et les faux, donc ce noyau les reçoit à zéro.
    pub(crate) fn from_radians(
        ell: Ellipsoid,
        lam0: f64,
        phi0: f64,
        k0: f64,
        x0: f64,
        y0: f64,
    ) -> Self {
        let (qn, zb, cgb, cbg, utg, gtu) = engsager(ell.n(), k0, phi0);
        Self {
            a: ell.a(),
            ra: ell.ra(),
            lam0,
            x0,
            y0,
            qn,
            zb,
            cgb,
            cbg,
            utg,
            gtu,
        }
    }

    /// UTM : `k0 = 0.9996`, faux est 500 000 m, méridien central de la zone.
    ///
    /// # Errors
    ///
    /// Propage l'erreur de [`Self::try_new`]. Avec un [`UtmZone`] valide et un
    /// [`Ellipsoid`] fini, cet appel réussit.
    pub fn utm(
        zone: UtmZone,
        hemisphere: Hemisphere,
        ell: Ellipsoid,
    ) -> Result<Self, ProjectionError> {
        let lam0 = utm_central_meridian(zone);
        let y0 = match hemisphere {
            Hemisphere::North => 0.0,
            Hemisphere::South => 10_000_000.0,
        };
        Ok(Self::from_radians(ell, lam0, 0.0, 0.9996, 500_000.0, y0))
    }

    /// Degrés vers mètres.
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NonFinite`] si une entrée n'est pas finie,
    /// [`ProjectionError::InvalidLatitude`] hors de [−90°, 90°],
    /// [`ProjectionError::OutsideDomain`] si PROJ aurait renvoyé `HUGE_VAL`.
    pub fn forward(&self, lon_deg: f64, lat_deg: f64) -> Result<Projected, ProjectionError> {
        self.forward_rad(lon_deg.to_radians(), lat_deg.to_radians())
    }

    /// Radians absolus vers mètres. Même contrat que `+proj=utm` passé à
    /// `proj_create` : l'entrée est en radians, la sortie en mètres.
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

    #[expect(
        clippy::similar_names,
        reason = "Cn and Ce are the paired northing/easting terms in PROJ exact_e_fwd"
    )]
    pub(crate) fn fwd_unit(&self, lam: f64, phi: f64) -> Result<(f64, f64), ProjectionError> {
        let mut cn = auxlat_convert(phi, &self.cbg);
        let sin_cn = cn.sin();
        let cos_cn = cn.cos();
        let sin_ce = lam.sin();
        let cos_ce = lam.cos();
        let cos_cn_cos_ce = cos_cn * cos_ce;
        cn = sin_cn.atan2(cos_cn_cos_ce);

        let inv_denom = 1.0 / sin_cn.hypot(cos_cn_cos_ce);
        let tan_ce = sin_ce * cos_cn * inv_denom;
        let mut ce = tan_ce.asinh();

        let two_inv = 2.0 * inv_denom;
        let two_inv_sq = two_inv * inv_denom;
        let tmp_r = cos_cn_cos_ce * two_inv_sq;
        let sin_arg_r = sin_cn * tmp_r;
        let cos_arg_r = cos_cn_cos_ce * tmp_r - 1.0;
        let sinh_arg_i = tan_ce * two_inv;
        let cosh_arg_i = two_inv_sq - 1.0;

        let (d_cn, d_ce) =
            clenshaw_complex(&self.gtu, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        cn += d_cn;
        ce += d_ce;
        if ce.abs() <= CE_LIMIT {
            Ok((self.qn * ce, self.qn * cn + self.zb))
        } else {
            Err(ProjectionError::OutsideDomain)
        }
    }

    #[expect(
        clippy::similar_names,
        reason = "Cn and Ce are the paired terms in PROJ exact_e_inv"
    )]
    pub(crate) fn inv_unit(&self, x: f64, y: f64) -> Result<(f64, f64), ProjectionError> {
        let mut cn = (y - self.zb) / self.qn;
        let mut ce = x / self.qn;
        if ce.abs() > CE_LIMIT {
            return Err(ProjectionError::OutsideDomain);
        }
        let sin_arg_r = (2.0 * cn).sin();
        let cos_arg_r = (2.0 * cn).cos();
        let exp_2_ce = (2.0 * ce).exp();
        let half_inv = 0.5 / exp_2_ce;
        let sinh_arg_i = 0.5 * exp_2_ce - half_inv;
        let cosh_arg_i = 0.5 * exp_2_ce + half_inv;
        let (d_cn, d_ce) =
            clenshaw_complex(&self.utg, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
        cn += d_cn;
        ce += d_ce;

        let sin_cn = cn.sin();
        let cos_cn = cn.cos();
        let sinh_ce = ce.sinh();
        ce = sinh_ce.atan2(cos_cn);
        let modulus = sinh_ce.hypot(cos_cn);
        let rr = sin_cn.hypot(modulus);
        let cn_geo = sin_cn.atan2(modulus);
        let phi = auxlat_convert_sincos(cn_geo, sin_cn / rr, modulus / rr, &self.cgb);
        Ok((ce, phi))
    }

    /// Convertit un lot de degrés en mètres. Un point hors domaine est écrit `NaN` (le
    /// `HUGE_VAL` de PROJ) et le lot continue.
    ///
    /// Avec AVX2, quatre points sont calculés à la fois ; l'écart avec [`Self::forward`] vient
    /// des fonctions mathématiques vectorielles, quelques dizaines de nanomètres au plus.
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
        let n = lon_deg.len();
        assert_eq!(
            lat_deg.len(),
            n,
            "latitude length {0} != {n}",
            lat_deg.len()
        );
        assert_eq!(east.len(), n, "easting length {0} != {n}", east.len());
        assert_eq!(north.len(), n, "northing length {0} != {n}", north.len());
        if crate::simd::batch_path() == "avx2" {
            crate::simd::run(&batch::Forward(self), lon_deg, lat_deg, east, north);
            return;
        }
        // Off AVX2, the four-lane loop: bit-identical to `forward`, and faster than one point
        // at a time (`docs/PORTING-batch-path.md`).
        self.forward_batch_portable(lon_deg, lat_deg, east, north);
    }

    /// Comme [`Self::forward_batch`], pour des radians absolus : le contrat de
    /// [`Self::forward_rad`].
    ///
    /// # Panics
    ///
    /// Panique si les quatre tranches n'ont pas la même longueur.
    pub fn forward_rad_batch(
        &self,
        lon_rad: &[f64],
        lat_rad: &[f64],
        east: &mut [f64],
        north: &mut [f64],
    ) {
        crate::simd::run(&batch::ForwardRad(self), lon_rad, lat_rad, east, north);
    }

    /// Convertit un lot de mètres en degrés (longitude, latitude). Un point hors domaine est
    /// écrit `NaN` et le lot continue.
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
}

/// Coefficients et constantes de `setup_exact` (PROJ 9.9). `n` est l'aplatissement troisième.
fn engsager(
    n: f64,
    k0: f64,
    phi0: f64,
) -> (
    f64,
    f64,
    [f64; ORDER],
    [f64; ORDER],
    [f64; ORDER],
    [f64; ORDER],
) {
    let coefficients = |auxin, auxout| {
        auxlat_coeffs(n, auxin, auxout).expect("tmerc conversion is generated by pj_auxlat_coeffs")
    };
    let cgb = coefficients(AuxLat::Conformal, AuxLat::Geographic);
    let cbg = coefficients(AuxLat::Geographic, AuxLat::Conformal);
    let utg = coefficients(AuxLat::Rectifying, AuxLat::Conformal);
    let gtu = coefficients(AuxLat::Conformal, AuxLat::Rectifying);
    let qn = k0 * rectifying_radius(n);
    let z = auxlat_convert(phi0, &cbg);
    let zb = -qn * auxlat_convert(z, &gtu);
    (qn, zb, cgb, cbg, utg, gtu)
}

fn clenshaw_complex(
    a: &[f64; ORDER],
    sin_arg_r: f64,
    cos_arg_r: f64,
    sinh_arg_i: f64,
    cosh_arg_i: f64,
) -> (f64, f64) {
    let mut r = 2.0 * cos_arg_r * cosh_arg_i;
    let mut i = -2.0 * sin_arg_r * sinh_arg_i;
    let mut hr1 = 0.0;
    let mut hi1 = 0.0;
    let mut hi = 0.0;
    let mut hr = a[ORDER - 1];
    for idx in (0..ORDER - 1).rev() {
        let hr2 = hr1;
        let hi2 = hi1;
        hr1 = hr;
        hi1 = hi;
        hr = -hr2 + r * hr1 - i * hi1 + a[idx];
        hi = -hi2 + i * hr1 + r * hi1;
    }
    r = sin_arg_r * cosh_arg_i;
    i = cos_arg_r * sinh_arg_i;
    (r * hr - i * hi, r * hi + i * hr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utm31() -> Tmerc {
        let zone = UtmZone::try_new(31).expect("zone 31");
        Tmerc::utm(zone, Hemisphere::North, Ellipsoid::wgs84()).expect("utm 31N")
    }

    fn assert_meters(got: Projected, east: f64, north: f64) {
        assert!(
            (got.east - east).abs() < 1e-6,
            "east {} vs {east}",
            got.east
        );
        assert!(
            (got.north - north).abs() < 1e-6,
            "north {} vs {north}",
            got.north
        );
    }

    #[test]
    fn proj_false_easting_on_central_meridian() {
        let xy = utm31().forward(3.0, 48.0).expect("forward");
        assert_meters(xy, 500_000.0, 5_316_300.224_451_5);
    }

    #[test]
    fn proj_equator_is_origin_of_northing() {
        let xy = utm31().forward(3.0, 0.0).expect("forward");
        assert_meters(xy, 500_000.0, 0.0);
    }

    #[test]
    fn proj_eiffel_tower() {
        let xy = utm31().forward(2.2945, 48.858_222).expect("forward");
        assert_meters(xy, 448_251.817_884_56, 5_411_935.123_220_22);
    }

    #[test]
    fn proj_sample_points() {
        let t = utm31();
        let cases = [
            (0.2, 43.5, 273_631.030_213_23, 4_820_150.399_393_36),
            (5.8, 49.9, 701_070.171_806_13, 5_531_271.830_333_88),
            (1.0, 10.0, 280_766.861_166_18, 1_106_077.129_705_66),
        ];
        for (lon, lat, east, north) in cases {
            assert_meters(t.forward(lon, lat).expect("forward"), east, north);
        }
    }

    #[test]
    fn proj_south_false_northing() {
        let zone = UtmZone::try_new(31).expect("zone");
        let t = Tmerc::utm(zone, Hemisphere::South, Ellipsoid::wgs84()).expect("south");
        let xy = t.forward(3.0, 0.0).expect("equator");
        assert_meters(xy, 500_000.0, 10_000_000.0);
        let south = t.forward(3.0, -10.0).expect("south lat");
        assert_meters(south, 500_000.0, 8_894_587.508_698_92);
    }

    #[test]
    fn roundtrip_matches_forward() {
        let t = utm31();
        for (lon, lat) in [(2.3, 48.85), (0.2, 43.5), (5.8, 49.9), (3.0, 60.0)] {
            let back = t.inverse(t.forward(lon, lat).expect("fwd")).expect("inv");
            assert!((back.lon_deg - lon).abs() < 1e-11, "{back:?}");
            assert!((back.lat_deg - lat).abs() < 1e-11, "{back:?}");
        }
    }

    #[test]
    fn batch_matches_scalar_within_ten_nanometres() {
        let t = utm31();
        let lon = [2.2, 3.0, 0.2, 5.5, 4.0, 1.1, 2.2945];
        let lat = [48.8, 46.0, 43.2, 49.5, 44.0, 50.0, 48.858_222];
        let mut east = [0.0; 7];
        let mut north = [0.0; 7];
        t.forward_batch(&lon, &lat, &mut east, &mut north);
        for i in 0..lon.len() {
            let xy = t.forward(lon[i], lat[i]).expect("scalar");
            assert!((east[i] - xy.east).abs() < 1e-8, "{i} east");
            assert!((north[i] - xy.north).abs() < 1e-8, "{i} north");
        }
    }

    #[test]
    fn rejects_bad_zone_and_latitude() {
        assert_eq!(
            UtmZone::try_new(0).expect_err("0"),
            ProjectionError::InvalidZone
        );
        assert_eq!(
            UtmZone::try_new(61).expect_err("61"),
            ProjectionError::InvalidZone
        );
        assert_eq!(
            utm31().forward(3.0, 91.0).expect_err("lat"),
            ProjectionError::InvalidLatitude
        );
    }
}
