//! `net.minecraft.client.resource.Identifier`, ported exactly from the
//! 1.7.10 decompiled source (calamus-modernized names, exact logic).

use std::fmt;

/// A namespaced resource path (`minecraft:stone`). An empty or missing
/// namespace becomes `minecraft`; the namespace is lowercased, and a colon at
/// index 0 or 1 keeps the default namespace (Java's `identifier.indexOf(58)`
/// logic).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identifier {
    namespace: String,
    path: String,
}

impl Identifier {
    /// `new Identifier(namespace, path)`: a null/empty namespace is replaced
    /// with `minecraft`.
    pub fn new(namespace: &str, path: &str) -> Self {
        let namespace = if namespace.is_empty() {
            "minecraft"
        } else {
            namespace
        };
        Self {
            namespace: namespace.to_owned(),
            path: path.to_owned(),
        }
    }

    /// `new Identifier(identifier)`: splits on the first `:`. The part before
    /// it is the namespace (lowercased, dropped when it is one character or
    /// the colon sits at index 0), the part after it the path.
    pub fn parse(identifier: &str) -> Self {
        let mut namespace = "minecraft";
        let mut path = identifier;
        if let Some(colon) = identifier.find(':') {
            path = &identifier[colon + 1..];
            if colon > 1 {
                namespace = &identifier[..colon];
            }
        }
        Self {
            namespace: namespace.to_lowercase(),
            path: path.to_owned(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_minecraft() {
        assert_eq!(
            Identifier::parse("stone"),
            Identifier::new("minecraft", "stone")
        );
        assert_eq!(Identifier::parse("stone").namespace(), "minecraft");
    }

    #[test]
    fn splits_on_first_colon() {
        let id = Identifier::parse("mymod:textures/blocks/x.png");
        assert_eq!(id.namespace(), "mymod");
        assert_eq!(id.path(), "textures/blocks/x.png");
    }

    #[test]
    fn lowercase_namespace() {
        assert_eq!(Identifier::parse("MyMod:stone").namespace(), "mymod");
        assert_eq!(Identifier::new("MyMod", "stone").namespace(), "MyMod");
    }

    #[test]
    fn one_character_namespace_is_dropped() {
        // Java: `if (var4 > 1) var2 = substring(0, var4)` — a namespace of
        // length 0 or 1 stays "minecraft".
        assert_eq!(Identifier::parse("a:stone").namespace(), "minecraft");
        assert_eq!(Identifier::parse(":stone").namespace(), "minecraft");
        assert_eq!(Identifier::parse(":stone").path(), "stone");
    }

    #[test]
    fn display_round_trip() {
        assert_eq!(Identifier::parse("mymod:stone").to_string(), "mymod:stone");
        assert_eq!(Identifier::new("", "stone").to_string(), "minecraft:stone");
    }
}
