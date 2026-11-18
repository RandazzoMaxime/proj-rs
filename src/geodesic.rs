//! Karney's geodesic algorithms in Rust, ported from the C library PROJ 9.9 vendors
//! (`geodesic.c`, `geodesic.h`).
//!
//! The algorithms are described in C. F. F. Karney, *Algorithms for geodesics*, J. Geodesy
//! 87, 43-55 (2013), <https://doi.org/10.1007/s00190-012-0578-z>.
//!
//! This is a port of the C implementation, Copyright (c) Charles Karney (2012-2025)
//! <karney@alum.mit.edu>, licensed under the MIT/X11 License; see
//! <https://geographiclib.sourceforge.io/> and `NOTICE.md`.
//!
//! The area (`S12`) and the polygon functions are not ported: nothing here asks for them. The
//! results are the same bits as the C library's on the same C math library (`docs/PORTING-geodesic.md`).
//!
//! The ellipsoid is `a` and the flattening `f`; angles are degrees, lengths are in the unit of
//! `a`. The ellipsoid of `gnom` and `aeqd` is the unit sphere scaled by the flattening only
//! (`a = 1`); callers multiply the lengths by the real semi-major axis.

use std::f64::consts::PI;

const QD: f64 = 90.0;
const HD: f64 = 180.0;
const TD: f64 = 360.0;
/// `pi / hd` of `Init` in `geodesic.c`.
const DEGREE: f64 = PI / HD;
const MAXIT1: u32 = 20;
/// `maxit1 + digits + 10`, `digits` being `DBL_MANT_DIG`.
const MAXIT2: u32 = MAXIT1 + 53 + 10;
const TOL0: f64 = f64::EPSILON;
const TOLB: f64 = TOL0;
/// Order of the series.
const N_SERIES: usize = 6;
/// Length of the coefficient arrays: index 0 is unused by the sine series.
const NC: usize = N_SERIES + 1;
const NC3X: usize = (N_SERIES * (N_SERIES - 1)) / 2;

const CAP_C1: u32 = 1 << 0;
const CAP_C1P: u32 = 1 << 1;
const CAP_C2: u32 = 1 << 2;
const CAP_C3: u32 = 1 << 3;
const OUT_ALL: u32 = 0x7F80;

// The `geod_mask` of `geodesic.h`: a high bit says what is computed, the low ones which series
// it needs.
pub(crate) const GEOD_LATITUDE: u32 = 1 << 7;
pub(crate) const GEOD_LONGITUDE: u32 = (1 << 8) | CAP_C3;
pub(crate) const GEOD_AZIMUTH: u32 = 1 << 9;
pub(crate) const GEOD_DISTANCE: u32 = (1 << 10) | CAP_C1;
pub(crate) const GEOD_DISTANCE_IN: u32 = (1 << 11) | CAP_C1 | CAP_C1P;
pub(crate) const GEOD_REDUCEDLENGTH: u32 = (1 << 12) | CAP_C1 | CAP_C2;
pub(crate) const GEOD_GEODESICSCALE: u32 = (1 << 13) | CAP_C1 | CAP_C2;

/// `GEOD_LONG_UNROLL` of `geod_flags`.
const GEOD_LONG_UNROLL: u32 = 1 << 15;
/// `GEOD_ARCMODE`.
const GEOD_ARCMODE: u32 = 1 << 0;

fn tiny() -> f64 {
    f64::MIN_POSITIVE.sqrt()
}

fn tol1() -> f64 {
    200.0 * TOL0
}

fn tol2() -> f64 {
    TOL0.sqrt()
}

fn xthresh() -> f64 {
    1000.0 * tol2()
}

fn sq(x: f64) -> f64 {
    x * x
}

/// Error-free sum: `u + v = s + t`.
fn sumx(u: f64, v: f64) -> (f64, f64) {
    let s = u + v;
    let mut up = s - v;
    let mut vpp = s - up;
    up -= u;
    vpp -= v;
    let t = if s != 0.0 { 0.0 - (up + vpp) } else { s };
    (s, t)
}

/// Horner: `p[0] x^n + p[1] x^(n-1) + .. + p[n]`, zero for a negative `n`.
fn polyvalx(n: i32, p: &[f64], x: f64) -> f64 {
    let mut index = 0;
    let mut y = if n < 0 {
        0.0
    } else {
        index = 1;
        p[0]
    };
    let mut left = n;
    loop {
        left -= 1;
        if left < 0 {
            break;
        }
        y = y * x + p[index];
        index += 1;
    }
    y
}

fn norm2(sinx: &mut f64, cosx: &mut f64) {
    let r = sinx.hypot(*cosx);
    *sinx /= r;
    *cosx /= r;
}

/// IEEE `remainder(x, 360)`: `x - 360 n` with `n` the integer nearest `x / 360`, a tie going to
/// the even one. Exact: `fmod` by 720 is exact, and the correction is a subtraction of operands
/// within a factor of two.
fn remainder_turn(x: f64) -> f64 {
    // A NaN or an infinity goes through the arithmetic as it does in the C library: `fmod` gives
    // the NaN, and its sign, which `signbit` reads further on.
    let r2 = x % (2.0 * TD);
    let a = r2.abs();
    let n2 = if a <= TD / 2.0 {
        0.0
    } else if a < 3.0 * TD / 2.0 {
        1.0
    } else {
        2.0
    };
    let r = r2 - (n2 * TD).copysign(r2);
    if r == 0.0 {
        0.0_f64.copysign(x)
    } else {
        r
    }
}

/// `remquo(x, 90)`: the remainder `x - 90 n` (`n` nearest `x / 90`, a tie to the even one) and
/// the two low bits of `n`. Exact for the same reason as [`remainder_turn`].
fn remquo_quarter(x: f64) -> (f64, u32) {
    let r2 = x % TD;
    let a = r2.abs();
    // The boundary between n = k and n = k + 1 is 45 + 90 k; on it the even one of the two.
    let mut n2 = 0_u32;
    for k in 0..4_u32 {
        let boundary = QD / 2.0 + QD * f64::from(k);
        if a > boundary || (a == boundary && k % 2 == 1) {
            n2 = k + 1;
        }
    }
    let r = r2 - (QD * f64::from(n2)).copysign(r2);
    let r = if r == 0.0 { 0.0_f64.copysign(x) } else { r };
    // `n = 4 m + sign n2`: the low two bits come from the signed `n2`.
    let low = if r2.is_sign_negative() {
        (4 - n2 % 4) % 4
    } else {
        n2 % 4
    };
    (r, low)
}

fn ang_normalize(x: f64) -> f64 {
    let y = remainder_turn(x);
    if y.abs() == HD {
        HD.copysign(x)
    } else {
        y
    }
}

fn lat_fix(x: f64) -> f64 {
    if x.abs() > QD {
        f64::NAN
    } else {
        x
    }
}

