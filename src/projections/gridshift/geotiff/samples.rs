//! The samples of an image, decoded whole, and the conversion of each sample type to `f32`, as
//! PROJ stores every grid sample.

use std::io::{Read, Seek};

use tiff::decoder::{Decoder, DecodingResult};

use crate::projections::gridshift::tiles::Storage;
use crate::projections::tiff_codec::ByteOrder;

pub(super) fn decode_samples<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    width: u32,
    height: u32,
    spp: usize,
    storage: Storage,
) -> Result<Vec<f32>, ()> {
    let (tile_w, tile_h) = decoder.chunk_dimensions();
    if tile_w == 0 || tile_h == 0 {
        return Err(());
    }
    let across = width.div_ceil(tile_w);
    let down = height.div_ceil(tile_h);
    let per_plane = across.checked_mul(down).ok_or(())?;
    let planes = if matches!(storage, Storage::Separate) {
        u32::try_from(spp).map_err(|_| ())?
    } else {
        1
    };
    let total = per_plane.checked_mul(planes).ok_or(())?;
    let width_us = usize::try_from(width).map_err(|_| ())?;
    let height_us = usize::try_from(height).map_err(|_| ())?;
    let len = width_us
        .checked_mul(height_us)
        .and_then(|pixels| pixels.checked_mul(spp))
        .ok_or(())?;
    if len > 64 * 1024 * 1024 {
        return Err(());
    }
    let mut dest = vec![0.0_f32; len];
    for index in 0..total {
        let chunk = decoder.read_chunk(index).map_err(|_| ())?;
        let values = to_f32(chunk)?;
        let (data_w, data_h) = decoder.chunk_data_dimensions(index);
        let local = index % per_plane;
        let plane = usize::try_from(index / per_plane).map_err(|_| ())?;
        let col = local % across;
        let row = local / across;
        let x0 = usize::try_from(col.saturating_mul(tile_w)).map_err(|_| ())?;
        let y0 = usize::try_from(row.saturating_mul(tile_h)).map_err(|_| ())?;
        let valid_w = width_us
            .saturating_sub(x0)
            .min(usize::try_from(tile_w).unwrap_or(0));
        let valid_h = height_us
            .saturating_sub(y0)
            .min(usize::try_from(tile_h).unwrap_or(0));
        let data_w_us = usize::try_from(data_w).map_err(|_| ())?;
        let data_h_us = usize::try_from(data_h).map_err(|_| ())?;
        let copy_w = valid_w.min(data_w_us);
        let copy_h = valid_h.min(data_h_us);
        place(
            &mut dest, width_us, spp, storage, plane, x0, y0, data_w_us, copy_w, copy_h, &values,
        )?;
    }
    Ok(dest)
}

pub(super) fn place(
    dest: &mut [f32],
    width: usize,
    spp: usize,
    storage: Storage,
    plane: usize,
    x0: usize,
    y0: usize,
    data_w: usize,
    copy_w: usize,
    copy_h: usize,
    chunk: &[f32],
) -> Result<(), ()> {
    let separate = matches!(storage, Storage::Separate);
    let chunk_spp = if separate { 1 } else { spp };
    if separate && plane >= spp {
        return Err(());
    }
    if copy_w == 0 || spp == 0 {
        return Ok(());
    }
    // One row of the chunk at a time: its `copy_w` pixels are consecutive in the chunk, and each
    // is `spp` samples apart in `dest`. A row that runs past either end is an error, as the
    // sample of it that is out of range would be.
    for row in 0..copy_h {
        let src = (row * data_w).checked_mul(chunk_spp).ok_or(())?;
        let dst = ((y0 + row) * width + x0).checked_mul(spp).ok_or(())?;
        if separate {
            let from = src
                .checked_add(copy_w)
                .and_then(|end| chunk.get(src..end))
                .ok_or(())?;
            let first = dst.checked_add(plane).ok_or(())?;
            let last = (copy_w - 1)
                .checked_mul(spp)
                .and_then(|offset| first.checked_add(offset))
                .ok_or(())?;
            let to = dest.get_mut(first..=last).ok_or(())?;
            for (slot, sample) in to.iter_mut().step_by(spp).zip(from) {
                *slot = *sample;
            }
        } else {
            let len = copy_w.checked_mul(spp).ok_or(())?;
            let from = src
                .checked_add(len)
                .and_then(|end| chunk.get(src..end))
                .ok_or(())?;
            let to = dst
                .checked_add(len)
                .and_then(|end| dest.get_mut(dst..end))
                .ok_or(())?;
            to.copy_from_slice(from);
        }
    }
    Ok(())
}

