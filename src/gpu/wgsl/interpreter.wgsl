// Interpréteur : un thread par point, la même liste d'opérations pour tous (PROG).
// Une opération agit sur la paire (a, b). Les noms OP_* et les constantes k_*() sont générés par
// `shader.rs` ; les indices de paramètres viennent du programme (`base`).

struct SC { s: R, c: R }
struct PR { a: R, b: R }

// Un point que le GPU ne sait pas garantir : le bloc entier repasse par le CPU.
fn raise() { atomicOr(&STATUS[0], 1u); }

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

fn atanh_(x: R) -> R {
    return ml(c(0.5), log_(dv(ad(c(1.0), x), sb(c(1.0), x))));
}

// `adjlon` de PROJ : une longitude à moins de 1e-12 au-delà de pi reste telle quelle.
fn adjlon_(x: R) -> R {
    if (lt0(sb(absr(x), k_adj()))) { return x; }
    var y = ad(x, k_pi());
    y = sb(y, ml(k_tau(), floorr(dv(y, k_tau()))));
    return sb(y, k_pi());
}

// ---- Opérations de cadre (`Classic` : fwd_prepare / fwd_finalize et leurs inverses) ----

fn op_prep_fwd(a: R, b: R, off: R) -> PR {
    if (bad(a) || bad(b)) { raise(); }
    // Au-delà du pôle : porte de latitude du CPU (écrêtage ou erreur), donc le CPU tranche.
    if (lt0(sb(k_halfpi(), absr(b)))) { raise(); }
    var lon = a;
    if (lt0(sb(k_pi(), absr(lon)))) {
        if (lt0(sb(c(10.0), absr(lon)))) { raise(); }
        lon = adjlon_(lon);
    }
    return PR(adjlon_(sb(lon, off)), b);
}

fn op_finish_fwd(a: R, b: R, base: u32) -> PR {
    let fa = ld(base);
    return PR(ad(ml(a, fa), ld(base + 1u)), ad(ml(b, fa), ld(base + 2u)));
}

fn op_prep_inv(a: R, b: R, base: u32) -> PR {
    if (bad(a) || bad(b)) { raise(); }
    let tm = ld(base);
    let ra = ld(base + 3u);
    return PR(ml(sb(ml(tm, a), ld(base + 1u)), ra), ml(sb(ml(tm, b), ld(base + 2u)), ra));
}

fn op_finish_inv(a: R, b: R, off: R) -> PR {
    return PR(adjlon_(ad(a, off)), b);
}

// ---- Mercator sphérique (webmerc et merc sans excentricité) ----

fn op_merc_fwd(a: R, b: R, k0: R) -> PR {
    if (lt0(sb(k_merc_lim(), absr(b)))) { raise(); }
    // Pour |phi| : (1 + sin) / cos sans soustraction, le signe revient après.
    let neg = lt0(b);
    let sc = sincos_(absr(b));
    var y = log_(dv(ad(c(1.0), sc.s), sc.c));
    if (neg) { y = ng(y); }
    return PR(ml(k0, a), ml(k0, y));
}

fn op_merc_inv(a: R, b: R, k0: R) -> PR {
    let north = dv(b, k0);
    if (lt0(sb(c(50.0), absr(north)))) { raise(); }
    return PR(dv(a, k0), atan2_(sinh_(north), c(1.0)));
}

// ---- Transverse Mercator exact (Poder/Engsager, ordre 6) : UTM et dérivés ----
// base : qn, zb, puis cbg, cgb, gtu, utg (6 chacun).

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

fn op_tmerc_fwd(lam: R, phi: R, base: u32) -> PR {
    let cn0 = auxlat_convert(phi, base + 2u);
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
    let d = clen_complex(base + 14u, sin_arg_r, cos_arg_r, sinh_arg_i, cosh_arg_i);
    let cn = ad(cn1, d.a);
    let ce = ad(ce1, d.b);
    if (lt0(sb(k_ce_limit(), absr(ce)))) { raise(); }
    let qn = ld(base);
    return PR(ml(qn, ce), ad(ml(qn, cn), ld(base + 1u)));
}

fn op_tmerc_inv(x: R, y: R, base: u32) -> PR {
    let qn = ld(base);
    var cn = dv(sb(y, ld(base + 1u)), qn);
    var ce = dv(x, qn);
    if (lt0(sb(k_ce_limit(), absr(ce)))) { raise(); }
    let sa = sincos_(ml(c(2.0), cn));
    let exp_2ce = exp_(ml(c(2.0), ce));
    let half_inv = dv(c(0.5), exp_2ce);
    let sinh_i = sb(ml(c(0.5), exp_2ce), half_inv);
    let cosh_i = ad(ml(c(0.5), exp_2ce), half_inv);
    let d = clen_complex(base + 20u, sa.s, sa.c, sinh_i, cosh_i);
    cn = ad(cn, d.a);
    ce = ad(ce, d.b);
    let sc = sincos_(cn);
    let sinh_ce = sinh_(ce);
    let lam = atan2_(sinh_ce, sc.c);
    let modulus = hypot_(sinh_ce, sc.c);
    let rr = hypot_(sc.s, modulus);
    let cn_geo = atan2_(sc.s, modulus);
    let phi = ad(cn_geo, clen_real(dv(sc.s, rr), dv(modulus, rr), base + 8u));
    return PR(lam, phi);
}

