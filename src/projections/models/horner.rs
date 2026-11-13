//! Horner polynomial evaluation (`+proj=horner`), from PROJ 9.9.0
//! `transformations/horner.cpp` (the Poder/Engsager `gen_pol` scheme).
//!
//! A real polynomial pair maps `(e, n)` to `(E, N)`: `E` is the sum of
//! `fwd_u` (or `inv_u`) and `N` of `fwd_v` (or `inv_v`), each a double Horner
//! sum over the offset `(e, n)` from its origin. A complex polynomial
//! (`fwd_c`, `inv_c`) is the same scheme on `E + iN`. Without an inverse
//! polynomial the inverse is a Newton iteration on the forward one. The
//! coefficients are the comma-separated values of `+fwd_u=...`, and `+range` and
//! `+inv_tolerance` bound the domain and the iteration.
//!
//! Not ported: the PROJ `dual autocheck` (a PROJ-internal quality check that
//! `gen_pol` carries and PROJ itself does not call from the library).

use crate::engine::{Coord, CreateError, Frame, Params, Step, TransformError};

/// The coefficients of one horner transform.
#[derive(Clone)]
struct Polynomials {
    order: u32,
    range: f64,
    inverse_tolerance: f64,
    uneg: bool,
    vneg: bool,
    fwd_origin: (f64, f64),
    inv_origin: (f64, f64),
    real: Option<RealCoefficients>,
    complex: Option<ComplexCoefficients>,
    has_inv: bool,
}

#[derive(Clone)]
struct RealCoefficients {
    fwd_u: Vec<f64>,
    fwd_v: Vec<f64>,
    inv_u: Vec<f64>,
    inv_v: Vec<f64>,
}

#[derive(Clone)]
struct ComplexCoefficients {
    fwd_c: Vec<f64>,
    inv_c: Vec<f64>,
}

/// `+proj=horner` as a step.
#[derive(Clone)]
pub(crate) struct Horner {
    poly: Polynomials,
    a: f64,
    es: f64,
}

/// The parameters `horner` reads, for the unknown-parameter check.
pub(super) fn keys() -> &'static [&'static str] {
    &[
        "deg",
        "range",
        "inv_tolerance",
        // Read by no horner.cpp code: PROJ ignores it, so it is accepted here too.
        "tolerance",
        "fwd_origin",
        "inv_origin",
        "fwd_u",
        "fwd_v",
        "inv_u",
        "inv_v",
        "fwd_c",
        "inv_c",
        "uneg",
        "vneg",
    ]
}

/// Number of real coefficients of a polynomial of `order`: `(n+1)(n+2)/2`.
fn real_count(order: u32) -> usize {
    let n = order as usize;
    (n + 1) * (n + 2) / 2
}

/// Number of complex coefficient pairs: `2n+2` reals.
fn complex_count(order: u32) -> usize {
    2 * order as usize + 2
}

/// `parse_coefs` of `horner.cpp`: `None` when the key is absent. Each value is a
/// number, comma separated, and the list must hold `count` values.
fn coefficients(params: &Params, key: &str, count: usize) -> Result<Option<Vec<f64>>, CreateError> {
    let Some(text) = params.text(key) else {
        return Ok(None);
    };
    let mut values = Vec::with_capacity(count);
    for part in text.split(',').take(count) {
        let value: f64 = part
            .trim()
            .parse()
            .map_err(|_| CreateError::IllegalValue("horner coefficient"))?;
        values.push(value);
    }
    if values.len() != count {
        return Err(CreateError::IllegalValue("horner coefficient count"));
    }
    Ok(Some(values))
}

fn pair(params: &Params, key: &str) -> Result<Option<(f64, f64)>, CreateError> {
    Ok(coefficients(params, key, 2)?.map(|v| (v[0], v[1])))
}

