//! Java `java.util.UUID` semantics: the 128 bits, the `toString`/`fromString`
//! form (lowercase, 8-4-4-4-12 with dashes) and `nameUUIDFromBytes`, the MD5
//! version-3 UUID the login flow uses for offline players.

use md5::{Digest, Md5};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Uuid {
    bytes: [u8; 16],
}

impl Uuid {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// `UUID.nameUUIDFromBytes`: MD5 of the bytes, version 3, IETF variant.
    pub fn name_uuid_from_bytes(data: &[u8]) -> Self {
        let mut digest = Md5::new();
        digest.update(data);
        let mut bytes: [u8; 16] = digest.finalize().into();
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self { bytes }
    }

    /// The offline-mode login UUID: `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`.
    pub fn offline_player(name: &str) -> Self {
        Self::name_uuid_from_bytes(format!("OfflinePlayer:{name}").as_bytes())
    }

    /// `UUID.toString`: lowercase hex, dashes after 8, 4, 4 and 4 digits.
    pub fn to_java_string(&self) -> String {
        let hex: String = self.bytes.iter().map(|b| format!("{b:02x}")).collect();
        format!(
            "{}-{}-{}-{}-{}",
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..]
        )
    }

    /// `UUID.fromString`: canonical form with or without dashes.
    pub fn parse(text: &str) -> Option<Self> {
        let stripped: String = text.chars().filter(|c| *c != '-').collect();
        if stripped.len() != 32 || !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&stripped[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self { bytes })
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_java_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_player_uuid_matches_java() {
        // java.util.UUID.nameUUIDFromBytes("OfflinePlayer:Notch")
        assert_eq!(
            Uuid::offline_player("Notch").to_java_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(
            Uuid::offline_player("Player").to_java_string(),
            "a01e3843-e521-3998-958a-f459800e4d11"
        );
    }

    #[test]
    fn parse_round_trips() {
        let text = "b50ad385-829d-3141-a216-7e7d7539ba7f";
        assert_eq!(Uuid::parse(text).unwrap().to_java_string(), text);
        let dashed = Uuid::offline_player("Notch");
        assert_eq!(Uuid::parse(&dashed.to_java_string()).unwrap(), dashed);
        assert!(Uuid::parse("nope").is_none());
        assert!(Uuid::parse("b50ad385829d3141a2167e7d7539ba7f").is_some());
    }
}