/// `y - x` in `[-180, 180]` and its error `e`.
fn ang_diff(x: f64, y: f64) -> (f64, f64) {
    let (mut d, mut t) = sumx(remainder_turn(-x), remainder_turn(y));
    (d, t) = sumx(remainder_turn(d), t);
    if d == 0.0 || d.abs() == HD {
        d = d.copysign(if t == 0.0 { y - x } else { -t });
    }
    (d, t)
}

fn ang_round(x: f64) -> f64 {
    let z = 1.0 / 16.0;
    let mut y = x.abs();
    let w = z - y;
    y = if w > 0.0 { z - w } else { y };
    y.copysign(x)
}

/// Sine and cosine of `r` degrees, the argument reduced exactly to `[-45, 45]`.
fn sincos_quadrant(r: f64, q: u32, x: f64) -> (f64, f64) {
    let r = r * DEGREE;
    let (s, c) = (r.sin(), r.cos());
    let (mut sinx, mut cosx) = match q & 3 {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    cosx += 0.0;
    if sinx == 0.0 {
        sinx = sinx.copysign(x);
    }
    (sinx, cosx)
}

fn sincosdx(x: f64) -> (f64, f64) {
    let (r, q) = remquo_quarter(x);
    sincos_quadrant(r, q, x)
}

fn sincosde(x: f64, t: f64) -> (f64, f64) {
    let (r, q) = remquo_quarter(x);
    sincos_quadrant(ang_round(r + t), q, x)
}

fn atan2dx(y: f64, x: f64) -> f64 {
    let (mut x, mut y) = (x, y);
    let mut q = 0;
    if y.abs() > x.abs() {
        std::mem::swap(&mut x, &mut y);
        q = 2;
    }
    if x.is_sign_negative() {
        x = -x;
        q += 1;
    }
    let mut ang = y.atan2(x) / DEGREE;
    match q {
        1 => ang = HD.copysign(y) - ang,
        2 => ang = QD - ang,
        3 => ang = -QD + ang,
        _ => {}
    }
    ang
}

/// `y = sum(c[i] sin(2 i x))` for `i` from 1 to `n`, by Clenshaw summation. `c[0]` is unused.
fn sin_series(sinx: f64, cosx: f64, c: &[f64], n: usize) -> f64 {
    let mut index = n + 1;
    let ar = 2.0 * (cosx - sinx) * (cosx + sinx);
    let mut y0 = if n & 1 == 1 {
        index -= 1;
        c[index]
    } else {
        0.0
    };
    let mut y1 = 0.0;
    for _ in 0..n / 2 {
        index -= 1;
        y1 = ar * y0 - y1 + c[index];
        index -= 1;
        y0 = ar * y1 - y0 + c[index];
    }
    2.0 * sinx * cosx * y0
}

/// The ellipsoid and the series that depend only on it.
pub(crate) struct Geodesic {
    a: f64,
    f: f64,
    f1: f64,
    ep2: f64,
    n: f64,
    b: f64,
    etol2: f64,
    a3x: [f64; N_SERIES],
    c3x: [f64; NC3X],
}

/// A geodesic line: what depends on the start point and the azimuth.
pub(crate) struct GeodesicLine {
    lon1: f64,
    b: f64,
    f: f64,
    f1: f64,
    salp0: f64,
    calp0: f64,
    k2: f64,
    ssig1: f64,
    csig1: f64,
    dn1: f64,
    stau1: f64,
    ctau1: f64,
    somg1: f64,
    comg1: f64,
    a1m1: f64,
    a2m1: f64,
    a3c: f64,
    b11: f64,
    b21: f64,
    b31: f64,
    c1a: [f64; NC],
    c1pa: [f64; NC],
    c2a: [f64; NC],
    c3a: [f64; N_SERIES],
    caps: u32,
}

/// What `genposition` computed, for the outputs it was asked for and the line allows.
pub(crate) struct Position {
    /// The outputs actually computed (high bits of `GEOD_*`).
    pub(crate) computed: u32,
    pub(crate) lat2: f64,
    pub(crate) lon2: f64,
    pub(crate) azi2: f64,
    pub(crate) s12: f64,
    pub(crate) m12: f64,
    pub(crate) big_m12: f64,
    pub(crate) big_m21: f64,
}

/// The unit vectors and lengths of an inverse solution, in the frame of the points as given.
struct Inverse {
    salp1: f64,
    calp1: f64,
    salp2: f64,
    calp2: f64,
    s12: f64,
    m12: f64,
    big_m12: f64,
}

impl Geodesic {
    /// `geod_init(&g, a, f)`.
    pub(crate) fn new(a: f64, f: f64) -> Self {
        let f1 = 1.0 - f;
        let e2 = f * (2.0 - f);
        let ep2 = e2 / sq(f1);
        let n = f / (2.0 - f);
        let b = a * f1;
        // The sig12 threshold for "really short" (see `geodesic.c`).
        let etol2 =
            0.1 * tol2() / (0.001_f64.max(f.abs()) * 1.0_f64.min(1.0 - f / 2.0) / 2.0).sqrt();
        let mut g = Self {
            a,
            f,
            f1,
            ep2,
            n,
            b,
            etol2,
            a3x: [0.0; N_SERIES],
            c3x: [0.0; NC3X],
        };
        g.a3_coefficients();
        g.c3_coefficients();
        g
    }

    /// Unit ellipsoid of flattening `f`, `geod_init(&g, 1, f)`.
    pub(crate) fn unit(f: f64) -> Self {
        Self::new(1.0, f)
    }

    /// The scale factor `A3`: a polynomial in `n` for each power of `eps`.
    fn a3_coefficients(&mut self) {
        const COEFF: [f64; 18] = [
            // A3, coeff of eps^5, polynomial in n of order 0
            -3.0, 128.0, // eps^4, order 1
            -2.0, -3.0, 64.0, // eps^3, order 2
            -1.0, -3.0, -1.0, 16.0, // eps^2, order 2
            3.0, -1.0, -2.0, 8.0, // eps^1, order 1
            1.0, -1.0, 2.0, // eps^0, order 0
            1.0, 1.0,
        ];
        let mut o = 0;
        for (k, j) in (0..N_SERIES).rev().enumerate() {
            let m = if N_SERIES - j - 1 < j {
                N_SERIES - j - 1
            } else {
                j
            };
            let degree = i32::try_from(m).expect("order of a polynomial");
            self.a3x[k] = polyvalx(degree, &COEFF[o..], self.n) / COEFF[o + m + 1];
            o += m + 2;
        }
    }

    /// The coefficients `C3[l]` of the Fourier expansion of `B3`.
    fn c3_coefficients(&mut self) {
        const COEFF: [f64; 45] = [
            // C3[1], eps^5, order 0
            3.0, 128.0, // eps^4, order 1
            2.0, 5.0, 128.0, // eps^3, order 2
            -1.0, 3.0, 3.0, 64.0, // eps^2, order 2
            -1.0, 0.0, 1.0, 8.0, // eps^1, order 1
            -1.0, 1.0, 4.0, // C3[2], eps^5, order 0
            5.0, 256.0, // eps^4, order 1
            1.0, 3.0, 128.0, // eps^3, order 2
            -3.0, -2.0, 3.0, 64.0, // eps^2, order 2
            1.0, -3.0, 2.0, 32.0, // C3[3], eps^5, order 0
            7.0, 512.0, // eps^4, order 1
            -10.0, 9.0, 384.0, // eps^3, order 2
            5.0, -9.0, 5.0, 192.0, // C3[4], eps^5, order 0
            7.0, 512.0, // eps^4, order 1
            -14.0, 7.0, 512.0, // C3[5], eps^5, order 0
            21.0, 2560.0,
        ];
        let mut o = 0;
        let mut k = 0;
        for l in 1..N_SERIES {
            for j in (l..N_SERIES).rev() {
                let m = if N_SERIES - j - 1 < j {
                    N_SERIES - j - 1
                } else {
                    j
                };
                let degree = i32::try_from(m).expect("order of a polynomial");
                self.c3x[k] = polyvalx(degree, &COEFF[o..], self.n) / COEFF[o + m + 1];
                k += 1;
                o += m + 2;
            }
        }
    }

    fn a3f(&self, eps: f64) -> f64 {
        polyvalx(i32::try_from(N_SERIES).expect("order") - 1, &self.a3x, eps)
    }

    /// Elements `c[1]` to `c[N_SERIES - 1]`.
    fn c3f(&self, eps: f64, c: &mut [f64]) {
        let mut mult = 1.0;
        let mut o = 0;
        for l in 1..N_SERIES {
            let m = N_SERIES - l - 1;
            mult *= eps;
            let degree = i32::try_from(m).expect("order of a polynomial");
            c[l] = mult * polyvalx(degree, &self.c3x[o..], eps);
            o += m + 1;
        }
    }
}

/// `A1 - 1`, the mean of `(d/dsigma) I1 - 1`.
fn a1m1f(eps: f64) -> f64 {
    const COEFF: [f64; 5] = [1.0, 4.0, 64.0, 0.0, 256.0];
    let m = N_SERIES / 2;
    let t = polyvalx(i32::try_from(m).expect("order"), &COEFF, sq(eps)) / COEFF[m + 1];
    (t + eps) / (1.0 - eps)
}

/// `A2 - 1`.
fn a2m1f(eps: f64) -> f64 {
    const COEFF: [f64; 5] = [-11.0, -28.0, -192.0, 0.0, 256.0];
    let m = N_SERIES / 2;
    let t = polyvalx(i32::try_from(m).expect("order"), &COEFF, sq(eps)) / COEFF[m + 1];
    (t - eps) / (1.0 + eps)
}

/// The coefficients of one Fourier expansion, `c[l] = eps^l P_l(eps^2) / d_l`.
fn fourier(eps: f64, coeff: &[f64], c: &mut [f64]) {
    let eps2 = sq(eps);
    let mut d = eps;
    let mut o = 0;
    for l in 1..=N_SERIES {
        let m = (N_SERIES - l) / 2;
        let degree = i32::try_from(m).expect("order of a polynomial");
        c[l] = d * polyvalx(degree, &coeff[o..], eps2) / coeff[o + m + 1];
        o += m + 2;
        d *= eps;
    }
}

/// `C1[l]`, the Fourier expansion of `B1`.
fn c1f(eps: f64, c: &mut [f64]) {
    const COEFF: [f64; 18] = [
        -1.0, 6.0, -16.0, 32.0, -9.0, 64.0, -128.0, 2048.0, 9.0, -16.0, 768.0, 3.0, -5.0, 512.0,
        -7.0, 1280.0, -7.0, 2048.0,
    ];
    fourier(eps, &COEFF, c);
}

/// `C1p[l]`, the Fourier expansion of `B1p`.
fn c1pf(eps: f64, c: &mut [f64]) {
    const COEFF: [f64; 18] = [
        205.0, -432.0, 768.0, 1536.0, 4005.0, -4736.0, 3840.0, 12288.0, -225.0, 116.0, 384.0,
        -7173.0, 2695.0, 7680.0, 3467.0, 7680.0, 38081.0, 61440.0,
    ];
    fourier(eps, &COEFF, c);
}

/// `C2[l]`, the Fourier expansion of `B2`.
fn c2f(eps: f64, c: &mut [f64]) {
    const COEFF: [f64; 18] = [
        1.0, 2.0, 16.0, 32.0, 35.0, 64.0, 384.0, 2048.0, 15.0, 80.0, 768.0, 7.0, 35.0, 512.0, 63.0,
        1280.0, 77.0, 2048.0,
    ];
    fourier(eps, &COEFF, c);
}

/// Which results `lengths` is asked for.
const WANT_S12B: u8 = 1;
const WANT_M12B: u8 = 2;
const WANT_M0: u8 = 4;
const WANT_M12: u8 = 8;
const WANT_M21: u8 = 16;

#[derive(Default)]
struct Lengths {
    s12b: f64,
    m12b: f64,
    m0: f64,
    big_m12: f64,
    big_m21: f64,
}

/// A point of the geodesic on the auxiliary sphere: `sin` and `cos` of `sigma`, and `dn`.
#[derive(Clone, Copy)]
struct Sig {
    s: f64,
    c: f64,
    dn: f64,
}

impl Geodesic {
    /// `Lengths` of `geodesic.c`: reduced length and distance (over `b`), and the scales.
    fn lengths(
        &self,
        eps: f64,
        sig12: f64,
        p1: Sig,
        p2: Sig,
        cbet: (f64, f64),
        want: u8,
        ca: &mut [f64; NC],
    ) -> Lengths {
        let (ssig1, csig1, dn1) = (p1.s, p1.c, p1.dn);
        let (ssig2, csig2, dn2) = (p2.s, p2.c, p2.dn);
        let (cbet1, cbet2) = cbet;
        let mut out = Lengths::default();
        let mut m0 = 0.0;
        let mut j12 = 0.0;
        let mut a1 = 0.0;
        let mut a2 = 0.0;
        let mut cb = [0.0; NC];
        let redlp = want & (WANT_M12B | WANT_M0 | WANT_M12 | WANT_M21) != 0;
        let want_s12b = want & WANT_S12B != 0;
        if want_s12b || redlp {
            a1 = a1m1f(eps);
            c1f(eps, ca);
            if redlp {
                a2 = a2m1f(eps);
                c2f(eps, &mut cb);
                m0 = a1 - a2;
                a2 = 1.0 + a2;
            }
            a1 = 1.0 + a1;
        }
        if want_s12b {
            let b1 =
                sin_series(ssig2, csig2, ca, N_SERIES) - sin_series(ssig1, csig1, ca, N_SERIES);
            out.s12b = a1 * (sig12 + b1);
            if redlp {
                let b2 = sin_series(ssig2, csig2, &cb, N_SERIES)
                    - sin_series(ssig1, csig1, &cb, N_SERIES);
                j12 = m0 * sig12 + (a1 * b1 - a2 * b2);
            }
        } else if redlp {
            for l in 1..=N_SERIES {
                cb[l] = a1 * ca[l] - a2 * cb[l];
            }
            j12 = m0 * sig12
                + (sin_series(ssig2, csig2, &cb, N_SERIES)
                    - sin_series(ssig1, csig1, &cb, N_SERIES));
        }
        out.m0 = m0;
        if want & WANT_M12B != 0 {
            out.m12b = dn2 * (csig1 * ssig2) - dn1 * (ssig1 * csig2) - csig1 * csig2 * j12;
        }
        if want & (WANT_M12 | WANT_M21) != 0 {
            let csig12 = csig1 * csig2 + ssig1 * ssig2;
            let t = self.ep2 * (cbet1 - cbet2) * (cbet1 + cbet2) / (dn1 + dn2);
            if want & WANT_M12 != 0 {
                out.big_m12 = csig12 + (t * ssig2 - csig2 * j12) * ssig1 / dn1;
            }
            if want & WANT_M21 != 0 {
                out.big_m21 = csig12 - (t * ssig1 - csig1 * j12) * ssig2 / dn2;
            }
        }
        out
    }
}

/// The positive root of `k^4 + 2 k^3 - (x^2 + y^2 - 1) k^2 - 2 y^2 k - y^2 = 0`.
fn astroid(x: f64, y: f64) -> f64 {
    let p = sq(x);
    let q = sq(y);
    let r = (p + q - 1.0) / 6.0;
    if !(q == 0.0 && r <= 0.0) {
        let s = p * q / 4.0;
        let r2 = sq(r);
        let r3 = r * r2;
        let disc = s * (s + 2.0 * r3);
        let mut u = r;
        if disc >= 0.0 {
            let mut t3 = s + r3;
            t3 += if t3 < 0.0 { -disc.sqrt() } else { disc.sqrt() };
            let t = t3.cbrt();
            u += t + if t != 0.0 { r2 / t } else { 0.0 };
        } else {
            let ang = (-disc).sqrt().atan2(-(s + r3));
            u += 2.0 * r * (ang / 3.0).cos();
        }
        let v = (sq(u) + q).sqrt();
        let uv = if u < 0.0 { q / (v - u) } else { u + v };
        let w = (uv - q) / (2.0 * v);
        uv / ((uv + sq(w)).sqrt() + w)
    } else {
        0.0
    }
}

/// The start of Newton's method (`InverseStart`).
struct Start {
    /// `>= 0` when the short-line solution is enough (then `salp2`, `calp2` are set).
    sig12: f64,
    salp1: f64,
    calp1: f64,
    salp2: f64,
    calp2: f64,
    dnm: f64,
}

/// Whether `lambda12` also computes the derivative Newton's method needs (`diffp` in C).
#[derive(Clone, Copy, Eq, PartialEq)]
enum Slope {
    Compute,
    Skip,
}

/// The longitude difference function and what Newton needs (`Lambda12`).
struct Lambda {
    lam12: f64,
    salp2: f64,
    calp2: f64,
    sig12: f64,
    p1: Sig,
    p2: Sig,
    eps: f64,
    dlam12: f64,
}

impl Geodesic {
    #[expect(
        clippy::too_many_arguments,
        reason = "the signature of InverseStart in geodesic.c"
    )]
    fn inverse_start(
        &self,
        sbet1: f64,
        cbet1: f64,
        dn1: f64,
        sbet2: f64,
        cbet2: f64,
        dn2: f64,
        lam12: f64,
        slam12: f64,
        clam12: f64,
        ca: &mut [f64; NC],
    ) -> Start {
        let (mut salp2, mut calp2, mut dnm) = (0.0, 0.0, 0.0);
        let mut salp1;
        let mut calp1;
        let mut sig12 = -1.0;
        let sbet12 = sbet2 * cbet1 - cbet2 * sbet1;
        let cbet12 = cbet2 * cbet1 + sbet2 * sbet1;
        let shortline = cbet12 >= 0.0 && sbet12 < 0.5 && cbet2 * lam12 < 0.5;
        let sbet12a = sbet2 * cbet1 + cbet2 * sbet1;
        let (mut somg12, mut comg12);
        if shortline {
            let mut sbetm2 = sq(sbet1 + sbet2);
            sbetm2 /= sbetm2 + sq(cbet1 + cbet2);
            dnm = (1.0 + self.ep2 * sbetm2).sqrt();
            let omg12 = lam12 / (self.f1 * dnm);
            somg12 = omg12.sin();
            comg12 = omg12.cos();
        } else {
            somg12 = slam12;
            comg12 = clam12;
        }
        salp1 = cbet2 * somg12;
        calp1 = if comg12 >= 0.0 {
            sbet12 + cbet2 * sbet1 * sq(somg12) / (1.0 + comg12)
        } else {
            sbet12a - cbet2 * sbet1 * sq(somg12) / (1.0 - comg12)
        };
        let ssig12 = salp1.hypot(calp1);
        let csig12 = sbet1 * sbet2 + cbet1 * cbet2 * comg12;
        if shortline && ssig12 < self.etol2 {
            salp2 = cbet1 * somg12;
            calp2 = sbet12
                - cbet1
                    * sbet2
                    * (if comg12 >= 0.0 {
                        sq(somg12) / (1.0 + comg12)
                    } else {
                        1.0 - comg12
                    });
            norm2(&mut salp2, &mut calp2);
            sig12 = ssig12.atan2(csig12);
        } else if self.n.abs() > 0.1
            || csig12 >= 0.0
            || ssig12 >= 6.0 * self.n.abs() * PI * sq(cbet1)
        {
            // The zeroth order spherical approximation is enough.
        } else {
            let (x, y, lamscale, betscale);
            let lam12x = (-slam12).atan2(-clam12);
            if self.f >= 0.0 {
                let k2 = sq(sbet1) * self.ep2;
                let eps = k2 / (2.0 * (1.0 + (1.0 + k2).sqrt()) + k2);
                lamscale = self.f * cbet1 * self.a3f(eps) * PI;
                betscale = lamscale * cbet1;
                x = lam12x / lamscale;
                y = sbet12a / betscale;
            } else {
                let cbet12a = cbet2 * cbet1 - sbet2 * sbet1;
                let bet12a = sbet12a.atan2(cbet12a);
                let m = self.lengths(
                    self.n,
                    PI + bet12a,
                    Sig {
                        s: sbet1,
                        c: -cbet1,
                        dn: dn1,
                    },
                    Sig {
                        s: sbet2,
                        c: cbet2,
                        dn: dn2,
                    },
                    (cbet1, cbet2),
                    WANT_M12B | WANT_M0,
                    ca,
                );
                x = -1.0 + m.m12b / (cbet1 * cbet2 * m.m0 * PI);
                betscale = if x < -0.01 {
                    sbet12a / x
                } else {
                    -self.f * sq(cbet1) * PI
                };
                lamscale = betscale / cbet1;
                y = lam12x / lamscale;
            }
            if y > -tol1() && x > -1.0 - xthresh() {
                if self.f >= 0.0 {
                    salp1 = 1.0_f64.min(-x);
                    calp1 = -(1.0 - sq(salp1)).sqrt();
                } else {
                    calp1 = (if x > -tol1() { 0.0_f64 } else { -1.0_f64 }).max(x);
                    salp1 = (1.0 - sq(calp1)).sqrt();
                }
            } else {
                let k = astroid(x, y);
                let omg12a = lamscale
                    * if self.f >= 0.0 {
                        -x * k / (1.0 + k)
                    } else {
                        -y * (1.0 + k) / k
                    };
                somg12 = omg12a.sin();
                comg12 = -omg12a.cos();
                salp1 = cbet2 * somg12;
                calp1 = sbet12a - cbet2 * sbet1 * sq(somg12) / (1.0 - comg12);
            }
        }
        if !(salp1 <= 0.0) {
            norm2(&mut salp1, &mut calp1);
        } else {
            salp1 = 1.0;
            calp1 = 0.0;
        }
        Start {
            sig12,
            salp1,
            calp1,
            salp2,
            calp2,
            dnm,
        }
    }
}

