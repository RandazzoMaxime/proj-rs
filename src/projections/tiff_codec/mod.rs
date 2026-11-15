//! TIFF codecs and predictors that PROJ reads through libtiff, and that this crate undoes itself:
//! LZW chunks and the predictors. The shift grids and the deformation grids share them.

pub(crate) mod lzw;
pub(crate) mod predictor;

/// The byte order of a TIFF file, from its first two bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ByteOrder {
    Little,
    Big,
}
