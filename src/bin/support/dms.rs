//! Number reading and writing shared by the PROJ command-line ports `proj`,
//! `invproj`, `geod` and `invgeod`.
//!
//! The functions are those of PROJ 9.9.0: `dmstor` (`src/dmstor.cpp`),
//! `rtodms` and `set_rtodms` (`src/rtodms.cpp`), and `limited_fprintf_for_number`
//! (`src/apps/utils.cpp`). Each keeps PROJ's order of operations, so the text
//! matches the reference binaries byte for byte.

use std::f64::consts::PI;

/// `DEG_TO_RAD` of PROJ, the constant of `src/geodesic.rs`.
pub(crate) const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;
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

/// Largest run of characters `dmstor` copies: its `MAX_WORK` buffer.
const MAX_WORK: usize = 64;
/// Multipliers of degrees, minutes and seconds, in radians.
const VM: [f64; 3] = [
    DEG_TO_RAD,
    0.000_290_888_208_665_721_6,
    0.000_004_848_136_811_095_36,
];
/// Hemisphere letters: index 4 and above is negative.
const SYMBOLS: &[u8] = b"NnEeSsWw";

/// Parses a degree, minute and second value as PROJ's `dmstor` does.
///
/// Returns the value in radians and the byte offset in `text` where the
/// parse stopped. A malformed group gives `f64::INFINITY`, PROJ's `HUGE_VAL`.
pub(crate) fn dmstor(text: &[u8]) -> (f64, usize) {
    let bytes = text;
    let mut start = 0;
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    let mut work = Vec::new();
    let mut index = start;
    while work.len() < MAX_WORK - 1
        && bytes
            .get(index)
            .is_some_and(|&c| c.is_ascii_graphic() || c == 0xC2 || c == 0xB0)
    {
        work.push(bytes[index]);
        index += 1;
    }
    let mut pos = 0;
    let mut sign = b'+';
    if let Some(&c) = work.first() {
        if c == b'+' || c == b'-' {
            sign = c;
            pos = 1;
        }
    }
    let mut value = 0.0;
    let mut nl = 0;
    while nl < 3 {
        if !work
            .get(pos)
            .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
        {
            break;
        }
        let (tv, used) = c_strtod_before_d(&work[pos..]);
        pos += used;
        let mut advance = 1;
        let next = work.get(pos).copied();
        let group = match next {
            Some(b'D' | b'd' | 0xB0) => 0,
            Some(b'\'') => 1,
            Some(b'"') => 2,
            Some(0xC2) if work.get(pos + 1) == Some(&0xB0) => {
                advance = 2;
                0
            }
            Some(b'r' | b'R') => {
                if nl != 0 {
                    return (f64::INFINITY, start + pos);
                }
                value = tv;
                pos += 1;
                break;
            }
            _ => {
                value += tv * VM[nl];
                break;
            }
        };
        if group < nl {
            return (f64::INFINITY, start + pos);
        }
        value += tv * VM[group];
        pos += advance;
        nl = group + 1;
    }
    if let Some(index) = work
        .get(pos)
        .and_then(|c| SYMBOLS.iter().position(|s| s == c))
    {
        sign = if index >= 4 { b'-' } else { b'+' };
        pos += 1;
    }
    if sign == b'-' {
        value = -value;
    }
    (value, start + pos)
}

/// `proj_strtod` of PROJ: C's `strtod`, cut at the first `d` or `D`.
fn c_strtod_before_d(text: &[u8]) -> (f64, usize) {
    let cut = text
        .iter()
        .position(|&c| c == b'd' || c == b'D')
        .unwrap_or(text.len());
    c_strtod(&text[..cut])
}

/// C's `strtod` for decimal input: leading blanks, a sign, digits with an
/// optional point, and an optional exponent. Returns the value and the number
/// of bytes read, which is zero when no digit was found.
pub(crate) fn c_strtod(text: &[u8]) -> (f64, usize) {
    let mut i = 0;
    while text.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    let number_start = i;
    if matches!(text.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    let digits_start = i;
    while text.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - digits_start;
    if text.get(i) == Some(&b'.') {
        i += 1;
        let fraction = i;
        while text.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - fraction;
    }
    if digits == 0 {
        return (0.0, 0);
    }
    if matches!(text.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(text.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exponent = j;
        while text.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        if j > exponent {
            i = j;
        }
    }
    let value = std::str::from_utf8(&text[number_start..i])
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    (value, i)
}

/// `set_rtodms` of PROJ: how `rtodms` writes seconds.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dms {
    fract: usize,
    width: Width,
}

/// Whether seconds are written with trailing zeros removed or at a fixed width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Width {
    /// Trailing zeros of the seconds are removed.
    Trimmed,
    /// Minutes and seconds are zero padded to a fixed width.
    Fixed,
}

