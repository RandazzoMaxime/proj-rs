//! Le driver par groupes de `N`, compilé avec les extensions du jeton (par `pulp`, sans `unsafe`
//! ici).

use super::{scalar, scalar3, Lanes, Lanes3, SimdToken, Vector};

/// Parcourt les tranches par groupes de `N`.
///
/// [`super::SimdToken::vectorize`] est le seul point où le compilateur reçoit les extensions du
/// jeton : la boucle [`groups`] et le noyau [`Lanes::lanes`] y sont insérés (`#[inline(always)]`),
/// donc leurs intrinsèques deviennent des instructions en ligne.
pub(super) fn run_simd<V: Vector<N>, K: Lanes, const N: usize>(
    tok: V::Token,
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    let mut out = (out_a, out_b);
    tok.vectorize(Groups::<V, K, N> {
        tok,
        kernel,
        in_a,
        in_b,
        out: &mut out,
    });
}

/// Les arguments de [`groups`], pour [`super::SimdToken::vectorize`]. Une fermeture n'y serait pas
/// insérée en ligne (elle n'hérite pas des extensions de la fonction qui l'appelle) : ses
/// intrinsèques deviendraient des appels, trente fois plus lents. Une méthode
/// `#[inline(always)]` l'est.
///
/// Les sorties passent par une seule référence : la structure tient alors en huit mots, et
/// `pulp` la passe en registres.
struct Groups<'a, 'o, V: Vector<N>, K, const N: usize> {
    tok: V::Token,
    kernel: &'a K,
    in_a: &'a [f64],
    in_b: &'a [f64],
    out: &'a mut (&'o mut [f64], &'o mut [f64]),
}

impl<V: Vector<N>, K: Lanes, const N: usize> pulp::NullaryFnOnce for Groups<'_, '_, V, K, N> {
    type Output = ();

    #[inline(always)]
    fn call(self) {
        let (out_a, out_b) = self.out;
        groups::<V, K, N>(self.tok, self.kernel, self.in_a, self.in_b, out_a, out_b);
    }
}

#[inline(always)]
fn groups<V: Vector<N>, K: Lanes, const N: usize>(
    tok: V::Token,
    kernel: &K,
    in_a: &[f64],
    in_b: &[f64],
    out_a: &mut [f64],
    out_b: &mut [f64],
) {
    let head = in_a.len() & !(N - 1);
    let (a_head, a_tail) = in_a.split_at(head);
    let (b_head, b_tail) = in_b.split_at(head);
    let (oa_head, oa_tail) = out_a.split_at_mut(head);
    let (ob_head, ob_tail) = out_b.split_at_mut(head);
    let a4 = a_head
        .chunks_exact(N)
        .map(|chunk| <&[f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let b4 = b_head
        .chunks_exact(N)
        .map(|chunk| <&[f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let oa4 = oa_head
        .chunks_exact_mut(N)
        .map(|chunk| <&mut [f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let ob4 = ob_head
        .chunks_exact_mut(N)
        .map(|chunk| <&mut [f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    for (((a, b), oa), ob) in a4.zip(b4).zip(oa4).zip(ob4) {
        if let Some((x, y)) = kernel.lanes::<V, N>(tok, V::load(tok, a), V::load(tok, b)) {
            x.store(oa);
            y.store(ob);
        } else {
            for lane in 0..N {
                (oa[lane], ob[lane]) = kernel.point(a[lane], b[lane]);
            }
        }
    }
    scalar(kernel, a_tail, b_tail, oa_tail, ob_tail);
}

/// [`run_simd`] pour un noyau à trois entrées et trois sorties.
pub(super) fn run3_simd<V: Vector<N>, K: Lanes3, const N: usize>(
    tok: V::Token,
    kernel: &K,
    input: [&[f64]; 3],
    output: [&mut [f64]; 3],
) {
    let mut output = output;
    tok.vectorize(Groups3::<V, K, N> {
        tok,
        kernel,
        input: &input,
        output: &mut output,
    });
}

/// Les arguments de [`groups3`], pour [`super::SimdToken::vectorize`] (voir [`Groups`]) : trois
/// mots, passés en registres.
struct Groups3<'a, 'o, V: Vector<N>, K, const N: usize> {
    tok: V::Token,
    kernel: &'a K,
    input: &'a [&'o [f64]; 3],
    output: &'a mut [&'o mut [f64]; 3],
}

impl<V: Vector<N>, K: Lanes3, const N: usize> pulp::NullaryFnOnce for Groups3<'_, '_, V, K, N> {
    type Output = ();

    #[inline(always)]
    fn call(self) {
        let [oa, ob, oc] = self.output;
        groups3::<V, K, N>(
            self.tok,
            self.kernel,
            *self.input,
            [&mut **oa, &mut **ob, &mut **oc],
        );
    }
}

#[inline(always)]
fn groups3<V: Vector<N>, K: Lanes3, const N: usize>(
    tok: V::Token,
    kernel: &K,
    input: [&[f64]; 3],
    output: [&mut [f64]; 3],
) {
    let head = input[0].len() & !(N - 1);
    let [ia, ib, ic] = input;
    let [oa, ob, oc] = output;
    let ((ia_head, ia_tail), (ib_head, ib_tail), (ic_head, ic_tail)) =
        (ia.split_at(head), ib.split_at(head), ic.split_at(head));
    let ((oa_head, oa_tail), (ob_head, ob_tail), (oc_head, oc_tail)) = (
        oa.split_at_mut(head),
        ob.split_at_mut(head),
        oc.split_at_mut(head),
    );
    let ia4 = ia_head
        .chunks_exact(N)
        .map(|chunk| <&[f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let ib4 = ib_head
        .chunks_exact(N)
        .map(|chunk| <&[f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let ic4 = ic_head
        .chunks_exact(N)
        .map(|chunk| <&[f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let oa4 = oa_head
        .chunks_exact_mut(N)
        .map(|chunk| <&mut [f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let ob4 = ob_head
        .chunks_exact_mut(N)
        .map(|chunk| <&mut [f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    let oc4 = oc_head
        .chunks_exact_mut(N)
        .map(|chunk| <&mut [f64; N]>::try_from(chunk).expect("a chunk of exactly N"));
    for (((((a, b), c), oa), ob), oc) in ia4.zip(ib4).zip(ic4).zip(oa4).zip(ob4).zip(oc4) {
        let loaded = (V::load(tok, a), V::load(tok, b), V::load(tok, c));
        if let Some((x, y, z)) = kernel.lanes::<V, N>(tok, loaded.0, loaded.1, loaded.2) {
            x.store(oa);
            y.store(ob);
            z.store(oc);
        } else {
            for lane in 0..N {
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
