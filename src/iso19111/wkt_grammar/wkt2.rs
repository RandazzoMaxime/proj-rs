//! `pj_wkt2_lex` (wkt2_parser.cpp:243-336).

use super::wkt2_tables::{KEYWORDS, T_STRING, T_UNSIGNED_INTEGER_DIFFERENT_ONE_TWO_THREE};
use super::Context;

/// The C `EOF` the lexer returns at the end of the input.
const EOF: i32 = -1;
/// The printed quotes U+201C and U+201D of the examples of OGC 12-063r5.
const START_PRINTED_QUOTE: &[u8] = "\u{201C}".as_bytes();
const END_PRINTED_QUOTE: &[u8] = "\u{201D}".as_bytes();

/// The next token: a keyword, an unsigned integer (`1`, `2` and `3` alone are their own
/// character), `"…"` with `""` escapes, `“…”`, or one character.
pub(super) fn lex(context: &mut Context<'_>) -> i32 {
    let mut start = context.next;
    while matches!(context.at(start), b' ' | b'\t' | 10 | 13) {
        start += 1;
    }
    context.last_success = start;
    let first = context.at(start);
    if first == 0 {
        context.next = start;
        return EOF;
    }
    if let Some(code) = context.keyword(start, &KEYWORDS) {
        return code;
    }
    if first.is_ascii_digit() {
        if matches!(first, b'1' | b'2' | b'3') && !context.at(start + 1).is_ascii_digit() {
            context.next = start + 1;
            return i32::from(first);
        }
        let mut end = start + 1;
        while context.at(end).is_ascii_digit() {
            end += 1;
        }
        context.next = end;
        return T_UNSIGNED_INTEGER_DIFFERENT_ONE_TWO_THREE;
    }
    if first == b'"' {
        let mut end = start + 1;
        while context.at(end) != 0 {
            if context.at(end) == b'"' {
                if context.at(end + 1) == b'"' {
                    end += 1;
                } else {
                    break;
                }
            }
            end += 1;
        }
        if context.at(end) == 0 {
            context.next = end;
            return EOF;
        }
        context.next = end + 1;
        return T_STRING;
    }
    if context.input[start..].starts_with(START_PRINTED_QUOTE) {
        let rest = &context.input[start..];
        let Some(offset) = rest
            .windows(END_PRINTED_QUOTE.len())
            .position(|window| window == END_PRINTED_QUOTE)
        else {
            context.next = context.input.len();
            return EOF;
        };
        context.next = start + offset + END_PRINTED_QUOTE.len();
        return T_STRING;
    }
    context.single_char(start)
}
