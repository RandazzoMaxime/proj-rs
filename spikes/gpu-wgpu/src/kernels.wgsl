// Noyaux communs aux deux chemins : tout passe par ad/sb/ml/dv/ng/sq, jamais par un opérateur,
// donc le même algorithme tourne en f64 et en double-float.

// Indices dans le buffer de paramètres P (une constante = une paire hi/lo).
const P_A: u32 = 0u;
const P_QN: u32 = 1u;
const P_ZB: u32 = 2u;
const P_X0: u32 = 3u;
const P_Y0: u32 = 4u;
const P_LON0: u32 = 5u;
const P_D2R: u32 = 6u;
const P_R2D: u32 = 7u;
const P_CBG: u32 = 8u;
const P_CGB: u32 = 14u;
const P_GTU: u32 = 20u;
const P_UTG: u32 = 26u;

struct SC { s: R, c: R }
struct PR { a: R, b: R }

fn sincos_(x: R) -> SC {
    let kf = floor(hi(x) * 0.6366197723675814 + 0.5);
    let k = i32(kf);
    let r = red_pio2(x, kf);
    let v = poly_sc(r);
    switch (k & 3) {
        case 0: { return v; }
        case 1: { return SC(v.c, ng(v.s)); }
        case 2: { return SC(ng(v.s), ng(v.c)); }
        default: { return SC(ng(v.c), v.s); }
    }
}

fn exp_(x: R) -> R {
    let kf = floor(hi(x) * 1.4426950408889634 + 0.5);
    let r = red_ln2(x, kf);
    return scale2(poly_exp(r), i32(kf));
}

// log : estimation f32 du matériel, puis une correction de Halley (convergence cubique).
fn log_(x: R) -> R {
    let y0 = c(log(hi(x)));
    let e = exp_(y0);
    let t = dv(sb(x, e), ad(x, e));
    return ad(y0, ml(c(2.0), t));
}

// atan2 : estimation f32, puis rotation exacte de l'écart restant et série de atan.
fn atan2_(y: R, x: R) -> R {
    let y0 = atan2(hi(y), hi(x));
    let sc = sincos_(c(y0));
    let n = sb(ml(y, sc.c), ml(x, sc.s));
    let d = ad(ml(x, sc.c), ml(y, sc.s));
    let t = dv(n, d);
    let t2 = ml(t, t);
    let series = sb(c(1.0), ml(t2, sb(k_third(), ml(t2, k_fifth()))));
    return ad(c(y0), ml(t, series));
}

fn hypot_(a: R, b: R) -> R { return sq(ad(ml(a, a), ml(b, b))); }

fn asinh_(x: R) -> R {
    var ax = x;
    if (lt0(x)) { ax = ng(x); }
    var v = log_(ad(ax, sq(ad(ml(ax, ax), c(1.0)))));
    if (lt0(x)) { v = ng(v); }
    return v;
}

fn sinh_(x: R) -> R {
    let e = exp_(x);
    return ml(c(0.5), sb(e, dv(c(1.0), e)));
}

// ---- Web Mercator (EPSG:3857) ----

fn merc_fwd_pt(lon: R, lat: R) -> PR {
    let a = ld(P_A);
    let d2r = ld(P_D2R);
    let phi = ml(lat, d2r);
    let sc = sincos_(phi);
    let y = ml(a, log_(dv(ad(c(1.0), sc.s), sc.c)));
    return PR(ml(a, ml(lon, d2r)), y);
}

fn merc_inv_pt(x: R, y: R) -> PR {
    let a = ld(P_A);
    let r2d = ld(P_R2D);
    let lat = atan2_(sinh_(dv(y, a)), c(1.0));
    return PR(ml(dv(x, a), r2d), ml(lat, r2d));
}

// ---- Transverse Mercator exact (Poder/Engsager, ordre 6) : UTM et dérivés ----

fn clen_real(szeta: R, czeta: R, base: u32) -> R {
    let x = ml(c(2.0), ml(sb(czeta, szeta), ad(czeta, szeta)));
    var u0 = c(0.0);
    var u1 = c(0.0);
    for (var k: i32 = 5; k >= 0; k--) {
        let t = ad(sb(ml(x, u0), u1), ld(base + u32(k)));
        u1 = u0;
        u0 = t;
    }
    return ml(ml(c(2.0), ml(szeta, czeta)), u0);
}

fn auxlat_convert(zeta: R, base: u32) -> R {
    let sc = sincos_(zeta);
    return ad(zeta, clen_real(sc.s, sc.c, base));
}

