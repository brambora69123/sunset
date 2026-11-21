//! The 1.7.10 lang files: `net.minecraft.client.resource.language.Locale`,
//! `Language`, `LanguageManager` and `I18n`, ported exactly. `.lang` files
//! live at `lang/<code>.lang` (`assets/minecraft/lang/en_US.lang`); the load
//! order is `en_US` first, then the current language over it.

use std::collections::BTreeMap;

/// `net.minecraft.client.resource.language.Language`: a code, its display
/// name and region, and the right-to-left flag.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Language {
    pub code: String,
    pub name: String,
    pub region: String,
    pub bidirectional: bool,
}

impl Language {
    /// Java's `toString`: `"%s (%s)"` of region and name.
    pub fn display(&self) -> String {
        format!("{} ({})", self.region, self.name)
    }
}

/// `net.minecraft.client.resource.language.Locale`: the translations of one
/// language set, loaded `.lang`-file by `.lang`-file.
#[derive(Clone, Debug, Default)]
pub struct Locale {
    translations: BTreeMap<String, String>,
    unicode: bool,
}

impl Locale {
    pub fn new() -> Self {
        Self {
            translations: BTreeMap::new(),
            unicode: false,
        }
    }

    pub fn is_unicode(&self) -> bool {
        self.unicode
    }

    pub fn translation_count(&self) -> usize {
        self.translations.len()
    }

    /// The raw translation of `key`, or `None` (Java's `translations.get`).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(String::as_str)
    }

    /// `Locale.load`: clears, then loads every language code over the last,
    /// each code across all namespaces and packs in the loader's order.
    /// `lookup` returns every copy of `lang/<code>.lang` for one namespace,
    /// bottom pack first.
    pub fn load(&mut self, codes: &[&str], mut lookup: impl FnMut(&str, &str) -> Vec<Vec<u8>>) {
        self.translations.clear();

        for code in codes {
            let path = format!("lang/{code}.lang");
            for namespace in ["minecraft"] {
                for data in lookup(namespace, &path) {
                    self.load_bytes(&data);
                }
            }
        }

        self.check_unicode_usage();
    }

    /// The exact parse: UTF-8 lines; empty lines and lines starting with
    /// `#` skipped; the first `=` splits key from value (later `=`s stay in
    /// the value); `%d`, `%f` and their positional/padded forms become `%s`.
    pub fn load_bytes(&mut self, data: &[u8]) {
        let text = String::from_utf8_lossy(data);
        for line in text.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = split_once_limit(line, '=') else {
                continue;
            };
            self.translations
                .insert(key.to_owned(), rewrite_format_chars(value));
        }
    }

    /// `checkUnicodeUsage`: unicode when more than a tenth of the loaded
    /// characters are outside the first 256 code points. An empty
    /// translation set is not unicode (Java divides 0/0 = NaN).
    pub fn check_unicode_usage(&mut self) {
        self.unicode = false;
        let mut outside = 0usize;
        let mut total = 0usize;

        for value in self.translations.values() {
            total += value.chars().count();
            outside += value.chars().filter(|c| *c >= '\u{100}').count();
        }

        if total > 0 {
            self.unicode = f64::from(outside as u32) / f64::from(total as u32) > 0.1;
        }
    }

    /// `getTranslationOrKey`: the key itself when nothing was loaded.
    pub fn get_translation_or_key<'a>(&'a self, key: &'a str) -> &'a str {
        match self.translations.get(key) {
            Some(value) => value.as_str(),
            None => key,
        }
    }

    /// `Locale.translate`: the loaded format string formatted with `args`
    /// (a Java `String.format` subset: `%s`, `%d`, `%f` with optional
    /// positional `n$`, flags, width and precision). A format error returns
    /// `"Format error: "` plus the format string, exactly like vanilla.
    pub fn translate(&self, key: &str, args: &[FormatArg]) -> String {
        let format = self.get_translation_or_key(key);
        match format_java(format, args) {
            Ok(text) => text,
            Err(_) => format!("Format error: {format}"),
        }
    }

    /// `LanguageManager.reload`: `en_US` first, then the current code over
    /// it (the same code twice when the language already is `en_US`).
    pub fn load_default_order(
        &mut self,
        current: &str,
        lookup: impl FnMut(&str, &str) -> Vec<Vec<u8>>,
    ) {
        let mut codes = vec!["en_US"];
        if current != "en_US" {
            codes.push(current);
        }
        self.load(&codes, lookup);
    }
}

/// `I18n.translate`: the static facade over the one locale.
pub struct I18n {
    locale: &'static Locale,
}

impl I18n {
    /// `I18n.setLocale`: points the facade at a leaked locale (the game has
    /// exactly one).
    pub fn set_locale(locale: &'static Locale) -> I18n {
        I18n { locale }
    }

    pub fn translate(&self, key: &str, args: &[FormatArg]) -> String {
        self.locale.translate(key, args)
    }
}

