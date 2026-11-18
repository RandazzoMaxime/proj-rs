// Double-float : un nombre = hi + lo, deux f32 (environ 48 bits de mantisse). Aucun f64.
struct DF { hi: f32, lo: f32 }
alias R = DF;

// Barrière : identité à l'exécution (CNT.y vaut 0xFFFFFFFF), opaque pour le compilateur. Sans elle,
// un pilote qui réassocie (fast-math : Mesa/lavapipe, Metal par défaut) replie (a+b)-a en b et
// détruit les termes d'erreur. Le test de contrôle `canary` vérifie que ça tient.
fn bar(x: f32) -> f32 { return bitcast<f32>(bitcast<u32>(x) & CNT.y); }

fn two_sum(a: f32, b: f32) -> DF {
    let s = bar(a + b);
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    return DF(s, e);
}
fn quick_two_sum(a: f32, b: f32) -> DF {
    let s = bar(a + b);
    let e = b - (s - a);
    return DF(s, e);
}
fn split(a: f32) -> DF {
    let t = bar(4097.0 * a);
    let h = t - (t - a);
    return DF(h, a - h);
}
// Produit exact sans fma (WGSL ne garantit pas que fma soit fusionnée) : Veltkamp-Dekker.
fn two_prod(a: f32, b: f32) -> DF {
    let p = bar(a * b);
    let sa = split(a);
    let sb_ = split(b);
    let e = ((sa.hi * sb_.hi - p) + sa.hi * sb_.lo + sa.lo * sb_.hi) + sa.lo * sb_.lo;
    return DF(p, e);
}

fn ng(a: R) -> R { return DF(-a.hi, -a.lo); }
fn ad(a: R, b: R) -> R {
    let s = two_sum(a.hi, b.hi);
    let t = two_sum(a.lo, b.lo);
    var e = s.lo + t.hi;
    let r = quick_two_sum(s.hi, e);
    e = r.lo + t.lo;
    return quick_two_sum(r.hi, e);
}
fn sb(a: R, b: R) -> R { return ad(a, ng(b)); }
fn mlf(a: R, b: f32) -> R {
    let p = two_prod(a.hi, b);
    let e = p.lo + a.lo * b;
    return quick_two_sum(p.hi, e);
}
fn ml(a: R, b: R) -> R {
    let p = two_prod(a.hi, b.hi);
    let e = p.lo + (a.hi * b.lo + a.lo * b.hi);
    return quick_two_sum(p.hi, e);
}
fn dv(a: R, b: R) -> R {
    let q1 = a.hi / b.hi;
    var r = sb(a, mlf(b, q1));
    let q2 = r.hi / b.hi;
    r = sb(r, mlf(b, q2));
    let q3 = r.hi / b.hi;
    var q = quick_two_sum(q1, q2);
    q = ad(q, DF(q3, 0.0));
    return q;
}
fn sq(a: R) -> R {
    if (a.hi <= 0.0) { return DF(0.0, 0.0); }
    var g = DF(sqrt(a.hi), 0.0);
    g = mlf(ad(g, dv(a, g)), 0.5);
    g = mlf(ad(g, dv(a, g)), 0.5);
    return g;
}
fn c(v: f32) -> R { return DF(v, 0.0); }
fn hi(a: R) -> f32 { return a.hi; }
fn lt0(a: R) -> bool { return a.hi < 0.0; }
fn scale2(a: R, k: i32) -> R {
    var r = a;
    for (var i: i32 = 0; i < abs(k); i++) {
        if (k > 0) { r = DF(r.hi * 2.0, r.lo * 2.0); } else { r = DF(r.hi * 0.5, r.lo * 0.5); }
    }
    return r;
}
fn red_pio2(x: R, kf: f32) -> R { return sb(x, ml(k_pio2(), c(kf))); }
fn red_ln2(x: R, kf: f32) -> R { return sb(x, ml(k_ln2(), c(kf))); }
fn ld(i: u32) -> R { let v = P[i]; return DF(v.x, v.y); }
fn load_a(i: u32) -> R { let p = IN[i]; return DF(p.x, p.y); }
fn load_b(i: u32) -> R { let p = IN[i]; return DF(p.z, p.w); }
fn store(i: u32, v: PR) { OUT[i] = vec4<f32>(v.a.hi, v.a.lo, v.b.hi, v.b.lo); }