/// The samples of an uncompressed tile, as `f32`. `format` and `bits` are the image's
/// `SampleFormat` and `BitsPerSample`, already checked by [`supported_sample`].
pub(in crate::projections::gridshift) fn raw_samples(
    bytes: &[u8],
    format: u32,
    bits: u32,
    order: ByteOrder,
) -> Result<Vec<f32>, ()> {
    let size = usize::try_from(bits / 8).map_err(|_| ())?;
    if size == 0 || !crate::compat::is_multiple_of(bytes.len(), size) {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / size);
    for piece in bytes.chunks_exact(size) {
        let value = match (format, bits) {
            (1, 16) => f32::from(word::<u16>(piece, order)?),
            (2, 16) => f32::from(word::<i16>(piece, order)?),
            (1, 32) => f32_from_u32(word::<u32>(piece, order)?),
            (2, 32) => f32_from_i32(word::<i32>(piece, order)?),
            (3, 32) => word::<f32>(piece, order)?,
            (3, 64) => f32_from_f64(word::<f64>(piece, order)?),
            _ => return Err(()),
        };
        out.push(value);
    }
    Ok(out)
}

/// One sample of `piece`, in the file's byte order.
fn word<T: TiffWord>(piece: &[u8], order: ByteOrder) -> Result<T, ()> {
    T::from_piece(piece, order).ok_or(())
}

/// A sample type that a TIFF stores in a fixed number of bytes.
trait TiffWord: Sized {
    fn from_piece(piece: &[u8], order: ByteOrder) -> Option<Self>;
}

macro_rules! tiff_word {
    ($($ty:ty),*) => {
        $(
            impl TiffWord for $ty {
                fn from_piece(piece: &[u8], order: ByteOrder) -> Option<Self> {
                    let bytes: [u8; std::mem::size_of::<$ty>()] = piece.try_into().ok()?;
                    Some(if order == ByteOrder::Big {
                        <$ty>::from_be_bytes(bytes)
                    } else {
                        <$ty>::from_le_bytes(bytes)
                    })
                }
            }
        )*
    };
}

tiff_word!(u16, i16, u32, i32, f32, f64);

/// The samples of one decoded chunk, as `f32` (every grid sample is a float in PROJ).
pub(in crate::projections::gridshift) fn to_f32(decoded: DecodingResult) -> Result<Vec<f32>, ()> {
    match decoded {
        DecodingResult::U16(values) => Ok(values.into_iter().map(f32::from).collect()),
        DecodingResult::I16(values) => Ok(values.into_iter().map(f32::from).collect()),
        DecodingResult::U32(values) => Ok(values.into_iter().map(f32_from_u32).collect()),
        DecodingResult::I32(values) => Ok(values.into_iter().map(f32_from_i32).collect()),
        DecodingResult::F32(values) => Ok(values),
        DecodingResult::F64(values) => Ok(values.into_iter().map(f32_from_f64).collect()),
        _ => Err(()),
    }
}

