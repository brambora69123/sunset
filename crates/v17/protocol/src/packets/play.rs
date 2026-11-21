//! Play state, the packets needed to log in and spawn, and the player
//! movement family. Ported from 1.7.10's
//! `net.minecraft.network.packet.{s2c,c2s}.play`.

use crate::buf::{Buf, Result};
use crate::slot::Slot;

// --- enums the packets carry -------------------------------------------------

/// `WorldSettings$GameMode`. `by_id` falls back to survival for unknown ids,
/// exactly as Java does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    NotSet = -1,
    #[default]
    Survival = 0,
    Creative = 1,
    Adventure = 2,
}

impl GameMode {
    pub fn id(self) -> i32 {
        self as i32
    }

    pub fn by_id(id: i32) -> Self {
        match id {
            -1 => Self::NotSet,
            0 => Self::Survival,
            1 => Self::Creative,
            2 => Self::Adventure,
            _ => Self::Survival,
        }
    }
}

/// `Difficulty`. Reads take the raw byte 0-255, so ids are never negative;
/// `by_id` takes the id modulo 4, exactly as Java's lookup table does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Peaceful = 0,
    Easy = 1,
    Normal = 2,
    Hard = 3,
}

impl Difficulty {
    pub fn id(self) -> i32 {
        self as i32
    }

    pub fn by_id(id: i32) -> Option<Self> {
        if id < 0 {
            return None;
        }
        match id % 4 {
            0 => Some(Self::Peaceful),
            1 => Some(Self::Easy),
            2 => Some(Self::Normal),
            _ => Some(Self::Hard),
        }
    }
}

/// `WorldGeneratorType` keys on the Join Game and Respawn wires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeneratorType {
    #[default]
    Default,
    Flat,
    LargeBiomes,
    Amplified,
    Default11,
}

impl GeneratorType {
    pub fn key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Flat => "flat",
            Self::LargeBiomes => "largeBiomes",
            Self::Amplified => "amplified",
            Self::Default11 => "default_1_1",
        }
    }

    /// Case-insensitive key lookup, exactly as `WorldGeneratorType.byKey`;
    /// unknown keys are the caller's default (vanilla uses `DEFAULT`).
    pub fn by_key(key: &str) -> Option<Self> {
        let matches = |other: &str| other.eq_ignore_ascii_case(key);
        if matches("default") {
            Some(Self::Default)
        } else if matches("flat") {
            Some(Self::Flat)
        } else if matches("largeBiomes") {
            Some(Self::LargeBiomes)
        } else if matches("amplified") {
            Some(Self::Amplified)
        } else if matches("default_1_1") {
            Some(Self::Default11)
        } else {
            None
        }
    }
}

/// `PlayerEntity$ChatVisibility`; `by_index` takes the index modulo 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatVisibility {
    #[default]
    Full = 0,
    System = 1,
    Hidden = 2,
}

impl ChatVisibility {
    pub fn index(self) -> i32 {
        self as i32
    }

    pub fn by_index(index: i32) -> Option<Self> {
        if index < 0 {
            return None;
        }
        match index % 3 {
            0 => Some(Self::Full),
            1 => Some(Self::System),
            _ => Some(Self::Hidden),
        }
    }
}

// --- serverbound play --------------------------------------------------------

/// Keep alive, c2s 0x00 and s2c 0x00, identical on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeepAlive {
    pub time_millis: i32,
}

pub const KEEP_ALIVE_ID: i32 = 0x00;

impl KeepAlive {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            time_millis: buf.read_i32()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i32(self.time_millis);
        Ok(())
    }
}

/// Flying, c2s 0x03 (`PlayerMoveC2SPacket`): on-ground only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flying {
    pub on_ground: bool,
}

pub const FLYING_ID: i32 = 0x03;

impl Flying {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            on_ground: buf.read_u8()? != 0,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_u8(self.on_ground as u8);
        Ok(())
    }
}

/// Position, c2s 0x04 (`PlayerMoveC2SPacket$Position`). The wire order is
/// the Java field order: x, stance (`minY`), feet y, z, then the on-ground
/// byte.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerPosition {
    pub x: f64,
    pub min_y: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

pub const PLAYER_POSITION_ID: i32 = 0x04;

impl PlayerPosition {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            x: buf.read_f64()?,
            min_y: buf.read_f64()?,
            y: buf.read_f64()?,
            z: buf.read_f64()?,
            on_ground: buf.read_u8()? != 0,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_f64(self.x);
        buf.write_f64(self.min_y);
        buf.write_f64(self.y);
        buf.write_f64(self.z);
        buf.write_u8(self.on_ground as u8);
        Ok(())
    }
}

