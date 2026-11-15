//! PackBits (TIFF `Compression` 32773): a header byte `n` gives `n + 1` literal bytes for
//! `0..=127`, is a no-op for 128, and repeats the next byte `257 - n` times for `129..=255`.

/// Decodes `input` into `expected` bytes, as libtiff does: a run that would pass `expected` is cut
/// there, and bytes after the last one needed are ignored. A stream that ends first is refused.
pub(crate) fn decode(input: &[u8], expected: usize) -> Result<Vec<u8>, ()> {
    let mut out = Vec::with_capacity(expected);
    let mut index = 0;
    while out.len() < expected {
        let header = *input.get(index).ok_or(())?;
        index += 1;
        let room = expected - out.len();
        match header {
            0..=127 => {
                let count = usize::from(header) + 1;
                let literal = input.get(index..index + count).ok_or(())?;
                out.extend_from_slice(&literal[..count.min(room)]);
                index += count;
            }
            128 => {}
            _ => {
                let count = 257 - usize::from(header);
                let byte = *input.get(index).ok_or(())?;
                index += 1;
                out.resize(out.len() + count.min(room), byte);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn literals_and_runs_decode_in_order() {
        let stream = [0x01, 0x10, 0x20, 0xFE, 0xAA, 0x00, 0x07];
        assert_eq!(
            decode(&stream, 6),
            Ok(vec![0x10, 0x20, 0xAA, 0xAA, 0xAA, 0x07])
        );
    }

    #[test]
    fn the_no_op_header_is_skipped() {
        assert_eq!(decode(&[0x80, 0x00, 0x09], 1), Ok(vec![0x09]));
    }

    #[test]
    fn a_run_past_the_expected_length_is_cut() {
        assert_eq!(decode(&[0xFD, 0xAA], 2), Ok(vec![0xAA, 0xAA]));
    }

    #[test]
    fn bytes_after_the_expected_length_are_ignored() {
        assert_eq!(decode(&[0x00, 0x01, 0x00, 0x02], 1), Ok(vec![0x01]));
    }

    #[test]
    fn a_stream_that_ends_early_is_refused() {
        assert_eq!(decode(&[0x02, 0x01], 3), Err(()));
        assert_eq!(decode(&[0xFE], 3), Err(()));
    }
}
