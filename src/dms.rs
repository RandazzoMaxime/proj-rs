//! Degrees, minutes and seconds: `proj_dmstor` and `proj_rtodms2` of PROJ 9.9.0,
//! which are `dmstor` (`src/dmstor.cpp`) and `rtodms` (`src/rtodms.cpp`), and the
//! C `strtod` they read numbers with.
//!
//! Each function keeps PROJ's order of operations, so its text matches PROJ
//! byte for byte. Text is bytes, not `str`: PROJ accepts a degree sign in UTF-8
//! and in ISO 8859-1 (cp1252) alike, and a byte `0xB0` is not valid UTF-8.

use std::f64::consts::PI;

/// `DEG_TO_RAD` of PROJ, the constant of `src/geodesic.rs`.
const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;

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

/// How the sign of an angle is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmsSign {
    /// The letter of a positive angle and the letter of a negative one, such as
    /// `N` and `S`. A NUL letter writes nothing, as PROJ's zero letter does.
    Hemisphere {
        /// Letter after a positive angle.
        positive: char,
        /// Letter after a negative angle.
        negative: char,
    },
    /// A minus sign before a negative angle, and no letter (PROJ's `pos == 0`).
    Minus,
}

/// Whether the seconds keep their trailing zeros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmsWidth {
    /// Trailing zeros of the seconds are removed, with the point when none is left.
    Trimmed,
    /// Minutes and seconds are zero padded to a fixed width.
    Fixed,
}

/// The format of the seconds: how many decimals, and the width. The default
/// is PROJ's: three decimals, trimmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DmsFormat {
    decimals: usize,
    width: DmsWidth,
}

impl Default for DmsFormat {
    fn default() -> Self {
        Self {
            decimals: 3,
            width: DmsWidth::Trimmed,
        }
    }
}

impl DmsFormat {
    /// A format with `decimals` decimals of seconds, which must be fewer than nine.
    ///
    /// Returns `None` for nine or more: PROJ's `set_rtodms` then keeps the format
    /// it had.
    #[must_use]
    pub fn new(decimals: usize, width: DmsWidth) -> Option<Self> {
        (decimals < 9).then_some(Self { decimals, width })
    }

    /// Decimals of the seconds.
    #[must_use]
    pub fn decimals(&self) -> usize {
        self.decimals
    }

    /// Width of the seconds.
    #[must_use]
    pub fn width(&self) -> DmsWidth {
        self.width
    }

    /// `set_rtodms(fract, width)` of PROJ: a count of decimals below nine changes
    /// the format, any other count leaves it as it was.
    pub fn set(&mut self, decimals: usize, width: DmsWidth) {
        if let Some(format) = Self::new(decimals, width) {
            *self = format;
        }
    }
}

/// `proj_dmstor(text, NULL)`: the angle in radians that `text` reads as.
///
/// A malformed group gives infinity, PROJ's `HUGE_VAL`.
#[must_use]
pub fn dmstor(text: &[u8]) -> f64 {
    dmstor_with_end(text).0
}