/// A `String.format` argument. `text` covers `%s`; `number` and `real`
/// cover `%d` and `%f`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormatArg {
    Text(&'static str),
    Number(i64),
    Real(f64),
}

/// Java's `Splitter.on('=').limit(2)`: splits at the first `=` only.
fn split_once_limit(line: &str, separator: char) -> Option<(&str, &str)> {
    let index = line.find(separator)?;
    Some((&line[..index], &line[index + 1..]))
}

/// Java's `ILLEGAL_CHARACTERS_PATTERN`:
/// `%(\\d+\\$)?[\\d\\.]*[df]` replaced with `%$1s` — every `%d`/`%f` and
/// their positional, flagged, padded and precise forms become the same
/// shape with `s`.
fn rewrite_format_chars(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let mut cursor = index + 1;
            // Optional positional `n$`.
            let positional_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            let positional =
                cursor < bytes.len() && bytes[cursor] == b'$' && cursor > positional_start;
            if positional {
                cursor += 1;
            }
            // Flags and width: `[\d\.]*`.
            let specs_start = cursor;
            while cursor < bytes.len() && (bytes[cursor].is_ascii_digit() || bytes[cursor] == b'.')
            {
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] == b'd' || bytes[cursor] == b'f') {
                // Only rewrite when the spec part is well formed: a bare
                // `%d`/`%f` or one with digits and dots between.
                if cursor == specs_start
                    || bytes[specs_start..cursor]
                        .iter()
                        .all(|b| b.is_ascii_digit() || *b == b'.')
                {
                    out.push('%');
                    if positional {
                        out.push_str(
                            &value[positional_start
                                ..positional_start + (cursor - 1 - positional_start)],
                        );
                        out.push('$');
                    }
                    out.push('s');
                    index = cursor + 1;
                    continue;
                }
            }
        }
        // Copy the UTF-8 sequence as-is.
        let len = match bytes[index] {
            0x00..=0x7f => 1,
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 4,
        };
        let end = (index + len).min(bytes.len());
        out.push_str(&value[index..end]);
        index = end;
    }
    out
}

