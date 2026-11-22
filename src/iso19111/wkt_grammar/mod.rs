//! The WKT grammar checks of PROJ 9.9.0: `pj_wkt1_parse` (`src/wkt1_parser.cpp`) and
//! `pj_wkt2_parse` (`src/wkt2_parser.cpp`), which `WKTParser::createFromWKT` runs on the text
//! once the object is built (io.cpp:8506-8522).
//!
//! PROJ's grammars (`src/wkt1_grammar.y`, `src/wkt2_grammar.y`) are compiled by bison 3.5.1
//! into the tables of `src/wkt{1,2}_generated_parser.c`, which `tools/wkt-grammar/extract.py`
//! copies into [`wkt1_tables`] and [`wkt2_tables`]. This module is the `yyparse` loop of
//! bison's `yacc.c` skeleton, its verbose `yysyntax_error`, and the message of
//! `pj_wkt_error` (`src/wkt_parser.cpp`). The lexers are in [`wkt1`] and [`wkt2`].
//! See docs/PORTING-wkt-grammar.md.

mod wkt1;
mod wkt1_tables;
mod wkt2;
mod wkt2_tables;

/// The parser tables of one grammar, as bison names them.
pub(super) struct Tables {
    yyfinal: i32,
    yylast: i32,
    yyntokens: i32,
    yyundeftok: i32,
    yymaxutok: i32,
    yypact_ninf: i32,
    yytname: &'static [&'static str],
    yytranslate: &'static [i16],
    yypact: &'static [i16],
    yydefact: &'static [i16],
    yypgoto: &'static [i16],
    yydefgoto: &'static [i16],
    yytable: &'static [i16],
    yycheck: &'static [i16],
    yyr1: &'static [i16],
    yyr2: &'static [i16],
}

/// `YYEOF`: the symbol of the end of the string.
const YYEOF: i32 = 0;
/// `YYEMPTY`: no lookahead token is held.
const YYEMPTY: i32 = -2;
/// `YYTERROR`: the `error` symbol.
const YYTERROR: i32 = 1;
/// `YYMAXDEPTH`: the largest state stack `yyparse` grows to.
const YYMAXDEPTH: usize = 10000;
/// `YYERROR_VERBOSE_ARGS_MAXIMUM`: the unexpected token and up to four expected ones.
const YYERROR_VERBOSE_ARGS_MAXIMUM: usize = 5;

/// `pj_wkt1_parse`: the message PROJ records when the WKT1 grammar rejects `wkt`. It is
/// bytes: the context window of `pj_wkt_error` may cut a UTF-8 sequence, as PROJ's does.
#[must_use]
pub(crate) fn wkt1_parse(wkt: &str) -> Option<Vec<u8>> {
    yyparse(&wkt1_tables::TABLES, &mut Context::new(wkt), wkt1::lex)
}

/// `pj_wkt2_parse`: the message PROJ records when the WKT2 grammar rejects `wkt`, as bytes.
#[must_use]
pub(crate) fn wkt2_parse(wkt: &str) -> Option<Vec<u8>> {
    yyparse(&wkt2_tables::TABLES, &mut Context::new(wkt), wkt2::lex)
}

/// `pj_wkt_parse_context`: the input, `pszNext` and `pszLastSuccess` as byte offsets.
struct Context<'a> {
    input: &'a [u8],
    next: usize,
    last_success: usize,
}

impl<'a> Context<'a> {
    /// The C string of `wkt.c_str()`: it ends at the first NUL.
    fn new(wkt: &'a str) -> Self {
        let bytes = wkt.as_bytes();
        let end = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        Self {
            input: &bytes[..end],
            next: 0,
            last_success: 0,
        }
    }

    /// The byte at `index`, or the terminating NUL past the end.
    fn at(&self, index: usize) -> u8 {
        self.input.get(index).copied().unwrap_or(0)
    }

