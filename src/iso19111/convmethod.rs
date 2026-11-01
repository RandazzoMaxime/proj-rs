//! PROJ `Conversion::convertToOtherMethod` for the four method pairs that PROJ 9.9
//! implements, and the `EQUIVALENT` check that compares two conversions across them.
//!
//! The pairs are Mercator variant A (EPSG:9804) and variant B (EPSG:9805), and
//! Lambert Conic Conformal 1SP (EPSG:9801) and 2SP (EPSG:9802), in both directions.
//! PROJ returns `nullptr` when it has no conversion; that is `None` here.
//!
//! PROJ reads the ellipsoid from the source CRS of the conversion, so the base CRS is
//! an argument here: a conversion has no answer without one.

use std::f64::consts::FRAC_PI_2;

use super::buildop::OpProps;
use super::equiv::Equivalence;
use super::esri_parammap::squared_eccentricity;
use super::model::{
    Conversion, DerivedConversion, GeodeticCRS, Measure, ParameterValue, UnitOfMeasure,
};
use crate::projmath::{msfn, tsfn};

const MERCATOR_A: i32 = 9804;
const MERCATOR_B: i32 = 9805;
const LCC_1SP: i32 = 9801;
const LCC_2SP: i32 = 9802;

const LATITUDE_NATURAL_ORIGIN: i32 = 8801;
const LONGITUDE_NATURAL_ORIGIN: i32 = 8802;
const SCALE_FACTOR: i32 = 8805;
const FALSE_EASTING: i32 = 8806;
const FALSE_NORTHING: i32 = 8807;
const LATITUDE_FALSE_ORIGIN: i32 = 8821;
const LONGITUDE_FALSE_ORIGIN: i32 = 8822;
const LATITUDE_1ST_STD_PARALLEL: i32 = 8823;
const LATITUDE_2ND_STD_PARALLEL: i32 = 8824;
const EASTING_FALSE_ORIGIN: i32 = 8826;
const NORTHING_FALSE_ORIGIN: i32 = 8827;

impl DerivedConversion<'_> {
    /// PROJ `Conversion::convertToOtherMethod`, on the base CRS of this projected CRS.
    ///
    /// Returns a copy of the conversion when `method_code` is its own method. Returns
    /// `None` when PROJ has no conversion to that method, or when the parameters or the
    /// ellipsoid make it undefined.
    #[must_use]
    pub fn convert_to_other_method(&self, method_code: i32) -> Option<Conversion> {
        convert_to_other_method(self.source_crs(), self.conversion(), method_code)
    }
}

impl Conversion {
    /// PROJ `SingleOperation::_isEquivalentTo` for two conversions that share `source` as
    /// base CRS, with `criterion`.
    ///
    /// Under `Strict` this is [`Conversion::is_equivalent_to_with`]. Under `Equivalent`, the
    /// parameters are compared first. If they differ, PROJ's cross-method rules apply:
    /// Mercator variant A and B convert `self` to the other's method, and LCC 2SP converts
    /// `self` to 1SP. A 1SP `self` against a 2SP `other` defers to the reverse call. Two LCC
    /// 2SP conversions that still differ compare their 1SP forms.
    #[must_use]
    pub fn is_equivalent_to_with_source(
        &self,
        other: &Self,
        source: &GeodeticCRS,
        criterion: Equivalence,
    ) -> bool {
        if self.is_equivalent_to_with(other, criterion) {
            return true;
        }
        if criterion == Equivalence::Strict {
            return false;
        }
        let (Some(own_code), Some(other_code)) = (method_code(self), method_code(other)) else {
            return false;
        };
        match (own_code, other_code) {
            (LCC_2SP, LCC_2SP) => {
                // Two LCC 2SP that differ only by their origin describe one cone when their
                // 1SP forms agree.
                convert_to_other_method(source, self, LCC_1SP).is_some_and(|left| {
                    convert_to_other_method(source, other, LCC_1SP)
                        .is_some_and(|right| left.is_equivalent_to_with(&right, criterion))
                })
            }
            (LCC_1SP, LCC_2SP) => other.is_equivalent_to_with_source(self, source, criterion),
            (MERCATOR_A, MERCATOR_B) | (MERCATOR_B, MERCATOR_A) | (LCC_2SP, LCC_1SP) => {
                convert_to_other_method(source, self, other_code)
                    .is_some_and(|converted| converted.is_equivalent_to_with(other, criterion))
            }
            _ => false,
        }
    }
}