// ---- Lambert conforme conique ----
// base : e, n, c, rho0, k0.

fn powr(x: R, y: R) -> R { return exp_(ml(y, log_(x))); }

// `pj_sinhpsi2tanphi` (Newton de GeographicLib), cinq itérations fixes : après convergence
// l'itération ne bouge plus, et le CPU s'arrête au premier pas sous la tolérance.
fn sinhpsi2tanphi_(taup: R, e: R) -> R {
    let e2m = sb(c(1.0), ml(e, e));
    var tau = dv(taup, e2m);
    if (lt0(sb(c(70.0), absr(taup)))) { tau = ml(taup, exp_(ml(e, atanh_(e)))); }
    if (lt0(sb(k_tmax(), absr(tau)))) { raise(); }
    for (var i: i32 = 0; i < 5; i++) {
        let tau1 = sq(ad(c(1.0), ml(tau, tau)));
        let sig = sinh_(ml(e, atanh_(dv(ml(e, tau), tau1))));
        let taupa = sb(ml(sq(ad(c(1.0), ml(sig, sig))), tau), ml(sig, tau1));
        let num = ml(sb(taup, taupa), ad(c(1.0), ml(e2m, ml(tau, tau))));
        let den = ml(ml(e2m, tau1), sq(ad(c(1.0), ml(taupa, taupa))));
        tau = ad(tau, dv(num, den));
    }
    return tau;
}

fn op_lcc_fwd(lam: R, phi: R, base: u32) -> PR {
    // Pôle (|phi| à moins de 1e-10 de pi/2) : cas particulier du CPU, donc le CPU tranche.
    if (lt0(sb(absr(sb(absr(phi), k_halfpi())), k_eps10()))) { raise(); }
    let e = ld(base);
    let n = ld(base + 1u);
    let sc = sincos_(phi);
    // Selon le signe de sin(phi), sans soustraction de deux quantités voisines.
    var t = dv(sb(c(1.0), sc.s), sc.c);
    if (lt0(ng(sc.s))) { t = dv(sc.c, ad(c(1.0), sc.s)); }
    let ts = ml(exp_(ml(e, atanh_(ml(e, sc.s)))), t);
    let rho = ml(ld(base + 2u), powr(ts, n));
    let ln = sincos_(ml(lam, n));
    let k0 = ld(base + 4u);
    return PR(ml(k0, ml(rho, ln.s)), ml(k0, sb(ld(base + 3u), ml(rho, ln.c))));
}

fn op_lcc_inv(x0: R, y0: R, base: u32) -> PR {
    let e = ld(base);
    let n = ld(base + 1u);
    let k0 = ld(base + 4u);
    var x = dv(x0, k0);
    var y = sb(ld(base + 3u), dv(y0, k0));
    var rho = hypot_(x, y);
    if (bad(rho) || lt0(sb(rho, k_eps10()))) { raise(); }
    if (lt0(n)) {
        rho = ng(rho);
        x = ng(x);
        y = ng(y);
    }
    let ts = powr(dv(rho, ld(base + 2u)), dv(c(1.0), n));
    let taup = ml(c(0.5), sb(dv(c(1.0), ts), ts));
    let phi = atan2_(sinhpsi2tanphi_(taup, e), c(1.0));
    return PR(dv(atan2_(x, y), n), phi);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    var v = PR(load_a(g.x), load_b(g.x));
    for (var k: u32 = 0u; k < CNT.z; k++) {
        let op = PROG[k];
        let base = op.y;
        switch (op.x) {
            case OP_AFFINE: {
                var first = v.a;
                var second = v.b;
                if ((op.z & 1u) != 0u) { first = v.b; second = v.a; }
                v = PR(ml(first, ld(base)), ml(second, ld(base + 1u)));
            }
            case OP_PREP_FWD: { v = op_prep_fwd(v.a, v.b, ld(base)); }
            case OP_FINISH_FWD: { v = op_finish_fwd(v.a, v.b, base); }
            case OP_PREP_INV: { v = op_prep_inv(v.a, v.b, base); }
            case OP_FINISH_INV: { v = op_finish_inv(v.a, v.b, ld(base)); }
            case OP_MERC_FWD: { v = op_merc_fwd(v.a, v.b, ld(base)); }
            case OP_MERC_INV: { v = op_merc_inv(v.a, v.b, ld(base)); }
            case OP_TMERC_FWD: { v = op_tmerc_fwd(v.a, v.b, base); }
            case OP_TMERC_INV: { v = op_tmerc_inv(v.a, v.b, base); }
            case OP_LCC_FWD: { v = op_lcc_fwd(v.a, v.b, base); }
            case OP_LCC_INV: { v = op_lcc_inv(v.a, v.b, base); }
            default: { raise(); }
        }
    }
    store(g.x, v);
}