/// Look, c2s 0x05 (`PlayerMoveC2SPacket$Angles`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerLook {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

pub const PLAYER_LOOK_ID: i32 = 0x05;

impl PlayerLook {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            yaw: buf.read_f32()?,
            pitch: buf.read_f32()?,
            on_ground: buf.read_u8()? != 0,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_f32(self.yaw);
        buf.write_f32(self.pitch);
        buf.write_u8(self.on_ground as u8);
        Ok(())
    }
}

/// Position and look, c2s 0x06 (`PlayerMoveC2SPacket$PositionAndAngles`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerPositionLook {
    pub x: f64,
    pub min_y: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

pub const PLAYER_POSITION_LOOK_ID: i32 = 0x06;

impl PlayerPositionLook {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            x: buf.read_f64()?,
            min_y: buf.read_f64()?,
            y: buf.read_f64()?,
            z: buf.read_f64()?,
            yaw: buf.read_f32()?,
            pitch: buf.read_f32()?,
            on_ground: buf.read_u8()? != 0,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_f64(self.x);
        buf.write_f64(self.min_y);
        buf.write_f64(self.y);
        buf.write_f64(self.z);
        buf.write_f32(self.yaw);
        buf.write_f32(self.pitch);
        buf.write_u8(self.on_ground as u8);
        Ok(())
    }
}

/// Client settings, c2s 0x15 (`ClientSettingsC2SPacket`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientSettings {
    pub language: String,
    pub view_distance: i8,
    pub chat_visibility: Option<ChatVisibility>,
    pub chat_colors: bool,
    pub difficulty: Option<Difficulty>,
    pub show_cape: bool,
}

pub const CLIENT_SETTINGS_ID: i32 = 0x15;

impl ClientSettings {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            language: buf.read_string(7)?,
            view_distance: buf.read_i8()?,
            chat_visibility: ChatVisibility::by_index(buf.read_i8()? as i32),
            chat_colors: buf.read_bool()?,
            difficulty: Difficulty::by_id(buf.read_u8()? as i32),
            show_cape: buf.read_bool()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.language)?;
        buf.write_i8(self.view_distance);
        buf.write_i8(self.chat_visibility.map_or(0, |v| v.index()) as i8);
        buf.write_bool(self.chat_colors);
        buf.write_u8(self.difficulty.map_or(0, |d| d.id()) as u8);
        buf.write_bool(self.show_cape);
        Ok(())
    }
}

/// Client status, c2s 0x16 (`ClientStatusC2SPacket`): 0 performs respawn,
/// 1 requests statistics. The action id is kept raw; the mapping is the
/// handler's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClientStatus {
    pub action: u8,
}

pub const CLIENT_STATUS_ID: i32 = 0x16;

impl ClientStatus {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            action: buf.read_u8()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_u8(self.action);
        Ok(())
    }
}

// --- clientbound play --------------------------------------------------------

/// Join Game, s2c 0x01 (`LoginS2CPacket`): entity id, game mode with the
/// hardcore bit in bit 3, dimension, difficulty, max players and the
/// generator key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JoinGame {
    pub entity_id: i32,
    pub hardcore: bool,
    pub game_mode: GameMode,
    pub dimension: i8,
    pub difficulty: Option<Difficulty>,
    pub max_player_count: u8,
    pub generator_type: GeneratorType,
}

pub const JOIN_GAME_ID: i32 = 0x01;

impl JoinGame {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        let entity_id = buf.read_i32()?;
        let mut mode_bits = buf.read_u8()? as i32;
        let hardcore = mode_bits & 8 == 8;
        mode_bits &= !8;
        let dimension = buf.read_i8()?;
        let difficulty = Difficulty::by_id(buf.read_u8()? as i32);
        Ok(Self {
            entity_id,
            hardcore,
            game_mode: GameMode::by_id(mode_bits),
            dimension,
            difficulty,
            max_player_count: buf.read_u8()?,
            generator_type: GeneratorType::by_key(&buf.read_string(16)?).unwrap_or_default(),
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i32(self.entity_id);
        let mut mode_bits = self.game_mode.id();
        if self.hardcore {
            mode_bits |= 8;
        }
        buf.write_u8(mode_bits as u8);
        buf.write_i8(self.dimension);
        buf.write_u8(self.difficulty.map_or(0, |d| d.id()) as u8);
        buf.write_u8(self.max_player_count);
        buf.write_string(self.generator_type.key())
    }
}

/// Chat message, s2c 0x02 (`ChatMessageS2CPacket`): the chat JSON string.
/// Kept raw; the text parse belongs to the text crate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatMessage {
    pub json: String,
}