pub(super) fn build(params: &Params, frame: &Frame) -> Result<Box<dyn Step>, CreateError> {
    let Some(degree) = params.integer("deg")? else {
        return Err(CreateError::MissingArgument("deg"));
    };
    if !(0..=10_000).contains(&degree) {
        return Err(CreateError::IllegalValue("deg"));
    }
    let order = u32::try_from(degree).map_err(|_| CreateError::IllegalValue("deg"))?;
    let complex = params.text("fwd_c").is_some() || params.text("inv_c").is_some();
    let has_inv = if complex {
        params.text("inv_c").is_some() || params.text("inv_origin").is_some()
    } else {
        params.text("inv_u").is_some()
            || params.text("inv_v").is_some()
            || params.text("inv_origin").is_some()
    };
    let (real, complex_coefs) = if complex {
        let n = complex_count(order);
        let fwd_c =
            coefficients(params, "fwd_c", n)?.ok_or(CreateError::MissingArgument("fwd_c"))?;
        let inv_c = if has_inv {
            coefficients(params, "inv_c", n)?.ok_or(CreateError::MissingArgument("inv_c"))?
        } else {
            vec![0.0; n]
        };
        (None, Some(ComplexCoefficients { fwd_c, inv_c }))
    } else {
        let n = real_count(order);
        let fwd_u =
            coefficients(params, "fwd_u", n)?.ok_or(CreateError::MissingArgument("fwd_u"))?;
        let fwd_v =
            coefficients(params, "fwd_v", n)?.ok_or(CreateError::MissingArgument("fwd_v"))?;
        let (inv_u, inv_v) = if has_inv {
            (
                coefficients(params, "inv_u", n)?.ok_or(CreateError::MissingArgument("inv_u"))?,
                coefficients(params, "inv_v", n)?.ok_or(CreateError::MissingArgument("inv_v"))?,
            )
        } else {
            (vec![0.0; n], vec![0.0; n])
        };
        (
            Some(RealCoefficients {
                fwd_u,
                fwd_v,
                inv_u,
                inv_v,
            }),
            None,
        )
    };
    let fwd_origin =
        pair(params, "fwd_origin")?.ok_or(CreateError::MissingArgument("fwd_origin"))?;
    let inv_origin = if has_inv {
        pair(params, "inv_origin")?.ok_or(CreateError::MissingArgument("inv_origin"))?
    } else {
        (0.0, 0.0)
    };
    let range = params.number("range")?.unwrap_or(500_000.0);
    let inverse_tolerance = params.number("inv_tolerance")?.unwrap_or(0.001);
    let poly = Polynomials {
        order,
        range,
        inverse_tolerance,
        uneg: params.flag("uneg"),
        vneg: params.flag("vneg"),
        fwd_origin,
        inv_origin,
        real,
        complex: complex_coefs,
        has_inv,
    };
    Ok(Box::new(Horner {
        poly,
        a: frame.earth.a(),
        es: frame.earth.es(),
    }))
}

/// Double Horner sum of `gen_pol`: the real pair `(E, N)` at offset `(e, n)`.
/// `order_offset` skips the lowest orders, as the iterative inverse needs.
fn double_real_eval(
    order: u32,
    cx: &[f64],
    cy: &[f64],
    e: f64,
    n: f64,
    order_offset: u32,
) -> (f64, f64) {
    let sz = real_count(order);
    let mut ix = sz;
    let mut iy = sz;
    iy -= 1;
    let mut big_n = cy[iy];
    ix -= 1;
    let mut big_e = cx[ix];
    let mut r = order;
    while r > order_offset {
        iy -= 1;
        let mut u = cy[iy];
        ix -= 1;
        let mut v = cx[ix];
        for _c in (r..=order).rev() {
            iy -= 1;
            u = n * u + cy[iy];
            ix -= 1;
            v = e * v + cx[ix];
        }
        big_n = e * big_n + u;
        big_e = n * big_e + v;
        r -= 1;
    }
    (big_e, big_n)
}

/// One-dimensional Horner sum of `gen_pol` on the coefficients `cx` at `x`.
fn single_real_eval(order: u32, cx: &[f64], x: f64, order_offset: u32) -> f64 {
    let sz = order as usize + 1;
    let mut ix = sz;
    ix -= 1;
    let mut u = cx[ix];
    let mut r = order;
    while r > order_offset {
        ix -= 1;
        u = x * u + cx[ix];
        r -= 1;
    }
    u
}

/// Complex Horner sum: coefficients are `(Cn0 + i Ce0, Cn1 + i Ce1, ...)`.
fn complex_eval(order: u32, c: &[f64], e: f64, n: f64, order_offset: u32) -> (f64, f64) {
    let sz = complex_count(order);
    let beg = order_offset as usize * 2;
    let mut idx = sz;
    idx -= 1;
    let mut big_e = c[idx];
    idx -= 1;
    let mut big_n = c[idx];
    while idx > beg {
        idx -= 1;
        let w = n * big_e + e * big_n + c[idx];
        idx -= 1;
        big_n = n * big_n - e * big_e + c[idx];
        big_e = w;
    }
    (big_e, big_n)
}

