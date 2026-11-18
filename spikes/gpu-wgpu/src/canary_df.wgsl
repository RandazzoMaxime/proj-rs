// Contrôle des transformations sans erreur du double-float. Si le compilateur du pilote
// réassocie (fast-math), ces résultats sont faux et le chemin df ne vaut rien.
// IN[0] = (1, 2^-30, 1 + 2^-12, 0), lus depuis le buffer pour empêcher le repli de constantes.
@compute @workgroup_size(1)
fn canary(@builtin(global_invocation_id) g: vec3<u32>) {
    if (g.x >= CNT.x) { return; }
    let v = IN[0];
    let s = two_sum(v.x, v.y);
    let q = two_prod(v.z, v.z);
    let z = P[0].x * 0.0;
    OUT[0] = vec4<f32>(s.hi + z, s.lo, q.hi, q.lo);
}