fn f32_from_u32(value: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

fn f32_from_i32(value: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

pub(super) fn f32_from_f64(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "PROJ stores every grid sample as float"
    )]
    {
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{place, raw_samples};
    use crate::projections::gridshift::tiles::Storage;
    use crate::projections::tiff_codec::ByteOrder;

    /// A chunky chunk of two samples per pixel lands in the pixels it covers, at the offset of
    /// its origin: a 2 x 2 window of a 3-pixel-wide image at (1, 1), from a chunk 2 pixels wide.
    #[test]
    fn place_copies_a_chunky_window_at_its_origin() {
        let chunk: Vec<f32> = (1..=8_u8).map(f32::from).collect();
        let mut dest = vec![0.0_f32; 3 * 3 * 2];
        place(&mut dest, 3, 2, Storage::Chunky, 0, 1, 1, 2, 2, 2, &chunk).expect("placed");
        // Pixels (1,1), (2,1), (1,2), (2,2) are 4, 5, 7, 8; their samples start at twice that.
        let mut expected = vec![0.0_f32; 18];
        expected[8..12].copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        expected[14..18].copy_from_slice(&[5.0, 6.0, 7.0, 8.0]);
        assert_eq!(dest, expected);
    }

    /// A separate plane lands in its own sample of each pixel, and leaves the others alone.
    #[test]
    fn place_writes_one_plane_of_a_separate_image() {
        let chunk = [10.0_f32, 20.0, 30.0, 40.0];
        let mut dest = vec![0.0_f32; 3 * 3 * 2];
        place(&mut dest, 3, 2, Storage::Separate, 1, 0, 0, 2, 2, 2, &chunk).expect("placed");
        // Pixels (0,0), (1,0), (0,1), (1,1) are 0, 1, 3, 4: their second sample is 1, 3, 7, 9.
        let mut expected = vec![0.0_f32; 18];
        expected[1] = 10.0;
        expected[3] = 20.0;
        expected[7] = 30.0;
        expected[9] = 40.0;
        assert_eq!(dest, expected);
    }

    /// A chunk too short for its window, or a destination too small for it, is an error, and an
    /// empty window is nothing to copy.
    #[test]
    fn place_refuses_a_window_that_runs_out_and_copies_nothing_for_an_empty_one() {
        let short = [0.0_f32; 3];
        let mut dest = vec![0.0_f32; 18];
        assert_eq!(
            place(&mut dest, 3, 2, Storage::Chunky, 0, 0, 0, 2, 2, 2, &short),
            Err(())
        );
        let chunk = [0.0_f32; 8];
        let mut small = vec![0.0_f32; 4];
        assert_eq!(
            place(&mut small, 3, 2, Storage::Chunky, 0, 1, 1, 2, 2, 2, &chunk),
            Err(())
        );
        assert_eq!(
            place(&mut small, 3, 2, Storage::Chunky, 0, 0, 0, 2, 0, 2, &short),
            Ok(())
        );
    }

    /// The samples of a tile are read in the file's byte order, whatever the sample type: a
    /// `u16` of `0x0102` is 258 in a big-endian file and 513 in a little-endian one.
    #[test]
    fn raw_samples_follow_the_file_byte_order() {
        assert_eq!(
            raw_samples(&[0x01, 0x02], 1, 16, ByteOrder::Big),
            Ok(vec![258.0])
        );
        assert_eq!(
            raw_samples(&[0x01, 0x02], 1, 16, ByteOrder::Little),
            Ok(vec![513.0])
        );
        assert_eq!(
            raw_samples(&[0xFF, 0xFE], 2, 16, ByteOrder::Big),
            Ok(vec![-2.0])
        );
        let float = 1.25_f32.to_be_bytes();
        assert_eq!(raw_samples(&float, 3, 32, ByteOrder::Big), Ok(vec![1.25]));
        let float = 1.25_f32.to_le_bytes();
        assert_eq!(
            raw_samples(&float, 3, 32, ByteOrder::Little),
            Ok(vec![1.25])
        );
    }

    /// A tile of a type the grid readers do not take, or a byte count that is not whole
    /// samples, is an error and not a short tile.
    #[test]
    fn raw_samples_refuse_other_types_and_partial_samples() {
        assert_eq!(raw_samples(&[0x01], 1, 8, ByteOrder::Big), Err(()));
        assert_eq!(
            raw_samples(&[0x01, 0x02, 0x03], 1, 16, ByteOrder::Big),
            Err(())
        );
    }
}
