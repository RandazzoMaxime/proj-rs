//! Le driver par groupes de quatre, compilé avec AVX2 et FMA.

use super::{scalar, Avx2, Lanes, V4};

/// Parcourt les tranches par groupes de quatre.
///
/// La fonction est la seule où le compilateur reçoit `avx2,fma` : le noyau [`Lanes::lanes`] y
/// est inséré (`#[inline(always)]`), donc ses intrinsèques deviennent des instructions en
/// ligne.
pub(super) fn run_avx2<K: Lanes>(
    tok: Avx2,
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    // SAFETY: `tok` n'existe que si le processeur supporte AVX2 et FMA.
    unsafe { groups(tok, kernel, in_a, in_b, out_a, out_b) };
}

/// # Safety
///
/// Le processeur doit supporter AVX2 et FMA. `tok` en est la preuve.
#[target_feature(enable = "avx2,fma")]
unsafe fn groups<K: Lanes>(
    tok: Avx2,
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    let head = in_a.len() & !3;
    let (a4, a_tail) = in_a.split_at(head);
    let (b4, b_tail) = in_b.split_at(head);
    let (oa4, oa_tail) = out_a.split_at_mut(head);
    let (ob4, ob_tail) = out_b.split_at_mut(head);
    for (((a, b), oa), ob) in a4
        .chunks_exact(4)
        .zip(b4.chunks_exact(4))
        .zip(oa4.chunks_exact_mut(4))
        .zip(ob4.chunks_exact_mut(4))
    {
        let (Some(a), Some(b), Some(oa), Some(ob)) = (
            a.first_chunk::<4>(),
            b.first_chunk::<4>(),
            oa.first_chunk_mut::<4>(),
            ob.first_chunk_mut::<4>(),
        ) else {
            continue;
        };
        if let Some((x, y)) = kernel.lanes(tok, V4::load(tok, a), V4::load(tok, b)) {
            x.store(oa);
            y.store(ob);
        } else {
            for lane in 0..4 {
                (oa[lane], ob[lane]) = kernel.point(a[lane], b[lane]);
            }
        }
    }
    scalar(kernel, a_tail, b_tail, oa_tail, ob_tail);
}