impl Geodesic {
    /// `Lambda12`: the longitude difference for the azimuth `alp1`, and its derivative when
    /// `diffp` asks for it.
    #[expect(
        clippy::too_many_arguments,
        reason = "the signature of Lambda12 in geodesic.c"
    )]
    fn lambda12(
        &self,
        sbet1: f64,
        cbet1: f64,
        dn1: f64,
        sbet2: f64,
        cbet2: f64,
        dn2: f64,
        salp1: f64,
        calp1: f64,
        slam120: f64,
        clam120: f64,
        slope: Slope,
        ca: &mut [f64; NC],
    ) -> Lambda {
        let mut calp1 = calp1;
        if sbet1 == 0.0 && calp1 == 0.0 {
            calp1 = -tiny();
        }
        let salp0 = salp1 * cbet1;
        let calp0 = calp1.hypot(salp1 * sbet1);
        let mut ssig1 = sbet1;
        let somg1 = salp0 * sbet1;
        let mut csig1 = calp1 * cbet1;
        let comg1 = csig1;
        norm2(&mut ssig1, &mut csig1);
        let salp2 = if cbet2 != cbet1 { salp0 / cbet2 } else { salp1 };
        let calp2 = if cbet2 != cbet1 || sbet2.abs() != -sbet1 {
            (sq(calp1 * cbet1)
                + if cbet1 < -sbet1 {
                    (cbet2 - cbet1) * (cbet1 + cbet2)
                } else {
                    (sbet1 - sbet2) * (sbet1 + sbet2)
                })
            .sqrt()
                / cbet2
        } else {
            calp1.abs()
        };
        let mut ssig2 = sbet2;
        let somg2 = salp0 * sbet2;
        let mut csig2 = calp2 * cbet2;
        let comg2 = csig2;
        norm2(&mut ssig2, &mut csig2);
        let sig12 =
            (0.0_f64.max(csig1 * ssig2 - ssig1 * csig2) + 0.0).atan2(csig1 * csig2 + ssig1 * ssig2);
        let somg12 = 0.0_f64.max(comg1 * somg2 - somg1 * comg2) + 0.0;
        let comg12 = comg1 * comg2 + somg1 * somg2;
        let eta = (somg12 * clam120 - comg12 * slam120).atan2(comg12 * clam120 + somg12 * slam120);
        let k2 = sq(calp0) * self.ep2;
        let eps = k2 / (2.0 * (1.0 + (1.0 + k2).sqrt()) + k2);
        self.c3f(eps, ca);
        let b312 =
            sin_series(ssig2, csig2, ca, N_SERIES - 1) - sin_series(ssig1, csig1, ca, N_SERIES - 1);
        let domg12 = -self.f * self.a3f(eps) * salp0 * (sig12 + b312);
        let lam12 = eta + domg12;
        let p1 = Sig {
            s: ssig1,
            c: csig1,
            dn: dn1,
        };
        let p2 = Sig {
            s: ssig2,
            c: csig2,
            dn: dn2,
        };
        let mut dlam12 = 0.0;
        if slope == Slope::Compute {
            if calp2 == 0.0 {
                dlam12 = -2.0 * self.f1 * dn1 / sbet1;
            } else {
                dlam12 = self
                    .lengths(eps, sig12, p1, p2, (cbet1, cbet2), WANT_M12B, ca)
                    .m12b;
                dlam12 *= self.f1 / (calp2 * cbet2);
            }
        }
        Lambda {
            lam12,
            salp2,
            calp2,
            sig12,
            p1,
            p2,
            eps,
            dlam12,
        }
    }

    /// `geod_geninverse_int`, without the area. `want` holds the high bits of `GEOD_*`.
    fn inverse_solution(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64, want: u32) -> Inverse {
        let (mut lat1, mut lat2) = (lat1, lat2);
        let mut s12 = 0.0;
        let mut m12 = 0.0;
        let mut big_m12 = 0.0;
        let mut big_m21 = 0.0;
        let (mut s12x, mut m12x) = (0.0, 0.0);
        let (mut calp1, mut salp1, mut calp2, mut salp2) = (0.0, 0.0, 0.0, 0.0);
        let mut ca = [0.0; NC];
        let outmask = want & OUT_ALL;
        let scale = outmask & GEOD_GEODESICSCALE != 0;

        let (mut lon12, mut lon12s) = ang_diff(lon1, lon2);
        let mut lonsign = if lon12.is_sign_negative() { -1.0 } else { 1.0 };
        lon12 *= lonsign;
        lon12s *= lonsign;
        let lam12 = lon12 * DEGREE;
        let (slam12, clam12) = sincosde(lon12, lon12s);
        lon12s = (HD - lon12) - lon12s;

        lat1 = ang_round(lat_fix(lat1));
        lat2 = ang_round(lat_fix(lat2));
        let swapp = if lat1.abs() < lat2.abs() || lat2.is_nan() {
            -1.0
        } else {
            1.0
        };
        if swapp < 0.0 {
            lonsign *= -1.0;
            std::mem::swap(&mut lat1, &mut lat2);
        }
        let latsign = if lat1.is_sign_negative() { 1.0 } else { -1.0 };
        lat1 *= latsign;
        lat2 *= latsign;

        let (mut sbet1, mut cbet1) = sincosdx(lat1);
        sbet1 *= self.f1;
        norm2(&mut sbet1, &mut cbet1);
        cbet1 = tiny().max(cbet1);
        let (mut sbet2, mut cbet2) = sincosdx(lat2);
        sbet2 *= self.f1;
        norm2(&mut sbet2, &mut cbet2);
        cbet2 = tiny().max(cbet2);

        if cbet1 < -sbet1 {
            if cbet2 == cbet1 {
                sbet2 = sbet1.copysign(sbet2);
            }
        } else if sbet2.abs() == -sbet1 {
            cbet2 = cbet1;
        }

        let dn1 = (1.0 + self.ep2 * sq(sbet1)).sqrt();
        let dn2 = (1.0 + self.ep2 * sq(sbet2)).sqrt();
        let cbet = (cbet1, cbet2);
        let mut scale_mask = WANT_S12B | WANT_M12B;
        if scale {
            scale_mask |= WANT_M12 | WANT_M21;
        }

        let mut meridian = lat1 == -QD || slam12 == 0.0;
        if meridian {
            calp1 = clam12;
            salp1 = slam12;
            calp2 = 1.0;
            salp2 = 0.0;
            let (ssig1, csig1) = (sbet1, calp1 * cbet1);
            let (ssig2, csig2) = (sbet2, calp2 * cbet2);
            let sig12 = (0.0_f64.max(csig1 * ssig2 - ssig1 * csig2) + 0.0)
                .atan2(csig1 * csig2 + ssig1 * ssig2);
            let l = self.lengths(
                self.n,
                sig12,
                Sig {
                    s: ssig1,
                    c: csig1,
                    dn: dn1,
                },
                Sig {
                    s: ssig2,
                    c: csig2,
                    dn: dn2,
                },
                cbet,
                scale_mask,
                &mut ca,
            );
            s12x = l.s12b;
            m12x = l.m12b;
            if scale {
                big_m12 = l.big_m12;
                big_m21 = l.big_m21;
            }
            if sig12 < tol2() || m12x >= 0.0 {
                if sig12 < 3.0 * tiny() || (sig12 < TOL0 && (s12x < 0.0 || m12x < 0.0)) {
                    m12x = 0.0;
                    s12x = 0.0;
                }
                m12x *= self.b;
                s12x *= self.b;
            } else {
                meridian = false;
            }
        }

        if !meridian && sbet1 == 0.0 && (self.f <= 0.0 || lon12s >= self.f * HD) {
            // The geodesic runs along the equator.
            calp1 = 0.0;
            calp2 = 0.0;
            salp1 = 1.0;
            salp2 = 1.0;
            s12x = self.a * lam12;
            let sig12 = lam12 / self.f1;
            m12x = self.b * sig12.sin();
            if scale {
                big_m12 = sig12.cos();
                big_m21 = big_m12;
            }
        } else if !meridian {
            let start = self.inverse_start(
                sbet1, cbet1, dn1, sbet2, cbet2, dn2, lam12, slam12, clam12, &mut ca,
            );
            salp1 = start.salp1;
            calp1 = start.calp1;
            if start.sig12 >= 0.0 {
                // A short line: `inverse_start` solved it.
                salp2 = start.salp2;
                calp2 = start.calp2;
                let dnm = start.dnm;
                let sig12 = start.sig12;
                s12x = sig12 * self.b * dnm;
                m12x = sq(dnm) * self.b * (sig12 / dnm).sin();
                if scale {
                    big_m12 = (sig12 / dnm).cos();
                    big_m21 = big_m12;
                }
            } else {
                // Newton's method on lambda12(alp1) - lam12 = 0, with a bracketing range.
                let mut numit = 0_u32;
                let (mut salp1a, mut calp1a, mut salp1b, mut calp1b) = (tiny(), 1.0, tiny(), -1.0);
                let mut tripn = false;
                let mut tripb = false;
                let lam = loop {
                    let lam = self.lambda12(
                        sbet1,
                        cbet1,
                        dn1,
                        sbet2,
                        cbet2,
                        dn2,
                        salp1,
                        calp1,
                        slam12,
                        clam12,
                        if numit < MAXIT1 {
                            Slope::Compute
                        } else {
                            Slope::Skip
                        },
                        &mut ca,
                    );
                    let v = lam.lam12;
                    let dv = lam.dlam12;
                    if tripb
                        || !(v.abs() >= (if tripn { 8.0 } else { 1.0 }) * TOL0)
                        || numit == MAXIT2
                    {
                        break lam;
                    }
                    if v > 0.0 && (numit > MAXIT1 || calp1 / salp1 > calp1b / salp1b) {
                        salp1b = salp1;
                        calp1b = calp1;
                    } else if v < 0.0 && (numit > MAXIT1 || calp1 / salp1 < calp1a / salp1a) {
                        salp1a = salp1;
                        calp1a = calp1;
                    }
                    if numit < MAXIT1 && dv > 0.0 {
                        let dalp1 = -v / dv;
                        if dalp1.abs() < PI {
                            let sdalp1 = dalp1.sin();
                            let cdalp1 = dalp1.cos();
                            let nsalp1 = salp1 * cdalp1 + calp1 * sdalp1;
                            if nsalp1 > 0.0 {
                                calp1 = calp1 * cdalp1 - salp1 * sdalp1;
                                salp1 = nsalp1;
                                norm2(&mut salp1, &mut calp1);
                                tripn = v.abs() <= 16.0 * TOL0;
                                numit += 1;
                                continue;
                            }
                        }
                    }
                    // The midpoint of the bracket is the next estimate.
                    salp1 = (salp1a + salp1b) / 2.0;
                    calp1 = (calp1a + calp1b) / 2.0;
                    norm2(&mut salp1, &mut calp1);
                    tripn = false;
                    tripb = (salp1a - salp1).abs() + (calp1a - calp1) < TOLB
                        || (salp1 - salp1b).abs() + (calp1 - calp1b) < TOLB;
                    numit += 1;
                };
                salp2 = lam.salp2;
                calp2 = lam.calp2;
                let l = self.lengths(
                    lam.eps, lam.sig12, lam.p1, lam.p2, cbet, scale_mask, &mut ca,
                );
                s12x = l.s12b;
                m12x = l.m12b;
                if scale {
                    big_m12 = l.big_m12;
                    big_m21 = l.big_m21;
                }
                m12x *= self.b;
                s12x *= self.b;
            }
        }

        if outmask & GEOD_DISTANCE != 0 {
            s12 = 0.0 + s12x;
        }
        if outmask & GEOD_REDUCEDLENGTH != 0 {
            m12 = 0.0 + m12x;
        }

        // Back to the azimuths of the points as given: undo the swap and the signs.
        if swapp < 0.0 {
            std::mem::swap(&mut salp1, &mut salp2);
            std::mem::swap(&mut calp1, &mut calp2);
            if scale {
                std::mem::swap(&mut big_m12, &mut big_m21);
            }
        }
        salp1 *= swapp * lonsign;
        calp1 *= swapp * latsign;
        salp2 *= swapp * lonsign;
        calp2 *= swapp * latsign;

        Inverse {
            salp1,
            calp1,
            salp2,
            calp2,
            s12,
            m12,
            big_m12,
        }
    }
}