fn clen_complex(base: u32, sin_r: R, cos_r: R, sinh_i: R, cosh_i: R) -> PR {
    var r = ml(ml(c(2.0), cos_r), cosh_i);
    var i = ng(ml(ml(c(2.0), sin_r), sinh_i));
    var hr1 = c(0.0);
    var hi1 = c(0.0);
    var hi_ = c(0.0);
    var hr = ld(base + 5u);
    for (var idx: i32 = 4; idx >= 0; idx--) {
        let hr2 = hr1;
        let hi2 = hi1;
        hr1 = hr;
        hi1 = hi_;
        hr = ad(sb(sb(ml(r, hr1), ml(i, hi1)), hr2), ld(base + u32(idx)));
        hi_ = sb(ad(ml(i, hr1), ml(r, hi1)), hi2);
    }
    r = ml(sin_r, cosh_i);
    i = ml(cos_r, sinh_i);
    return PR(sb(ml(r, hr), ml(i, hi_)), ad(ml(r, hi_), ml(i, hr)));
}

fn tmerc_fwd_pt(lon: R, lat: R) -> PR {
    let d2r = ld(P_D2R);
    let lam = ml(sb(lon, ld(P_LON0)), d2r);
    let phi = ml(lat, d2r);
    let cn0 = auxlat_convert(phi, P_CBG);
    let sn = sincos_(cn0);
    let se = sincos_(lam);
    let cos_cn_cos_ce = ml(sn.c, se.c);
    let cn1 = atan2_(sn.s, cos_cn_cos_ce);
    let inv_denom = dv(c(1.0), hypot_(sn.s, cos_cn_cos_ce));
    let tan_ce = ml(ml(se.s, sn.c), inv_denom);
    let ce1 = asinh_(tan_ce);
    let two_inv = ml(c(2.0), inv_denom);
    let two_inv_sq = ml(two_inv, inv_denom);
    let tmp_r = ml(cos_cn_cos_ce, two_inv_sq);
    let sin_arg_r = ml(sn.s, tmp_r);
    let cos_arg_r = sb(ml(cos_cn_cos_ce, tmp_r), c(1.0));
    let sinh_arg_i = ml(tan_ce, two_inv);
    let cosh_arg_i = sb(two_inv_sq, c(1.0));
    let d = clen_complex(P_GTU, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
    let cn = ad(cn1, d.a);
    let ce = ad(ce1, d.b);
    let qn = ld(P_QN);
    let x = ad(ml(qn, ce), ld(P_X0));
    let y = ad(ad(ml(qn, cn), ld(P_ZB)), ld(P_Y0));
    return PR(x, y);
}

fn tmerc_inv_pt(x: R, y: R) -> PR {
    let qn = ld(P_QN);
    var cn = dv(sb(sb(y, ld(P_Y0)), ld(P_ZB)), qn);
    var ce = dv(sb(x, ld(P_X0)), qn);
    let sa = sincos_(ml(c(2.0), cn));
    let exp_2ce = exp_(ml(c(2.0), ce));
    let half_inv = dv(c(0.5), exp_2ce);
    let sinh_i = sb(ml(c(0.5), exp_2ce), half_inv);
    let cosh_i = ad(ml(c(0.5), exp_2ce), half_inv);
    let d = clen_complex(P_UTG, sa.s, sa.c, sinh_i, cosh_i);
    cn = ad(cn, d.a);
    ce = ad(ce, d.b);
    let sc = sincos_(cn);
    let sinh_ce = sinh_(ce);
    let lam = atan2_(sinh_ce, sc.c);
    let modulus = hypot_(sinh_ce, sc.c);
    let rr = hypot_(sc.s, modulus);
    let cn_geo = atan2_(sc.s, modulus);
    let phi = ad(cn_geo, clen_real(dv(sc.s, rr), dv(modulus, rr), P_CGB));
    let r2d = ld(P_R2D);
    return PR(ad(ml(lam, r2d), ld(P_LON0)), ml(phi, r2d));
}

// ---- Points d'entrée : un thread par point ----


@compute @workgroup_size(64)
fn merc_fwd(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    store(g.x, merc_fwd_pt(load_a(g.x), load_b(g.x)));
}
@compute @workgroup_size(64)
fn merc_inv(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    store(g.x, merc_inv_pt(load_a(g.x), load_b(g.x)));
}
@compute @workgroup_size(64)
fn utm_fwd(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    store(g.x, tmerc_fwd_pt(load_a(g.x), load_b(g.x)));
}
@compute @workgroup_size(64)
fn utm_inv(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    store(g.x, tmerc_inv_pt(load_a(g.x), load_b(g.x)));
}
