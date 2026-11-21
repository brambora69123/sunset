//! 1.7.10 version constants: `RealmsSharedConstants`, the handshake call
//! sites and the pack format the era's resource packs carry.

/// The game version this build speaks (`RealmsSharedConstants.f_59638686`,
/// also the window title "Minecraft 1.7.10").
pub const GAME_VERSION: &str = "1.7.10";

/// The protocol version every handshake sends (`HandshakeC2SPacket(5, ...)`
/// at each connect site; `RealmsSharedConstants.f_42649555`).
pub const PROTOCOL_VERSION: i32 = 5;

/// The resource pack format of the era: `pack.mcmeta`'s `pack_format`, read
/// and written by `PackMetadataSerializer` (vanilla's packs use 1).
pub const RESOURCE_PACK_FORMAT: i32 = 1;

/// 1.7.10 has no DataVersion: the tag first appears in saved worlds after
/// this release, so `level.dat` carries no version stamp and no datafixer
/// runs.
pub const DATA_VERSION: Option<i32> = None;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_1_7_10() {
        assert_eq!(GAME_VERSION, "1.7.10");
        assert_eq!(PROTOCOL_VERSION, 5);
        assert_eq!(RESOURCE_PACK_FORMAT, 1);
        assert_eq!(DATA_VERSION, None);
    }
}
