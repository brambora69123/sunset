//! Argument parsing shared by every command: Java's `Integer.parseInt` and
//! `Double.parseDouble` (via `sunrise_core`'s exact parsers), the range
//! checks, booleans, `~` coordinates, and text joining.

use crate::error::CommandError;
use crate::text::Text;
use sunrise_core::java_float_parse::parse_java_double;
use sunrise_core::java_number::double_to_string;

/// Java's `Double.toString`, which the number exceptions interpolate.
pub fn format_double(value: f64) -> String {
    double_to_string(value)
}

/// `AbstractCommand.parseInt(source, s)`: Java `Integer.parseInt` or
/// `commands.generic.num.invalid`.
pub fn parse_int(text: &str) -> Result<i32, CommandError> {
    // Java's Integer.parseInt: an optional sign and decimal digits, in i32 range.
    let bytes = text.as_bytes();
    let (negative, digits) = match bytes.first() {
        Some(b'-') => (true, &bytes[1..]),
        Some(b'+') => (false, &bytes[1..]),
        Some(_) => (false, bytes),
        None => return Err(CommandError::num_invalid(text)),
    };
    if digits.is_empty() || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Err(CommandError::num_invalid(text));
    }
    let mut value: i64 = 0;
    for &b in digits {
        value = value * 10 + i64::from(b - b'0');
        if value > i32::MAX as i64 + 1 {
            return Err(CommandError::num_invalid(text));
        }
    }
    if negative {
        value = -value;
    }
    if value < i32::MIN as i64 || value > i32::MAX as i64 {
        return Err(CommandError::num_invalid(text));
    }
    Ok(value as i32)
}

/// `parseInt(source, s, min, max)`.
pub fn parse_int_range(text: &str, min: i32, max: i32) -> Result<i32, CommandError> {
    let value = parse_int(text)?;
    if value < min {
        return Err(CommandError::num_too_small(value, min));
    }
    if value > max {
        return Err(CommandError::num_too_big(value, max));
    }
    Ok(value)
}

/// `AbstractCommand.parseDouble(source, s)`: Java `Double.parseDouble`, then
/// Guava's `Doubles.isFinite` (NaN and the infinities are refused).
pub fn parse_double(text: &str) -> Result<f64, CommandError> {
    let value = parse_java_double(text).ok_or_else(|| CommandError::num_invalid(text))?;
    if !value.is_finite() {
        return Err(CommandError::num_invalid(text));
    }
    Ok(value)
}

/// `parseDouble(source, s, min, max)`.
pub fn parse_double_range(text: &str, min: f64, max: f64) -> Result<f64, CommandError> {
    let value = parse_double(text)?;
    if value < min {
        return Err(CommandError::double_too_small(value, min as i32));
    }
    if value > max {
        return Err(CommandError::double_too_big(value, max as i32));
    }
    Ok(value)
}

/// `AbstractCommand.parseBoolean`: `true`/`1`, `false`/`0`, else an error.
pub fn parse_boolean(text: &str) -> Result<bool, CommandError> {
    match text {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(CommandError::exception_args(
            "commands.generic.boolean.invalid",
            [Text::literal(text)],
        )),
    }
}

/// `AbstractCommand.parseCoordinate(source, c, s, min, max)` with the default
/// world-edge bounds of -30000000..30000000. A `~` prefix adds to `c`; a bare
/// integer gains the 0.5 centering. `min == max == 0` skips the bounds.
pub fn parse_coordinate(origin: f64, text: &str, min: i32, max: i32) -> Result<f64, CommandError> {
    let relative = text.starts_with('~');
    if relative && origin.is_nan() {
        return Err(CommandError::num_invalid(format!("{origin}")));
    }

    let mut value = if relative { origin } else { 0.0 };
    if !relative || text.len() > 1 {
        let has_dot = text.contains('.');
        let rest = if relative { &text[1..] } else { text };
        value += parse_double(rest)?;
        if !has_dot && !relative {
            value += 0.5;
        }
    }

    if min != 0 || max != 0 {
        if value < f64::from(min) {
            return Err(CommandError::double_too_small(value, min));
        }
        if value > f64::from(max) {
            return Err(CommandError::double_too_big(value, max));
        }
    }
    Ok(value)
}

/// `parseCoordinate(source, c, s)`: the world-edge bounds.
pub fn parse_coordinate_default(origin: f64, text: &str) -> Result<f64, CommandError> {
    parse_coordinate(origin, text, -30_000_000, 30_000_000)
}

/// `AbstractCommand.parseString(source, args, start)`: the args joined with
/// single spaces.
pub fn parse_string(args: &[String], start: usize) -> String {
    args[start..].join(" ")
}

/// `AbstractCommand.listArgs(Object[])`: "a, b and c".
pub fn list_args<I: AsRef<str>>(args: &[I]) -> String {
    let mut out = String::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(if i == args.len() - 1 { " and " } else { ", " });
        }
        out.push_str(arg.as_ref());
    }
    out
}