/// Complex division `a / b`, textbook form.
fn cdiv(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

impl Polynomials {
    fn in_range(&self, n: f64, e: f64) -> Result<(), TransformError> {
        if n.abs() > self.range || e.abs() > self.range {
            Err(TransformError::OutsideDomain)
        } else {
            Ok(())
        }
    }

    /// `real_default_impl` and `complex_default_impl` (the direct polynomial).
    fn direct(&self, forward: bool, u: f64, v: f64) -> Result<(f64, f64), TransformError> {
        let origin = if forward {
            self.fwd_origin
        } else {
            self.inv_origin
        };
        let mut e = u - origin.0;
        let mut n = v - origin.1;
        if let Some(c) = &self.complex {
            if self.uneg {
                e = -e;
            }
            if self.vneg {
                n = -n;
            }
            self.in_range(n, e)?;
            let coefs = if forward { &c.fwd_c } else { &c.inv_c };
            return Ok(complex_eval(self.order, coefs, e, n, 0));
        }
        self.in_range(n, e)?;
        let Some(r) = &self.real else {
            return Err(TransformError::NoInverse);
        };
        let (cx, cy) = if forward {
            (&r.fwd_u, &r.fwd_v)
        } else {
            (&r.inv_u, &r.inv_v)
        };
        Ok(double_real_eval(self.order, cx, cy, e, n, 0))
    }

    /// `real_iterative_inverse_impl`: Newton on the forward polynomial.
    fn iterative_real(&self, u: f64, v: f64) -> Result<(f64, f64), TransformError> {
        self.in_range(v, u)?;
        let r = self.real.as_ref().ok_or(TransformError::NoInverse)?;
        let order = self.order;
        let tol = self.inverse_tolerance;
        let de = u - r.fwd_u[0];
        let dn = v - r.fwd_v[0];
        let (mut x0, mut y0) = (0.0_f64, 0.0_f64);
        let mut loops = 32;
        let mut converged = false;
        while loops > 0 && !converged {
            loops -= 1;
            let (mb, mc) = double_real_eval(order, &r.fwd_u, &r.fwd_v, x0, y0, 1);
            let ma = single_real_eval(order, &r.fwd_u, x0, 1);
            let md = single_real_eval(order, &r.fwd_v, y0, 1);
            let idet = 1.0 / (ma * md - mb * mc);
            let x = idet * (md * de - mb * dn);
            let y = idet * (ma * dn - mc * de);
            converged = (x - x0).abs() < tol && (y - y0).abs() < tol;
            x0 = x;
            y0 = y;
        }
        if !converged {
            return Err(TransformError::InvalidCoord);
        }
        Ok((x0 + self.fwd_origin.0, y0 + self.fwd_origin.1))
    }

    /// `complex_iterative_inverse_impl`: Newton on the complex forward polynomial.
    fn iterative_complex(&self, u: f64, v: f64) -> Result<(f64, f64), TransformError> {
        self.in_range(v, u)?;
        let c = self.complex.as_ref().ok_or(TransformError::NoInverse)?;
        let tol = self.inverse_tolerance;
        // Real part is northing, imaginary part easting.
        let dz = (v - c.fwd_c[0], u - c.fwd_c[1]);
        let mut w0 = (0.0_f64, 0.0_f64);
        let mut loops = 32;
        let mut converged = false;
        while loops > 0 && !converged {
            loops -= 1;
            // The argument is (imag w0, real w0), as in the C++ code.
            let (det_e, det_n) = complex_eval(self.order, &c.fwd_c, w0.1, w0.0, 1);
            let w1 = cdiv(dz, (det_n, det_e));
            converged = (w1.0 - w0.0).abs() < tol && (w1.1 - w0.1).abs() < tol;
            w0 = w1;
        }
        if !converged {
            return Err(TransformError::InvalidCoord);
        }
        let mut big_e = w0.1;
        let mut big_n = w0.0;
        if self.uneg {
            big_e = -big_e;
        }
        if self.vneg {
            big_n = -big_n;
        }
        Ok((big_e + self.fwd_origin.0, big_n + self.fwd_origin.1))
    }
}

impl Step for Horner {
    fn forward(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.x.is_finite() || !coord.y.is_finite() {
            return Err(TransformError::NonFinite);
        }
        let (x, y) = self.poly.direct(true, coord.x, coord.y)?;
        Ok(Coord {
            x,
            y,
            z: coord.z,
            t: coord.t,
        })
    }

    fn inverse(&self, coord: Coord) -> Result<Coord, TransformError> {
        if !coord.x.is_finite() || !coord.y.is_finite() {
            return Err(TransformError::NonFinite);
        }
        let (x, y) = if self.poly.has_inv {
            self.poly.direct(false, coord.x, coord.y)?
        } else if self.poly.complex.is_some() {
            self.poly.iterative_complex(coord.x, coord.y)?
        } else {
            self.poly.iterative_real(coord.x, coord.y)?
        };
        Ok(Coord {
            x,
            y,
            z: coord.z,
            t: coord.t,
        })
    }

    fn input_kind(&self) -> crate::engine::IoKind {
        crate::engine::IoKind::Any
    }

    fn output_kind(&self) -> crate::engine::IoKind {
        crate::engine::IoKind::Any
    }

    fn distance_ellipsoid(&self) -> (f64, f64) {
        (self.a, self.es)
    }
}
