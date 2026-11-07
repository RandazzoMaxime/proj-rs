//! The WKT1 syntax check of PROJ 9.9.0: `pj_wkt1_parse` (`src/wkt1_parser.cpp`).
//!
//! PROJ's grammar is `src/wkt1_grammar.y`, compiled by bison 3.5.1 into the tables
//! of `src/wkt1_generated_parser.c`, which [`tables`] carries. This module is the
//! lexer `pj_wkt1_lex`, the table-driven driver `yyparse`, bison's verbose syntax
//! error (`yysyntax_error`), and the message of `pj_wkt_error` (`src/wkt_parser.cpp`).
//!
//! The grammar has no `error` rule, so the first syntax error ends the parse, with
//! one message. The module checks syntax only: the object is built by
//! [`super::wkt_import`], and the grammar is applied to the text it was given.

mod tables;

use tables::{
    YYCHECK, YYDEFACT, YYDEFGOTO, YYFINAL, YYLAST, YYMAXUTOK, YYNTOKENS, YYPACT, YYPACT_NINF,
    YYPGOTO, YYR1, YYR2, YYTABLE, YYTNAME, YYTRANSLATE,
};

/// `YYEOF`: the token number of the end of the string.
const YYEOF: i32 = 0;
/// `YYEMPTY`: no lookahead token is held.
const YYEMPTY: i32 = -2;
/// `YYUNDEFTOK`: the symbol of a lexer value outside `yytranslate`.
const YYUNDEFTOK: i32 = 2;
/// `YYTERROR`: the `error` symbol, which the grammar never uses.
const YYTERROR: i32 = 1;
/// `YYERROR_VERBOSE_ARGS_MAXIMUM` of bison: the unexpected token and up to four expected.
const YYERROR_VERBOSE_ARGS_MAXIMUM: usize = 5;

/// The external token numbers of `wkt1_generated_parser.h` (`T_STRING`, `T_NUMBER`,
/// `T_IDENTIFIER`); the keywords are in [`KEYWORDS`].
const T_STRING: i32 = 284;
const T_NUMBER: i32 = 285;
const T_IDENTIFIER: i32 = 286;

/// `tokens[]` of `pj_wkt1_lex` (wkt1_parser.cpp), in its order, with the external
/// token numbers of `wkt1_generated_parser.h`.
const KEYWORDS: [(&str, i32); 26] = [
    ("PARAM_MT", 258),
    ("PARAMETER", 282),
    ("CONCAT_MT", 259),
    ("INVERSE_MT", 260),
    ("PASSTHROUGH_MT", 261),
    ("PROJCS", 262),
    ("PROJECTION", 263),
    ("GEOGCS", 264),
    ("DATUM", 265),
    ("SPHEROID", 266),
    ("PRIMEM", 267),
    ("UNIT", 268),
    ("GEOCCS", 270),
    ("AUTHORITY", 271),
    ("VERT_CS", 272),
    ("VERTCS", 273),
    ("VERT_DATUM", 274),
    ("VDATUM", 275),
    ("COMPD_CS", 276),
    ("AXIS", 277),
    ("TOWGS84", 278),
    ("FITTED_CS", 279),
    ("LOCAL_CS", 280),
    ("LOCAL_DATUM", 281),
    ("LINUNIT", 269),
    ("EXTENSION", 283),
];

/// `pj_wkt1_parse`: the message PROJ records for `wkt` when the WKT1 grammar rejects
/// it, or `None` when the grammar accepts it.
#[must_use]
pub(crate) fn wkt1_syntax_error(wkt: &str) -> Option<String> {
    let mut lexer = Lexer {
        input: wkt.as_bytes(),
        next: 0,
        last_success: 0,
    };
    match parse(&mut lexer) {
        Ok(()) => None,
        Err(message) => Some(error_text(lexer.input, lexer.last_success, &message)),
    }
}

/// The lexer state of `pj_wkt1_parse_context`: `pszNext` and `pszLastSuccess`.
struct Lexer<'a> {
    input: &'a [u8],
    next: usize,
    last_success: usize,
}

