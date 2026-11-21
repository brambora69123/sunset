//! Chat text as commands produce it: a `Formatting` code table and a
//! `Text` message (literal or translation key with substitution args).

/// `net.minecraft.text.Formatting`: the 22 `§` codes, in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Formatting {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Obfuscated,
    Bold,
    Strikethrough,
    Underline,
    Italic,
    Reset,
}

impl Formatting {
    /// The `§` code character (`getCode`).
    pub fn code(self) -> char {
        match self {
            Self::Black => '0',
            Self::DarkBlue => '1',
            Self::DarkGreen => '2',
            Self::DarkAqua => '3',
            Self::DarkRed => '4',
            Self::DarkPurple => '5',
            Self::Gold => '6',
            Self::Gray => '7',
            Self::DarkGray => '8',
            Self::Blue => '9',
            Self::Green => 'a',
            Self::Aqua => 'b',
            Self::Red => 'c',
            Self::LightPurple => 'd',
            Self::Yellow => 'e',
            Self::White => 'f',
            Self::Obfuscated => 'k',
            Self::Bold => 'l',
            Self::Strikethrough => 'm',
            Self::Underline => 'n',
            Self::Italic => 'o',
            Self::Reset => 'r',
        }
    }

    /// The lowercase enum name (`getName`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::DarkBlue => "dark_blue",
            Self::DarkGreen => "dark_green",
            Self::DarkAqua => "dark_aqua",
            Self::DarkRed => "dark_red",
            Self::DarkPurple => "dark_purple",
            Self::Gold => "gold",
            Self::Gray => "gray",
            Self::DarkGray => "dark_gray",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Aqua => "aqua",
            Self::Red => "red",
            Self::LightPurple => "light_purple",
            Self::Yellow => "yellow",
            Self::White => "white",
            Self::Obfuscated => "obfuscated",
            Self::Bold => "bold",
            Self::Strikethrough => "strikethrough",
            Self::Underline => "underline",
            Self::Italic => "italic",
            Self::Reset => "reset",
        }
    }

    /// `isModifier`: the five format codes.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Self::Obfuscated | Self::Bold | Self::Strikethrough | Self::Underline | Self::Italic
        )
    }

    /// `isColor`: a color code, which `Reset` is not.
    pub fn is_color(self) -> bool {
        !self.is_modifier() && self != Self::Reset
    }

    /// `toString`: the `§` sequence written into team prefixes and suffixes.
    pub fn prefix(self) -> String {
        format!("\u{a7}{}", self.code())
    }

    /// `byName`, case-insensitive over the lowercase names.
    pub fn by_name(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        Self::all().into_iter().find(|f| f.name() == lower)
    }

    /// `getNames(colors, modifiers)`, in declaration order.
    pub fn get_names(colors: bool, modifiers: bool) -> Vec<&'static str> {
        Self::all()
            .into_iter()
            .filter(|f| (!f.is_color() || colors) && (!f.is_modifier() || modifiers))
            .map(Self::name)
            .collect()
    }

    /// The enum's `values()`, in declaration order.
    pub fn all() -> [Self; 22] {
        [
            Self::Black,
            Self::DarkBlue,
            Self::DarkGreen,
            Self::DarkAqua,
            Self::DarkRed,
            Self::DarkPurple,
            Self::Gold,
            Self::Gray,
            Self::DarkGray,
            Self::Blue,
            Self::Green,
            Self::Aqua,
            Self::Red,
            Self::LightPurple,
            Self::Yellow,
            Self::White,
            Self::Obfuscated,
            Self::Bold,
            Self::Strikethrough,
            Self::Underline,
            Self::Italic,
            Self::Reset,
        ]
    }
}

/// One message as a command produces it: a literal string or a translation
/// key with substitution args, plus the style flags commands set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    /// The literal text, or the translation key when `literal` is false.
    pub key: String,
    pub args: Vec<Text>,
    pub literal: bool,
    pub color: Option<Formatting>,
    pub italic: bool,
}

impl Text {
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            key: text.into(),
            args: Vec::new(),
            literal: true,
            color: None,
            italic: false,
        }
    }

    pub fn translated(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
            literal: false,
            color: None,
            italic: false,
        }
    }

    /// A translation with substitution args (`new TranslatableText(key, args)`).
    pub fn translated_args(key: impl Into<String>, args: impl IntoIterator<Item = Text>) -> Self {
        Self {
            args: args.into_iter().collect(),
            ..Self::translated(key)
        }
    }

    pub fn with_color(mut self, color: Formatting) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// The flat text a player reads: the literal text, or the key followed by
    /// its args (a lang lookup is the caller's job).
    pub fn plain(&self) -> String {
        if self.args.is_empty() {
            return self.key.clone();
        }
        let mut out = String::from(&*self.key);
        for arg in &self.args {
            out.push(' ');
            out.push_str(&arg.plain());
        }
        out
    }
}
