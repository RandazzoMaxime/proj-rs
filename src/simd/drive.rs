//! Le driver par groupes de quatre, compilé avec AVX2 et FMA (par `pulp`, sans `unsafe` ici).

use super::{scalar, scalar3, Lanes, Lanes3, Token, V4};

/// Parcourt les tranches par groupes de quatre.
///
/// [`Token::vectorize`] est le seul point où le compilateur reçoit `avx2,fma` : la boucle
/// [`groups`] et le noyau [`Lanes::lanes`] y sont insérés (`#[inline(always)]`), donc leurs
/// intrinsèques deviennent des instructions en ligne.
pub(super) fn run_avx2<K: Lanes>(
    tok: Token,
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    let mut out = (out_a, out_b);
    tok.vectorize(Groups {
        tok,
        kernel,
        in_a,
        in_b,
        out: &mut out,
    });
}

/// Les arguments de [`groups`], pour [`Token::vectorize`]. Une fermeture n'y serait pas insérée
/// en ligne (elle n'hérite pas des extensions de la fonction qui l'appelle) : ses intrinsèques
/// deviendraient des appels, trente fois plus lents. Une méthode `#[inline(always)]` l'est.
///
/// Les sorties passent par une seule référence : la structure tient alors en huit mots, et
/// `pulp` la passe en registres.
struct Groups<'a, 'o, K> {
    tok: Token,
    kernel: &'a K,
    in_a: &'a [f64],
    in_b: &'a [f64],
    out: &'a mut (&'o mut [f64], &'o mut [f64]),
}

impl<K: Lanes> pulp::NullaryFnOnce for Groups<'_, '_, K> {
    type Output = ();

    #[inline(always)]
    fn call(self) {
        let (out_a, out_b) = self.out;
        groups(self.tok, self.kernel, self.in_a, self.in_b, out_a, out_b);
    }
}

#[inline(always)]
fn groups<K: Lanes>(
    tok: Token,
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
    tok: Token,
    kernel: &K,
    input: [&[f64]; 3],
    output: [&mut [f64]; 3],
) {
    let mut output = output;
    tok.vectorize(Groups3 {
        tok,
        kernel,
        input: &input,
        output: &mut output,
    });
}

/// Les arguments de [`groups3`], pour [`Token::vectorize`] (voir [`Groups`]) : trois mots, passés
/// en registres.
struct Groups3<'a, 'o, K> {
    tok: Token,
    kernel: &'a K,
    input: &'a [&'o [f64]; 3],
    output: &'a mut [&'o mut [f64]; 3],
}

impl<K: Lanes3> pulp::NullaryFnOnce for Groups3<'_, '_, K> {
    type Output = ();

    #[inline(always)]
    fn call(self) {
        let [oa, ob, oc] = self.output;
        groups3(
            self.tok,
            self.kernel,
            *self.input,
            [&mut **oa, &mut **ob, &mut **oc],
        );
    }
}

#[inline(always)]
fn groups3<K: Lanes3>(tok: Token, kernel: &K, input: [&[f64]; 3], output: [&mut [f64]; 3]) {
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