impl Lexer<'_> {
    /// The byte at `index`, or the terminating NUL past the end, as C reads it.
    fn at(&self, index: usize) -> u8 {
        self.input.get(index).copied().unwrap_or(0)
    }

    /// `ci_starts_with(pszInput, token)` at `start`.
    fn starts_with_ignore_case(&self, start: usize, token: &str) -> bool {
        self.input
            .get(start..start + token.len())
            .is_some_and(|bytes| bytes.eq_ignore_ascii_case(token.as_bytes()))
    }

    /// `pj_wkt1_lex`: the next token number, or the C `EOF` (-1) at the end.
    fn lex(&mut self) -> i32 {
        let mut start = self.next;
        while matches!(self.at(start), b' ' | b'\t' | 10 | 13) {
            start += 1;
        }
        self.last_success = start;
        if self.at(start) == 0 {
            self.next = start;
            return -1;
        }
        // Node names: the first keyword that starts here and is not followed by a letter.
        if self.at(start).is_ascii_alphabetic() {
            for (keyword, code) in KEYWORDS {
                if self.starts_with_ignore_case(start, keyword)
                    && !self.at(start + keyword.len()).is_ascii_alphabetic()
                {
                    self.next = start + keyword.len();
                    return code;
                }
            }
        }
        // Double-quoted strings. An unterminated one is the end of the input.
        if self.at(start) == b'"' {
            let mut end = start + 1;
            while self.at(end) != 0 && self.at(end) != b'"' {
                end += 1;
            }
            if self.at(end) == 0 {
                self.next = end;
                return -1;
            }
            self.next = end + 1;
            return T_STRING;
        }
        // Numbers: an optional sign, digits, an optional fraction, an optional exponent.
        let first = self.at(start);
        if ((first == b'-' || first == b'+') && self.at(start + 1).is_ascii_digit())
            || first.is_ascii_digit()
        {
            let mut end = start;
            if first == b'-' || first == b'+' {
                end += 1;
            }
            while self.at(end).is_ascii_digit() {
                end += 1;
            }
            if self.at(end) == b'.' {
                end += 1;
                while self.at(end).is_ascii_digit() {
                    end += 1;
                }
            }
            if matches!(self.at(end), b'e' | b'E') {
                end += 1;
                if matches!(self.at(end), b'-' | b'+') {
                    end += 1;
                }
                while self.at(end).is_ascii_digit() {
                    end += 1;
                }
            }
            self.next = end;
            return T_NUMBER;
        }
        // Identifiers: letters only.
        if first.is_ascii_alphabetic() {
            let mut end = start + 1;
            while self.at(end).is_ascii_alphabetic() {
                end += 1;
            }
            self.next = end;
            return T_IDENTIFIER;
        }
        // Any other character is its own token. `*pszInput` is a C `char`: a byte from
        // 0x80 is negative, and bison reads a lookahead that is not positive as the end.
        self.next = start + 1;
        i32::from(i8::from_ne_bytes([first]))
    }
}

/// The entry of a table at `index`. Every index the driver forms is in range, as the C
/// code assumes; a negative or past-the-end index reads 0 here instead of out of bounds.
fn cell(table: &[i32], index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|index| table.get(index))
        .copied()
        .unwrap_or(0)
}

/// `yytranslate`: the symbol number of a lexer value (`YYTRANSLATE`).
fn translate(token: i32) -> i32 {
    if (0..=YYMAXUTOK).contains(&token) {
        cell(&YYTRANSLATE, token)
    } else {
        YYUNDEFTOK
    }
}

/// `yytname[symbol]`, as the name bison stores for it.
fn symbol_name(symbol: i32) -> String {
    usize::try_from(symbol)
        .ok()
        .and_then(|index| YYTNAME.get(index))
        .map_or_else(String::new, |name| token_name(name))
}

/// `yytnamerr`: the display name of a symbol. A name in double quotes loses them,
/// unless it holds a quote-comma or an apostrophe (bison keeps those).
fn token_name(name: &str) -> String {
    let Some(rest) = name.strip_prefix('"') else {
        return name.to_owned();
    };
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\'' | ',' => return name.to_owned(),
            '\\' => {
                if chars.next() != Some('\\') {
                    return name.to_owned();
                }
                out.push('\\');
            }
            '"' => return out,
            other => out.push(other),
        }
    }
    name.to_owned()
}