/// `Position` with nothing computed.
fn nothing_computed() -> Position {
    Position {
        computed: 0,
        lat2: 0.0,
        lon2: 0.0,
        azi2: 0.0,
        s12: 0.0,
        m12: 0.0,
        big_m12: 0.0,
        big_m21: 0.0,
    }
}

impl GeodesicLine {
    /// `geod_lineinit_int`: the line from `(lat1, lon1)` with the azimuth `(salp1, calp1)`.
    fn start(g: &Geodesic, lat1: f64, lon1: f64, salp1: f64, calp1: f64, caps: u32) -> Self {
        let caps = (if caps != 0 {
            caps
        } else {
            GEOD_DISTANCE_IN | GEOD_LONGITUDE
        }) | GEOD_LATITUDE
            | GEOD_AZIMUTH
            | GEOD_LONG_UNROLL;
        let lat1 = lat_fix(lat1);
        let (mut sbet1, mut cbet1) = sincosdx(ang_round(lat1));
        sbet1 *= g.f1;
        norm2(&mut sbet1, &mut cbet1);
        cbet1 = tiny().max(cbet1);
        let dn1 = (1.0 + g.ep2 * sq(sbet1)).sqrt();
        let salp0 = salp1 * cbet1;
        let calp0 = calp1.hypot(salp1 * sbet1);
        let mut ssig1 = sbet1;
        let somg1 = salp0 * sbet1;
        let mut csig1 = if sbet1 != 0.0 || calp1 != 0.0 {
            cbet1 * calp1
        } else {
            1.0
        };
        let comg1 = csig1;
        norm2(&mut ssig1, &mut csig1);
        let k2 = sq(calp0) * g.ep2;
        let eps = k2 / (2.0 * (1.0 + (1.0 + k2).sqrt()) + k2);
        let mut line = Self {
            lon1,
            b: g.b,
            f: g.f,
            f1: g.f1,
            salp0,
            calp0,
            k2,
            ssig1,
            csig1,
            dn1,
            stau1: 0.0,
            ctau1: 0.0,
            somg1,
            comg1,
            a1m1: 0.0,
            a2m1: 0.0,
            a3c: 0.0,
            b11: 0.0,
            b21: 0.0,
            b31: 0.0,
            c1a: [0.0; NC],
            c1pa: [0.0; NC],
            c2a: [0.0; NC],
            c3a: [0.0; N_SERIES],
            caps,
        };
        if caps & CAP_C1 != 0 {
            line.a1m1 = a1m1f(eps);
            c1f(eps, &mut line.c1a);
            line.b11 = sin_series(line.ssig1, line.csig1, &line.c1a, N_SERIES);
            let s = line.b11.sin();
            let c = line.b11.cos();
            line.stau1 = line.ssig1 * c + line.csig1 * s;
            line.ctau1 = line.csig1 * c - line.ssig1 * s;
        }
        if caps & CAP_C1P != 0 {
            c1pf(eps, &mut line.c1pa);
        }
        if caps & CAP_C2 != 0 {
            line.a2m1 = a2m1f(eps);
            c2f(eps, &mut line.c2a);
            line.b21 = sin_series(line.ssig1, line.csig1, &line.c2a, N_SERIES);
        }
        if caps & CAP_C3 != 0 {
            g.c3f(eps, &mut line.c3a);
            line.a3c = -line.f * line.salp0 * g.a3f(eps);
            line.b31 = sin_series(line.ssig1, line.csig1, &line.c3a, N_SERIES - 1);
        }
        line
    }

