//! Small JSON reader for the deformation-model and triangulation files.
//!
//! The grammars are the objects those PROJ parsers accept. It is not a general
//! JSON library: nesting is capped, and a number is an `f64`.

use std::collections::BTreeMap;

const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug)]
#[expect(
    dead_code,
    reason = "bool and null appear in model JSON that the evaluators do not read"
)]
pub(super) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub(super) fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    pub(super) fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub(super) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    pub(super) fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }
}

pub(super) fn parse(text: &str) -> Option<Value> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        index: 0,
    };
    if parser.bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        parser.index = 3;
    }
    let value = parser.value(0)?;
    parser.skip();
    if parser.index != parser.bytes.len() {
        return None;
    }
    Some(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.index += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.bytes.get(self.index).copied()
    }

    fn eat(&mut self, expected: u8) -> Option<()> {
        if self.peek()? == expected {
            self.index += 1;
            Some(())
        } else {
            None
        }
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        match self.peek()? {
            b'n' => self.literal(b"null", Value::Null),
            b't' => self.literal(b"true", Value::Bool(true)),
            b'f' => self.literal(b"false", Value::Bool(false)),
            b'"' => self.string().map(Value::String),
            b'[' => self.array(depth),
            b'{' => self.object(depth),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn literal(&mut self, text: &[u8], value: Value) -> Option<Value> {
        self.skip();
        if self.bytes.get(self.index..)?.starts_with(text) {
            self.index += text.len();
            Some(value)
        } else {
            None
        }
    }

    fn string(&mut self) -> Option<String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let byte = *self.bytes.get(self.index)?;
            self.index += 1;
            match byte {
                b'"' => return Some(out),
                b'\\' => {
                    let escaped = *self.bytes.get(self.index)?;
                    self.index += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => out.push(char::from(escaped)),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self.bytes.get(self.index..self.index + 4)?;
                            self.index += 4;
                            let text = std::str::from_utf8(hex).ok()?;
                            let code = u32::from_str_radix(text, 16).ok()?;
                            out.push(char::from_u32(code)?);
                        }
                        _ => return None,
                    }
                }
                0x20..=0x7e => out.push(char::from(byte)),
                _ => {
                    // Continue a UTF-8 sequence that started on this byte.
                    self.index -= 1;
                    let rest = std::str::from_utf8(self.bytes.get(self.index..)?).ok()?;
                    let ch = rest.chars().next()?;
                    if ch == '"' || ch == '\\' || u32::from(ch) < 0x20 {
                        return None;
                    }
                    out.push(ch);
                    self.index += ch.len_utf8();
                }
            }
        }
    }

    fn number(&mut self) -> Option<Value> {
        self.skip();
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        let digits = self.index;
        while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
            self.index += 1;
        }
        if self.index == digits {
            return None;
        }
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            let fraction = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if self.index == fraction {
                return None;
            }
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            let exp = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if self.index == exp {
                return None;
            }
        }
        let text = std::str::from_utf8(self.bytes.get(start..self.index)?).ok()?;
        let value = text.parse::<f64>().ok()?;
        if value.is_finite() {
            Some(Value::Number(value))
        } else {
            None
        }
    }

    fn array(&mut self, depth: usize) -> Option<Value> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        if self.peek()? == b']' {
            self.index += 1;
            return Some(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            match self.peek()? {
                b',' => self.index += 1,
                b']' => {
                    self.index += 1;
                    return Some(Value::Array(items));
                }
                _ => return None,
            }
        }
    }

    fn object(&mut self, depth: usize) -> Option<Value> {
        self.eat(b'{')?;
        let mut map = BTreeMap::new();
        if self.peek()? == b'}' {
            self.index += 1;
            return Some(Value::Object(map));
        }
        loop {
            let key = self.string()?;
            self.eat(b':')?;
            let value = self.value(depth + 1)?;
            map.insert(key, value);
            match self.peek()? {
                b',' => self.index += 1,
                b'}' => {
                    self.index += 1;
                    return Some(Value::Object(map));
                }
                _ => return None,
            }
        }
    }
}

pub(super) fn obj<'a>(
    map: &'a BTreeMap<String, Value>,
    key: &str,
) -> Option<&'a BTreeMap<String, Value>> {
    map.get(key)?.as_object()
}

pub(super) fn arr<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a [Value]> {
    map.get(key)?.as_array()
}

pub(super) fn req_str<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key)?.as_str()
}

pub(super) fn opt_str<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

pub(super) fn req_f64(map: &BTreeMap<String, Value>, key: &str) -> Option<f64> {
    map.get(key)?.as_f64()
}
