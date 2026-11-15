//! The monitor of `proj -v` and `proj -V` (PROJ 9.9.0 `apps/proj.cpp`, `pr_list.cpp`): the
//! parameter list of the operation, the figure of its ellipsoid, and for `-V` one labelled block
//! per point. The lines of the list break at 72 columns, as `pr_list` breaks them.

use proj_rs::ParamUse;

use crate::dms::{c_strtod, dmstor, format_number, rtodms, RAD_TO_DEG};

use super::{Job, Options};

/// `pr_list`: a `#`, then each parameter as ` +text`. A parameter that would pass column 72 starts
/// a new `#` line. `n` counts the columns as `pr_list` does, which leaves out the `+`.
fn push_list<'a>(out: &mut String, items: impl Iterator<Item = &'a ParamUse>) {
    const LINE_LEN: usize = 72;
    out.push('#');
    let mut n = 1;
    for item in items {
        let l = item.text.len() + 1;
        if n + l > LINE_LEN {
            out.push_str("\n#");
            n = 2;
        }
        out.push_str(" +");
        out.push_str(&item.text);
        n += l;
    }
    if n > 1 {
        out.push('\n');
    }
}

/// `pj_pr_list`: the description of the projection, the parameters that were read, then those
/// that were not.
pub(super) fn parameter_list(job: &Job) -> String {
    let parameters = job.operation.parameters();
    let name = parameters
        .iter()
        .find_map(|parameter| parameter.text.strip_prefix("proj="))
        .unwrap_or("");
    let description = proj_rs::listing::PROJECTIONS
        .iter()
        .find(|entry| entry.id == name)
        .map_or("", |entry| entry.description);
    let mut out = String::from("#");
    for ch in description.chars() {
        out.push(ch);
        if ch == '\n' {
            out.push('#');
        }
    }
    out.push('\n');
    push_list(
        &mut out,
        parameters.iter().filter(|parameter| parameter.used),
    );
    if parameters.iter().any(|parameter| !parameter.used) {
        out.push_str("#--- following specified but NOT used\n");
        push_list(
            &mut out,
            parameters.iter().filter(|parameter| !parameter.used),
        );
    }
    out
}

/// The `-V` figure (`proj.cpp`, after the list): the ellipsoid, or the sphere of `+R`. Its
/// semi-major axis takes the `-f` format, and `%.3f` without one.
pub(super) fn figure(job: &Job, options: &Options) -> String {
    let (a, es) = job.operation.distance_ellipsoid();
    let number = options.oform.as_deref().unwrap_or("%.3f");
    if es == 0.0 {
        return format!(
            "#Final Earth figure: sphere\n#  Radius: {}\n",
            fmt(number, a)
        );
    }
    let flattening = 1.0 / (1.0 - (1.0 - es).sqrt());
    format!(
        "#Final Earth figure: ellipsoid\n#  Major axis (a): {}\n#  1/flattening: {}\n#  squared eccentricity: {}\n",
        fmt(number, a),
        fmt("%.6f", flattening),
        fmt("%.12f", es),
    )
}

