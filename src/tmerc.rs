//! Transverse Mercator exact (Poder / Engsager), ordre 6.
//!
//! Portage du comportement de `src/projections/tmerc.cpp` (PROJ 9.4), algorithme
//! `PODER_ENGSAGER`. C'est le défaut `tmerc_default_algo` et donc le chemin UTM.
//! La correspondance avec le C est dans `docs/PORTING-tmerc.md`.
//!
//! L'algorithme approché d'Evenden / Snyder (`+approx`) n'est pas dans ce lot.

use crate::angle::adjlon;
use crate::{Ellipsoid, Geographic, Projected, ProjectionError};

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
        let phi0 = lat0_deg.to_radians();
        let (qn, zb, cgb, cbg, utg, gtu) = engsager(ell.n(), k0, phi0);
        Ok(Self {
            a: ell.a(),
            ra: ell.ra(),
            lam0: lon0_deg.to_radians(),
            x0,
            y0,
            qn,
            zb,
            cgb,
            cbg,
            utg,
            gtu,
        })
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
        let lon0 = (f64::from(zone.0) - 0.5) * 6.0 - 180.0;
        let y0 = match hemisphere {
            Hemisphere::North => 0.0,
            Hemisphere::South => 10_000_000.0,
        };
        Self::try_new(ell, lon0, 0.0, 0.9996, 500_000.0, y0)
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
    fn fwd_unit(&self, lam: f64, phi: f64) -> Result<(f64, f64), ProjectionError> {
        let mut cn = gatg(&self.cbg, phi, (2.0 * phi).cos(), (2.0 * phi).sin());
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
    fn inv_unit(&self, x: f64, y: f64) -> Result<(f64, f64), ProjectionError> {
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
        cn = sin_cn.atan2(modulus);

        let tmp = 2.0 * modulus / (sinh_ce * sinh_ce + 1.0);
        let sin_2_cn = sin_cn * tmp;
        let cos_2_cn = tmp * modulus - 1.0;
        let phi = gatg(&self.cgb, cn, cos_2_cn, sin_2_cn);
        Ok((ce, phi))
    }

    /// Convertit un lot. Un point hors domaine est écrit `NaN` (le `HUGE_VAL`
    /// de PROJ) et le lot continue.
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
        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
            // SAFETY: AVX2 and FMA were just detected on this CPU.
            unsafe { self.forward_batch_avx2(lon_deg, lat_deg, east, north) };
            return;
        }
        for i in 0..n {
            if let Ok(xy) = self.forward(lon_deg[i], lat_deg[i]) {
                east[i] = xy.east;
                north[i] = xy.north;
            } else {
                east[i] = f64::NAN;
                north[i] = f64::NAN;
            }
        }
    }

    /// # Safety
    ///
    /// Le fil d'exécution doit supporter AVX2 et FMA.
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn forward_batch_avx2(
        &self,
        lon_deg: &[f64],
        lat_deg: &[f64],
        east: &mut [f64],
        north: &mut [f64],
    ) {
        let n = lon_deg.len();
        let n4 = n & !3;
        let mut i = 0;
        while i < n4 {
            // SAFETY: `i + 4 <= n` and this function requires AVX2 and FMA.
            unsafe { self.forward4(lon_deg, lat_deg, east, north, i) };
            i += 4;
        }
        while i < n {
            if let Ok(xy) = self.forward(lon_deg[i], lat_deg[i]) {
                east[i] = xy.east;
                north[i] = xy.north;
            } else {
                east[i] = f64::NAN;
                north[i] = f64::NAN;
            }
            i += 1;
        }
    }

    /// # Safety
    ///
    /// AVX2 et FMA requis. `i + 4` ne dépasse aucune des quatre tranches.
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    #[target_feature(enable = "avx2,fma")]
    #[expect(
        clippy::similar_names,
        reason = "Cn and Ce are the paired terms of PROJ exact_e_fwd"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "the vector form stays one straight-line copy of exact_e_fwd"
    )]
    unsafe fn forward4(
        &self,
        lon_deg: &[f64],
        lat_deg: &[f64],
        east: &mut [f64],
        north: &mut [f64],
        i: usize,
    ) {
        use crate::simd::{atan2_4, cos4, load4, sin4, sqrt4, store4};
        use std::arch::x86_64::{
            _mm256_add_pd, _mm256_div_pd, _mm256_mul_pd, _mm256_set1_pd, _mm256_sub_pd,
        };

        // SAFETY: AVX2 kernel. Slice loads and stores stay inside `i..i+4`.
        #[expect(
            clippy::multiple_unsafe_ops_per_block,
            reason = "one Poder/Engsager formula; a block per intrinsic would hide it"
        )]
        unsafe {
            let deg = _mm256_set1_pd(std::f64::consts::PI / 180.0);
            let lon_r = _mm256_mul_pd(load4(lon_deg, i), deg);
            let lat_r = _mm256_mul_pd(load4(lat_deg, i), deg);
            let mut lat_s = [0.0; 4];
            let mut lon_s = [0.0; 4];
            store4(&mut lat_s, 0, lat_r);
            store4(&mut lon_s, 0, lon_r);
            let half = std::f64::consts::FRAC_PI_2;
            if lat_s
                .iter()
                .any(|p| !p.is_finite() || p.abs() > half + 1e-12)
            {
                for k in 0..4 {
                    if let Ok(xy) = self.forward(lon_deg[i + k], lat_deg[i + k]) {
                        east[i + k] = xy.east;
                        north[i + k] = xy.north;
                    } else {
                        east[i + k] = f64::NAN;
                        north[i + k] = f64::NAN;
                    }
                }
                return;
            }
            for lat in &mut lat_s {
                *lat = (*lat).clamp(-half, half);
            }
            for lon in &mut lon_s {
                *lon = adjlon(adjlon(*lon) - self.lam0);
            }
            let phi = load4(&lat_s, 0);
            let lam = load4(&lon_s, 0);
            let two_phi = _mm256_add_pd(phi, phi);
            let sin_2 = sin4(two_phi);
            let cos_2 = cos4(two_phi);
            // gatg sur 4 voies : même récurrence que le scalaire, coefficients diffusés.
            let mut h2 = _mm256_set1_pd(0.0);
            let mut h1 = _mm256_set1_pd(self.cbg[ORDER - 1]);
            let two = _mm256_add_pd(cos_2, cos_2);
            let mut h = h2;
            for idx in (0..ORDER - 1).rev() {
                let p = _mm256_set1_pd(self.cbg[idx]);
                h = _mm256_add_pd(_mm256_mul_pd(two, h1), _mm256_sub_pd(p, h2));
                h2 = h1;
                h1 = h;
            }
            let mut cn = _mm256_add_pd(phi, _mm256_mul_pd(h, sin_2));
            let sin_cn = sin4(cn);
            let cos_cn = cos4(cn);
            let sin_ce = sin4(lam);
            let cos_ce = cos4(lam);
            let prod = _mm256_mul_pd(cos_cn, cos_ce);
            cn = atan2_4(sin_cn, prod);
            let inv_denom = _mm256_div_pd(
                _mm256_set1_pd(1.0),
                sqrt4(_mm256_add_pd(
                    _mm256_mul_pd(sin_cn, sin_cn),
                    _mm256_mul_pd(prod, prod),
                )),
            );
            let tan_ce = _mm256_mul_pd(_mm256_mul_pd(sin_ce, cos_cn), inv_denom);
            let mut ce_s = [0.0; 4];
            let mut cn_s = [0.0; 4];
            let mut sin_a = [0.0; 4];
            let mut cos_a = [0.0; 4];
            let mut sinh_a = [0.0; 4];
            let mut cosh_a = [0.0; 4];
            store4(&mut ce_s, 0, tan_ce);
            for v in &mut ce_s {
                let ax = v.abs();
                *v = (ax + (ax * ax + 1.0).sqrt()).ln().copysign(*v);
            }
            let two_inv = _mm256_add_pd(inv_denom, inv_denom);
            let two_inv_sq = _mm256_mul_pd(two_inv, inv_denom);
            let tmp_r = _mm256_mul_pd(prod, two_inv_sq);
            store4(&mut cn_s, 0, cn);
            store4(&mut sin_a, 0, _mm256_mul_pd(sin_cn, tmp_r));
            store4(
                &mut cos_a,
                0,
                _mm256_sub_pd(_mm256_mul_pd(prod, tmp_r), _mm256_set1_pd(1.0)),
            );
            store4(&mut sinh_a, 0, _mm256_mul_pd(tan_ce, two_inv));
            store4(
                &mut cosh_a,
                0,
                _mm256_sub_pd(two_inv_sq, _mm256_set1_pd(1.0)),
            );
            for k in 0..4 {
                let (d_cn, d_ce) =
                    clenshaw_complex(&self.gtu, sin_a[k], cos_a[k], sinh_a[k], cosh_a[k]);
                cn_s[k] += d_cn;
                ce_s[k] += d_ce;
                if ce_s[k].abs() <= CE_LIMIT {
                    east[i + k] = self.a * (self.qn * ce_s[k]) + self.x0;
                    north[i + k] = self.a * (self.qn * cn_s[k] + self.zb) + self.y0;
                } else {
                    east[i + k] = f64::NAN;
                    north[i + k] = f64::NAN;
                }
            }
        }
    }
}

