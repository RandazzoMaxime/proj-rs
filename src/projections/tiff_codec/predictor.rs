//! The TIFF predictors (`Predictor` tag 317), undone row by row after a chunk is decompressed. Shared
//! by the shift grids and the deformation grids, so that each predictor exists once.

use super::ByteOrder;

/// The predictor of a chunk (`Predictor` 1 to 3). `Horizontal` adds the sample `stride` samples
/// before; `Float` is libtiff's floating point predictor, `fpAcc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Predictor {
    Plain,
    Horizontal,
    Float,
}

/// Undoes `predictor` in place, row by row: each row is `row_bytes` long, and its samples are
/// `stride` samples apart (the samples of a pixel when chunky, one when planar). `bits` is the
/// sample size; `Float` is for little-endian files only.
pub(crate) fn undo_predictor(
    out: &mut [u8],
    row_bytes: usize,
    stride: usize,
    bits: u32,
    predictor: Predictor,
    order: ByteOrder,
) -> Result<(), ()> {
    if predictor == Predictor::Plain {
        return Ok(());
    }
    if row_bytes == 0 || out.len() % row_bytes != 0 || stride == 0 {
        return Err(());
    }
    let size = usize::try_from(bits / 8).map_err(|_| ())?;
    match predictor {
        Predictor::Plain => Ok(()),
        Predictor::Horizontal => {
            for row in out.chunks_exact_mut(row_bytes) {
                undo_horizontal(row, stride, size, order)?;
            }
            Ok(())
        }
        Predictor::Float => {
            if order != ByteOrder::Little {
                return Err(());
            }
            for row in out.chunks_exact_mut(row_bytes) {
                undo_float(row, stride, size)?;
            }
            Ok(())
        }
    }
}

fn undo_horizontal(row: &mut [u8], stride: usize, size: usize, order: ByteOrder) -> Result<(), ()> {
    if !matches!(size, 2 | 4) || row.len() % size != 0 {
        return Err(());
    }
    let mask = if size == 4 {
        u32::MAX
    } else {
        (1u32 << (8 * size)) - 1
    };
    let count = row.len() / size;
    for index in stride..count {
        let before = word(&row[(index - stride) * size..][..size], order);
        let current = word(&row[index * size..][..size], order);
        let sum = current.wrapping_add(before) & mask;
        put(&mut row[index * size..][..size], sum, order);
    }
    Ok(())
}

/// libtiff's `fpAcc` in reverse: a running sum of the bytes `stride` apart over the whole row, then
/// the byte planes, most significant first, gathered into little-endian samples.
fn undo_float(row: &mut [u8], stride: usize, size: usize) -> Result<(), ()> {
    if size == 0 || row.len() % size != 0 {
        return Err(());
    }
    for index in stride..row.len() {
        row[index] = row[index].wrapping_add(row[index - stride]);
    }
    let words = row.len() / size;
    let planes = row.to_vec();
    for index in 0..words {
        for byte in 0..size {
            row[index * size + byte] = planes[(size - 1 - byte) * words + index];
        }
    }
    Ok(())
}

fn word(bytes: &[u8], order: ByteOrder) -> u32 {
    match order {
        ByteOrder::Little => bytes
            .iter()
            .rev()
            .fold(0, |acc, &byte| (acc << 8) | u32::from(byte)),
        ByteOrder::Big => bytes
            .iter()
            .fold(0, |acc, &byte| (acc << 8) | u32::from(byte)),
    }
}

fn put(bytes: &mut [u8], value: u32, order: ByteOrder) {
    let len = bytes.len();
    for index in 0..len {
        let shift = match order {
            ByteOrder::Little => 8 * index,
            ByteOrder::Big => 8 * (len - 1 - index),
        };
        bytes[index] = ((value >> shift) & 0xFF) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The floating point predictor of libtiff's `fpDiff`: byte planes, most significant first,
    /// then a difference `stride` bytes apart, from the end of the row.
    fn float_encode(values: &[f32], stride: usize) -> Vec<u8> {
        let count = values.len();
        let mut planes = vec![0u8; 4 * count];
        for (index, value) in values.iter().enumerate() {
            for (byte, source) in value.to_le_bytes().iter().enumerate() {
                planes[(3 - byte) * count + index] = *source;
            }
        }
        for index in (stride..planes.len()).rev() {
            planes[index] = planes[index].wrapping_sub(planes[index - stride]);
        }
        planes
    }

    #[test]
    fn horizontal_predictor_adds_the_sample_before() {
        let mut row = vec![1, 0, 1, 0, 1, 0, 1, 0];
        undo_predictor(&mut row, 8, 1, 16, Predictor::Horizontal, ByteOrder::Little).expect("row");
        assert_eq!(row, vec![1, 0, 2, 0, 3, 0, 4, 0]);
    }

    #[test]
    fn horizontal_predictor_runs_per_pixel_and_wraps_at_the_sample_size() {
        let mut row = vec![1, 0, 10, 0, 1, 0, 10, 0];
        undo_predictor(&mut row, 8, 2, 16, Predictor::Horizontal, ByteOrder::Little).expect("row");
        assert_eq!(row, vec![1, 0, 10, 0, 2, 0, 20, 0]);

        let mut wrap = vec![0xFF, 0xFF, 0x01, 0x00];
        undo_predictor(
            &mut wrap,
            4,
            1,
            16,
            Predictor::Horizontal,
            ByteOrder::Little,
        )
        .expect("row");
        assert_eq!(wrap, vec![0xFF, 0xFF, 0x00, 0x00]);
    }

    #[test]
    fn float_predictor_inverts_the_libtiff_encoder() {
        let values = [0.25_f32, -1.5, 3.75e-3, 1024.0, -0.0, 7.0];
        for stride in [1, 2, 3] {
            let mut row = float_encode(&values, stride);
            let row_bytes = row.len();
            undo_predictor(
                &mut row,
                row_bytes,
                stride,
                32,
                Predictor::Float,
                ByteOrder::Little,
            )
            .expect("row");
            let expected: Vec<u8> = values
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            assert_eq!(row, expected, "stride {stride}");
        }
    }
}