/// `proj_dmstor(text, &end)`: the angle in radians, and the number of bytes
/// read, which is where PROJ's `end` pointer stops.
#[must_use]
pub fn dmstor_with_end(text: &[u8]) -> (f64, usize) {
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
        let (tv, used) = strtod_before_d(&work[pos..]);
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
fn strtod_before_d(text: &[u8]) -> (f64, usize) {
    let cut = text
        .iter()
        .position(|&c| c == b'd' || c == b'D')
        .unwrap_or(text.len());
    strtod(&text[..cut])
}

/// C's `strtod` for decimal text: leading blanks, a sign, digits with an optional
/// point, and an optional exponent. Returns the value and the number of bytes
/// read, which is zero when no digit was found.
#[must_use]
pub fn strtod(text: &[u8]) -> (f64, usize) {
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

/// `proj_rtodms2` of PROJ: an angle in radians as degrees, minutes and seconds.
///
/// `max_len` is the most bytes the text may take. PROJ's buffer of `sizeof_s`
/// bytes holds `sizeof_s - 1` characters and the terminating nul, so its `max_len`
/// is `sizeof_s - 1`. A text longer than that is cut, as `snprintf` cuts it, and
/// PROJ's removal of trailing zeros then works on what remains. A `max_len` of
/// zero gives an empty text.
#[must_use]
pub fn rtodms(angle: f64, sign: DmsSign, format: DmsFormat, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    let mut room = max_len;
    let mut value = angle;
    let letter = match sign {
        DmsSign::Hemisphere { positive, negative } => {
            if angle < 0.0 {
                value = -value;
                letter_of(negative)
            } else {
                letter_of(positive)
            }
        }
        DmsSign::Minus => {
            if angle < 0.0 {
                value = -value;
                prefix.push('-');
                room -= 1;
            }
            None
        }
    };
    let body = degrees_text(value, letter, format, room);
    prefix.push_str(&String::from_utf8_lossy(&body));
    prefix
}

/// The letter a sign writes; a NUL letter writes none, as PROJ's zero does.
fn letter_of(letter: char) -> Option<char> {
    (letter != '\0').then_some(letter)
}

/// The text of a non-negative angle, with its letter, cut to `room` bytes.
fn degrees_text(value: f64, letter: Option<char>, format: DmsFormat, room: usize) -> Vec<u8> {
    let fract = format.decimals;
    let mut res = 1.0_f64;
    for _ in 0..fract {
        res *= 10.0;
    }
    let res60 = res * 60.0;
    let conv = if fract == 3 {
        206_264_806.247_096_36
    } else {
        180.0 * 3600.0 * res / PI
    };
    let r = (value * conv + 0.5).floor();
    let sec = (r / res) % 60.0;
    let r = (r / res60).floor();
    // Whole numbers: `{min}` and `{deg}` print them as C's `%d` does.
    let min = r % 60.0;
    let deg = (r / 60.0).floor();
    let suffix = letter.map(String::from).unwrap_or_default();
    if format.width == DmsWidth::Fixed {
        let width = fract + 2 + usize::from(fract > 0);
        let text = format!("{deg}d{min:02}'{sec:0width$.fract$}\"{suffix}");
        cut(&text, room)
    } else if sec != 0.0 {
        let text = format!("{deg}d{min}'{sec:.fract$}\"{suffix}");
        let mut bytes = cut(&text, room);
        let suffix_len = if letter.is_some() { 3 } else { 2 };
        trim_seconds(&mut bytes, suffix_len);
        bytes
    } else if min != 0.0 {
        cut(&format!("{deg}d{min}'{suffix}"), room)
    } else {
        cut(&format!("{deg}d{suffix}"), room)
    }
}

/// The first `room` bytes of `text`.
fn cut(text: &str, room: usize) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.truncate(room);
    bytes
}

/// PROJ's removal of the zeros before a suffix of `suffix_len` bytes, which is the
/// closing `"`, then the letter when there is one (`rtodms.cpp`). The trailing
/// zeros of the seconds go, with the point when no digit is left, and the suffix
/// moves up to close the gap.
fn trim_seconds(bytes: &mut Vec<u8>, suffix_len: usize) {
    let len = bytes.len();
    if suffix_len > len {
        return;
    }
    let mut q = len - suffix_len;
    let mut p = q;
    while p > 0 && bytes[p] == b'0' {
        p -= 1;
    }
    if bytes[p] != b'.' {
        p += 1;
    }
    q += 1;
    if q != p {
        let tail = bytes[q..].to_vec();
        bytes.truncate(p);
        bytes.extend_from_slice(&tail);
    }
}

#[cfg(test)]
mod tests {
    use super::{dmstor, dmstor_with_end, rtodms, strtod, DmsFormat, DmsSign, DmsWidth};

    const HEMISPHERE: DmsSign = DmsSign::Hemisphere {
        positive: 'N',
        negative: 'S',
    };

    #[test]
    fn dmstor_reads_degrees_minutes_seconds_and_rest() {
        let (value, used) = dmstor_with_end(b"-134d00'00.000  55d00'00.000 rest");
        assert!((value.to_degrees() + 134.0).abs() < 1e-12);
        assert_eq!(used, 14);
        let (value, used) = dmstor_with_end(b"40.5 1");
        assert!((value.to_degrees() - 40.5).abs() < 1e-12);
        assert_eq!(used, 4);
    }

    #[test]
    fn dmstor_hemisphere_letter_is_a_sign() {
        assert!((dmstor(b"74W").to_degrees() + 74.0).abs() < 1e-12);
    }

    #[test]
    fn strtod_stops_at_the_first_non_number_byte() {
        assert_eq!(strtod(b"12d30"), (12.0, 2));
        assert_eq!(strtod(b"1e3x"), (1000.0, 3));
        assert_eq!(strtod(b"x"), (0.0, 0));
    }

    #[test]
    fn rtodms_matches_reference_text() {
        let format = DmsFormat::default();
        assert_eq!(
            rtodms(40.5_f64.to_radians(), HEMISPHERE, format, usize::MAX),
            "40d30'N"
        );
        assert_eq!(
            rtodms(-40.0_f64.to_radians(), HEMISPHERE, format, usize::MAX),
            "40dS"
        );
        assert_eq!(
            rtodms(40.0_f64.to_radians(), DmsSign::Minus, format, usize::MAX),
            "40d"
        );
        assert_eq!(
            rtodms(-40.0_f64.to_radians(), DmsSign::Minus, format, usize::MAX),
            "-40d"
        );
    }

    #[test]
    fn rtodms_fixed_width_keeps_zeros() {
        let format = DmsFormat::new(2, DmsWidth::Fixed).expect("two decimals");
        assert_eq!(rtodms(0.0, HEMISPHERE, format, usize::MAX), "0d00'00.00\"N");
    }

    #[test]
    fn dms_format_ignores_nine_decimals() {
        let mut format = DmsFormat::new(1, DmsWidth::Trimmed).expect("one decimal");
        format.set(9, DmsWidth::Fixed);
        assert_eq!(format.decimals(), 1);
        assert_eq!(format.width(), DmsWidth::Trimmed);
        assert!(DmsFormat::new(9, DmsWidth::Fixed).is_none());
    }
}
