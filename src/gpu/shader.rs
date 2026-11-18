//! Assembles the WGSL of one precision: bindings, prelude, generated constants and polynomials,
//! and the interpreter. `df` is double-float (two `f32` per number), `f64` the native type.
//!
//! The interpreter is written once, with calls (`ad`, `sb`, `ml`, `dv`...) instead of operators;
//! the prelude of each precision gives those calls their meaning.

use super::program::Opcode;
use super::Precision;

/// A constant as an expression of the shader type `R`.
fn konst(precision: Precision, value: f64) -> String {
    match precision {
        Precision::Float64 => format!("({value:e}lf)"),
        Precision::DoubleFloat => {
            let (hi, lo) = split(value);
            format!("DF({hi:e}, {lo:e})")
        }
    }
}

/// `value` as `hi + lo`, two `f32`.
pub(crate) fn split(value: f64) -> (f32, f32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the split into two f32 is the point"
    )]
    let hi = value as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the split into two f32 is the point"
    )]
    let lo = (value - f64::from(hi)) as f32;
    (hi, lo)
}

/// `high + low` (a double-double) as three `f32` that add back to it to about 72 bits.
fn split3(high: f64, low: f64) -> (f32, f32, f32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the split into three f32 is the point"
    )]
    let first = high as f32;
    let rest = high - f64::from(first);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the split into three f32 is the point"
    )]
    let second = rest as f32;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "the split into three f32 is the point"
    )]
    let third = ((rest - f64::from(second)) + low) as f32;
    (first, second, third)
}

/// One `f32` part as an expression of the shader type `R`.
fn konst_part(precision: Precision, part: f32) -> String {
    match precision {
        Precision::Float64 => format!("({:e}lf)", f64::from(part)),
        Precision::DoubleFloat => format!("DF({part:e}, 0.0)"),
    }
}

fn factorial(n: u32) -> f64 {
    (1..=n).map(f64::from).product()
}

/// Horner: `q = c[m]; q = q * var + c[j]` for `j` from `m - 1` down to 0, as WGSL lines.
fn horner(precision: Precision, name: &str, coefficients: &[f64], var: &str) -> String {
    let last = coefficients.len() - 1;
    let mut out = format!(
        "    var {name} = {};\n",
        konst(precision, coefficients[last])
    );
    for j in (0..last).rev() {
        out += &format!(
            "    {name} = ad(ml({name}, {var}), {});\n",
            konst(precision, coefficients[j])
        );
    }
    out
}

