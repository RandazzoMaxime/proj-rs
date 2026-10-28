//! Zlib wrapper around DEFLATE, for the small GeoTIFF strips in the model tests.
//!
//! The bit order and the Huffman construction follow RFC 1951 and puff.c.

#![allow(clippy::pedantic, reason = "the bit loops follow RFC 1951")]

const MAX_BITS: usize = 15;
const MAX_OUT: usize = 32 * 1024 * 1024;

struct Huff {
    count: [u16; MAX_BITS + 1],
    symbol: Vec<u16>,
}

fn construct(lengths: &[u16]) -> Option<Huff> {
    let mut count = [0u16; MAX_BITS + 1];
    for &length in lengths {
        if usize::from(length) > MAX_BITS {
            return None;
        }
        count[usize::from(length)] = count[usize::from(length)].checked_add(1)?;
    }
    if usize::from(count[0]) == lengths.len() {
        return Some(Huff {
            count,
            symbol: Vec::new(),
        });
    }
    let mut left = 1i32;
    for len in 1..=MAX_BITS {
        left <<= 1;
        left -= i32::from(count[len]);
        if left < 0 {
            return None;
        }
    }
    let mut offs = [0usize; MAX_BITS + 1];
    for len in 1..MAX_BITS {
        offs[len + 1] = offs[len] + usize::from(count[len]);
    }
    let mut symbol = vec![0u16; lengths.len() - usize::from(count[0])];
    for (index, &length) in lengths.iter().enumerate() {
        if length != 0 {
            let slot = offs[usize::from(length)];
            offs[usize::from(length)] = slot + 1;
            let code = u16::try_from(index).ok()?;
            *symbol.get_mut(slot)? = code;
        }
    }
    Some(Huff { count, symbol })
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u32,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            acc: 0,
            bits: 0,
        }
    }

    fn pull(&mut self, n: u32) -> Option<u32> {
        while self.bits < n {
            let byte = *self.data.get(self.pos)?;
            self.pos += 1;
            self.acc |= u32::from(byte) << self.bits;
            self.bits += 8;
        }
        let mask = if n == 32 { u32::MAX } else { (1_u32 << n) - 1 };
        let out = self.acc & mask;
        self.acc >>= n;
        self.bits -= n;
        Some(out)
    }

    fn decode(&mut self, huff: &Huff) -> Option<u16> {
        let mut code = 0u32;
        let mut first = 0u32;
        let mut index = 0usize;
        for len in 1..=MAX_BITS {
            code |= self.pull(1)?;
            let count = u32::from(huff.count[len]);
            if code < first + count {
                let slot = index + usize::try_from(code - first).ok()?;
                return huff.symbol.get(slot).copied();
            }
            index += usize::try_from(count).ok()?;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

const LEN_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn fixed_pair() -> Option<(Huff, Huff)> {
    let mut lit = vec![0u16; 288];
    for (index, slot) in lit.iter_mut().enumerate() {
        *slot = match index {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    let dist = vec![5u16; 30];
    Some((construct(&lit)?, construct(&dist)?))
}

fn dynamic_pair(bits: &mut Bits<'_>) -> Option<(Huff, Huff)> {
    let n_lit = usize::try_from(bits.pull(5)? + 257).ok()?;
    let n_dist = usize::try_from(bits.pull(5)? + 1).ok()?;
    let n_code = usize::try_from(bits.pull(4)? + 4).ok()?;
    if n_lit > 286 || n_dist > 30 || n_code > 19 {
        return None;
    }
    let mut code_len = [0u16; 19];
    for index in 0..n_code {
        code_len[ORDER[index]] = u16::try_from(bits.pull(3)?).ok()?;
    }
    let coder = construct(&code_len)?;
    let mut lengths = vec![0u16; n_lit + n_dist];
    let mut index = 0usize;
    while index < lengths.len() {
        let symbol = bits.decode(&coder)?;
        match symbol {
            0..=15 => {
                lengths[index] = symbol;
                index += 1;
            }
            16 => {
                if index == 0 {
                    return None;
                }
                let run = usize::try_from(bits.pull(2)? + 3).ok()?;
                let previous = lengths[index - 1];
                for _ in 0..run {
                    *lengths.get_mut(index)? = previous;
                    index += 1;
                }
            }
            17 => {
                let run = usize::try_from(bits.pull(3)? + 3).ok()?;
                index = index.checked_add(run)?;
            }
            18 => {
                let run = usize::try_from(bits.pull(7)? + 11).ok()?;
                index = index.checked_add(run)?;
            }
            _ => return None,
        }
    }
    if index != lengths.len() {
        return None;
    }
    Some((construct(&lengths[..n_lit])?, construct(&lengths[n_lit..])?))
}

fn inflate_codes(bits: &mut Bits<'_>, lit: &Huff, dist: &Huff, out: &mut Vec<u8>) -> Option<()> {
    loop {
        let symbol = usize::from(bits.decode(lit)?);
        if symbol < 256 {
            out.push(u8::try_from(symbol).ok()?);
        } else if symbol == 256 {
            return Some(());
        } else {
            let length_index = symbol - 257;
            let base = *LEN_BASE.get(length_index)?;
            let extra = *LEN_EXTRA.get(length_index)?;
            let length = usize::from(base) + usize::try_from(bits.pull(u32::from(extra))?).ok()?;
            let dist_symbol = usize::from(bits.decode(dist)?);
            let base = *DIST_BASE.get(dist_symbol)?;
            let extra = *DIST_EXTRA.get(dist_symbol)?;
            let distance =
                usize::from(base) + usize::try_from(bits.pull(u32::from(extra))?).ok()?;
            if distance == 0 || distance > out.len() {
                return None;
            }
            for _ in 0..length {
                let source = out.len() - distance;
                let byte = *out.get(source)?;
                out.push(byte);
                if out.len() > MAX_OUT {
                    return None;
                }
            }
        }
        if out.len() > MAX_OUT {
            return None;
        }
    }
}

fn inflate_blocks(data: &[u8]) -> Option<Vec<u8>> {
    let mut bits = Bits::new(data);
    let mut out = Vec::new();
    let (fixed_lit, fixed_dist) = fixed_pair()?;
    loop {
        let last = bits.pull(1)?;
        let kind = bits.pull(2)?;
        match kind {
            0 => {
                let leftover = bits.bits % 8;
                if leftover != 0 {
                    bits.pull(leftover)?;
                }
                let len = bits.pull(16)?;
                let nlen = bits.pull(16)?;
                if len ^ 0xffff != nlen {
                    return None;
                }
                let start = bits.pos;
                let count = usize::try_from(len).ok()?;
                let end = start.checked_add(count)?;
                out.extend_from_slice(data.get(start..end)?);
                bits.pos = end;
                bits.acc = 0;
                bits.bits = 0;
            }
            1 => inflate_codes(&mut bits, &fixed_lit, &fixed_dist, &mut out)?,
            2 => {
                let (lit, dist) = dynamic_pair(&mut bits)?;
                inflate_codes(&mut bits, &lit, &dist, &mut out)?;
            }
            _ => return None,
        }
        if last == 1 {
            return Some(out);
        }
    }
}

/// Inflate a zlib stream (RFC 1950), as stored by GeoTIFF compression 8.
pub(super) fn zlib_decode(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 6 {
        return None;
    }
    let cmf = u16::from(data[0]);
    let flg = u16::from(data[1]);
    if (cmf * 256 + flg) % 31 != 0 || cmf & 0x0f != 8 || flg & 0x20 != 0 {
        return None;
    }
    inflate_blocks(&data[2..])
}