    /// `geod_lineinit(l, g, lat1, lon1, azi1, caps)`.
    pub(crate) fn new(g: &Geodesic, lat1: f64, lon1: f64, azi1: f64, caps: u32) -> Self {
        let azi1 = ang_normalize(azi1);
        let (salp1, calp1) = sincosdx(ang_round(azi1));
        Self::start(g, lat1, lon1, salp1, calp1, caps)
    }

    /// `geod_genposition`: the point `s12_a12` along the line (a distance, or an arc in degrees
    /// with `GEOD_ARCMODE`). `want` holds the outputs asked for, as the high bits of `GEOD_*`.
    pub(crate) fn genposition(&self, flags: u32, s12_a12: f64, want: u32) -> Position {
        let mut out = nothing_computed();
        let outmask = want & self.caps & OUT_ALL;
        let arc = flags & GEOD_ARCMODE != 0;
        if !(arc || self.caps & (GEOD_DISTANCE_IN & OUT_ALL) != 0) {
            return out;
        }
        let sig12;
        let ssig12;
        let csig12;
        let mut b12 = 0.0;
        let mut ab1 = 0.0;
        if arc {
            sig12 = s12_a12 * DEGREE;
            (ssig12, csig12) = sincosdx(s12_a12);
        } else {
            let tau12 = s12_a12 / (self.b * (1.0 + self.a1m1));
            let s = tau12.sin();
            let c = tau12.cos();
            b12 = -sin_series(
                self.stau1 * c + self.ctau1 * s,
                self.ctau1 * c - self.stau1 * s,
                &self.c1pa,
                N_SERIES,
            );
            let mut sig = tau12 - (b12 - self.b11);
            let mut ss = sig.sin();
            let mut cs = sig.cos();
            if self.f.abs() > 0.01 {
                // The reverted distance series is inaccurate for |f| > 1/100: one Newton step.
                let ssig2 = self.ssig1 * cs + self.csig1 * ss;
                let csig2 = self.csig1 * cs - self.ssig1 * ss;
                b12 = sin_series(ssig2, csig2, &self.c1a, N_SERIES);
                let serr = (1.0 + self.a1m1) * (sig + (b12 - self.b11)) - s12_a12 / self.b;
                sig -= serr / (1.0 + self.k2 * sq(ssig2)).sqrt();
                ss = sig.sin();
                cs = sig.cos();
            }
            sig12 = sig;
            ssig12 = ss;
            csig12 = cs;
        }

        let ssig2 = self.ssig1 * csig12 + self.csig1 * ssig12;
        let mut csig2 = self.csig1 * csig12 - self.ssig1 * ssig12;
        let dn2 = (1.0 + self.k2 * sq(ssig2)).sqrt();
        if outmask & (GEOD_DISTANCE | GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE) != 0 {
            if arc || self.f.abs() > 0.01 {
                b12 = sin_series(ssig2, csig2, &self.c1a, N_SERIES);
            }
            ab1 = (1.0 + self.a1m1) * (b12 - self.b11);
        }
        let sbet2 = self.calp0 * ssig2;
        let mut cbet2 = self.salp0.hypot(self.calp0 * csig2);
        if cbet2 == 0.0 {
            // salp0 = 0 and csig2 = 0: break the degeneracy.
            csig2 = tiny();
            cbet2 = csig2;
        }
        let salp2 = self.salp0;
        let calp2 = self.calp0 * csig2;

        if outmask & GEOD_DISTANCE != 0 {
            out.s12 = if arc {
                self.b * ((1.0 + self.a1m1) * sig12 + ab1)
            } else {
                s12_a12
            };
        }
        if outmask & GEOD_LONGITUDE != 0 {
            let e = 1.0_f64.copysign(self.salp0);
            let somg2 = self.salp0 * ssig2;
            let comg2 = csig2;
            let omg12 = if flags & GEOD_LONG_UNROLL != 0 {
                e * (sig12 - (ssig2.atan2(csig2) - self.ssig1.atan2(self.csig1))
                    + ((e * somg2).atan2(comg2) - (e * self.somg1).atan2(self.comg1)))
            } else {
                (somg2 * self.comg1 - comg2 * self.somg1)
                    .atan2(comg2 * self.comg1 + somg2 * self.somg1)
            };
            let lam12 = omg12
                + self.a3c
                    * (sig12 + (sin_series(ssig2, csig2, &self.c3a, N_SERIES - 1) - self.b31));
            let lon12 = lam12 / DEGREE;
            out.lon2 = if flags & GEOD_LONG_UNROLL != 0 {
                self.lon1 + lon12
            } else {
                ang_normalize(ang_normalize(self.lon1) + ang_normalize(lon12))
            };
        }
        if outmask & GEOD_LATITUDE != 0 {
            out.lat2 = atan2dx(sbet2, self.f1 * cbet2);
        }
        if outmask & GEOD_AZIMUTH != 0 {
            out.azi2 = atan2dx(salp2, calp2);
        }
        if outmask & (GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE) != 0 {
            let b22 = sin_series(ssig2, csig2, &self.c2a, N_SERIES);
            let ab2 = (1.0 + self.a2m1) * (b22 - self.b21);
            let j12 = (self.a1m1 - self.a2m1) * sig12 + (ab1 - ab2);
            if outmask & GEOD_REDUCEDLENGTH != 0 {
                out.m12 = self.b
                    * ((dn2 * (self.csig1 * ssig2) - self.dn1 * (self.ssig1 * csig2))
                        - self.csig1 * csig2 * j12);
            }
            if outmask & GEOD_GEODESICSCALE != 0 {
                let t = self.k2 * (ssig2 - self.ssig1) * (ssig2 + self.ssig1) / (self.dn1 + dn2);
                out.big_m12 = csig12 + (t * ssig2 - csig2 * j12) * self.ssig1 / self.dn1;
                out.big_m21 = csig12 - (t * self.ssig1 - self.csig1 * j12) * ssig2 / dn2;
            }
        }
        out.computed = outmask;
        out
    }
}

