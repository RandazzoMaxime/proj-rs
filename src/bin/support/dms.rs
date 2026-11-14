//! Number reading and writing shared by the PROJ command-line ports `proj`,
//! `invproj`, `geod` and `invgeod`.
//!
//! The degrees, minutes and seconds functions are the library's
//! ([`proj_rs::dms`]: `dmstor` and `rtodms` of PROJ 9.9.0, `src/dmstor.cpp` and
//! `src/rtodms.cpp`). This file keeps the names the commands use for them, the
//! program name and direction, and `limited_fprintf_for_number` (`src/apps/utils.cpp`),
//! so the text matches the reference binaries byte for byte.

use proj_rs::dms::{self, DmsSign};

pub(crate) use proj_rs::dms::{strtod as c_strtod, DmsFormat as Dms, DmsWidth as Width};

/// `RAD_TO_DEG` of PROJ.
pub(crate) const RAD_TO_DEG: f64 = 57.295_779_513_082_32;

/// Which way a program runs: `proj` and `geod` forward, `invproj` and
/// `invgeod` inverse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Longitude and latitude in, projected coordinates or end point out.
    Forward,
    /// The inverse of the forward program.
    Inverse,
}

/// The name the program was started with: the last part of `argv[0]`.
pub(crate) fn program_name() -> String {
    let argv0 = std::env::args().next().unwrap_or_default();
    std::path::Path::new(&argv0)
        .file_name()
        .map_or_else(|| argv0.clone(), |name| name.to_string_lossy().into_owned())
}

/// PROJ's rule: a program whose name starts with `inv` runs in inverse mode.
pub(crate) fn direction_of(program: &str) -> Direction {
    if program.starts_with("inv") {
        Direction::Inverse
    } else {
        Direction::Forward
    }
}

/// `dmstor` as the commands call it: the angle in radians and the bytes read.
pub(crate) fn dmstor(text: &[u8]) -> (f64, usize) {
    dms::dmstor_with_end(text)
}

/// `rtodms` as the commands call it, with no length limit. A zero `pos` writes
/// a minus sign instead of letters, as for azimuths.
pub(crate) fn rtodms(value: f64, format: Dms, pos: u8, neg: u8) -> String {
    let sign = if pos == 0 {
        DmsSign::Minus
    } else {
        DmsSign::Hemisphere {
            positive: char::from(pos),
            negative: char::from(neg),
        }
    };
    dms::rtodms(value, sign, format, usize::MAX)
}

/// `limited_fprintf_for_number` of PROJ: one number in a C-style format
/// `%[+][width][.precision](e|E|f|F|g|G)`, as the `-f` and `-F` options take.
///
/// Returns `None` for a format PROJ rejects, which prints nothing.
pub(crate) fn format_number(spec: &str, value: f64) -> Option<String> {
    let bytes = spec.as_bytes();
    if bytes.first() != Some(&b'%') {
        return None;
    }
    let mut i = 1;
    let plus = bytes.get(i) == Some(&b'+');
    if plus {
        i += 1;
    }
    let width_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let zero_pad = bytes.get(width_start) == Some(&b'0');
    let width = if i > width_start {
        spec.get(width_start..i)?.parse::<usize>().ok()?
    } else {
        0
    };
    let mut precision = 6;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let precision_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        precision = if i > precision_start {
            spec.get(precision_start..i)?.parse::<usize>().ok()?
        } else {
            0
        };
    }
    if i + 1 != bytes.len() {
        return None;
    }
    let conversion = bytes[i];
    let upper = conversion.is_ascii_uppercase();
    let body = match conversion.to_ascii_lowercase() {
        b'f' => c_fixed(value, precision),
        b'e' => c_exponent(value, precision),
        b'g' => c_general(value, precision),
        _ => return None,
    };
    let body = if upper {
        body.to_ascii_uppercase()
    } else {
        body
    };
    let body = if plus && !body.starts_with('-') {
        format!("+{body}")
    } else {
        body
    };
    let length = body.len();
    if length >= width {
        return Some(body);
    }
    let fill = width - length;
    if zero_pad && value.is_finite() {
        let (sign, digits) = match body.strip_prefix(['-', '+']) {
            Some(rest) => (&body[..1], rest),
            None => ("", body.as_str()),
        };
        Some(format!("{sign}{}{digits}", "0".repeat(fill)))
    } else {
        Some(format!("{}{body}", " ".repeat(fill)))
    }
}

/// `%.Nf`, with the non-finite spellings of C.
fn c_fixed(value: f64, precision: usize) -> String {
    if let Some(text) = non_finite(value) {
        return text;
    }
    format!("{value:.precision$}")
}

/// `%.Ne`: a mantissa and an exponent of at least two digits.
fn c_exponent(value: f64, precision: usize) -> String {
    if let Some(text) = non_finite(value) {
        return text;
    }
    let text = format!("{value:.precision$e}");
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exponent.unsigned_abs())
}

/// `%.Ng` with the trailing zeros removed, as `%g` without the `#` flag.
fn c_general(value: f64, precision: usize) -> String {
    if let Some(text) = non_finite(value) {
        return text;
    }
    let p = precision.max(1);
    let scientific = format!("{value:.p$e}", p = p - 1);
    let exponent: i64 = scientific
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    let p = i64::try_from(p).unwrap_or(1);
    if exponent < -4 || exponent >= p {
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let mantissa = trim_fraction(mantissa);
        let exponent: i64 = exponent.parse().unwrap_or(0);
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{mantissa}e{sign}{:02}", exponent.unsigned_abs())
    } else {
        let fixed = usize::try_from(p - 1 - exponent).unwrap_or(0);
        trim_fraction(&format!("{value:.fixed$}"))
    }
}

/// Removes the trailing zeros of a fraction, and the point when none is left.
fn trim_fraction(text: &str) -> String {
    if !text.contains('.') {
        return text.to_owned();
    }
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn non_finite(value: f64) -> Option<String> {
    if value.is_nan() {
        Some("nan".to_owned())
    } else if value.is_infinite() {
        Some(if value > 0.0 { "inf" } else { "-inf" }.to_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn format_number_follows_printf() {
        assert_eq!(format_number("%.3f", 2.0).as_deref(), Some("2.000"));
        assert_eq!(format_number("%.2f", -0.001).as_deref(), Some("-0.00"));
        assert_eq!(format_number("%8.2f", 1.5).as_deref(), Some("    1.50"));
        assert_eq!(format_number("%08.2f", -1.5).as_deref(), Some("-0001.50"));
        assert_eq!(
            format_number("%g", 1_234_567.0).as_deref(),
            Some("1.23457e+06")
        );
        assert_eq!(format_number("%g", 0.000_1).as_deref(), Some("0.0001"));
        assert_eq!(format_number("%.3e", 12345.0).as_deref(), Some("1.234e+04"));
        assert_eq!(format_number("3f", 1.0), None);
    }
}
