//! TIFF LZW (`Compression` 5), for the chunks that the shift grids read. The `tiff` crate of this
//! build has no LZW reader (its `lzw` feature is off, and enabling it would add a crate).

const CLEAR: u16 = 256;
const END: u16 = 257;
const FIRST: usize = 258;
const TABLE: usize = 4096;

/// Decodes the LZW codes of one chunk into exactly `expected` bytes. A stream that ends without its
/// end code is accepted; any other stream that does not give `expected` bytes is refused.
pub(crate) fn decode(input: &[u8], expected: usize) -> Result<Vec<u8>, ()> {
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
}