pub const CHAT_MESSAGE_ID: i32 = 0x02;

impl ChatMessage {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            json: buf.read_string(32767)?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.json)
    }
}

/// Time update, s2c 0x03 (`WorldTimeS2CPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldTime {
    pub time: i64,
    pub time_of_day: i64,
}

pub const WORLD_TIME_ID: i32 = 0x03;

impl WorldTime {
    /// Exactly `WorldTimeS2CPacket`'s constructor: with the daylight cycle
    /// off the time of day is negated, and -0 becomes -1.
    pub fn new(time: i64, time_of_day: i64, do_daylight_cycle: bool) -> Self {
        if do_daylight_cycle {
            Self { time, time_of_day }
        } else {
            let time_of_day = if -time_of_day == 0 { -1 } else { -time_of_day };
            Self { time, time_of_day }
        }
    }

    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            time: buf.read_i64()?,
            time_of_day: buf.read_i64()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i64(self.time);
        buf.write_i64(self.time_of_day);
        Ok(())
    }
}

/// Entity equipment, s2c 0x04 (`EntityEquipmentS2CPacket`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityEquipment {
    pub entity_id: i32,
    pub slot: i16,
    pub item: Option<Slot>,
}

pub const ENTITY_EQUIPMENT_ID: i32 = 0x04;

impl EntityEquipment {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            entity_id: buf.read_i32()?,
            slot: buf.read_i16()?,
            item: buf.read_slot()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i32(self.entity_id);
        buf.write_i16(self.slot);
        buf.write_slot(self.item.as_ref());
        Ok(())
    }
}

/// Spawn position, s2c 0x05 (`SpawnPointS2CPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpawnPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub const SPAWN_POSITION_ID: i32 = 0x05;

impl SpawnPosition {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            x: buf.read_i32()?,
            y: buf.read_i32()?,
            z: buf.read_i32()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i32(self.x);
        buf.write_i32(self.y);
        buf.write_i32(self.z);
        Ok(())
    }
}

/// Update health, s2c 0x06 (`PlayerHealthS2CPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UpdateHealth {
    pub health: f32,
    pub hunger: i16,
    pub saturation: f32,
}

pub const UPDATE_HEALTH_ID: i32 = 0x06;

impl UpdateHealth {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            health: buf.read_f32()?,
            hunger: buf.read_i16()?,
            saturation: buf.read_f32()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_f32(self.health);
        buf.write_i16(self.hunger);
        buf.write_f32(self.saturation);
        Ok(())
    }
}

/// Respawn, s2c 0x07 (`PlayerRespawnS2CPacket`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Respawn {
    pub dimension: i32,
    pub difficulty: Option<Difficulty>,
    pub game_mode: GameMode,
    pub generator_type: GeneratorType,
}

pub const RESPAWN_ID: i32 = 0x07;

impl Respawn {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            dimension: buf.read_i32()?,
            difficulty: Difficulty::by_id(buf.read_u8()? as i32),
            game_mode: GameMode::by_id(buf.read_u8()? as i32),
            generator_type: GeneratorType::by_key(&buf.read_string(16)?).unwrap_or_default(),
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i32(self.dimension);
        buf.write_u8(self.difficulty.map_or(0, |d| d.id()) as u8);
        buf.write_u8(self.game_mode.id() as u8);
        buf.write_string(self.generator_type.key())
    }
}

/// Position and look, s2c 0x08 (`PlayerMoveS2CPacket`): no stance here, the
/// client computes it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionAndLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

pub const POSITION_AND_LOOK_ID: i32 = 0x08;

impl PositionAndLook {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            x: buf.read_f64()?,
            y: buf.read_f64()?,
            z: buf.read_f64()?,
            yaw: buf.read_f32()?,
            pitch: buf.read_f32()?,
            on_ground: buf.read_bool()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_f64(self.x);
        buf.write_f64(self.y);
        buf.write_f64(self.z);
        buf.write_f32(self.yaw);
        buf.write_f32(self.pitch);
        buf.write_bool(self.on_ground);
        Ok(())
    }
}

/// Held item change, s2c 0x09 (`SelectSlotS2CPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeldItemChange {
    pub slot: i8,
}

pub const HELD_ITEM_CHANGE_ID: i32 = 0x09;

impl HeldItemChange {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            slot: buf.read_i8()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i8(self.slot);
        Ok(())
    }
}