/// `vprocess`: each line is read in the direction its first character asks for (`I` or `i`
/// reverses it; otherwise the direction of the command). The text after the two numbers is
/// printed before the block. A tag line is copied. A point that cannot be projected, or whose
/// factors cannot be computed, is reported on `errors` and skipped.
pub(super) fn verbose_lines(
    input: &[u8],
    options: &Options,
    job: &Job,
    out: &mut Vec<u8>,
    errors: &mut Vec<String>,
) {
    let mut pos = 0;
    while let Some(mut line) = super::next_line(input, &mut pos) {
        if line.last() != Some(&b'\n') {
            line.push(b'\n');
        }
        if line.first() == Some(&options.tag()) {
            out.extend_from_slice(&line);
            continue;
        }
        let (inverse, start) = match line.first() {
            Some(b'I' | b'i') => (true, 1),
            _ => (options.inverse, 0),
        };
        let reader: fn(&[u8]) -> (f64, usize) = if inverse { c_strtod } else { dmstor };
        let (first, second, rest) = read_pair(&line[start..], reader);
        let (u, v) = if options.reversal.input {
            (second, first)
        } else {
            (first, second)
        };
        let point = if inverse {
            let (x, y) = match options.prescale {
                Some(factor) => (u * factor, v * factor),
                None => (u, v),
            };
            job.unproject(x, y).map(|(lam, phi)| (lam, phi, x, y))
        } else {
            job.project(u, v).map(|(x, y)| {
                let (x, y) = match options.postscale {
                    Some(factor) => (x * factor, y * factor),
                    None => (x, y),
                };
                (u, v, x, y)
            })
        };
        let Ok((lam, phi, x, y)) = point else {
            errors.push("point cannot be projected".to_owned());
            continue;
        };
        let Some(block) = block(lam, phi, x, y, job, options) else {
            errors.push("failed to compute factors".to_owned());
            continue;
        };
        if rest != b"\n" {
            out.extend_from_slice(rest);
        }
        out.extend_from_slice(block.as_bytes());
    }
}

/// The two numbers at the start of a line, read by `read`, and what follows them. Inverse input
/// is plain numbers (`c_strtod`), forward input is degrees, minutes and seconds (`dmstor`).
fn read_pair(text: &[u8], read: fn(&[u8]) -> (f64, usize)) -> (f64, f64, &[u8]) {
    let (first, used) = read(text);
    let (second, more) = read(&text[used..]);
    (first, second, &text[used + more..])
}

/// The labelled block of one point, at longitude and latitude `lam`, `phi` (radians) and
/// coordinates `x`, `y`. `None` when the factors cannot be computed.
fn block(lam: f64, phi: f64, x: f64, y: f64, job: &Job, options: &Options) -> Option<String> {
    let factors = job.operation.factors(lam, phi).ok()?;
    let number = options
        .oform
        .as_deref()
        .unwrap_or(if options.inverse { "%.3f" } else { "%.2f" });
    let (x_label, y_label) = if job.north_first {
        ("Northing (y):  ", "Easting (x):   ")
    } else {
        ("Easting (x):   ", "Northing (y):  ")
    };
    let h = factors.meridional_scale;
    let k = factors.parallel_scale;
    let s = factors.areal_scale;
    Some(format!(
        "Longitude: {} [ {} ]\n\
         Latitude:  {} [ {} ]\n\
         {}{}\n\
         {}{}\n\
         Meridian scale (h) : {}  ( {} % error )\n\
         Parallel scale (k) : {}  ( {} % error )\n\
         Areal scale (s):     {}  ( {} % error )\n\
         Angular distortion (w): {}\n\
         Meridian/Parallel angle: {}\n\
         Convergence : {} [ {} ]\n\
         Max-min (Tissot axis a-b) scale error: {} {}\n\n",
        rtodms(lam, options.dms, b'E', b'W'),
        fmt("%.11g", lam * RAD_TO_DEG),
        rtodms(phi, options.dms, b'N', b'S'),
        fmt("%.11g", phi * RAD_TO_DEG),
        x_label,
        fmt(number, x),
        y_label,
        fmt(number, y),
        fmt("%.8f", h),
        fmt("%.4g", (h - 1.0) * 100.0),
        fmt("%.8f", k),
        fmt("%.4g", (k - 1.0) * 100.0),
        fmt("%.8f", s),
        fmt("%.4g", (s - 1.0) * 100.0),
        fmt("%.3f", factors.angular_distortion * RAD_TO_DEG),
        fmt("%.5f", factors.meridian_parallel_angle * RAD_TO_DEG),
        rtodms(factors.meridian_convergence, options.dms, 0, 0),
        fmt("%.8f", factors.meridian_convergence * RAD_TO_DEG),
        fmt("%.5f", factors.tissot_semimajor),
        fmt("%.5f", factors.tissot_semiminor),
    ))
}

/// A C format applied to one number, as `limited_fprintf_for_number` does.
fn fmt(spec: &str, value: f64) -> String {
    format_number(spec, value).unwrap_or_default()
}