impl Geodesic {
    /// Distance and forward azimuth, in the unit of `a` and degrees.
    pub(crate) fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
        let solution = self.inverse_solution(lat1, lon1, lat2, lon2, GEOD_DISTANCE);
        (solution.s12, atan2dx(solution.salp1, solution.calp1))
    }

    /// Length and both azimuths in degrees.
    pub(crate) fn inverse_full(
        &self,
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
    ) -> (f64, f64, f64) {
        let solution = self.inverse_solution(lat1, lon1, lat2, lon2, GEOD_DISTANCE);
        (
            solution.s12,
            atan2dx(solution.salp1, solution.calp1),
            atan2dx(solution.salp2, solution.calp2),
        )
    }

    /// Point reached by travelling `distance` along `azimuth` degrees.
    pub(crate) fn direct(&self, lat1: f64, lon1: f64, azimuth: f64, distance: f64) -> (f64, f64) {
        let want = GEOD_LATITUDE | GEOD_LONGITUDE;
        let line = GeodesicLine::new(self, lat1, lon1, azimuth, want | GEOD_DISTANCE_IN);
        let position = line.genposition(0, distance, want);
        (position.lat2, position.lon2)
    }

    /// Forward azimuth in degrees, reduced length `m12`, and scale `M12`.
    pub(crate) fn inverse_scale(
        &self,
        lat1: f64,
        lon1: f64,
        lat2: f64,
        lon2: f64,
    ) -> (f64, f64, f64) {
        let solution = self.inverse_solution(
            lat1,
            lon1,
            lat2,
            lon2,
            GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE,
        );
        (
            atan2dx(solution.salp1, solution.calp1),
            solution.m12,
            solution.big_m12,
        )
    }
}

