//! The images that the TIFF decoder of this build does not read: LZW and PackBits. Their chunks are
//! read and decoded here, by the same functions as the lazy reader ([`tiles::decode_coded`]), so a
//! local grid and a remote one give the same samples.

use std::io::{Read, Seek, SeekFrom};

use tiff::decoder::Decoder;
use tiff::tags::Tag;

use super::samples::place;
use super::tags::optional_u32;
use crate::projections::gridshift::tiles::{decode_coded, Chunk, Coded, Geometry, Layout, Storage};
use crate::projections::tiff_codec::predictor::Predictor;
use crate::projections::tiff_codec::ByteOrder;

/// The codec of an image that is decoded here, from its `Compression` and `Predictor`. LZW has the
/// predictors of [`lzw_predictor`]. libtiff applies the predictor to LZW only, so a PackBits image
/// is read without one. `None` for the other compressions.
pub(super) fn coded_of(compression: u32, predictor: u32, bits: u32, format: u32) -> Option<Coded> {
    match (compression, predictor) {
        (5, 1..=3) => lzw_predictor(predictor, bits, format).map(Coded::Lzw),
        (32_773, 1..=3) => Some(Coded::PackBits),
        _ => None,
    }
}

/// The compressions whose samples are never read: JPEG (7) and CCITT (2, 3, 4). libtiff refuses them
/// and PROJ still opens the grid. Old JPEG (6) is refused at creation by PROJ too.
pub(super) fn is_undecoded(compression: u32) -> bool {
    matches!(compression, 2 | 3 | 4 | 7)
}

/// The predictor of an LZW image: none, 2 for integers of 16 or 32 bits, and 3 for floats of 32 or
/// 64 bits. `None` for the others.
fn lzw_predictor(predictor: u32, bits: u32, format: u32) -> Option<Predictor> {
    match predictor {
        1 => Some(Predictor::Plain),
        2 if matches!(format, 1 | 2) && matches!(bits, 16 | 32) => Some(Predictor::Horizontal),
        3 if format == 3 && matches!(bits, 32 | 64) => Some(Predictor::Float),
        _ => None,
    }
}

/// An image decoded whole: its size and sample layout, as its tags give them.
pub(super) struct Whole {
    pub(super) size: (u32, u32),
    pub(super) spp: usize,
    pub(super) storage: Storage,
    pub(super) bits: u32,
    pub(super) format: u32,
    pub(super) order: ByteOrder,
}

/// The samples of an image decoded by `coded`, every chunk read from the file and placed.
pub(super) fn decode_coded_samples<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    coded: Coded,
    image: &Whole,
) -> Result<Vec<f32>, ()> {
    let layout = if decoder.get_tag_u32(Tag::TileWidth).is_ok() {
        Layout::Tiles
    } else {
        Layout::Strips
    };
    let (offset_tag, count_tag) = match layout {
        Layout::Tiles => (Tag::TileOffsets, Tag::TileByteCounts),
        Layout::Strips => (Tag::StripOffsets, Tag::StripByteCounts),
    };
    let tile = decoder.chunk_dimensions();
    let geo = Geometry::new(image.size, tile, layout, image.spp, image.storage)?;
    let offsets = decoder.get_tag_u64_vec(offset_tag).map_err(|_| ())?;
    let counts = decoder.get_tag_u64_vec(count_tag).map_err(|_| ())?;
    if offsets.len() != counts.len() || offsets.len() != geo.chunk_count()? {
        return Err(());
    }
    let width = geo.width;
    let len = width
        .checked_mul(geo.height)
        .and_then(|pixels| pixels.checked_mul(geo.spp))
        .ok_or(())?;
    if len > 64 * 1024 * 1024 {
        return Err(());
    }
    let mut dest = vec![0.0_f32; len];
    for (index, (offset, count)) in offsets.into_iter().zip(counts).enumerate() {
        let plane = index / geo.per_plane;
        let tile_index = index % geo.per_plane;
        let (valid_w, valid_h) = geo.data_dims(tile_index);
        let x0 = (tile_index % geo.across) * geo.tile_w;
        let y0 = (tile_index / geo.across) * geo.tile_h;
        let size = usize::try_from(count).map_err(|_| ())?;
        let bytes = chunk_bytes(decoder.inner(), offset, size)?;
        let chunk = Chunk {
            len: geo.expected_len(tile_index),
            row_samples: geo.tile_w * geo.chunk_spp,
            chunk_spp: geo.chunk_spp,
            bits: image.bits,
            format: image.format,
            order: image.order,
        };
        let values = decode_coded(&bytes, coded, chunk)?;
        place(
            &mut dest,
            width,
            geo.spp,
            image.storage,
            plane,
            x0,
            y0,
            geo.tile_w,
            valid_w,
            valid_h,
            &values,
        )?;
    }
    Ok(dest)
}

/// `size` bytes at `offset` of the file: one read, as a chunk is read by the lazy reader.
fn chunk_bytes<R: Read + Seek>(reader: &mut R, offset: u64, size: usize) -> Result<Vec<u8>, ()> {
    reader.seek(SeekFrom::Start(offset)).map_err(|_| ())?;
    // `size` is a byte count the file declares: allocating it first lets a 40-byte file ask for
    // gigabytes. Read what the file holds, then compare.
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(u64::try_from(size).map_err(|_| ())?)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() != size {
        return Err(());
    }
    Ok(bytes)
}

/// The `Compression` and `Predictor` of an image: the tags that choose how its chunks are decoded.
pub(super) fn codec_tags<R: Read + Seek>(decoder: &mut Decoder<R>) -> Result<(u32, u32), ()> {
    let compression = optional_u32(decoder, Tag::Compression)?.unwrap_or(1);
    let predictor = optional_u32(decoder, Tag::Predictor)?.unwrap_or(1);
    Ok((compression, predictor))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::chunk_bytes;

    #[test]
    fn a_declared_size_past_the_file_is_an_error_not_an_allocation() {
        let mut file = Cursor::new(vec![0_u8; 40]);
        assert!(chunk_bytes(&mut file, 8, usize::MAX / 2).is_err());
        assert!(chunk_bytes(&mut file, 8, 33).is_err());
        assert_eq!(chunk_bytes(&mut file, 8, 32).map(|bytes| bytes.len()), Ok(32));
    }
}