fn generated(precision: Precision) -> String {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let mut s = String::new();
    for (opcode, name) in Opcode::ALL {
        s += &format!("const {name}: u32 = {}u;\n", opcode.code());
    }
    let named = [
        ("k_pi", PI),
        ("k_tau", TAU),
        ("k_halfpi", FRAC_PI_2),
        ("k_third", 1.0 / 3.0),
        ("k_fifth", 0.2),
        // adjlon leaves a longitude within 1e-12 beyond pi as it is.
        ("k_adj", PI + 1e-12),
        // Spherical Mercator: |phi| beyond this goes to the CPU. In double-float the latitude
        // is known to 48 bits, 5e-15 rad, and Mercator multiplies that by R / cos(phi): 4e-7 m at
        // 85 degrees, more beyond. Past 85.1 degrees the CPU answers, so the result stays within
        // a micrometre. In `f64` only the pole itself is out.
        (
            "k_merc_lim",
            match precision {
                Precision::DoubleFloat => 85.1_f64.to_radians(),
                Precision::Float64 => FRAC_PI_2 - 1e-9,
            },
        ),
        // Transverse Mercator: the limit of the easting angle, as `CE_LIMIT` in `tmerc.rs`.
        ("k_ce_limit", 2.623_395_162_778),
        ("k_eps10", 1e-10),
        // pj_sinhpsi2tanphi: a tau beyond 2 / sqrt(eps) is returned as it is.
        ("k_tmax", 2.0 / f64::EPSILON.sqrt()),
    ];
    for (name, value) in named {
        s += &format!(
            "fn {name}() -> R {{ return {}; }}\n",
            konst(precision, value)
        );
    }
    // pi/2 and ln 2 to about 72 bits, as three f32: the argument reduction subtracts `k` times
    // each part, so its constant does not add its own 5e-15 to the 48-bit input of the
    // double-float.
    for (name, high, low) in [
        (
            "pio2",
            1.570_796_326_794_896_6_f64,
            6.123_233_995_736_766e-17_f64,
        ),
        (
            "ln2",
            0.693_147_180_559_945_3_f64,
            2.319_046_813_846_299_6e-17_f64,
        ),
    ] {
        let (a, b, c) = split3(high, low);
        for (letter, part) in [("a", a), ("b", b), ("c", c)] {
            s += &format!(
                "fn k_{name}_{letter}() -> R {{ return {}; }}\n",
                konst_part(precision, part)
            );
        }
    }
    // sin r = r + r r2 (-1/3! + r2 (1/5! - ...)), cos r = 1 + r2 (-1/2! + r2 (1/4! - ...)).
    // Cut at r^17 and r^16 on |r| <= pi/4: less than 5e-17.
    let alternating = |first: u32| -> Vec<f64> {
        (0..8)
            .map(|j| {
                let sign = if j % 2 == 0 { -1.0 } else { 1.0 };
                sign / factorial(2 * j + first)
            })
            .collect()
    };
    s += "fn poly_sc(r: R) -> SC {\n    let r2 = ml(r, r);\n";
    s += &horner(precision, "qs", &alternating(3), "r2");
    s += &horner(precision, "qc", &alternating(2), "r2");
    s += "    return SC(ad(r, ml(ml(r, r2), qs)), ad(c(1.0), ml(r2, qc)));\n}\n";
    // exp r on |r| <= ln2 / 2, degree 13: cut at 4e-18.
    let exp_coefficients: Vec<f64> = (0..=13).map(|n| 1.0 / factorial(n)).collect();
    s += "fn poly_exp(r: R) -> R {\n";
    s += &horner(precision, "q", &exp_coefficients, "r");
    s += "    return q;\n}\n";
    s
}

/// The whole source of one precision. `canary` is only in the double-float one.
pub(crate) fn source(precision: Precision) -> String {
    let mut s = String::new();
    s += match precision {
        Precision::DoubleFloat => include_str!("wgsl/bindings_df.wgsl"),
        Precision::Float64 => include_str!("wgsl/bindings_f64.wgsl"),
    };
    s += match precision {
        Precision::DoubleFloat => include_str!("wgsl/prelude_df.wgsl"),
        Precision::Float64 => include_str!("wgsl/prelude_f64.wgsl"),
    };
    s += &generated(precision);
    s += precision_io(precision);
    s += include_str!("wgsl/interpreter.wgsl");
    if precision == Precision::DoubleFloat {
        s += include_str!("wgsl/canary_df.wgsl");
    }
    s
}

/// How a point is read from `IN` and written to `OUT`.
fn precision_io(precision: Precision) -> &'static str {
    match precision {
        Precision::DoubleFloat => {
            "fn load_a(i: u32) -> R { let p = IN[i]; return DF(p.x, p.y); }\n\
             fn load_b(i: u32) -> R { let p = IN[i]; return DF(p.z, p.w); }\n\
             fn store(i: u32, v: PR) { OUT[i] = vec4<f32>(v.a.hi, v.a.lo, v.b.hi, v.b.lo); }\n"
        }
        Precision::Float64 => {
            "fn load_a(i: u32) -> R { return IN[2u * i]; }\n\
             fn load_b(i: u32) -> R { return IN[2u * i + 1u]; }\n\
             fn store(i: u32, v: PR) { OUT[2u * i] = v.a; OUT[2u * i + 1u] = v.b; }\n"
        }
    }
}
