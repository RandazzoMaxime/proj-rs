//! `pj_wkt1_lex` (wkt1_parser.cpp:94-193).

use super::wkt1_tables::{KEYWORDS, T_IDENTIFIER, T_NUMBER, T_STRING};
use super::Context;

/// The C `EOF` the lexer returns at the end of the input.
const EOF: i32 = -1;

/// The next token: a keyword, `"…"`, a number, an identifier, or one character.
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
    if first == b'"' {
        let mut end = start + 1;
        while context.at(end) != 0 && context.at(end) != b'"' {
            end += 1;
        }
        if context.at(end) == 0 {
            context.next = end;
            return EOF;
        }
        context.next = end + 1;
        return T_STRING;
    }
    if ((first == b'-' || first == b'+') && context.at(start + 1).is_ascii_digit())
        || first.is_ascii_digit()
    {
        let mut end = start;
        if first == b'-' || first == b'+' {
            end += 1;
        }
        while context.at(end).is_ascii_digit() {
            end += 1;
        }
        if context.at(end) == b'.' {
            end += 1;
            while context.at(end).is_ascii_digit() {
                end += 1;
            }
        }
        if matches!(context.at(end), b'e' | b'E') {
            end += 1;
            if matches!(context.at(end), b'-' | b'+') {
                end += 1;
            }
            while context.at(end).is_ascii_digit() {
                end += 1;
            }
        }
        context.next = end;
        return T_NUMBER;
    }
    if first.is_ascii_alphabetic() {
        let mut end = start + 1;
        while context.at(end).is_ascii_alphabetic() {
            end += 1;
        }
        context.next = end;
        return T_IDENTIFIER;
    }
    context.single_char(start)
}
