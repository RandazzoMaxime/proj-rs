//! Repli de `Tmerc::forward_batch` hors AVX2 : paquets de quatre voies, mêmes
//! opérations que `forward`, donc même résultat au bit près. Mesure : `docs/PORTING-batch-path.md`.

use super::{clenshaw_complex, Tmerc, CE_LIMIT};
use crate::angle::adjlon;
use crate::projmath::auxlat_convert;

impl Tmerc {
    pub(crate) fn forward_batch_portable(
        &self,
        lon_deg: &[f64],
        lat_deg: &[f64],
        east: &mut [f64],
        north: &mut [f64],
    ) {
        let half = std::f64::consts::FRAC_PI_2;
        let n = lon_deg.len();
        let mut i = 0;
        while i + 4 <= n {
            let mut ok = [false; 4];
            let mut phi = [0.0; 4];
            let mut lam = [0.0; 4];
            for k in 0..4 {
                let lon_r = lon_deg[i + k].to_radians();
                let lat_r = lat_deg[i + k].to_radians();
                ok[k] = lon_r.is_finite() && lat_r.is_finite() && lat_r.abs() <= half + 1e-12;
                phi[k] = lat_r.clamp(-half, half);
                lam[k] = if ok[k] {
                    adjlon(adjlon(lon_r) - self.lam0)
                } else {
                    0.0
                };
            }
            let mut cn = [0.0; 4];
            for k in 0..4 {
                cn[k] = auxlat_convert(phi[k], &self.cbg);
            }
            let mut sin_cn = [0.0; 4];
            let mut cos_cn = [0.0; 4];
            let mut sin_ce = [0.0; 4];
            let mut cos_ce = [0.0; 4];
            for k in 0..4 {
                sin_cn[k] = cn[k].sin();
                cos_cn[k] = cn[k].cos();
                sin_ce[k] = lam[k].sin();
                cos_ce[k] = lam[k].cos();
            }
            let mut ccc = [0.0; 4];
            let mut inv = [0.0; 4];
            let mut tan_ce = [0.0; 4];
            let mut ce = [0.0; 4];
            for k in 0..4 {
                ccc[k] = cos_cn[k] * cos_ce[k];
                cn[k] = sin_cn[k].atan2(ccc[k]);
                inv[k] = 1.0 / sin_cn[k].hypot(ccc[k]);
                tan_ce[k] = sin_ce[k] * cos_cn[k] * inv[k];
                ce[k] = tan_ce[k].asinh();
            }
            let mut sa = [0.0; 4];
            let mut ca = [0.0; 4];
            let mut sh = [0.0; 4];
            let mut ch = [0.0; 4];
            for k in 0..4 {
                let two_inv = 2.0 * inv[k];
                let two_inv_sq = two_inv * inv[k];
                let tmp_r = ccc[k] * two_inv_sq;
                sa[k] = sin_cn[k] * tmp_r;
                ca[k] = ccc[k] * tmp_r - 1.0;
                sh[k] = tan_ce[k] * two_inv;
                ch[k] = two_inv_sq - 1.0;
            }
            for k in 0..4 {
                let (d_cn, d_ce) = clenshaw_complex(&self.gtu, sa[k], ca[k], sh[k], ch[k]);
                cn[k] += d_cn;
                ce[k] += d_ce;
            }
            for k in 0..4 {
                if !ok[k] {
                    if let Ok(xy) = self.forward(lon_deg[i + k], lat_deg[i + k]) {
                        east[i + k] = xy.east;
                        north[i + k] = xy.north;
                    } else {
                        east[i + k] = f64::NAN;
                        north[i + k] = f64::NAN;
                    }
                } else if ce[k].abs() <= CE_LIMIT {
                    east[i + k] = self.a * (self.qn * ce[k]) + self.x0;
                    north[i + k] = self.a * (self.qn * cn[k] + self.zb) + self.y0;
                } else {
                    east[i + k] = f64::NAN;
                    north[i + k] = f64::NAN;
                }
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ellipsoid, Hemisphere, UtmZone};

    fn same_bits(a: f64, b: f64) -> bool {
        (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
    }

    fn check_against_scalar(t: &Tmerc, lon: &[f64], lat: &[f64]) -> (usize, usize) {
        let n = lon.len();
        let (mut east, mut north) = (vec![0.0; n], vec![0.0; n]);
        t.forward_batch_portable(lon, lat, &mut east, &mut north);
        let mut finite = 0;
        let mut nan = 0;
        for i in 0..n {
            let expected = t
                .forward(lon[i], lat[i])
                .map_or((f64::NAN, f64::NAN), |p| (p.east, p.north));
            assert!(
                same_bits(east[i], expected.0) && same_bits(north[i], expected.1),
                "point {i} ({}, {}): batch ({}, {}) vs scalar {:?}",
                lon[i],
                lat[i],
                east[i],
                north[i],
                expected
            );
            if expected.0.is_nan() {
                nan += 1;
            } else {
                finite += 1;
            }
        }
        (finite, nan)
    }

    #[test]
    fn portable_batch_matches_scalar_bit_for_bit() {
        let utm31 = UtmZone::try_new(31).expect("zone 31");
        let north = Tmerc::utm(utm31, Hemisphere::North, Ellipsoid::wgs84()).expect("utm 31N");
        let south = Tmerc::utm(
            UtmZone::try_new(1).expect("zone 1"),
            Hemisphere::South,
            Ellipsoid::wgs84(),
        )
        .expect("utm 1S");

        let lon = vec![
            f64::NAN,
            3.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            179.9,
            3.0,
            3.0,
            181.0,
            3.0,
            -180.0,
        ];
        let lat = vec![
            45.0,
            f64::NAN,
            0.0,
            f64::NEG_INFINITY,
            0.0,
            90.0,
            -90.0,
            10.0,
            91.0,
            -10.0,
        ];
        for (t, centre) in [(&north, 3.0), (&south, -177.0)] {
            let mut lon_t = lon.clone();
            let mut lat_t = lat.clone();
            for i in 0..40 {
                for j in 0..40 {
                    lon_t.push(centre - 10.0 + 20.0 * f64::from(i) / 39.0);
                    lat_t.push(-89.0 + 178.0 * f64::from(j) / 39.0);
                }
            }
            let (finite, nan) = check_against_scalar(t, &lon_t, &lat_t);
            assert!(
                finite > 1000,
                "only {finite} finite outputs: test is vacuous"
            );
            assert!(nan > 0, "no refused point: the NaN lanes are not exercised");
        }
    }

    #[test]
    fn portable_batch_handles_fewer_than_four_points() {
        let t = Tmerc::utm(
            UtmZone::try_new(31).expect("zone 31"),
            Hemisphere::North,
            Ellipsoid::wgs84(),
        )
        .expect("utm 31N");
        let lon = [2.0, 3.0, 4.0];
        let lat = [43.0, 44.0, 45.0];
        for n in 0..4 {
            let (finite, _) = check_against_scalar(&t, &lon[..n], &lat[..n]);
            assert_eq!(finite, n);
        }
    }
}