    /// `ci_starts_with(pszInput, token)` at `start`.
    fn starts_with_ignore_case(&self, start: usize, token: &str) -> bool {
        self.input
            .get(start..start + token.len())
            .is_some_and(|bytes| bytes.eq_ignore_ascii_case(token.as_bytes()))
    }

    /// The first keyword of `keywords` that starts at `start` and is not followed by a
    /// letter, as both lexers look up node names.
    fn keyword(&mut self, start: usize, keywords: &[(&str, i32)]) -> Option<i32> {
        if !self.at(start).is_ascii_alphabetic() {
            return None;
        }
        let &(name, code) = keywords.iter().find(|(name, _)| {
            self.starts_with_ignore_case(start, name)
                && !self.at(start + name.len()).is_ascii_alphabetic()
        })?;
        self.next = start + name.len();
        Some(code)
    }

    /// `*pszInput` returned as a token: a C `char`, so a byte from 0x80 is negative.
    fn single_char(&mut self, start: usize) -> i32 {
        self.next = start + 1;
        i32::from(i8::from_ne_bytes([self.at(start)]))
    }

    /// `pj_wkt_error`: the message, then the text up to 40 bytes around the last token
    /// read, on its line, and a caret under that token.
    fn error(&self, message: &str) -> Vec<u8> {
        let mut text = format!("Parsing error : {message}. Error occurred around:\n").into_bytes();
        let n = self.last_success;
        let mut start = n.saturating_sub(40);
        let mut around: Vec<u8> = Vec::new();
        let mut index = start;
        while index < n + 40 && index < self.input.len() {
            let byte = self.input[index];
            if byte == b'\r' || byte == b'\n' {
                if index > n {
                    break;
                }
                around.clear();
                start = index + 1;
            } else {
                around.push(byte);
            }
            index += 1;
        }
        text.extend_from_slice(&around);
        text.push(b'\n');
        text.resize(text.len() + n.saturating_sub(start), b' ');
        text.push(b'^');
        text
    }
}

impl Tables {
    /// `YYTRANSLATE`.
    fn translate(&self, token: i32) -> i32 {
        if (0..=self.yymaxutok).contains(&token) {
            cell(self.yytranslate, token)
        } else {
            self.yyundeftok
        }
    }

    /// `yytnamerr` of `yytname[symbol]`.
    fn name(&self, symbol: i32) -> String {
        usize::try_from(symbol)
            .ok()
            .and_then(|index| self.yytname.get(index))
            .map_or_else(String::new, |name| tnamerr(name))
    }

    /// The action-table entry of `state` for `symbol`, when `yycheck` holds one.
    fn action(&self, state: i32, symbol: i32) -> Option<i32> {
        let base = cell(self.yypact, state);
        if base == self.yypact_ninf {
            return None;
        }
        let index = base + symbol;
        ((0..=self.yylast).contains(&index) && cell(self.yycheck, index) == symbol)
            .then(|| cell(self.yytable, index))
    }

