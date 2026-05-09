//! Minimal VDF (Valve Data Format) parser.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum VdfError {
    MissingKey(String),
    TypeMismatch(String),
    Parse(String),
    UnterminatedString,
    UnexpectedEof,
}

impl fmt::Display for VdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VdfError::MissingKey(k) => write!(f, "Missing key: {}", k),
            VdfError::TypeMismatch(k) => write!(f, "Type mismatch for key: {}", k),
            VdfError::Parse(msg) => write!(f, "Parse error: {}", msg),
            VdfError::UnterminatedString => write!(f, "Unterminated string"),
            VdfError::UnexpectedEof => write!(f, "Unexpected EOF"),
        }
    }
}

impl Error for VdfError {}

/// Represents a parsed VDF value.
#[derive(Debug, Clone)]
pub enum VdfValue {
    String(String),
    Object(HashMap<String, VdfValue>),
}

impl VdfValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            VdfValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, VdfValue>> {
        match self {
            VdfValue::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// Represents a parsed VDF object (the root or a nested block).
#[derive(Debug, Clone)]
pub struct VdfObject {
    raw: HashMap<String, VdfValue>,
}

impl VdfObject {
    /// Valve's KeyValues format treats keys as case-insensitive, but real-world
    /// VDF files (e.g. localconfig.vdf) vary the casing per-user (`valve` vs `Valve`).
    fn find_ci(&self, key: &str) -> Option<&VdfValue> {
        self.raw
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    pub fn require_object(&self, key: &str) -> Result<VdfObject, VdfError> {
        match self.find_ci(key) {
            Some(VdfValue::Object(m)) => Ok(VdfObject { raw: m.clone() }),
            Some(_) => Err(VdfError::TypeMismatch(key.to_string())),
            None => Err(VdfError::MissingKey(key.to_string())),
        }
    }

    pub fn require_string(&self, key: &str) -> Result<String, VdfError> {
        match self.find_ci(key) {
            Some(VdfValue::String(s)) => Ok(s.clone()),
            Some(_) => Err(VdfError::TypeMismatch(key.to_string())),
            None => Err(VdfError::MissingKey(key.to_string())),
        }
    }

    pub fn require_integer(&self, key: &str) -> Result<i64, VdfError> {
        let s = self.require_string(key)?;
        s.parse::<i64>()
            .map_err(|_| VdfError::TypeMismatch(key.to_string()))
    }

    pub fn raw(&self) -> &HashMap<String, VdfValue> {
        &self.raw
    }
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Lexer {
            s: input.as_bytes(),
            i: 0,
        }
    }

    fn eof(&self) -> bool {
        self.i >= self.s.len()
    }

    fn peek(&self) -> Option<u8> {
        if self.eof() {
            None
        } else {
            Some(self.s[self.i])
        }
    }

    fn next(&mut self) -> Option<u8> {
        if self.eof() {
            None
        } else {
            let ch = self.s[self.i];
            self.i += 1;
            Some(ch)
        }
    }

    fn skip_ws(&mut self) {
        while !self.eof() {
            let ch = self.s[self.i];
            // Line comments
            if ch == b'/' && self.i + 1 < self.s.len() && self.s[self.i + 1] == b'/' {
                self.i += 2;
                while !self.eof() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            if ch.is_ascii_whitespace() {
                self.i += 1;
                continue;
            }
            break;
        }
    }

    fn read_quoted(&mut self) -> Result<String, VdfError> {
        if self.next() != Some(b'"') {
            return Err(VdfError::Parse("expected '\"'".to_string()));
        }

        let mut buf = Vec::new();
        while !self.eof() {
            let ch = self.next().ok_or(VdfError::UnterminatedString)?;
            if ch == b'"' {
                return Ok(String::from_utf8_lossy(&buf).to_string());
            }
            if ch == b'\\' && !self.eof() {
                let esc = self.next().ok_or(VdfError::UnterminatedString)?;
                match esc {
                    b'\\' | b'"' => buf.push(esc),
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    b'r' => buf.push(b'\r'),
                    _ => {
                        buf.push(b'\\');
                        buf.push(esc);
                    }
                }
                continue;
            }
            buf.push(ch);
        }
        Err(VdfError::UnterminatedString)
    }
}

pub fn parse_vdf(data: &str) -> Result<VdfObject, VdfError> {
    let mut lexer = Lexer::new(data);
    let obj = parse_object(&mut lexer)?;
    Ok(VdfObject { raw: obj })
}

fn parse_object(l: &mut Lexer) -> Result<HashMap<String, VdfValue>, VdfError> {
    l.skip_ws();
    let mut root = HashMap::new();

    // Some files start with a top-level quoted key followed by a block
    if l.peek() == Some(b'"') {
        let key = l.read_quoted()?;
        l.skip_ws();

        if l.peek() != Some(b'{') {
            let val = parse_value(l)?;
            root.insert(key, val);
            return Ok(root);
        }

        l.next(); // consume '{'
        let block = parse_block(l)?;
        root.insert(key, VdfValue::Object(block));
        l.skip_ws();
        return Ok(root);
    }

    if l.peek() == Some(b'{') {
        l.next();
        return parse_block(l);
    }

    l.skip_ws();
    if l.peek() == Some(b'"') {
        return parse_object(l);
    }

    Err(VdfError::Parse("unexpected start of VDF".to_string()))
}

fn parse_block(l: &mut Lexer) -> Result<HashMap<String, VdfValue>, VdfError> {
    let mut m = HashMap::new();

    loop {
        l.skip_ws();
        if l.eof() {
            return Err(VdfError::UnexpectedEof);
        }

        if l.peek() == Some(b'}') {
            l.next();
            return Ok(m);
        }

        let key = l.read_quoted()?;
        l.skip_ws();
        let val = parse_value(l)?;
        m.insert(key, val);
    }
}

fn parse_value(l: &mut Lexer) -> Result<VdfValue, VdfError> {
    l.skip_ws();
    match l.peek() {
        Some(b'"') => Ok(VdfValue::String(l.read_quoted()?)),
        Some(b'{') => {
            l.next();
            Ok(VdfValue::Object(parse_block(l)?))
        }
        _ => {
            // Bare tokens (legacy support)
            let mut buf = Vec::new();
            while !l.eof() {
                let ch = l.peek().unwrap();
                if ch.is_ascii_whitespace() || ch == b'}' {
                    break;
                }
                buf.push(ch);
                l.next();
            }
            Ok(VdfValue::String(
                String::from_utf8_lossy(&buf).trim().to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_vdf() {
        let input = r#"
"AppState"
{
    "appid"     "123"
    "name"      "Test Game"
    "installdir"    "testgame"
}
"#;
        let obj = parse_vdf(input).unwrap();
        let app_state = obj.require_object("AppState").unwrap();
        assert_eq!(app_state.require_string("appid").unwrap(), "123");
        assert_eq!(app_state.require_string("name").unwrap(), "Test Game");
    }

    #[test]
    fn test_escaped_strings() {
        let input = r#"
"Test"
{
    "value"     "hello \"world\""
}
"#;
        let obj = parse_vdf(input).unwrap();
        let test = obj.require_object("Test").unwrap();
        assert_eq!(test.require_string("value").unwrap(), r#"hello "world""#);
    }
}
