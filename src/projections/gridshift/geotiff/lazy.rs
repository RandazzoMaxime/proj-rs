//! The layout of an image read chunk by chunk: its tiles or strips, and the compression and
//! predictor that [`tiles`](super::super::tiles) undoes. Any other image is read whole.

use std::io::{Read, Seek};

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use super::tags::optional_u32;
use crate::projections::gridshift::tiles::{Layout, Reading};
use crate::projections::tiff_codec::predictor::Predictor;
use crate::projections::tiff_codec::ByteOrder;

/// How the chunks of an image are read from the file, when they can be read chunk by chunk: tiles
/// or strips, and the compression and the predictor are ones the reader undoes. `Raw` is no
/// compression and no predictor, read as the bytes are. `Decoded` is Deflate (`Compression` 8, or
/// 32946 for the old Deflate), with or without a predictor, or no compression with the predictor 2
/// or 3: the TIFF decoder undoes them. LZW (`Compression` 5) is decoded by [`super::lzw`], with
/// the predictors of [`lzw_predictor`]. `None` for anything else, which is read whole, as [`load`]
/// reads it.
pub(super) fn lazy_layout<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    order: ByteOrder,
    bits: u32,
    format: u32,
) -> Result<Option<(Layout, Reading)>, ()> {
    let compression = optional_u32(decoder, Tag::Compression)?.unwrap_or(1);
    let predictor = optional_u32(decoder, Tag::Predictor)?.unwrap_or(1);
    let reading = match (compression, predictor) {
        (1, 1) => Reading::Raw,
        (1 | 8 | 32_946, 1..=3) => Reading::Decoded,
        (5, 1..=3) => match lzw_predictor(predictor, order, bits, format) {
            Some(predictor) => Reading::Lzw(predictor),
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    let layout = if decoder.get_tag_u32(Tag::TileWidth).is_ok() {
        Layout::Tiles
    } else {
        Layout::Strips
    };
    Ok(Some((layout, reading)))
}

/// The predictor of an LZW image that [`super::lzw`] undoes: none, 2 for integers of 16 or 32
/// bits, and 3 for floats of 32 or 64 bits in a little-endian file. `None` for the others.
fn lzw_predictor(predictor: u32, order: ByteOrder, bits: u32, format: u32) -> Option<Predictor> {
    match predictor {
        1 => Some(Predictor::Plain),
        2 if matches!(format, 1 | 2) && matches!(bits, 16 | 32) => Some(Predictor::Horizontal),
        3 if format == 3 && matches!(bits, 32 | 64) && order == ByteOrder::Little => {
            Some(Predictor::Float)
        }
        _ => None,
    }
}
