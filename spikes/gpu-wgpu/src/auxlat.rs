#![allow(dead_code, reason = "extrait de src/projmath.rs du crate principal")]
//! Coefficients des latitudes auxiliaires (Karney, ordre 6), extraits tels quels de
//! `src/projmath.rs` : `pj_auxlat_coeffs` de PROJ 9.9, `src/latitudes.cpp`.

const ORDER: usize = 6;
const AUX_NUMBER: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum AuxLat {
    Geographic = 0,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "parametric latitude is used by later projections")
    )]
    Parametric = 1,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "geocentric latitude is used by later projections")
    )]
    Geocentric = 2,
    Rectifying = 3,
    Conformal = 4,
    Authalic = 5,
}

impl AuxLat {
    fn index(self) -> usize {
        match self {
            Self::Geographic => 0,
            Self::Parametric => 1,
            Self::Geocentric => 2,
            Self::Rectifying => 3,
            Self::Conformal => 4,
            Self::Authalic => 5,
        }
    }
}

fn polyval(x: f64, p: &[f64], mut n: i32) -> f64 {
    if n < 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "n is a non-negative polynomial degree"
    )]
    let mut y = p[n as usize];
    while n > 0 {
        n -= 1;
        #[expect(
            clippy::cast_sign_loss,
            clippy::as_conversions,
            reason = "n stays a non-negative polynomial degree"
        )]
        {
            y = y * x + p[n as usize];
        }
    }
    y
}

/// Fourier coefficients converting `auxin` to `auxout`, length [`ORDER`].
pub fn auxlat_coeffs(n: f64, auxin: AuxLat, auxout: AuxLat) -> Option<[f64; ORDER]> {
    let k = AUX_NUMBER * auxout.index() + auxin.index();
    let mut o = PTRS[k];
    if o == PTRS[k + 1] {
        return None;
    }
    let mut d = n;
    let n2 = n * n;
    let mut f = [0.0; ORDER];
    if auxin <= AuxLat::Rectifying && auxout <= AuxLat::Rectifying {
        for (l, slot) in f.iter_mut().enumerate() {
            let m = (ORDER - l - 1) / 2;
            *slot = d * polyval(n2, &COEFFS[o..], i32::try_from(m).unwrap_or(0));
            o += m + 1;
            d *= n;
        }
    } else {
        for (l, slot) in f.iter_mut().enumerate() {
            let m = ORDER - l - 1;
            *slot = d * polyval(n, &COEFFS[o..], i32::try_from(m).unwrap_or(0));
            o += m + 1;
            d *= n;
        }
    }
    Some(f)
}