/// The `yyparse` driver of bison's `yacc.c` skeleton, for a grammar with no `error`
/// rule. It returns the `yysyntax_error` message at the first error.
fn parse(lexer: &mut Lexer<'_>) -> Result<(), String> {
    let mut stack: Vec<i32> = vec![0];
    let mut lookahead = YYEMPTY;
    loop {
        let state = *stack.last().unwrap_or(&0);
        if state == YYFINAL {
            // YYACCEPT: the end of the string was shifted.
            return Ok(());
        }
        let mut rule: Option<i32> = None;
        let pact = cell(&YYPACT, state);
        if pact != YYPACT_NINF {
            if lookahead == YYEMPTY {
                lookahead = lexer.lex();
            }
            let token = if lookahead <= YYEOF {
                lookahead = YYEOF;
                YYEOF
            } else {
                translate(lookahead)
            };
            let index = pact + token;
            if (0..=YYLAST).contains(&index) && cell(&YYCHECK, index) == token {
                let action = cell(&YYTABLE, index);
                if action > 0 {
                    // Shift.
                    stack.push(action);
                    lookahead = YYEMPTY;
                    continue;
                }
                // Reduce by the rule `-action`.
                rule = Some(-action);
            }
        }
        // yydefault: the default reduction, or an error.
        let rule = match rule {
            Some(rule) => rule,
            None => {
                let default = cell(&YYDEFACT, state);
                if default == 0 {
                    let yytoken = if lookahead == YYEMPTY {
                        YYEMPTY
                    } else {
                        translate(lookahead)
                    };
                    return Err(syntax_error(&stack, yytoken));
                }
                default
            }
        };
        // yyreduce: pop the right-hand side, then goto the left-hand nonterminal.
        let length = usize::try_from(cell(&YYR2, rule)).unwrap_or(0);
        let keep = stack.len().saturating_sub(length).max(1);
        stack.truncate(keep);
        let top = *stack.last().unwrap_or(&0);
        let lhs = cell(&YYR1, rule) - YYNTOKENS;
        let index = cell(&YYPGOTO, lhs) + top;
        let next = if (0..=YYLAST).contains(&index) && cell(&YYCHECK, index) == top {
            cell(&YYTABLE, index)
        } else {
            cell(&YYDEFGOTO, lhs)
        };
        stack.push(next);
    }
}

/// `yysyntax_error`: the unexpected token, then up to four tokens the state expects, as
/// bison words them. `yytable_value_is_error` is always false for this grammar.
fn syntax_error(stack: &[i32], yytoken: i32) -> String {
    let mut args: Vec<String> = Vec::new();
    if yytoken != YYEMPTY {
        let top = *stack.last().unwrap_or(&0);
        let yyn = cell(&YYPACT, top);
        args.push(symbol_name(yytoken));
        if yyn != YYPACT_NINF {
            let yyxbegin = if yyn < 0 { -yyn } else { 0 };
            let yychecklim = YYLAST - yyn + 1;
            let yyxend = yychecklim.min(YYNTOKENS);
            for yyx in yyxbegin..yyxend {
                if cell(&YYCHECK, yyx + yyn) == yyx && yyx != YYTERROR {
                    if args.len() == YYERROR_VERBOSE_ARGS_MAXIMUM {
                        args.truncate(1);
                        break;
                    }
                    args.push(symbol_name(yyx));
                }
            }
        }
    }
    match args.split_first() {
        None => "syntax error".to_owned(),
        Some((unexpected, expected)) => {
            let mut message = format!("syntax error, unexpected {unexpected}");
            for (position, name) in expected.iter().enumerate() {
                message.push_str(if position == 0 {
                    ", expecting "
                } else {
                    " or "
                });
                message.push_str(name);
            }
            message
        }
    }
}

/// `pj_wkt_error` (wkt_parser.cpp): the message, then the text up to 40 bytes around the
/// last token read, on its line, and a caret under that token.
fn error_text(input: &[u8], last_success: usize, message: &str) -> String {
    let mut text = format!("Parsing error : {message}. Error occurred around:\n");
    let n = last_success;
    let mut start = n.saturating_sub(40);
    let mut context: Vec<u8> = Vec::new();
    let mut index = start;
    while index < n + 40 && index < input.len() && input[index] != 0 {
        if input[index] == b'\r' || input[index] == b'\n' {
            if index > n {
                break;
            }
            context.clear();
            start = index + 1;
        } else {
            context.push(input[index]);
        }
        index += 1;
    }
    text.push_str(&String::from_utf8_lossy(&context));
    text.push('\n');
    for _ in start..n {
        text.push(' ');
    }
    text.push('^');
    text
}