impl Default for Dms {
    /// PROJ's initial state: three decimals of seconds, trimmed.
    fn default() -> Self {
        Self {
            fract: 3,
            width: Width::Trimmed,
        }
    }
}

impl Dms {
    /// `set_rtodms(fract, width)`: a count of seconds decimals below nine
    /// changes the output, any other count leaves it as it was.
    pub(crate) fn set(&mut self, fract: usize, width: Width) {
        if fract < 9 {
            self.fract = fract;
            self.width = width;
        }
    }
}

/// `rtodms` of PROJ: an angle in radians as degrees, minutes and seconds.
///
/// `pos` and `neg` are the hemisphere letters for a positive or negative
/// value. A zero `pos` writes a minus sign instead, as for azimuths.
pub(crate) fn rtodms(value: f64, dms: Dms, pos: u8, neg: u8) -> String {
    let mut prefix = String::new();
    let mut r = value;
    let sign = if r < 0.0 {
        r = -r;
        if pos == 0 {
            prefix.push('-');
            0
        } else {
            neg
        }
    } else {
        pos
    };
    let mut res = 1.0;
    for _ in 0..dms.fract {
        res *= 10.0;
    }
    let conv = if dms.fract == 3 {
        206_264_806.247_096_36
    } else {
        180.0 * 3600.0 * res / PI
    };
    let res60 = res * 60.0;
    r = (r * conv + 0.5).floor();
    let sec = (r / res) % 60.0;
    r = (r / res60).floor();
    let min = r % 60.0;
    r = (r / 60.0).floor();
    let deg = r;
    let sign_text = if sign == 0 {
        String::new()
    } else {
        char::from(sign).to_string()
    };
    let fract = dms.fract;
    let body = if dms.width == Width::Fixed {
        let width = fract + 2 + usize::from(fract > 0);
        format!("{deg}d{min:02}'{sec:0width$.fract$}\"")
    } else if sec != 0.0 {
        let text = format!("{sec:.fract$}");
        let trimmed = text.trim_end_matches('0');
        let trimmed = trimmed.strip_suffix('.').unwrap_or(trimmed);
        format!("{deg}d{min}'{trimmed}\"")
    } else if min != 0.0 {
        format!("{deg}d{min}'")
    } else {
        format!("{deg}d")
    };
    format!("{prefix}{body}{sign_text}")
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
    use super::{c_strtod, dmstor, format_number, rtodms, Dms, Width, DEG_TO_RAD};

    #[test]
    fn dmstor_reads_degrees_minutes_seconds_and_rest() {
        let (value, used) = dmstor(b"-134d00'00.000  55d00'00.000 rest");
        assert!((value.to_degrees() + 134.0).abs() < 1e-12);
        assert_eq!(used, 14);
        let (value, used) = dmstor(b"40.5 1");
        assert!((value.to_degrees() - 40.5).abs() < 1e-12);
        assert_eq!(used, 4);
    }

    #[test]
    fn dmstor_hemisphere_letter_is_a_sign() {
        let (value, _) = dmstor(b"74W");
        assert!((value.to_degrees() + 74.0).abs() < 1e-12);
    }

    #[test]
    fn c_strtod_stops_at_d() {
        assert_eq!(c_strtod(b"12d30"), (12.0, 2));
        assert_eq!(c_strtod(b"1e3x"), (1000.0, 3));
        assert_eq!(c_strtod(b"x"), (0.0, 0));
    }

    #[test]
    fn rtodms_matches_reference_text() {
        let dms = Dms::default();
        assert_eq!(rtodms(40.5 * DEG_TO_RAD, dms, b'N', b'S'), "40d30'N");
        assert_eq!(rtodms(-40.0 * DEG_TO_RAD, dms, b'N', b'S'), "40dS");
        assert_eq!(rtodms(40.0 * DEG_TO_RAD, dms, 0, 0), "40d");
        assert_eq!(rtodms(-40.0 * DEG_TO_RAD, dms, 0, 0), "-40d");
    }

    #[test]
    fn rtodms_fixed_width_keeps_zeros() {
        let mut dms = Dms::default();
        dms.set(2, Width::Fixed);
        let text = rtodms(0.0, dms, b'N', b'S');
        assert_eq!(text, "0d00'00.00\"N");
    }

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