/// Convert auxiliary latitude `zeta` with coefficients from [`auxlat_coeffs`].
#[must_use]
pub fn rectifying_radius(n: f64) -> f64 {
    const COEFF_RAD: [f64; 4] = [1.0, 0.25, 1.0 / 64.0, 1.0 / 256.0];
    polyval(n * n, &COEFF_RAD, 3) / (1.0 + n)
}
#[rustfmt::skip]
const COEFFS: [f64; 150] = [
    3.0 / 2.0, -27.0 / 32.0, 269.0 / 512.0, 21.0 / 16.0, -55.0 / 32.0,
    6759.0 / 4096.0, 151.0 / 96.0, -417.0 / 128.0, 1097.0 / 512.0,
    -15543.0 / 2560.0, 8011.0 / 2560.0, 293_393.0 / 61440.0,
    2.0, -2.0 / 3.0, -2.0, 116.0 / 45.0, 26.0 / 45.0, -2854.0 / 675.0,
    7.0 / 3.0, -8.0 / 5.0, -227.0 / 45.0, 2704.0 / 315.0, 2323.0 / 945.0,
    56.0 / 15.0, -136.0 / 35.0, -1262.0 / 105.0, 73814.0 / 2835.0,
    4279.0 / 630.0, -332.0 / 35.0, -399_572.0 / 14175.0, 4174.0 / 315.0,
    -144_838.0 / 6237.0, 601_676.0 / 22275.0,
    4.0 / 3.0, 4.0 / 45.0, -16.0 / 35.0, -2582.0 / 14175.0,
    60136.0 / 467_775.0, 28_112_932.0 / 212_837_625.0, 46.0 / 45.0,
    152.0 / 945.0, -11966.0 / 14175.0, -21016.0 / 51975.0,
    251_310_128.0 / 638_512_875.0, 3044.0 / 2835.0, 3802.0 / 14175.0,
    -94388.0 / 66825.0, -8_797_648.0 / 10_945_935.0, 6059.0 / 4725.0,
    41072.0 / 93555.0, -1_472_637_812.0 / 638_512_875.0, 768_272.0 / 467_775.0,
    455_935_736.0 / 638_512_875.0, 4_210_684_958.0 / 1_915_538_625.0,
    -3.0 / 2.0, 9.0 / 16.0, -3.0 / 32.0, 15.0 / 16.0, -15.0 / 32.0,
    135.0 / 2048.0, -35.0 / 48.0, 105.0 / 256.0, 315.0 / 512.0,
    -189.0 / 512.0, -693.0 / 1280.0, 1001.0 / 2048.0,
    1.0 / 2.0, -2.0 / 3.0, 5.0 / 16.0, 41.0 / 180.0, -127.0 / 288.0,
    7891.0 / 37800.0, 13.0 / 48.0, -3.0 / 5.0, 557.0 / 1440.0,
    281.0 / 630.0, -1_983_433.0 / 1_935_360.0, 61.0 / 240.0, -103.0 / 140.0,
    15061.0 / 26880.0, 167_603.0 / 181_440.0, 49561.0 / 161_280.0,
    -179.0 / 168.0, 6_601_661.0 / 7_257_600.0, 34729.0 / 80640.0,
    -3_418_889.0 / 1_995_840.0, 212_378_941.0 / 319_334_400.0,
    -2.0, 2.0 / 3.0, 4.0 / 3.0, -82.0 / 45.0, 32.0 / 45.0, 4642.0 / 4725.0,
    5.0 / 3.0, -16.0 / 15.0, -13.0 / 9.0, 904.0 / 315.0, -1522.0 / 945.0,
    -26.0 / 15.0, 34.0 / 21.0, 8.0 / 5.0, -12686.0 / 2835.0, 1237.0 / 630.0,
    -12.0 / 5.0, -24832.0 / 14175.0, -734.0 / 315.0, 109_598.0 / 31185.0,
    444_337.0 / 155_925.0,
    -1.0 / 2.0, 2.0 / 3.0, -37.0 / 96.0, 1.0 / 360.0, 81.0 / 512.0,
    -96199.0 / 604_800.0, -1.0 / 48.0, -1.0 / 15.0, 437.0 / 1440.0,
    -46.0 / 105.0, 1_118_711.0 / 3_870_720.0, -17.0 / 480.0, 37.0 / 840.0,
    209.0 / 4480.0, -5569.0 / 90720.0, -4397.0 / 161_280.0, 11.0 / 504.0,
    830_251.0 / 7_257_600.0, -4583.0 / 161_280.0, 108_847.0 / 3_991_680.0,
    -20_648_693.0 / 638_668_800.0,
    -4.0 / 3.0, -4.0 / 45.0, 88.0 / 315.0, 538.0 / 4725.0,
    20824.0 / 467_775.0, -44732.0 / 2_837_835.0, 34.0 / 45.0, 8.0 / 105.0,
    -2482.0 / 14175.0, -37192.0 / 467_775.0, -12_467_764.0 / 212_837_625.0,
    -1532.0 / 2835.0, -898.0 / 14175.0, 54968.0 / 467_775.0,
    100_320_856.0 / 1_915_538_625.0, 6007.0 / 14175.0, 24496.0 / 467_775.0,
    -5_884_124.0 / 70_945_875.0, -23356.0 / 66825.0, -839_792.0 / 19_348_875.0,
    570_284_222.0 / 1_915_538_625.0,
];

const PTRS: [usize; 37] = [
    0, 0, 0, 0, 12, 33, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 54, 66, 66, 66, 66, 87, 87,
    108, 108, 108, 129, 129, 129, 150, 150, 150, 150, 150, 150,
];
