//! Assemble les sources WGSL : prélude du chemin, constantes et polynômes générés, noyaux communs.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Path {
    /// Deux f32 par nombre, partout.
    Df,
    /// `f64` natif, `Features::SHADER_F64`.
    F64,
}

impl Path {
    pub fn name(self) -> &'static str {
        match self {
            Self::Df => "df",
            Self::F64 => "f64",
        }
    }
}

/// Une constante f64 écrite comme expression de type `R` du chemin.
fn konst(path: Path, v: f64) -> String {
    match path {
        Path::F64 => format!("({v:e}lf)"),
        Path::Df => {
            #[expect(clippy::cast_possible_truncation, reason = "découpage hi/lo voulu")]
            let hi = v as f32;
            #[expect(clippy::cast_possible_truncation, reason = "découpage hi/lo voulu")]
            let lo = (v - f64::from(hi)) as f32;
            format!("DF({hi:e}, {lo:e})")
        }
    }
}

fn factorial(n: u32) -> f64 {
    (1..=n).map(f64::from).product()
}

/// Horner `q = c[m]; q = q * var + c[j]` pour j de m-1 à 0, en lignes de WGSL.
fn horner(path: Path, name: &str, coefficients: &[f64], var: &str) -> String {
    let last = coefficients.len() - 1;
    let mut out = format!("    var {name} = {};\n", konst(path, coefficients[last]));
    for j in (0..last).rev() {
        out += &format!(
            "    {name} = ad(ml({name}, {var}), {});\n",
            konst(path, coefficients[j])
        );
    }
    out
}

fn generated(path: Path) -> String {
    use std::f64::consts::{FRAC_PI_2, LN_2};
    let mut s = String::new();
    s += &format!(
        "fn k_pio2() -> R {{ return {}; }}\n",
        konst(path, FRAC_PI_2)
    );
    s += &format!("fn k_ln2() -> R {{ return {}; }}\n", konst(path, LN_2));
    s += &format!(
        "fn k_third() -> R {{ return {}; }}\n",
        konst(path, 1.0 / 3.0)
    );
    s += &format!("fn k_fifth() -> R {{ return {}; }}\n", konst(path, 0.2));
    // sin r = r + r r² (-1/3! + r² (1/5! - ...)) ; cos r = 1 + r² (-1/2! + r² (1/4! - ...)).
    // Troncature à r^17 et r^16 sur |r| <= pi/4 : moins de 5e-17.
    let sin_c: Vec<f64> = (0..8)
        .map(|j| {
            let sign = if j % 2 == 0 { -1.0 } else { 1.0 };
            sign / factorial(2 * j + 3)
        })
        .collect();
    let cos_c: Vec<f64> = (0..8)
        .map(|j| {
            let sign = if j % 2 == 0 { -1.0 } else { 1.0 };
            sign / factorial(2 * j + 2)
        })
        .collect();
    s += "fn poly_sc(r: R) -> SC {\n    let r2 = ml(r, r);\n";
    s += &horner(path, "qs", &sin_c, "r2");
    s += &horner(path, "qc", &cos_c, "r2");
    s += "    return SC(ad(r, ml(ml(r, r2), qs)), ad(c(1.0), ml(r2, qc)));\n}\n";
    // exp r, |r| <= ln2 / 2, degré 13 : troncature 4e-18.
    let exp_c: Vec<f64> = (0..=13).map(|n| 1.0 / factorial(n)).collect();
    s += "fn poly_exp(r: R) -> R {\n";
    s += &horner(path, "q", &exp_c, "r");
    s += "    return q;\n}\n";
    s
}

/// Source complète d'un chemin. `canary` ajoute le contrôle des transformations sans erreur (df).
pub fn source(path: Path) -> String {
    let mut s = String::new();
    s += match path {
        Path::Df => include_str!("bindings_df.wgsl"),
        Path::F64 => include_str!("bindings_f64.wgsl"),
    };
    s += match path {
        Path::Df => include_str!("prelude_df.wgsl"),
        Path::F64 => include_str!("prelude_f64.wgsl"),
    };
    s += &generated(path);
    s += include_str!("kernels.wgsl");
    if path == Path::Df {
        s += include_str!("canary_df.wgsl");
    }
    s
}
