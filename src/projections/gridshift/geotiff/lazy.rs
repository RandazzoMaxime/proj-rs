//! The layout of an image read chunk by chunk: its tiles or strips, and the compression and
//! predictor that [`tiles`](crate::projections::gridshift::tiles) undoes. Any other image is read
//! whole.

use std::io::{Read, Seek};

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use super::coded::coded_of;
use super::tags::optional_u32;
use crate::projections::gridshift::tiles::{Layout, Reading};

/// How the chunks of an image are read from the file, when they can be read chunk by chunk: tiles
/// or strips, and the compression and the predictor are ones the reader undoes. `Raw` is no
/// compression and no predictor, read as the bytes are. `Decoded` is Deflate (`Compression` 8, or
/// 32946 for the old Deflate), with or without a predictor, or no compression with the predictor 2
/// or 3: the TIFF decoder undoes them. LZW and PackBits are `Coded`, see [`coded_of`]. `None` for
/// anything else, which is read whole, as [`super::load`] reads it.
pub(super) fn lazy_layout<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    bits: u32,
    format: u32,
) -> Result<Option<(Layout, Reading)>, ()> {
    let compression = optional_u32(decoder, Tag::Compression)?.unwrap_or(1);
    let predictor = optional_u32(decoder, Tag::Predictor)?.unwrap_or(1);
    let reading = match (compression, predictor) {
        (1, 1) => Reading::Raw,
        (1 | 8 | 32_946, 1..=3) => Reading::Decoded,
        _ => match coded_of(compression, predictor, bits, format) {
            Some(coded) => Reading::Coded(coded),
            None => return Ok(None),
        },
    };
    let layout = if decoder.get_tag_u32(Tag::TileWidth).is_ok() {
        Layout::Tiles
    } else {
        Layout::Strips
    };
    Ok(Some((layout, reading)))
}