/// Java's `String.regionMatches(ignoreCase, toffset, other, ooffset, len)`.
pub fn region_matches(a: &str, toffset: usize, b: &str, ooffset: usize, len: usize) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if toffset > a.len() || ooffset > b.len() || toffset + len > a.len() || ooffset + len > b.len()
    {
        return false;
    }
    for i in 0..len {
        let x = a[toffset + i];
        let y = b[ooffset + i];
        if x != y && x.to_lowercase().next() != y.to_lowercase().next() {
            return false;
        }
    }
    true
}

/// `AbstractCommand.doesStringStartWith`, case-insensitive.
pub fn does_string_start_with(region: &str, text: &str) -> bool {
    region_matches(region, 0, text, 0, region.chars().count())
}

/// `AbstractCommand.suggestMatching(args, suggestions)`: the suggestions that
/// the last arg starts with, case-insensitively.
pub fn suggest_matching(args: &[&str], suggestions: &[&str]) -> Vec<String> {
    suggestions
        .iter()
        .filter(|s| does_string_start_with(args.last().copied().unwrap_or(""), s))
        .map(|s| (*s).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_matches_java() {
        assert_eq!(parse_int("0").unwrap(), 0);
        assert_eq!(parse_int("-2147483648").unwrap(), i32::MIN);
        assert_eq!(parse_int("2147483647").unwrap(), i32::MAX);
        assert_eq!(parse_int("+5").unwrap(), 5);
        assert_eq!(parse_int("0012").unwrap(), 12);
        assert!(parse_int("2147483648").is_err());
        assert!(parse_int("").is_err());
        assert!(parse_int("-").is_err());
        assert!(parse_int("1.5").is_err());
        assert!(parse_int("0x10").is_err());
        assert!(parse_int(" 1").is_err());
    }

    #[test]
    fn parse_int_range_bounds() {
        assert_eq!(parse_int_range("3", 1, 64).unwrap(), 3);
        assert!(matches!(
            parse_int_range("0", 1, 64),
            Err(CommandError { key, .. }) if key == "commands.generic.num.tooSmall"
        ));
        assert!(matches!(
            parse_int_range("65", 1, 64),
            Err(CommandError { key, .. }) if key == "commands.generic.num.tooBig"
        ));
    }

    #[test]
    fn parse_double_java_syntax() {
        assert_eq!(parse_double("1.5").unwrap(), 1.5);
        assert_eq!(parse_double("-3").unwrap(), -3.0);
        // Java accepts a trailing d/D and hex floats; Rust's f64 parse doesn't.
        assert_eq!(parse_double("1.5d").unwrap(), 1.5);
        assert_eq!(parse_double("0x1.8p1").unwrap(), 3.0);
        assert!(parse_double("NaN").is_err());
        assert!(parse_double("Infinity").is_err());
        assert!(parse_double("abc").is_err());
    }

    #[test]
    fn coordinate_tilde_and_centering() {
        // Bare numbers center on the block (+0.5); a tilde adds to the origin.
        assert_eq!(parse_coordinate_default(10.0, "5").unwrap(), 5.5);
        assert_eq!(parse_coordinate_default(10.0, "5.25").unwrap(), 5.25);
        assert_eq!(parse_coordinate_default(10.0, "~").unwrap(), 10.0);
        assert_eq!(parse_coordinate_default(10.0, "~3").unwrap(), 13.0);
        assert_eq!(parse_coordinate_default(10.0, "~-2.5").unwrap(), 7.5);
        assert_eq!(parse_coordinate_default(10.0, "~0").unwrap(), 10.0);
        assert!(parse_coordinate_default(f64::NAN, "~").is_err());
        assert!(parse_coordinate_default(0.0, "31000000").is_err());
        // min == max == 0 skips the bounds check, as /tp's y passes.
        assert_eq!(parse_coordinate(0.0, "40000000", 0, 0).unwrap(), 40000000.5);
    }

    #[test]
    fn boolean_forms() {
        assert!(parse_boolean("true").unwrap());
        assert!(parse_boolean("1").unwrap());
        assert!(!parse_boolean("0").unwrap());
        assert!(parse_boolean("yes").is_err());
    }

    #[test]
    fn list_args_uses_and() {
        assert_eq!(list_args(&["a"]), "a");
        assert_eq!(list_args(&["a", "b"]), "a and b");
        assert_eq!(list_args(&["a", "b", "c"]), "a, b and c");
    }

    #[test]
    fn suggest_matches_prefix() {
        assert_eq!(suggest_matching(&["da"], &["day", "night", "add"]), ["day"]);
        assert_eq!(suggest_matching(&["", "s"], &["set", "add"]), ["set"]);
    }
}
