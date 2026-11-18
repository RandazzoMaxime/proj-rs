//! Le driver par groupes de quatre, compilé avec AVX2 et FMA.

use super::{scalar, scalar3, Avx2, Lanes, Lanes3, V4};

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
    let a4 = crate::compat::quads(a4);
    let b4 = crate::compat::quads(b4);
    let oa4 = crate::compat::quads_mut(oa4);
    let ob4 = crate::compat::quads_mut(ob4);
    for (((a, b), oa), ob) in a4.zip(b4).zip(oa4).zip(ob4) {
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

/// [`run_avx2`] pour un noyau à trois entrées et trois sorties.
pub(super) fn run3_avx2<K: Lanes3>(
    tok: Avx2,
    kernel: &K,
    input: [&[f64]; 3],
    output: [&mut [f64]; 3],
) {
    // SAFETY: `tok` n'existe que si le processeur supporte AVX2 et FMA.
    unsafe { groups3(tok, kernel, input, output) };
}

/// # Safety
///
/// Le processeur doit supporter AVX2 et FMA. `tok` en est la preuve.
#[target_feature(enable = "avx2,fma")]
unsafe fn groups3<K: Lanes3>(tok: Avx2, kernel: &K, input: [&[f64]; 3], output: [&mut [f64]; 3]) {
    let head = input[0].len() & !3;
    let [ia, ib, ic] = input;
    let [oa, ob, oc] = output;
    let ((ia4, ia_tail), (ib4, ib_tail), (ic4, ic_tail)) =
        (ia.split_at(head), ib.split_at(head), ic.split_at(head));
    let ((oa4, oa_tail), (ob4, ob_tail), (oc4, oc_tail)) = (
        oa.split_at_mut(head),
        ob.split_at_mut(head),
        oc.split_at_mut(head),
    );
    let ia4 = crate::compat::quads(ia4);
    let ib4 = crate::compat::quads(ib4);
    let ic4 = crate::compat::quads(ic4);
    let oa4 = crate::compat::quads_mut(oa4);
    let ob4 = crate::compat::quads_mut(ob4);
    let oc4 = crate::compat::quads_mut(oc4);
    for (((((a, b), c), oa), ob), oc) in ia4.zip(ib4).zip(ic4).zip(oa4).zip(ob4).zip(oc4) {
        let loaded = (V4::load(tok, a), V4::load(tok, b), V4::load(tok, c));
        if let Some((x, y, z)) = kernel.lanes(tok, loaded.0, loaded.1, loaded.2) {
            x.store(oa);
            y.store(ob);
            z.store(oc);
        } else {
            for lane in 0..4 {
                (oa[lane], ob[lane], oc[lane]) = kernel.point(a[lane], b[lane], c[lane]);
            }
        }
    }
    scalar3(
        kernel,
        [ia_tail, ib_tail, ic_tail],
        [oa_tail, ob_tail, oc_tail],
    );
}