/// Set slot, s2c 0x2F (`InventoryMenuSlotContentS2CPacket`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetSlot {
    pub window_id: i8,
    pub slot: i16,
    pub item: Option<Slot>,
}

pub const SET_SLOT_ID: i32 = 0x2f;

impl SetSlot {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            window_id: buf.read_i8()?,
            slot: buf.read_i16()?,
            item: buf.read_slot()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i8(self.window_id);
        buf.write_i16(self.slot);
        buf.write_slot(self.item.as_ref());
        Ok(())
    }
}

/// Player abilities, s2c 0x39 (`PlayerAbilitiesS2CPacket`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerAbilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub creative_mode: bool,
    pub fly_speed: f32,
    pub walk_speed: f32,
}

pub const PLAYER_ABILITIES_ID: i32 = 0x39;

impl PlayerAbilities {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        let flags = buf.read_u8()?;
        Ok(Self {
            invulnerable: flags & 1 != 0,
            flying: flags & 2 != 0,
            allow_flying: flags & 4 != 0,
            creative_mode: flags & 8 != 0,
            fly_speed: buf.read_f32()?,
            walk_speed: buf.read_f32()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        let mut flags = 0u8;
        if self.invulnerable {
            flags |= 1;
        }
        if self.flying {
            flags |= 2;
        }
        if self.allow_flying {
            flags |= 4;
        }
        if self.creative_mode {
            flags |= 8;
        }
        buf.write_u8(flags);
        buf.write_f32(self.fly_speed);
        buf.write_f32(self.walk_speed);
        Ok(())
    }
}

/// Disconnect, s2c 0x40.
pub const DISCONNECT_ID: i32 = 0x40;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::var_int_size;

    #[test]
    fn join_game_known_bytes() {
        // Creative world on the overworld: entity id 1, creative, no hardcore,
        // dimension 0, easy, max players 8, default generator.
        let join = JoinGame {
            entity_id: 1,
            hardcore: false,
            game_mode: GameMode::Creative,
            dimension: 0,
            difficulty: Some(Difficulty::Easy),
            max_player_count: 8,
            generator_type: GeneratorType::Default,
        };
        let mut buf = Buf::new();
        join.write(&mut buf).unwrap();
        assert_eq!(
            buf.into_vec(),
            vec![
                0, 0, 0, 1, 1, 0, 1, 8, 7, b'd', b'e', b'f', b'a', b'u', b'l', b't'
            ]
        );

        // Hardcore bit rides in bit 3 of the game mode byte: 0x09 is hardcore
        // creative.
        let mut buf = Buf::from(vec![0, 0, 0, 1, 0x09, 0, 2, 0, 4, b'f', b'l', b'a', b't']);
        let join = JoinGame::read(&mut buf).unwrap();
        assert!(join.hardcore);
        assert_eq!(join.game_mode, GameMode::Creative);
        assert_eq!(join.dimension, 0);
        assert_eq!(join.difficulty, Some(Difficulty::Normal));
        assert_eq!(join.max_player_count, 0);
        assert_eq!(join.generator_type, GeneratorType::Flat);
    }

    #[test]
    fn join_game_unknown_generator_is_default() {
        let mut buf = Buf::from(vec![0, 0, 0, 0, 0, 0, 0, 0, 4, b'n', b'o', b'n', b'e']);
        let join = JoinGame::read(&mut buf).unwrap();
        assert_eq!(join.generator_type, GeneratorType::Default);
    }

    #[test]
    fn join_game_round_trip() {
        // Not_set does not survive the wire: vanilla writes -1, reads 0xff
        // back as survival (its own by_id fallback), so only the other three
        // round trip.
        for hardcore in [false, true] {
            for mode in [GameMode::Survival, GameMode::Creative, GameMode::Adventure] {
                let join = JoinGame {
                    entity_id: 42,
                    hardcore,
                    game_mode: mode,
                    dimension: -1,
                    difficulty: Some(Difficulty::Hard),
                    max_player_count: 255,
                    generator_type: GeneratorType::LargeBiomes,
                };
                let mut buf = Buf::new();
                join.write(&mut buf).unwrap();
                assert_eq!(JoinGame::read(&mut buf).unwrap(), join);
            }
        }
        // The vanilla quirk itself: -1 on the wire reads back as survival,
        // with the hardcore bit consumed.
        let mut buf = Buf::from(vec![0, 0, 0, 1, 0xff, 0, 0, 8, 4, b'f', b'l', b'a', b't']);
        let join = JoinGame::read(&mut buf).unwrap();
        assert_eq!(join.game_mode, GameMode::Survival);
        assert!(join.hardcore);
    }