/// `convertToOtherMethod` for a conversion on `base`.
pub(crate) fn convert_to_other_method(
    base: &GeodeticCRS,
    conversion: &Conversion,
    target: i32,
) -> Option<Conversion> {
    let current = method_code(conversion)?;
    if current == target {
        return Some(conversion.clone());
    }
    let ellipsoid = base.ellipsoid()?;
    let e2 = squared_eccentricity(ellipsoid);
    if e2 < 0.0 {
        return None;
    }
    match (current, target) {
        (MERCATOR_A, MERCATOR_B) => mercator_a_to_b(conversion, e2),
        (MERCATOR_B, MERCATOR_A) => mercator_b_to_a(conversion, e2),
        (LCC_1SP, LCC_2SP) => {
            lcc_1sp_to_2sp(conversion, e2, ellipsoid.semi_major_axis().si_value())
        }
        (LCC_2SP, LCC_1SP) => {
            lcc_2sp_to_1sp(conversion, e2, ellipsoid.semi_major_axis().si_value())
        }
        _ => None,
    }
}

fn method_code(conversion: &Conversion) -> Option<i32> {
    conversion.method.code.parse().ok()
}

fn degree() -> UnitOfMeasure {
    UnitOfMeasure::degree()
}

fn metre() -> UnitOfMeasure {
    UnitOfMeasure::metre()
}

/// The stored measure, unchanged, so that PROJ's value and unit pass through. An absent
/// parameter reads as zero in `default_unit`, as PROJ's `parameterValueMeasure` does.
fn measure(conversion: &Conversion, code: i32, default_unit: UnitOfMeasure) -> Measure {
    conversion
        .measure_of_code(code)
        .cloned()
        .unwrap_or(Measure {
            value: 0.0,
            unit: default_unit,
        })
}

/// PROJ `parameterValueNumericAsSI`: zero when absent.
fn si(conversion: &Conversion, code: i32) -> f64 {
    conversion.numeric_si(code)
}

fn angle_value(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: degree(),
    })
}

fn length_value(value: f64) -> ParameterValue {
    ParameterValue::Measure(Measure {
        value,
        unit: metre(),
    })
}

fn passthrough(measure: Measure) -> ParameterValue {
    ParameterValue::Measure(measure)
}

fn build(code: i32, values: Vec<ParameterValue>) -> Option<Conversion> {
    Conversion::from_code(&OpProps::new(), code, values).ok()
}

fn rad_to_deg(x: f64) -> f64 {
    x / std::f64::consts::PI * 180.0
}

fn deg_to_rad(x: f64) -> f64 {
    x / 180.0 * std::f64::consts::PI
}

/// Round to a thousandth of a degree when that is within `1e-8` of it.
fn round_thousandth_if_close(deg: f64) -> f64 {
    let scaled = deg * 1000.0;
    if (scaled - (scaled + 0.5).floor()).abs() < 1e-8 {
        (scaled + 0.5).floor() / 1000.0
    } else {
        deg
    }
}

fn mercator_a_to_b(conversion: &Conversion, e2: f64) -> Option<Conversion> {
    if si(conversion, LATITUDE_NATURAL_ORIGIN) != 0.0 {
        return None;
    }
    let k0 = si(conversion, SCALE_FACTOR);
    if !(k0 > 0.0 && k0 <= 1.0 + 1e-10) {
        return None;
    }
    let phi1 = if k0 >= 1.0 {
        0.0
    } else {
        ((1.0 - e2) / (1.0 / (k0 * k0) - e2)).sqrt().acos()
    };
    build(
        MERCATOR_B,
        vec![
            angle_value(phi1 / degree().conversion_to_si()),
            passthrough(measure(conversion, LONGITUDE_NATURAL_ORIGIN, degree())),
            passthrough(measure(conversion, FALSE_EASTING, metre())),
            passthrough(measure(conversion, FALSE_NORTHING, metre())),
        ],
    )
}