    /// `yysyntax_error`: the unexpected token, then up to four tokens `state` expects.
    fn syntax_error(&self, state: i32, yytoken: i32) -> String {
        let mut args: Vec<String> = Vec::new();
        if yytoken != YYEMPTY {
            let yyn = cell(self.yypact, state);
            args.push(self.name(yytoken));
            if yyn != self.yypact_ninf {
                let yyxbegin = if yyn < 0 { -yyn } else { 0 };
                let yyxend = (self.yylast - yyn + 1).min(self.yyntokens);
                for yyx in yyxbegin..yyxend {
                    if cell(self.yycheck, yyx + yyn) == yyx && yyx != YYTERROR {
                        if args.len() == YYERROR_VERBOSE_ARGS_MAXIMUM {
                            args.truncate(1);
                            break;
                        }
                        args.push(self.name(yyx));
                    }
                }
            }
        }
        let Some((unexpected, expected)) = args.split_first() else {
            return "syntax error".to_owned();
        };
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

/// The entry of `table` at `index`. Every index `yyparse` forms is in range; one that is not
/// reads 0 here instead of out of bounds.
fn cell(table: &[i16], index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|index| table.get(index))
        .map_or(0, |&value| i32::from(value))
}

/// `yytnamerr`: a name in double quotes loses them, unless it holds an apostrophe, a comma,
/// or a backslash that does not escape a backslash.
fn tnamerr(name: &str) -> String {
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

/// What `yybackup` decides for the current state.
enum Step {
    Shift(i32),
    Reduce(i32),
    Default,
}

/// The `yyparse` loop of `yacc.c`, with no semantic actions. `None` is `YYACCEPT`; otherwise
/// the last message `yyerror` wrote, as `pj_wkt*_parse` returns it on a non-zero result.
fn yyparse(
    t: &Tables,
    context: &mut Context<'_>,
    lex: fn(&mut Context<'_>) -> i32,
) -> Option<Vec<u8>> {
    let mut stack: Vec<i32> = vec![0];
    let mut yystate = 0;
    let mut yychar = YYEMPTY;
    let mut yyerrstatus = 0;
    let mut message: Option<Vec<u8>> = None;
    loop {
        // yysetstate
        if stack.len() >= YYMAXDEPTH {
            return Some(context.error("memory exhausted"));
        }
        if yystate == t.yyfinal {
            return None;
        }
        // yybackup
        let step = if cell(t.yypact, yystate) == t.yypact_ninf {
            Step::Default
        } else {
            if yychar == YYEMPTY {
                yychar = lex(context);
            }
            let yytoken = if yychar <= YYEOF {
                yychar = YYEOF;
                YYEOF
            } else {
                t.translate(yychar)
            };
            match t.action(yystate, yytoken) {
                None => Step::Default,
                Some(yyn) if yyn <= 0 => Step::Reduce(-yyn),
                Some(yyn) => Step::Shift(yyn),
            }
        };
        let rule = match step {
            Step::Shift(next) => {
                if yyerrstatus > 0 {
                    yyerrstatus -= 1;
                }
                yystate = next;
                stack.push(yystate);
                yychar = YYEMPTY;
                continue;
            }
            Step::Reduce(rule) => rule,
            Step::Default => cell(t.yydefact, yystate),
        };
        if rule != 0 {
            // yyreduce
            let length = usize::try_from(cell(t.yyr2, rule)).unwrap_or(0);
            stack.truncate(stack.len().saturating_sub(length).max(1));
            let top = stack.last().copied().unwrap_or(0);
            let lhs = cell(t.yyr1, rule) - t.yyntokens;
            let index = cell(t.yypgoto, lhs) + top;
            yystate = if (0..=t.yylast).contains(&index) && cell(t.yycheck, index) == top {
                cell(t.yytable, index)
            } else {
                cell(t.yydefgoto, lhs)
            };
            stack.push(yystate);
            continue;
        }
        // yyerrlab
        let yytoken = if yychar == YYEMPTY {
            YYEMPTY
        } else {
            t.translate(yychar)
        };
        if yyerrstatus == 0 {
            message = Some(context.error(&t.syntax_error(yystate, yytoken)));
        }
        if yyerrstatus == 3 {
            if yychar == YYEOF {
                return message;
            }
            if yychar > YYEOF {
                yychar = YYEMPTY;
            }
        }
        // yyerrlab1: pop to a state that shifts `error`; these grammars have none.
        yyerrstatus = 3;
        let shift = loop {
            if let Some(next) = t.action(yystate, YYTERROR).filter(|&next| next > 0) {
                break next;
            }
            if stack.len() == 1 {
                return message;
            }
            stack.pop();
            yystate = stack.last().copied().unwrap_or(0);
        };
        yystate = shift;
        stack.push(yystate);
    }
}