    #[test]
    fn enum_fallbacks_match_vanilla() {
        // GameMode.byId falls back to survival; -1 is NotSet.
        assert_eq!(GameMode::by_id(3), GameMode::Survival);
        assert_eq!(GameMode::by_id(-1), GameMode::NotSet);
        // Difficulty takes the id modulo 4.
        assert_eq!(Difficulty::by_id(255), Some(Difficulty::Hard));
        assert_eq!(Difficulty::by_id(-1), None);
        // ChatVisibility takes the index modulo 3.
        assert_eq!(ChatVisibility::by_index(5), Some(ChatVisibility::Hidden));
        assert_eq!(ChatVisibility::by_index(-1), None);
        // Generator keys are case-insensitive.
        assert_eq!(
            GeneratorType::by_key("LARGEBIOMES"),
            Some(GeneratorType::LargeBiomes)
        );
        assert_eq!(
            GeneratorType::by_key("default_1_1"),
            Some(GeneratorType::Default11)
        );
    }

    #[test]
    fn world_time_daylight_cycle_off_negates() {
        assert_eq!(
            WorldTime::new(100, 6000, true),
            WorldTime {
                time: 100,
                time_of_day: 6000
            }
        );
        assert_eq!(
            WorldTime::new(100, 6000, false),
            WorldTime {
                time: 100,
                time_of_day: -6000
            }
        );
        // -0 becomes -1.
        assert_eq!(
            WorldTime::new(100, 0, false),
            WorldTime {
                time: 100,
                time_of_day: -1
            }
        );
    }

    #[test]
    fn position_and_look_round_trip() {
        let packet = PositionAndLook {
            x: 8.5,
            y: 64.0,
            z: -12.25,
            yaw: 180.0,
            pitch: -90.0,
            on_ground: true,
        };
        let mut buf = Buf::new();
        packet.write(&mut buf).unwrap();
        assert_eq!(buf.len(), 8 * 3 + 4 * 2 + 1);
        assert_eq!(PositionAndLook::read(&mut buf).unwrap(), packet);
    }

    #[test]
    fn movement_family_wire_orders() {
        // Position writes x, stance, feet y, z, on-ground — the Java order.
        let position = PlayerPosition {
            x: 1.0,
            min_y: 3.62,
            y: 2.0,
            z: 4.0,
            on_ground: true,
        };
        let mut buf = Buf::new();
        position.write(&mut buf).unwrap();
        assert_eq!(PlayerPosition::read(&mut buf).unwrap(), position);
        assert_eq!(buf.remaining(), 0);

        let look = PlayerLook {
            yaw: 90.0,
            pitch: 45.0,
            on_ground: false,
        };
        let mut buf = Buf::new();
        look.write(&mut buf).unwrap();
        assert_eq!(PlayerLook::read(&mut buf).unwrap(), look);

        let both = PlayerPositionLook {
            x: 1.0,
            min_y: 3.62,
            y: 2.0,
            z: 4.0,
            yaw: 90.0,
            pitch: 45.0,
            on_ground: true,
        };
        let mut buf = Buf::new();
        both.write(&mut buf).unwrap();
        assert_eq!(PlayerPositionLook::read(&mut buf).unwrap(), both);
        assert_eq!(var_int_size(0), 1);
    }

    #[test]
    fn slot_packets_round_trip() {
        for item in [
            None,
            Some(Slot::new(1, 64, 0)),
            Some(Slot::with_nbt(278, 1, 0, vec![0x0a, 0x00])),
        ] {
            let set = SetSlot {
                window_id: 0,
                slot: 36,
                item: item.clone(),
            };
            let mut buf = Buf::new();
            set.write(&mut buf).unwrap();
            assert_eq!(SetSlot::read(&mut buf).unwrap(), set);

            let equip = EntityEquipment {
                entity_id: 7,
                slot: 0,
                item: item.clone(),
            };
            let mut buf = Buf::new();
            equip.write(&mut buf).unwrap();
            assert_eq!(EntityEquipment::read(&mut buf).unwrap(), equip);
        }
    }

    #[test]
    fn player_abilities_flags() {
        let abilities = PlayerAbilities {
            invulnerable: true,
            flying: false,
            allow_flying: true,
            creative_mode: true,
            fly_speed: 0.05,
            walk_speed: 0.1,
        };
        let mut buf = Buf::new();
        abilities.write(&mut buf).unwrap();
        assert_eq!(buf.as_slice()[0], 0b1101);
        assert_eq!(PlayerAbilities::read(&mut buf).unwrap(), abilities);
    }
}