fn mercator_b_to_a(conversion: &Conversion, e2: f64) -> Option<Conversion> {
    let phi1 = si(conversion, LATITUDE_1ST_STD_PARALLEL);
    if !(phi1.abs() < FRAC_PI_2) {
        return None;
    }
    let k0 = msfn(phi1.sin(), phi1.cos(), e2);
    build(
        MERCATOR_A,
        vec![
            angle_value(0.0),
            passthrough(measure(conversion, LONGITUDE_NATURAL_ORIGIN, degree())),
            ParameterValue::Measure(Measure::scale(k0)),
            passthrough(measure(conversion, FALSE_EASTING, metre())),
            passthrough(measure(conversion, FALSE_NORTHING, metre())),
        ],
    )
}

/// Function whose zeroes are the sines of the standard parallels of LCC 2SP.
fn lcc_1sp_to_2sp_f(sinphi: f64, big_k: f64, ec: f64, n: f64) -> f64 {
    let x = sinphi;
    let ecx = ec * x;
    (1.0 - x * x) / (1.0 - ecx * ecx)
        - big_k * big_k * (((1.0 - x) / (1.0 + x)) * ((1.0 + ecx) / (1.0 - ecx)).powf(ec)).powf(n)
}

/// Which of the two standard parallels of LCC 2SP to search for.
#[derive(Clone, Copy)]
enum Parallel {
    North,
    South,
}

/// Dichotomy search for the sine of a standard parallel of LCC 2SP.
fn find_zero_lcc_1sp_to_2sp(sinphi0: f64, parallel: Parallel, big_k: f64, ec: f64) -> f64 {
    let (mut a, mut b, mut f_a) = match parallel {
        Parallel::North => (sinphi0, 1.0, 1.0),
        Parallel::South => (-1.0, sinphi0, -1.0),
    };
    for _ in 0..100 {
        let c = (a + b) / 2.0;
        let f_c = lcc_1sp_to_2sp_f(c, big_k, ec, sinphi0);
        if f_c == 0.0 || (b - a) < 1e-18 {
            return c;
        }
        if (f_c > 0.0 && f_a > 0.0) || (f_c < 0.0 && f_a < 0.0) {
            a = c;
            f_a = f_c;
        } else {
            b = c;
        }
    }
    (a + b) / 2.0
}

fn lcc_1sp_to_2sp(conversion: &Conversion, e2: f64, a: f64) -> Option<Conversion> {
    // Notation of the EPSG guidance (1.3.1.1 and 1.3.1.2) and Snyder pages 106-109.
    let latitude_of_origin = measure(conversion, LATITUDE_NATURAL_ORIGIN, degree());
    let phi0 = latitude_of_origin.si_value();
    let k0 = si(conversion, SCALE_FACTOR);
    if !(phi0.abs() < FRAC_PI_2) {
        return None;
    }
    if !(k0 > 0.0 && k0 <= 1.0 + 1e-10) {
        return None;
    }
    let ec = e2.sqrt();
    let m0 = msfn(phi0.sin(), phi0.cos(), e2);
    let t0 = tsfn(phi0, phi0.sin(), ec);
    let n = phi0.sin();
    if n.abs() < 1e-10 {
        return None;
    }
    let longitude_of_origin = measure(conversion, LONGITUDE_NATURAL_ORIGIN, degree());
    let easting = measure(conversion, FALSE_EASTING, metre());
    let northing_si = si(conversion, FALSE_NORTHING);

    if (k0 - 1.0).abs() <= 1e-10 {
        return build(
            LCC_2SP,
            vec![
                passthrough(latitude_of_origin.clone()),
                passthrough(longitude_of_origin),
                passthrough(latitude_of_origin.clone()),
                passthrough(latitude_of_origin),
                passthrough(easting),
                passthrough(measure(conversion, FALSE_NORTHING, metre())),
            ],
        );
    }

    let big_k = k0 * m0 / t0.powf(n);
    let phi1 = find_zero_lcc_1sp_to_2sp(n, Parallel::North, big_k, ec).asin();
    let phi2 = find_zero_lcc_1sp_to_2sp(n, Parallel::South, big_k, ec).asin();
    let phi1_deg = round_thousandth_if_close(rad_to_deg(phi1));
    let phi2_deg = round_thousandth_if_close(rad_to_deg(phi2));

    // When the latitude of origin is close to a half degree, check whether rounding it
    // gives a false northing close to an integer: that is the LCC 2SP of EPSG:2154.
    let latitude_of_origin_deg = phi0 / degree().conversion_to_si();
    if (latitude_of_origin_deg * 2.0 - (latitude_of_origin_deg * 2.0 + 0.5).floor()).abs() < 0.2 {
        let rounded_lat_of_origin = (latitude_of_origin_deg * 2.0 + 0.5).floor() / 2.0;
        let m1 = msfn(phi1.sin(), phi1.cos(), e2);
        let t1 = tsfn(phi1, phi1.sin(), ec);
        let f = m1 / (n * t1.powf(n));
        let t_rounded = tsfn(
            deg_to_rad(rounded_lat_of_origin),
            deg_to_rad(rounded_lat_of_origin).sin(),
            ec,
        );
        let fn_correction = a * f * (t_rounded.powf(n) - t0.powf(n));
        let fn_corrected = northing_si - fn_correction;
        let fn_corrected_rounded = (fn_corrected + 0.5).floor();
        if (fn_corrected - fn_corrected_rounded).abs() < 1e-8 {
            return build(
                LCC_2SP,
                vec![
                    angle_value(rounded_lat_of_origin),
                    passthrough(longitude_of_origin),
                    angle_value(phi1_deg),
                    angle_value(phi2_deg),
                    passthrough(easting),
                    length_value(fn_corrected_rounded),
                ],
            );
        }
    }

    build(
        LCC_2SP,
        vec![
            passthrough(latitude_of_origin),
            passthrough(longitude_of_origin),
            angle_value(phi1_deg),
            angle_value(phi2_deg),
            passthrough(easting),
            length_value(northing_si),
        ],
    )
}