/// Coefficients de `setup_exact` (PROJ 9.4). `n` est l'aplatissement troisième.
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
    let mut cgb = [0.0; ORDER];
    let mut cbg = [0.0; ORDER];
    let mut utg = [0.0; ORDER];
    let mut gtu = [0.0; ORDER];

    let mut np = n;
    cgb[0] = n
        * (2.0
            + n * (-2.0 / 3.0
                + n * (-2.0 + n * (116.0 / 45.0 + n * (26.0 / 45.0 + n * (-2854.0 / 675.0))))));
    cbg[0] = n
        * (-2.0
            + n * (2.0 / 3.0
                + n * (4.0 / 3.0
                    + n * (-82.0 / 45.0 + n * (32.0 / 45.0 + n * (4642.0 / 4725.0))))));
    np *= n;
    cgb[1] = np
        * (7.0 / 3.0
            + n * (-8.0 / 5.0 + n * (-227.0 / 45.0 + n * (2704.0 / 315.0 + n * (2323.0 / 945.0)))));
    cbg[1] = np
        * (5.0 / 3.0
            + n * (-16.0 / 15.0 + n * (-13.0 / 9.0 + n * (904.0 / 315.0 + n * (-1522.0 / 945.0)))));
    np *= n;
    cgb[2] =
        np * (56.0 / 15.0 + n * (-136.0 / 35.0 + n * (-1262.0 / 105.0 + n * (73814.0 / 2835.0))));
    cbg[2] = np * (-26.0 / 15.0 + n * (34.0 / 21.0 + n * (8.0 / 5.0 + n * (-12686.0 / 2835.0))));
    np *= n;
    cgb[3] = np * (4279.0 / 630.0 + n * (-332.0 / 35.0 + n * (-399_572.0 / 14_175.0)));
    cbg[3] = np * (1237.0 / 630.0 + n * (-12.0 / 5.0 + n * (-24_832.0 / 14_175.0)));
    np *= n;
    cgb[4] = np * (4174.0 / 315.0 + n * (-144_838.0 / 6237.0));
    cbg[4] = np * (-734.0 / 315.0 + n * (109_598.0 / 31_185.0));
    np *= n;
    cgb[5] = np * (601_676.0 / 22_275.0);
    cbg[5] = np * (444_337.0 / 155_925.0);

    np = n * n;
    let qn = k0 / (1.0 + n) * (1.0 + np * (1.0 / 4.0 + np * (1.0 / 64.0 + np / 256.0)));
    utg[0] = n
        * (-0.5
            + n * (2.0 / 3.0
                + n * (-37.0 / 96.0
                    + n * (1.0 / 360.0 + n * (81.0 / 512.0 + n * (-96_199.0 / 604_800.0))))));
    gtu[0] = n
        * (0.5
            + n * (-2.0 / 3.0
                + n * (5.0 / 16.0
                    + n * (41.0 / 180.0 + n * (-127.0 / 288.0 + n * (7891.0 / 37_800.0))))));
    utg[1] = np
        * (-1.0 / 48.0
            + n * (-1.0 / 15.0
                + n * (437.0 / 1440.0 + n * (-46.0 / 105.0 + n * (1_118_711.0 / 3_870_720.0)))));
    gtu[1] = np
        * (13.0 / 48.0
            + n * (-3.0 / 5.0
                + n * (557.0 / 1440.0 + n * (281.0 / 630.0 + n * (-1_983_433.0 / 1_935_360.0)))));
    np *= n;
    utg[2] =
        np * (-17.0 / 480.0 + n * (37.0 / 840.0 + n * (209.0 / 4480.0 + n * (-5569.0 / 90_720.0))));
    gtu[2] = np
        * (61.0 / 240.0
            + n * (-103.0 / 140.0 + n * (15_061.0 / 26_880.0 + n * (167_603.0 / 181_440.0))));
    np *= n;
    utg[3] = np * (-4397.0 / 161_280.0 + n * (11.0 / 504.0 + n * (830_251.0 / 7_257_600.0)));
    gtu[3] = np * (49_561.0 / 161_280.0 + n * (-179.0 / 168.0 + n * (6_601_661.0 / 7_257_600.0)));
    np *= n;
    utg[4] = np * (-4583.0 / 161_280.0 + n * (108_847.0 / 3_991_680.0));
    gtu[4] = np * (34_729.0 / 80_640.0 + n * (-3_418_889.0 / 1_995_840.0));
    np *= n;
    utg[5] = np * (-20_648_693.0 / 638_668_800.0);
    gtu[5] = np * (212_378_941.0 / 319_334_400.0);

    let z = gatg(&cbg, phi0, (2.0 * phi0).cos(), (2.0 * phi0).sin());
    let zb = -qn * (z + clens(&gtu, 2.0 * z));
    (qn, zb, cgb, cbg, utg, gtu)
}

fn gatg(p1: &[f64; ORDER], b: f64, cos_2b: f64, sin_2b: f64) -> f64 {
    let two = 2.0 * cos_2b;
    let mut h2 = 0.0;
    let mut h1 = p1[ORDER - 1];
    let mut h = 0.0;
    for idx in (0..ORDER - 1).rev() {
        h = -h2 + two * h1 + p1[idx];
        h2 = h1;
        h1 = h;
    }
    b + h * sin_2b
}

fn clens(a: &[f64; ORDER], arg_r: f64) -> f64 {
    let r = 2.0 * arg_r.cos();
    let mut hr1 = 0.0;
    let mut hr = a[ORDER - 1];
    for idx in (0..ORDER - 1).rev() {
        let hr2 = hr1;
        hr1 = hr;
        hr = -hr2 + r * hr1 + a[idx];
    }
    arg_r.sin() * hr
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
