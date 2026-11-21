//! Minimal JSON reader for the 1.7.10 asset formats (`sounds.json`,
//! `pack.mcmeta`). Written in place of a dependency; parses exactly what the
//! vanilla loaders read and ignores the rest of the grammar's frills.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// `JsonUtils.asJsonObject`: an object or an error naming `field`.
    pub fn as_object(&self, field: &str) -> Result<&Vec<(String, JsonValue)>, String> {
        match self {
            JsonValue::Object(entries) => Ok(entries),
            _ => Err(format!("{field} is not an object")),
        }
    }

    /// `JsonUtils.getJsonArray`.
    pub fn as_array(&self, field: &str) -> Result<&Vec<JsonValue>, String> {
        match self {
            JsonValue::Array(items) => Ok(items),
            _ => Err(format!("{field} is not an array")),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    /// `JsonUtils.asString(element, field)`.
    pub fn as_string(&self, field: &str) -> Result<&str, String> {
        match self {
            JsonValue::String(text) => Ok(text),
            _ => Err(format!("{field} is not a string")),
        }
    }

    pub fn as_bool(&self, field: &str) -> Result<bool, String> {
        match self {
            JsonValue::Bool(value) => Ok(*value),
            _ => Err(format!("{field} is not a boolean")),
        }
    }

    /// `JsonUtils.getFloat` / `getInteger`: a number, NaN and infinities
    /// rejected like Gson rejects them.
    pub fn as_f64(&self, field: &str) -> Result<f64, String> {
        match self {
            JsonValue::Number(value) if value.is_finite() => Ok(*value),
            _ => Err(format!("{field} is not a number")),
        }
    }

    pub fn as_i64(&self, field: &str) -> Result<i64, String> {
        self.as_f64(field).map(|value| value as i64)
    }

    /// Object member lookup.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValue::Null => f.write_str("null"),
            JsonValue::Bool(true) => f.write_str("true"),
            JsonValue::Bool(false) => f.write_str("false"),
            JsonValue::Number(value) => write!(f, "{value}"),
            JsonValue::String(text) => write!(f, "\"{text}\""),
            JsonValue::Array(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            JsonValue::Object(entries) => {
                f.write_str("{")?;
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "\"{key}\":{value}")?;
                }
                f.write_str("}")
            }
        }
    }
}

/// Parses a complete JSON document.
pub fn parse(text: &str) -> Result<JsonValue, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    parser.skip_whitespace();
    let value = parser.value()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(format!("trailing data at byte {}", parser.pos));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn next(&mut self) -> Result<u8, String> {
        let byte = self
            .peek()
            .ok_or_else(|| "unexpected end of input".to_owned())?;
        self.pos += 1;
        Ok(byte)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.next()? == byte {
            Ok(())
        } else {
            Err(format!("expected '{byte}' at byte {}", self.pos - 1))
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(())
        } else {
            Err(format!("expected '{word}' at byte {}", self.pos))
        }
    }

    fn value(&mut self) -> Result<JsonValue, String> {
        match self
            .peek()
            .ok_or_else(|| "unexpected end of input".to_owned())?
        {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(JsonValue::String(self.string()?)),
            b't' => {
                self.literal("true")?;
                Ok(JsonValue::Bool(true))
            }
            b'f' => {
                self.literal("false")?;
                Ok(JsonValue::Bool(false))
            }
            b'n' => {
                self.literal("null")?;
                Ok(JsonValue::Null)
            }
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Result<JsonValue, String> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.next()? {
                b',' => continue,
                b'}' => break,
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos - 1)),
            }
        }
        Ok(JsonValue::Object(entries))
    }

    fn array(&mut self) -> Result<JsonValue, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value()?);
            self.skip_whitespace();
            match self.next()? {
                b',' => continue,
                b']' => break,
                _ => return Err(format!("expected ',' or ']' at byte {}", self.pos - 1)),
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut text = String::new();
        loop {
            let byte = self.next()?;
            match byte {
                b'"' => break,
                b'\\' => match self.next()? {
                    b'"' => text.push('"'),
                    b'\\' => text.push('\\'),
                    b'/' => text.push('/'),
                    b'b' => text.push('\u{8}'),
                    b'f' => text.push('\u{c}'),
                    b'n' => text.push('\n'),
                    b'r' => text.push('\r'),
                    b't' => text.push('\t'),
                    b'u' => {
                        let mut code = 0u32;
                        for _ in 0..4 {
                            let digit = self.next()?;
                            code = code * 16
                                + u32::from_str_radix(
                                    std::str::from_utf8(&[digit]).map_err(|_| "bad \\u escape")?,
                                    16,
                                )
                                .map_err(|_| "bad \\u escape")?;
                        }
                        // Surrogate pairs.
                        if (0xd800..0xdc00).contains(&code) {
                            self.expect(b'\\')?;
                            self.expect(b'u')?;
                            let mut low = 0u32;
                            for _ in 0..4 {
                                let digit = self.next()?;
                                low = low * 16
                                    + u32::from_str_radix(
                                        std::str::from_utf8(&[digit])
                                            .map_err(|_| "bad \\u escape")?,
                                        16,
                                    )
                                    .map_err(|_| "bad \\u escape")?;
                            }
                            code = 0x1_0000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                        }
                        text.push(char::from_u32(code).ok_or_else(|| "bad \\u escape".to_owned())?);
                    }
                    other => return Err(format!("bad escape '\\{other}'")),
                },
                other if other < 0x20 => return Err(format!("unescaped control byte {other}")),
                other => {
                    // Re-assemble the UTF-8 sequence.
                    let len = utf8_len(other);
                    let start = self.pos - 1;
                    self.pos += len - 1;
                    let chunk = self
                        .bytes
                        .get(start..self.pos)
                        .ok_or_else(|| "truncated UTF-8".to_owned())?;
                    text.push_str(std::str::from_utf8(chunk).map_err(|_| "bad UTF-8".to_owned())?);
                }
            }
        }
        Ok(text)
    }

    fn number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "bad number".to_owned())?;
        text.parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| format!("bad number '{text}'"))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_object() {
        let value = parse(r#"{"a": 1, "b": [true, null, "x"], "c": {"d": -0.5}}"#).unwrap();
        assert_eq!(value.get("a").unwrap().as_i64("a").unwrap(), 1);
        assert_eq!(value.get("b").unwrap().as_array("b").unwrap().len(), 3);
        assert_eq!(
            value
                .get("c")
                .unwrap()
                .get("d")
                .unwrap()
                .as_f64("d")
                .unwrap(),
            -0.5
        );
    }

    #[test]
    fn parses_escapes() {
        let value = parse(r#""Aé\nü""#).unwrap();
        assert_eq!(value.as_string("s").unwrap(), "Aé\nü");
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("{").is_err());
        assert!(parse("nope").is_err());
        assert!(parse(r#"{"a":}"#).is_err());
        assert!(parse("1 2").is_err());
    }
}
