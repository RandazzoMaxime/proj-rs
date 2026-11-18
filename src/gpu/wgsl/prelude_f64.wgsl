// f64 natif : exige Features::SHADER_F64 (Vulkan). Buffers f64 : 53 bits en entrée et en sortie.
alias R = f64;

fn ng(a: R) -> R { return -a; }
fn ad(a: R, b: R) -> R { return a + b; }
fn sb(a: R, b: R) -> R { return a - b; }
fn ml(a: R, b: R) -> R { return a * b; }
fn dv(a: R, b: R) -> R { return a / b; }
fn sq(a: R) -> R { return sqrt(a); }
fn c(v: f32) -> R { return f64(v); }
fn hi(a: R) -> f32 { return f32(a); }
fn lt0(a: R) -> bool { return a < 0.0lf; }
fn scale2(a: R, k: i32) -> R {
    var r = a;
    for (var i: i32 = 0; i < abs(k); i++) {
        if (k > 0) { r = r * 2.0lf; } else { r = r * 0.5lf; }
    }
    return r;
}
// fdlibm : pio2_1 sur 33 bits, donc k * pio2_1 est exact pour les petits k.
fn red_pio2(x: R, kf: f32) -> R {
    let k = f64(kf);
    return (x - k * 1.57079632673412561417e+00lf) - k * 6.07710050650619224932e-11lf;
}
fn red_ln2(x: R, kf: f32) -> R {
    let k = f64(kf);
    return (x - k * 6.93147180369123816490e-01lf) - k * 1.90821492927058770002e-10lf;
}
fn ld(i: u32) -> R { return P[i]; }
fn absr(a: R) -> R { return abs(a); }
fn floorr(x: R) -> R { return floor(x); }
fn bad(a: R) -> bool { return (bitcast<u32>(f32(a)) & 0x7f800000u) == 0x7f800000u; }
