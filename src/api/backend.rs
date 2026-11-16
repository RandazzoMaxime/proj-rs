/// The code path a [`Transformer`](crate::Transformer) runs a slice of points on.
///
/// The enum is `#[non_exhaustive]`: `Simd` and `Gpu` will join it without a breaking change.
/// Every backend gives the result of PROJ 9.9.0 to the tolerance of `docs/PARITY.md`; the
/// batch path can differ from the scalar one by 11 nm at most (`docs/PORTING-simd.md`).
///
/// ```
/// use proj_rs::Backend;
///
/// assert_eq!(Backend::default(), Backend::Auto);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Backend {
    /// The best path available on this machine: today the vector path of the projections that
    /// have one (AVX2 on x86-64 with FMA), the scalar path elsewhere.
    #[default]
    Auto,
    /// One point at a time, the path of [`Transformer::forward`](crate::Transformer::forward).
    Scalar,
}
