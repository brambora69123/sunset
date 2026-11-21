//! `Text` serialization for the disconnect and kick reasons, the subset of
//! `Text$Serializer` the connection flow needs: literal and translatable
//! components in 1.7.10's JSON form.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Text {
    Literal(String),
    /// `TranslatableText`: the key and its `%s` arguments.
    Translatable(String, Vec<String>),
}

impl Text {
    pub fn literal(text: impl Into<String>) -> Self {
        Self::Literal(text.into())
    }

    /// `TranslatableText(key, args...)`; args render as their plain strings.
    pub fn translatable(key: impl Into<String>, args: Vec<String>) -> Self {
        Self::Translatable(key.into(), args)
    }

    /// `disconnect.genericReason`: "Internal Exception: {0}".
    pub fn generic_reason(details: &str) -> Self {
        Self::translatable("disconnect.genericReason", vec![details.to_string()])
    }

    /// `Text.getString` with 1.7.10's `%s` substitution.
    pub fn as_string(&self) -> String {
        match self {
            Self::Literal(text) => text.clone(),
            Self::Translatable(key, args) => {
                let mut text = key.clone();
                for arg in args {
                    if let Some(index) = text.find("%s") {
                        text.replace_range(index..index + 2, arg);
                    }
                }
                text
            }
        }
    }

    /// `Text$Serializer.toJson`.
    pub fn to_json(&self) -> String {
        match self {
            Self::Literal(text) => format!("{{\"text\":\"{}\"}}", json_escape(text)),
            Self::Translatable(key, args) if args.is_empty() => {
                format!("{{\"translate\":\"{}\"}}", json_escape(key))
            }
            Self::Translatable(key, args) => {
                let with: Vec<String> = args
                    .iter()
                    .map(|arg| format!("{{\"text\":\"{}\"}}", json_escape(arg)))
                    .collect();
                format!(
                    "{{\"translate\":\"{}\",\"with\":[{}]}}",
                    json_escape(key),
                    with.join(",")
                )
            }
        }
    }

    /// `Text$Serializer.fromJson` for the shapes this crate writes.
    pub fn from_json(json: &str) -> Option<Self> {
        let object = json.trim();
        let get_field = |name: &str| -> Option<String> {
            let needle = format!("\"{name}\":");
            let start = object.find(&needle)? + needle.len();
            let rest = object[start..].trim_start();
            if !rest.starts_with('"') {
                return None;
            }
            let mut out = String::new();
            let mut chars = rest[1..].chars();
            while let Some(c) = chars.next() {
                match c {
                    '"' => return Some(out),
                    '\\' => match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('r') => out.push('\r'),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some(other) => {
                            out.push('\\');
                            out.push(other);
                        }
                        None => return None,
                    },
                    other => out.push(other),
                }
            }
            None
        };
        if object.contains("\"translate\":") {
            let key = get_field("translate")?;
            let mut args = Vec::new();
            if let Some(with_start) = object.find("\"with\":") {
                let array = &object[with_start + 7..];
                let inner = extract_array(array)?;
                let mut depth = 0;
                let mut current = String::new();
                for c in inner.chars() {
                    match c {
                        '{' if depth == 0 => {
                            depth = 1;
                            current.clear();
                        }
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                if let Some(Text::Literal(text)) = Self::from_json(&current) {
                                    args.push(text);
                                } else {
                                    args.push(current.clone());
                                }
                            }
                        }
                        other if depth > 0 => current.push(other),
                        _ => {}
                    }
                }
            }
            return Some(Self::Translatable(key, args));
        }
        if let Some(text) = get_field("text") {
            return Some(Self::Literal(text));
        }
        None
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

fn extract_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let bytes = text.as_bytes();
    let mut depth = 0;
    for (index, &byte) in bytes[start..].iter().enumerate() {
        match byte {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=start + index]);
                }
            }
            _ => {}
        }
    }
    None
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_json_round_trips() {
        let text = Text::literal("Outdated server! I'm still on 1.7.10");
        let json = text.to_json();
        assert_eq!(json, "{\"text\":\"Outdated server! I'm still on 1.7.10\"}");
        assert_eq!(Text::from_json(&json), Some(text));
    }

    #[test]
    fn translatable_json_round_trips() {
        let text = Text::generic_reason("Internal Exception: java.io.IOException: boom");
        let json = text.to_json();
        assert!(json.starts_with("{\"translate\":\"disconnect.genericReason\",\"with\":["));
        assert_eq!(Text::from_json(&json), Some(text));
    }

    #[test]
    fn string_renders_args() {
        // Java's untranslated getString(): %s in the key becomes the arg, and
        // a key without %s stays the key.
        let text = Text::translatable("a %s b", vec!["X".to_string()]);
        assert_eq!(text.as_string(), "a X b");
        let text = Text::translatable(
            "disconnect.genericReason",
            vec!["Internal Exception: x".to_string()],
        );
        assert_eq!(text.as_string(), "disconnect.genericReason");
        assert_eq!(Text::literal("plain").as_string(), "plain");
    }

    #[test]
    fn json_escapes() {
        let text = Text::literal("a\"b\\c\nd");
        let json = text.to_json();
        assert_eq!(json, "{\"text\":\"a\\\"b\\\\c\\nd\"}");
        assert_eq!(Text::from_json(&json), Some(text));
    }
}
