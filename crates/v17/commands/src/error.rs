//! The 1.7.10 command exception hierarchy as one error: a message key with
//! substitution args, and the `IncorrectUsageException` mark.

use crate::text::Text;

/// `CommandException` and its subclasses. `usage` distinguishes
/// `IncorrectUsageException` (the registry prints the usage line for it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandError {
    pub key: String,
    pub args: Vec<Text>,
    pub usage: bool,
}

impl CommandError {
    /// `new CommandException(key, args)`.
    pub fn exception(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
            usage: false,
        }
    }

    pub fn exception_args(key: impl Into<String>, args: impl IntoIterator<Item = Text>) -> Self {
        Self {
            key: key.into(),
            args: args.into_iter().collect(),
            usage: false,
        }
    }

    /// `new IncorrectUsageException(key)`.
    pub fn usage(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
            usage: true,
        }
    }

    /// `new IncorrectUsageException(key, args)`.
    pub fn usage_args(key: impl Into<String>, args: impl IntoIterator<Item = Text>) -> Self {
        Self {
            key: key.into(),
            args: args.into_iter().collect(),
            usage: true,
        }
    }

    /// `new CommandNotFoundException()`.
    pub fn not_found() -> Self {
        Self::exception("commands.generic.notFound")
    }

    /// `new InvalidNumberException("commands.generic.num.invalid", s)`.
    pub fn num_invalid(text: impl Into<String>) -> Self {
        Self::exception_args("commands.generic.num.invalid", [Text::literal(text.into())])
    }

    /// `new InvalidNumberException("commands.generic.num.tooSmall", v, min)`.
    pub fn num_too_small(value: i32, min: i32) -> Self {
        Self::exception_args(
            "commands.generic.num.tooSmall",
            [
                Text::literal(value.to_string()),
                Text::literal(min.to_string()),
            ],
        )
    }

    /// `new InvalidNumberException("commands.generic.num.tooBig", v, max)`.
    pub fn num_too_big(value: i32, max: i32) -> Self {
        Self::exception_args(
            "commands.generic.num.tooBig",
            [
                Text::literal(value.to_string()),
                Text::literal(max.to_string()),
            ],
        )
    }

    /// `new InvalidNumberException("commands.generic.double.tooSmall", v, min)`.
    pub fn double_too_small(value: f64, min: i32) -> Self {
        Self::exception_args(
            "commands.generic.double.tooSmall",
            [
                Text::literal(crate::args::format_double(value)),
                Text::literal(min.to_string()),
            ],
        )
    }

    /// `new InvalidNumberException("commands.generic.double.tooBig", v, max)`.
    pub fn double_too_big(value: f64, max: i32) -> Self {
        Self::exception_args(
            "commands.generic.double.tooBig",
            [
                Text::literal(crate::args::format_double(value)),
                Text::literal(max.to_string()),
            ],
        )
    }

    /// `new PlayerNotFoundException()`.
    pub fn player_not_found() -> Self {
        Self::exception("commands.generic.player.notFound")
    }

    /// `new PlayerNotFoundException("You must specify which player...")`.
    pub fn must_specify_player() -> Self {
        Self::exception("You must specify which player you wish to perform this action on.")
    }
}