/// A Java `String.format` subset for the translation path. Exotic flags
/// (`,`, `(`, `<`, time and date specs) are not covered; a spec that does
/// not parse leaves an error and the caller returns
/// `"Format error: <format>"` like vanilla.
fn format_java(format: &str, args: &[FormatArg]) -> Result<String, ()> {
    let bytes = format.as_bytes();
    let mut out = String::with_capacity(format.len());
    let mut index = 0;
    let mut next_arg = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'%' {
            let len = match byte {
                0x00..=0x7f => 1,
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            let end = (index + len).min(bytes.len());
            out.push_str(&format[index..end]);
            index = end;
            continue;
        }
        if index + 1 < bytes.len() && bytes[index + 1] == b'%' {
            out.push('%');
            index += 2;
            continue;
        }

        let mut cursor = index + 1;
        // Positional: `n$`.
        let mut positional: Option<usize> = None;
        let digits_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] == b'$' && cursor > digits_start {
            positional = Some(
                format[digits_start..cursor]
                    .parse::<usize>()
                    .map_err(|_| ())?,
            );
            cursor += 1;
        }
        // Flags.
        while cursor < bytes.len() && matches!(bytes[cursor], b'-' | b'#' | b'+' | b' ' | b'0') {
            cursor += 1;
        }
        // Width.
        let width_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        let width: Option<usize> = if cursor > width_start {
            Some(
                format[width_start..cursor]
                    .parse::<usize>()
                    .map_err(|_| ())?,
            )
        } else {
            None
        };
        // Precision.
        let mut precision: Option<usize> = None;
        if cursor < bytes.len() && bytes[cursor] == b'.' {
            cursor += 1;
            let precision_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            precision = Some(
                format[precision_start..cursor]
                    .parse::<usize>()
                    .map_err(|_| ())?,
            );
        }
        let spec = bytes.get(cursor).copied().ok_or(())?;
        cursor += 1;

        let arg_index = positional
            .map(|position| position.saturating_sub(1))
            .unwrap_or(next_arg);
        next_arg = arg_index + 1;
        let arg = args.get(arg_index).ok_or(())?;

        let mut text = match spec {
            b's' | b'S' => match arg {
                FormatArg::Text(text) => (*text).to_owned(),
                other => match other {
                    FormatArg::Number(value) => value.to_string(),
                    FormatArg::Real(value) => value.to_string(),
                    FormatArg::Text(text) => (*text).to_owned(),
                },
            },
            b'd' => match arg {
                FormatArg::Number(value) => value.to_string(),
                FormatArg::Real(value) => (*value as i64).to_string(),
                FormatArg::Text(_) => return Err(()),
            },
            b'f' => {
                let value = match arg {
                    FormatArg::Real(value) => *value,
                    FormatArg::Number(value) => f64::from(*value as i32),
                    FormatArg::Text(_) => return Err(()),
                };
                let decimals = precision.unwrap_or(6);
                format!("{value:.decimals$}")
            }
            _ => return Err(()),
        };
        if spec == b'S' {
            text = text.to_uppercase();
        }
        if let Some(width) = width {
            for _ in 0..width.saturating_sub(text.chars().count()) {
                out.push(' ');
            }
        }
        out.push_str(&text);
        index = cursor;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lang_lines() {
        let mut locale = Locale::new();
        locale.load_bytes(b"# comment\nkey=value\nblank\n\nname=Stone\npath=a=b\n");
        assert_eq!(locale.get("key"), Some("value"));
        assert_eq!(locale.get("name"), Some("Stone"));
        assert_eq!(locale.get("path"), Some("a=b"));
        assert_eq!(locale.translation_count(), 3);
    }

    #[test]
    fn skips_comments_and_valueless_lines() {
        let mut locale = Locale::new();
        locale.load_bytes(b"#note=kept out\njust text\nkey=value\n");
        assert_eq!(locale.translation_count(), 1);
    }

    #[test]
    fn rewrites_format_chars() {
        let mut locale = Locale::new();
        locale.load_bytes(b"a=%s tiles\nb=%d\nc=%1$d\nd=%.2f\ne=%5.2f\ng=%s %% %d\n");
        assert_eq!(locale.get("a"), Some("%s tiles"));
        assert_eq!(locale.get("b"), Some("%s"));
        assert_eq!(locale.get("c"), Some("%1$s"));
        assert_eq!(locale.get("d"), Some("%s"));
        // The Java pattern `%(\\d+\\$)?[\\d\\.]*[df]` keeps only the
        // positional group: the padded/precise forms lose their specs.
        assert_eq!(locale.get("e"), Some("%s"));
        assert_eq!(locale.get("g"), Some("%s %% %s"));
    }

    #[test]
    fn unicode_detection() {
        let mut locale = Locale::new();
        locale.load_bytes(b"a=plain ascii\n");
        locale.check_unicode_usage();
        assert!(!locale.is_unicode());
        // `load` runs the check; a raw `load_bytes` does not (vanilla only
        // checks at the end of `Locale.load`).
        locale.load_bytes("a=подробности\u{1f}\n".as_bytes());
        locale.check_unicode_usage();
        assert!(locale.is_unicode());
        let mut empty = Locale::new();
        empty.check_unicode_usage();
        assert!(!empty.is_unicode());
    }

    #[test]
    fn translate_uses_loaded_key_or_the_key_itself() {
        let mut locale = Locale::new();
        locale.load_bytes(b"tile.stone.name=Stone\nitems.count=%s items\n");
        assert_eq!(locale.translate("tile.stone.name", &[]), "Stone");
        assert_eq!(
            locale.translate("tile.missing.name", &[]),
            "tile.missing.name"
        );
        assert_eq!(
            locale.translate("items.count", &[FormatArg::Text("three")]),
            "three items"
        );
    }

    #[test]
    fn translate_formats_java_style() {
        let mut locale = Locale::new();
        // The load-time rewrite makes every spec a `%s`, so precision never
        // applies to loaded translations (vanilla formats them the same).
        locale.load_bytes(b"x=%s/%d/%.1f\ny=%2$s then %1$s\n");
        assert_eq!(
            locale.translate(
                "x",
                &[
                    FormatArg::Text("a"),
                    FormatArg::Number(3),
                    FormatArg::Real(1.25)
                ]
            ),
            "a/3/1.25"
        );
        assert_eq!(
            locale.translate("y", &[FormatArg::Text("one"), FormatArg::Text("two")]),
            "two then one"
        );
    }

    #[test]
    fn format_error_matches_vanilla() {
        let mut locale = Locale::new();
        locale.load_bytes(b"bad=%d\n");
        // "%d" loads as "%s"; a format error needs a missing argument.
        assert_eq!(locale.translate("bad", &[]), "Format error: %s");
    }

    #[test]
    fn default_load_order_is_en_us_then_current() {
        let mut locale = Locale::new();
        locale.load_default_order("de_DE", |namespace, path| {
            assert_eq!(namespace, "minecraft");
            let mut sets = Vec::new();
            if path.ends_with("en_US.lang") {
                sets.push(b"tile.stone.name=Stone\nitems.stone.name=Stone".to_vec());
            } else {
                sets.push(b"tile.stone.name=Stein".to_vec());
            }
            sets
        });
        assert_eq!(locale.get("tile.stone.name"), Some("Stein"));
        assert_eq!(locale.get("items.stone.name"), Some("Stone"));
    }

    #[test]
    fn language_display_matches_vanilla() {
        let language = Language {
            code: "de_DE".to_owned(),
            name: "Deutsch".to_owned(),
            region: "Deutschland".to_owned(),
            bidirectional: false,
        };
        assert_eq!(language.display(), "Deutschland (Deutsch)");
    }
}