fn lcc_2sp_to_1sp(conversion: &Conversion, e2: f64, a: f64) -> Option<Conversion> {
    // Notation of the EPSG guidance (1.3.1.1 and 1.3.1.2) and Snyder pages 106-109.
    let phi_f = measure(conversion, LATITUDE_FALSE_ORIGIN, degree()).si_value();
    let phi1 = si(conversion, LATITUDE_1ST_STD_PARALLEL);
    let phi2 = si(conversion, LATITUDE_2ND_STD_PARALLEL);
    if !(phi_f.abs() < FRAC_PI_2) || !(phi1.abs() < FRAC_PI_2) || !(phi2.abs() < FRAC_PI_2) {
        return None;
    }
    let ec = e2.sqrt();
    let m1 = msfn(phi1.sin(), phi1.cos(), e2);
    let m2 = msfn(phi2.sin(), phi2.cos(), e2);
    let t1 = tsfn(phi1, phi1.sin(), ec);
    let t2 = tsfn(phi2, phi2.sin(), ec);
    let n_denom = t1.ln() - t2.ln();
    let n = if n_denom.abs() < 1e-10 {
        phi1.sin()
    } else {
        (m1.ln() - m2.ln()) / n_denom
    };
    if n.abs() < 1e-10 {
        return None;
    }
    let f = m1 / (n * t1.powf(n));
    let phi0 = n.asin();
    let m0 = msfn(phi0.sin(), phi0.cos(), e2);
    let t0 = tsfn(phi0, phi0.sin(), ec);
    let f0 = m0 / (n * t0.powf(n));
    let k0 = f / f0;
    let t_f = tsfn(phi_f, phi_f.sin(), ec);
    let fn_correction = a * f * (t_f.powf(n) - t0.powf(n));

    let phi0_deg = round_thousandth_if_close(rad_to_deg(phi0));
    let northing = si(conversion, NORTHING_FALSE_ORIGIN)
        + if fn_correction.abs() > 1e-8 {
            fn_correction
        } else {
            0.0
        };
    build(
        LCC_1SP,
        vec![
            angle_value(phi0_deg),
            passthrough(measure(conversion, LONGITUDE_FALSE_ORIGIN, degree())),
            ParameterValue::Measure(Measure::scale(k0)),
            passthrough(measure(conversion, EASTING_FALSE_ORIGIN, metre())),
            length_value(northing),
        ],
    )
}
