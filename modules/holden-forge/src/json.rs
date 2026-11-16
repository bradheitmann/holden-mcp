//! A small strict JSON reader and canonical writer.
//!
//! The forge has no third-party dependency, so it carries its own JSON. The
//! reader refuses trailing input, duplicate object keys and invalid escapes.
//! Object members keep their order. The writer emits one canonical layout
//! (two-space indent, one member or element per line, a final newline), so a
//! file written by it reads back and re-writes to the same bytes.

use std::fmt::Write as _;

/// A JSON value. Numbers keep their literal text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// A number, as written.
    Number(String),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object; members in document order.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Resolve a JSON pointer such as `/a/0/b` (RFC 6901, without `~` escapes,
    /// which the forge never emits).
    pub fn pointer(&self, path: &str) -> Option<&Value> {
        if path.is_empty() {
            return Some(self);
        }
        let mut cur = self;
        for part in path.strip_prefix('/')?.split('/') {
            cur = match cur {
                Value::Object(_) => cur.get(part)?,
                Value::Array(items) => items.get(part.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(cur)
    }

    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

/// A parse error with its byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset of the problem.
    pub offset: usize,
    /// What was wrong.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.offset, self.message)
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            offset: self.i,
            message: message.into(),
        })
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\n' | b'\r' | b'\t') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > 64 {
            return self.err("nesting deeper than 64");
        }
        self.ws();
        match self.s.get(self.i) {
            None => self.err("unexpected end of input"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') if self.eat("true") => Ok(Value::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Value::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.err("unexpected character"),
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.i;
        if self.s.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let d = p.i;
            while p.s.get(p.i).is_some_and(u8::is_ascii_digit) {
                p.i += 1;
            }
            p.i > d
        };
        if self.s.get(self.i) == Some(&b'0') {
            self.i += 1;
        } else if !digits(self) {
            return self.err("invalid number");
        }
        if self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
            if !digits(self) {
                return self.err("invalid fraction");
            }
        }
        if let Some(b'e' | b'E') = self.s.get(self.i) {
            self.i += 1;
            if let Some(b'+' | b'-') = self.s.get(self.i) {
                self.i += 1;
            }
            if !digits(self) {
                return self.err("invalid exponent");
            }
        }
        let text = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
        Ok(Value::Number(text))
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let Some(h) = self.s.get(self.i..self.i + 4) else {
            return self.err("short \\u escape");
        };
        let Ok(text) = std::str::from_utf8(h) else {
            return self.err("invalid \\u escape");
        };
        let Ok(v) = u32::from_str_radix(text, 16) else {
            return self.err("invalid \\u escape");
        };
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.i += 1; // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&b) = self.s.get(self.i) else {
                return self.err("unterminated string");
            };
            self.i += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return self.err("unterminated escape");
                    };
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                if !self.eat("\\u") {
                                    return self.err("lone high surrogate");
                                }
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return self.err("invalid low surrogate");
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            match char::from_u32(code) {
                                Some(c) => c,
                                None => return self.err("invalid code point"),
                            }
                        }
                        _ => return self.err("invalid escape"),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                0x00..=0x1f => return self.err("control character in string"),
                _ => out.push(b),
            }
        }
        match String::from_utf8(out) {
            Ok(s) => Ok(s),
            Err(_) => self.err("string is not UTF-8"),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.eat("]") {
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.ws();
            if self.eat(",") {
                continue;
            }
            if self.eat("]") {
                return Ok(Value::Array(items));
            }
            return self.err("expected ',' or ']'");
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.i += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        self.ws();
        if self.eat("}") {
            return Ok(Value::Object(members));
        }
        loop {
            self.ws();
            if self.s.get(self.i) != Some(&b'"') {
                return self.err("expected a member name");
            }
            let key = self.string()?;
            if members.iter().any(|(k, _)| *k == key) {
                return self.err(format!("duplicate member '{key}'"));
            }
            self.ws();
            if !self.eat(":") {
                return self.err("expected ':'");
            }
            let v = self.value(depth + 1)?;
            members.push((key, v));
            self.ws();
            if self.eat(",") {
                continue;
            }
            if self.eat("}") {
                return Ok(Value::Object(members));
            }
            return self.err("expected ',' or '}'");
        }
    }
}

/// Parse one JSON document.
pub fn parse(text: &str) -> Result<Value, ParseError> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
    };
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return p.err("trailing input");
    }
    Ok(v)
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_value(out: &mut String, v: &Value, indent: usize) {
    let pad = |out: &mut String, n: usize| out.extend(std::iter::repeat_n(' ', n));
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(n),
        Value::String(s) => write_string(out, s),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Array(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                pad(out, indent + 2);
                write_value(out, item, indent + 2);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(out, indent);
            out.push(']');
        }
        Value::Object(members) if members.is_empty() => out.push_str("{}"),
        Value::Object(members) => {
            out.push_str("{\n");
            for (i, (k, item)) in members.iter().enumerate() {
                pad(out, indent + 2);
                write_string(out, k);
                out.push_str(": ");
                write_value(out, item, indent + 2);
                if i + 1 < members.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(out, indent);
            out.push('}');
        }
    }
}

/// The canonical text of `v`, ending in a newline.
pub fn to_canonical(v: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, v, 0);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_reader_refuses_bad_documents() {
        assert!(parse("{\"a\": 1, \"a\": 2}").is_err());
        assert!(parse("[1, 2] x").is_err());
        assert!(parse("\"\\q\"").is_err());
        assert!(parse("[1,]").is_err());
        assert!(parse("01").is_err());
    }

    #[test]
    fn canonical_round_trip() {
        let text = "{\n  \"a\": [\n    \"x\\n\\u0001\",\n    true,\n    null,\n    -1.5e3\n  ],\n  \"b\": {},\n  \"c\": []\n}\n";
        let v = parse(text).unwrap();
        assert_eq!(to_canonical(&v), text);
        assert_eq!(v.pointer("/a/0").and_then(Value::as_str), Some("x\n\u{1}"));
        assert_eq!(
            parse("\"\\ud83d\\ude00\"").unwrap(),
            Value::String("😀".into())
        );
    }
}
