//! TIFF LZW (`Compression` 5) and the predictors that go with it, for the chunks that
//! [`super::tiles`] reads. The `tiff` crate of this build has no LZW reader (its `lzw` feature is
//! off, and enabling it would add a crate), so the chunks are decoded here.

use super::geotiff::ByteOrder;

/// The predictor of a chunk (`Predictor` 1 to 3). `Horizontal` adds the sample `stride` samples
/// before; `Float` is libtiff's floating point predictor, `fpAcc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Predictor {
    Plain,
    Horizontal,
    Float,
}

const CLEAR: u16 = 256;
const END: u16 = 257;
const FIRST: usize = 258;
const TABLE: usize = 4096;

/// Decodes the LZW codes of one chunk into exactly `expected` bytes. A stream that ends without its
/// end code is accepted; any other stream that does not give `expected` bytes is refused.
pub(super) fn decode(input: &[u8], expected: usize) -> Result<Vec<u8>, ()> {
    let mut codes = Codes {
        bytes: input,
        bit: 0,
    };
    let mut table = Table::new();
    let mut out = Vec::with_capacity(expected);
    let mut width = 9;
    let mut previous: Option<u16> = None;
    while let Some(code) = codes.read(width) {
        if code == CLEAR {
            table.next = FIRST;
            width = 9;
            previous = None;
            continue;
        }
        if code == END {
            break;
        }
        let Some(prev) = previous else {
            let byte = u8::try_from(code).map_err(|_| ())?;
            out.push(byte);
            previous = Some(code);
            continue;
        };
        let known = usize::from(code) < table.next;
        if !known && (usize::from(code) != table.next || table.next >= TABLE) {
            return Err(());
        }
        let first = table.first[usize::from(if known { code } else { prev })];
        if table.next < TABLE {
            table.add(prev, first);
            if table.next + 1 >= 1 << width && width < 12 {
                width += 1;
            }
        }
        table.emit(code, &mut out);
        if out.len() > expected {
            return Err(());
        }
        previous = Some(code);
    }
    if out.len() == expected {
        Ok(out)
    } else {
        Err(())
    }
}

/// Undoes `predictor` in place, row by row: each row is `row_bytes` long, and its samples are
/// `stride` samples apart (the samples of a pixel when chunky, one when planar). `bits` is the
/// sample size; `Float` is for little-endian files only.
pub(super) fn undo_predictor(
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

/// The LZW table: each entry is a string of its `prefix` entry and one byte, `last`, kept with the
/// first byte of its string and its length.
struct Table {
    prefix: Vec<u16>,
    last: Vec<u8>,
    first: Vec<u8>,
    len: Vec<usize>,
    next: usize,
}

impl Table {
    fn new() -> Self {
        // Entries from 256 on are written when they are added, before any code reads them.
        let last: Vec<u8> = (0..TABLE).map(|index| index as u8).collect();
        Self {
            prefix: vec![0; TABLE],
            first: last.clone(),
            last,
            len: vec![1; TABLE],
            next: FIRST,
        }
    }

    fn add(&mut self, prefix: u16, byte: u8) {
        let index = self.next;
        let before = usize::from(prefix);
        self.prefix[index] = prefix;
        self.last[index] = byte;
        self.first[index] = self.first[before];
        self.len[index] = self.len[before] + 1;
        self.next += 1;
    }

    fn emit(&self, code: u16, out: &mut Vec<u8>) {
        let len = self.len[usize::from(code)];
        let start = out.len();
        out.resize(start + len, 0);
        let mut current = usize::from(code);
        for slot in out[start..].iter_mut().rev() {
            *slot = self.last[current];
            current = usize::from(self.prefix[current]);
        }
    }
}

/// The codes of a stream, most significant bit first.
struct Codes<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl Codes<'_> {
    fn read(&mut self, width: u32) -> Option<u16> {
        let end = self.bit + usize::try_from(width).ok()?;
        if end > self.bytes.len() * 8 {
            return None;
        }
        let mut value = 0u16;
        for _ in 0..width {
            let byte = self.bytes[self.bit / 8];
            let bit = (byte >> (7 - self.bit % 8)) & 1;
            value = (value << 1) | u16::from(bit);
            self.bit += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example of the TIFF 6.0 specification, section 13: `-----A---B` in nine bytes.
    const SPEC_STREAM: [u8; 9] = [0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];

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
    fn decodes_the_specification_example() {
        let decoded = decode(&SPEC_STREAM, 10).expect("the example decodes");
        assert_eq!(decoded, b"-----A---B");
    }

    #[test]
    fn refuses_a_first_code_that_is_not_a_byte() {
        let stream = [0x80, 0x4B, 0x00];
        assert!(decode(&stream, 1).is_err());
    }

    #[test]
    fn refuses_a_length_other_than_expected() {
        assert!(decode(&SPEC_STREAM, 9).is_err());
        assert!(decode(&SPEC_STREAM, 11).is_err());
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