/// Capabilities `gnom` asks of a geodesic line.
pub(crate) const GNOM_CAPS: u32 =
    GEOD_LATITUDE | GEOD_LONGITUDE | GEOD_DISTANCE_IN | GEOD_REDUCEDLENGTH | GEOD_GEODESICSCALE;

impl GeodesicLine {
    pub(crate) fn from_azimuth(earth: &Geodesic, lat1: f64, lon1: f64, azimuth: f64) -> Self {
        Self::new(earth, lat1, lon1, azimuth, GNOM_CAPS)
    }

    /// `geod_lineinit` with no capability, as `geod_pre` of PROJ's geod. Angles are in degrees.
    pub(crate) fn plain(earth: &Geodesic, lat1: f64, lon1: f64, azimuth: f64) -> Self {
        Self::new(earth, lat1, lon1, azimuth, 0)
    }

    /// Latitude, longitude and back azimuth in degrees, at `distance` along the line.
    pub(crate) fn position_azimuth(&self, distance: f64) -> (f64, f64, f64) {
        let position = self.genposition(0, distance, GEOD_LATITUDE | GEOD_LONGITUDE | GEOD_AZIMUTH);
        (position.lat2, position.lon2, position.azi2)
    }

    /// Latitude, longitude, the distance, the reduced length and the scale at `distance`.
    pub(crate) fn position(&self, distance: f64) -> (f64, f64, f64, f64, f64) {
        let want = GEOD_LATITUDE
            | GEOD_LONGITUDE
            | GEOD_DISTANCE
            | GEOD_REDUCEDLENGTH
            | GEOD_GEODESICSCALE;
        let position = self.genposition(0, distance, want);
        // Without `GEOD_DISTANCE` in the line's capabilities the distance is not computed and the
        // caller's value stands.
        let along = if position.computed & GEOD_DISTANCE != 0 {
            position.s12
        } else {
            distance
        };
        (
            position.lat2,
            position.lon2,
            along,
            position.m12,
            position.big_m12,
        )
    }
}

/// `DEG_TO_RAD` of PROJ.
const DEG_TO_RAD: f64 = 0.017_453_292_519_943_296;

/// Radians to degrees with PROJ's `DEG_TO_RAD`.
pub(crate) fn degrees(radians: f64) -> f64 {
    radians / DEG_TO_RAD
}

/// Degrees to radians with PROJ's `DEG_TO_RAD`.
pub(crate) fn radians(degrees: f64) -> f64 {
    degrees * DEG_TO_RAD
}
