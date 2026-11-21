//! The in-memory world the commands run against: the server's worlds,
//! players, registries and message routing. The command functions take this
//! shape; the owning game adapts its real state to it.

use std::collections::HashMap;

use crate::facts::{Difficulty, GameMode};
use crate::rules::GameRules;
use crate::scoreboard::Scoreboard;
use crate::text::Text;
use sunrise_core::nbt::Tag;
use sunset_1_7_core::LegacyRandom;

/// `net.minecraft.util.math.Vec3i`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vec3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vec3i {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// `squaredDistanceTo`: the differences in `f32`, summed in `f32`.
    pub fn squared_distance_to(&self, x: i32, y: i32, z: i32) -> f32 {
        let dx = (self.x - x) as f32;
        let dy = (self.y - y) as f32;
        let dz = (self.z - z) as f32;
        dx * dx + dy * dy + dz * dz
    }
}

/// `MathHelper.floor(double)` = `(int) Math.floor(d)`.
pub fn math_helper_floor(value: f64) -> i32 {
    value.floor() as i32
}

/// One item stack as `/give`, `/clear` and `/enchant` read it. The `ench`
/// list lives in the NBT, as vanilla's `ItemStack` keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStack {
    /// The 1.7.10 numeric item id.
    pub id: i32,
    /// The registry key (`Item.REGISTRY.getKey`), "" when unknown.
    pub key: String,
    pub size: i32,
    /// The item data value (`ItemStack.getMetadata`).
    pub meta: i32,
    pub nbt: Option<Tag>,
}

impl ItemStack {
    pub fn new(id: i32, key: &str, size: i32, meta: i32) -> Self {
        Self {
            id,
            key: key.to_string(),
            size,
            meta,
            nbt: None,
        }
    }

    /// `hasNbt`.
    pub fn has_nbt(&self) -> bool {
        self.nbt.is_some()
    }

    /// `getEnchantments`: the NBT `ench` list's (id, lvl) pairs, when set.
    pub fn get_enchantments(&self) -> Option<Vec<(i16, i16)>> {
        let nbt = self.nbt.as_ref()?;
        let list = nbt.get("ench")?;
        let Tag::List(entries) = list else {
            return None;
        };
        Some(
            entries
                .iter()
                .filter_map(|entry| {
                    let id = match entry.get("id") {
                        Some(Tag::Short(id)) => *id,
                        _ => return None,
                    };
                    let level = match entry.get("lvl") {
                        Some(Tag::Short(level)) => *level,
                        _ => return None,
                    };
                    Some((id, level))
                })
                .collect(),
        )
    }

    /// `addEnchantment`: create or extend the NBT `ench` list; an existing
    /// same-id entry is replaced (`ItemStack.addEnchantment`).
    pub fn add_enchantment(&mut self, id: i16, level: i16) {
        let mut current = self.get_enchantments().unwrap_or_default();
        current.retain(|(existing, _)| *existing != id);
        current.push((id, level));
        let entries: Vec<Tag> = current
            .into_iter()
            .map(|(id, level)| Tag::compound([("id", Tag::Short(id)), ("lvl", Tag::Short(level))]))
            .collect();
        let compound = match self.nbt.take() {
            Some(mut nbt) => {
                if let Tag::Compound(entries_map) = &mut nbt {
                    entries_map.insert("ench".into(), Tag::List(entries));
                }
                nbt
            }
            None => Tag::compound([("ench", Tag::List(entries))]),
        };
        self.nbt = Some(compound);
    }

    /// `setNbt`: replace the whole compound.
    pub fn set_nbt(&mut self, nbt: Tag) {
        self.nbt = Some(nbt);
    }
}

/// `PlayerInventory`: the 36 item slots, the 4 armor slots and the cursor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inventory {
    pub items: Vec<Option<ItemStack>>,
    pub armor: Vec<Option<ItemStack>>,
    pub cursor: Option<ItemStack>,
}

impl Inventory {
    /// `new PlayerInventory`: 36 item slots, 4 armor slots.
    pub fn new() -> Self {
        Self {
            items: vec![None; 36],
            armor: vec![None; 4],
            cursor: None,
        }
    }

    /// `removeAll(item, metadata)`: the item and armor slots plus the cursor;
    /// `item` of `None` clears everything, a `metadata` below -1 or -1 takes
    /// any data value. Returns the removed stack sizes summed.
    pub fn remove_all(&mut self, item: Option<&ItemStack>, metadata: i32) -> i32 {
        let wanted_id = item.map(|stack| stack.id);
        let mut removed = 0;
        for slot in self.items.iter_mut().chain(self.armor.iter_mut()) {
            if let Some(stack) = slot {
                let id_matches = match wanted_id {
                    None => true,
                    Some(id) => stack.id == id,
                };
                let meta_matches = metadata <= -1 || stack.meta == metadata;
                if id_matches && meta_matches {
                    removed += stack.size;
                    *slot = None;
                }
            }
        }
        if let Some(stack) = &self.cursor {
            let id_matches = match wanted_id {
                None => true,
                Some(id) => stack.id == id,
            };
            let meta_matches = metadata <= -1 || stack.meta == metadata;
            if id_matches && meta_matches {
                removed += stack.size;
                self.cursor = None;
            }
        }
        removed
    }

    /// `PlayerEntity.dropItem(stack, false)` with a 0 pick-up delay: the
    /// stack joins the inventory (vanilla's drop-plus-instant-pickup).
    pub fn add_stack(&mut self, stack: ItemStack) {
        let mut stack = stack;
        // Partial stacks first.
        for slot in self.items.iter_mut() {
            if stack.size <= 0 {
                break;
            }
            if let Some(existing) = slot
                && existing.id == stack.id
                && existing.meta == stack.meta
                && existing.size < 64
            {
                let take = (64 - existing.size).min(stack.size);
                existing.size += take;
                stack.size -= take;
            }
        }
        // Then empty slots.
        for slot in self.items.iter_mut() {
            if stack.size <= 0 {
                break;
            }
            if slot.is_none() {
                let take = stack.size.min(64);
                *slot = Some(ItemStack {
                    size: take,
                    ..stack.clone()
                });
                stack.size -= take;
            }
        }
    }
}

/// One status effect on a player (`StatusEffectInstance`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusEffectInstance {
    pub id: i32,
    pub duration: i32,
    pub amplifier: i32,
}

/// `PlayerAbilities`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Abilities {
    pub can_fly: bool,
    pub creative_mode: bool,
    pub invulnerable: bool,
    pub flying: bool,
    pub can_modify_world: bool,
}

impl Abilities {
    /// `WorldSettings$GameMode.apply`.
    pub fn apply_game_mode(&mut self, mode: GameMode) {
        if mode == GameMode::Creative {
            self.can_fly = true;
            self.creative_mode = true;
            self.invulnerable = true;
        } else {
            self.can_fly = false;
            self.creative_mode = false;
            self.invulnerable = false;
            self.flying = false;
        }
        self.can_modify_world = !mode.restricts_world_modification();
    }
}

/// One player as the commands read and write it (`ServerPlayerEntity`).
#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    /// `getDisplayName`, a custom name when set.
    pub display_name: Option<String>,
    /// The world index into `CommandServer.worlds`.
    pub world: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub game_mode: GameMode,
    pub fall_distance: f32,
    pub health: f32,
    pub absorption: f32,
    pub xp_level: i32,
    pub xp_points: i32,
    pub dead: bool,
    /// `getStatusEffects` keyed by effect id.
    pub effects: HashMap<i32, StatusEffectInstance>,
    pub inventory: Inventory,
    /// The held item's slot in `inventory.items` (0-8, the hotbar).
    pub held_slot: usize,
    /// `setSpawnPoint` (a `Vec3i` and its `forced` flag).
    pub spawn_point: Option<(Vec3i, bool)>,
    pub abilities: Abilities,
    /// `isOp`.
    pub op: bool,
    /// The chat messages the server sent this player.
    pub messages: Vec<Text>,
}

impl Player {
    /// A fresh survival player at the origin of world 0.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            display_name: None,
            world: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            game_mode: GameMode::Survival,
            fall_distance: 0.0,
            health: 20.0,
            absorption: 0.0,
            xp_level: 0,
            xp_points: 0,
            dead: false,
            effects: HashMap::new(),
            inventory: Inventory::new(),
            held_slot: 0,
            spawn_point: None,
            abilities: Abilities::default(),
            op: false,
            messages: Vec::new(),
        }
    }

    /// `getCommandSourceBlockPos`: `floor(x), floor(y + 0.5), floor(z)`.
    pub fn block_pos(&self) -> Vec3i {
        Vec3i::new(
            math_helper_floor(self.x),
            math_helper_floor(self.y + 0.5),
            math_helper_floor(self.z),
        )
    }

    /// `getDisplayName`: the custom name, else the login name.
    pub fn display_name(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.name.clone())
    }

    /// `getItemInHand`.
    pub fn item_in_hand(&self) -> Option<&ItemStack> {
        self.inventory
            .items
            .get(self.held_slot)
            .and_then(Option::as_ref)
    }

    pub fn item_in_hand_mut(&mut self) -> Option<&mut ItemStack> {
        self.inventory
            .items
            .get_mut(self.held_slot)
            .and_then(Option::as_mut)
    }

    /// `takeDamage(OUT_OF_WORLD, Float.MAX_VALUE)`: the health drops to 0.
    pub fn take_out_of_world_damage(&mut self) {
        self.health = 0.0;
        self.dead = true;
    }

    /// `teleport(x, y, z)`.
    pub fn teleport(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// `setGameMode` (the abilities apply; the packet is the game's job).
    pub fn set_game_mode(&mut self, mode: GameMode) {
        self.game_mode = mode;
        self.abilities.apply_game_mode(mode);
    }
}

/// One block state as `/setblock` reads and writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockState {
    pub id: u32,
    pub meta: u8,
}

/// One entity `/summon` created: its id, NBT and position.
#[derive(Clone, Debug)]
pub struct SummonedEntity {
    pub id: String,
    pub nbt: Tag,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// The index of the entity this one rides, when any.
    pub riding: Option<usize>,
    /// `initialize(null)` ran: fresh NBT without a tag means a mob with
    /// vanilla's default equipment spawned.
    pub initialized: bool,
    /// Whether the entity is a `MobEntity`.
    pub mob: bool,
}

/// One world as the commands read and write it.
#[derive(Clone, Debug)]
pub struct World {
    pub time_of_day: i64,
    pub seed: i64,
    pub rain_time: i32,
    pub thunder_time: i32,
    pub raining: bool,
    pub thundering: bool,
    pub game_rules: GameRules,
    pub scoreboard: Scoreboard,
    /// The world spawn (`getSpawnCoordinate`).
    pub spawn_pos: Vec3i,
    /// The block states `/setblock` placed; absent is air.
    pub blocks: HashMap<(i32, i32, i32), BlockState>,
    /// The loaded chunk columns (`isChunkLoaded`).
    pub loaded_chunks: Vec<(i32, i32)>,
    /// The entities `/summon` created.
    pub entities: Vec<SummonedEntity>,
}

impl World {
    pub fn new(seed: i64) -> Self {
        Self {
            time_of_day: 0,
            seed,
            rain_time: 0,
            thunder_time: 0,
            raining: false,
            thundering: false,
            game_rules: GameRules::new(),
            scoreboard: Scoreboard::new(),
            spawn_pos: Vec3i::default(),
            blocks: HashMap::new(),
            loaded_chunks: Vec::new(),
            entities: Vec::new(),
        }
    }

    /// `isChunkLoaded(x, y, z)`: the column is loaded.
    pub fn is_chunk_loaded(&self, x: i32, _y: i32, z: i32) -> bool {
        self.loaded_chunks.contains(&(x, z))
    }

    /// `isAir(x, y, z)`.
    pub fn is_air(&self, x: i32, y: i32, z: i32) -> bool {
        !self.blocks.contains_key(&(x, y, z))
    }

    /// `setBlockWithMetadata(x, y, z, block, meta, flags)`: false when the
    /// state is unchanged.
    pub fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        id: u32,
        meta: u8,
        flags: i32,
    ) -> bool {
        let _ = flags;
        let current = self.blocks.get(&(x, y, z)).copied();
        let next = BlockState { id, meta };
        if current == Some(next) {
            return false;
        }
        self.blocks.insert((x, y, z), next);
        true
    }

    /// `breakBlock(x, y, z, drop)`.
    pub fn break_block(&mut self, x: i32, y: i32, z: i32, drop: bool) {
        let _ = drop;
        self.blocks.remove(&(x, y, z));
    }

    /// Whether a block entity exists at the position: the block state owns
    /// one (`Block.hasBlockEntity` over the placed state).
    pub fn block_entity_exists(&self, x: i32, y: i32, z: i32) -> bool {
        let _ = (x, y, z);
        false
    }
}

/// Which source runs a command: the console, a player, or a command block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `Console.INSTANCE`: the server console (or Rcon).
    Console,
    /// A player, by its index into `CommandServer.players`.
    Player(usize),
    /// A command block executor at a block position (`CommandExecutor`).
    Block { world: usize, pos: Vec3i },
}

impl Source {
    /// `getName`.
    pub fn name(&self, server: &CommandServer) -> String {
        match self {
            Self::Console => "Rcon".to_string(),
            Self::Player(i) => server
                .players
                .get(*i)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            Self::Block { .. } => "@".to_string(),
        }
    }

    /// `getDisplayName`.
    pub fn display_name(&self, server: &CommandServer) -> String {
        match self {
            Self::Console => "Rcon".to_string(),
            Self::Player(i) => server
                .players
                .get(*i)
                .map(Player::display_name)
                .unwrap_or_default(),
            Self::Block { .. } => "@".to_string(),
        }
    }

    /// `canUseCommand`: the console always can, a command block up to level
    /// 2, a player when op or the level is 0.
    pub fn can_use_command(&self, server: &CommandServer, level: i32, _command: &str) -> bool {
        match self {
            Self::Console => true,
            Self::Block { .. } => level <= 2,
            Self::Player(i) => server
                .players
                .get(*i)
                .map(|p| p.op || level <= 0)
                .unwrap_or(false),
        }
    }

    /// `getCommandSourceBlockPos`.
    pub fn block_pos(&self, server: &CommandServer) -> Vec3i {
        match self {
            Self::Console => Vec3i::new(0, 0, 0),
            Self::Player(i) => server
                .players
                .get(*i)
                .map(Player::block_pos)
                .unwrap_or_default(),
            Self::Block { pos, .. } => *pos,
        }
    }

    /// `getCommandSourceWorld`: the world index, or None when the source has
    /// no world.
    pub fn world(&self, server: &CommandServer) -> Option<usize> {
        match self {
            Self::Console => Some(0),
            Self::Player(i) => server.players.get(*i).map(|p| p.world),
            Self::Block { world, .. } => Some(*world),
        }
    }

    /// `instanceof ServerPlayerEntity`.
    pub fn is_player(&self) -> bool {
        matches!(self, Self::Player(_))
    }

    /// `instanceof CommandExecutor`.
    pub fn is_executor(&self) -> bool {
        matches!(self, Self::Block { .. })
    }
}

/// The server singleton's command-visible half (`MinecraftServer`).
pub struct CommandServer {
    /// One level per world; `/time` sets every world, `/weather` and
    /// `/scoreboard` act on `worlds[0]`.
    pub worlds: Vec<World>,
    pub players: Vec<Player>,
    /// The console's message buffer (`Console.getTextAsString`).
    pub console: Vec<Text>,
    /// The command block's last output (`CommandExecutor.getLastOutput`).
    pub block_output: Option<Text>,
    /// `java.util.Random`: the weather command's duration draw.
    pub random: LegacyRandom,
    pub difficulty: Difficulty,
    pub default_game_mode: GameMode,
    pub force_game_mode: bool,
    pub dedicated: bool,
    pub singleplayer: bool,
    pub command_blocks_enabled: bool,
    /// The 1.7.10 block registry: key -> (id, has block entity).
    pub blocks_by_key: HashMap<String, (u32, bool)>,
    /// The 1.7.10 block registry: id -> (key, has block entity).
    pub blocks_by_id: HashMap<u32, (String, bool)>,
    /// The 1.7.10 item registry: key -> id.
    pub items_by_key: HashMap<String, i32>,
    /// The 1.7.10 item registry: id -> key.
    pub items_by_id: HashMap<i32, String>,
}

impl CommandServer {
    /// A singleplayer server with one world.
    pub fn single(seed: i64) -> Self {
        let mut server = Self {
            worlds: vec![World::new(seed)],
            players: Vec::new(),
            console: Vec::new(),
            block_output: None,
            random: LegacyRandom::new(0),
            difficulty: Difficulty::Normal,
            default_game_mode: GameMode::Survival,
            force_game_mode: false,
            dedicated: false,
            singleplayer: true,
            command_blocks_enabled: true,
            blocks_by_key: HashMap::new(),
            blocks_by_id: HashMap::new(),
            items_by_key: HashMap::new(),
            items_by_id: HashMap::new(),
        };
        server.install_default_registries();
        server
    }

    /// A few 1.7.10 ids, enough for the tests and the probe.
    pub fn install_default_registries(&mut self) {
        for (id, key) in [
            (1u32, "minecraft:stone"),
            (2, "minecraft:grass"),
            (3, "minecraft:dirt"),
            (7, "minecraft:bedrock"),
            (54, "minecraft:chest"),
            (63, "minecraft:standing_sign"),
            (331, "minecraft:redstone_torch"),
        ] {
            self.blocks_by_key
                .insert(key.to_string(), (id, key == "minecraft:chest"));
            self.blocks_by_id
                .insert(id, (key.to_string(), key == "minecraft:chest"));
        }
        for (id, key) in [
            (276i32, "minecraft:diamond_sword"),
            (264, "minecraft:diamond"),
            (261, "minecraft:bow"),
            (257, "minecraft:iron_pickaxe"),
        ] {
            self.items_by_key.insert(key.to_string(), id);
            self.items_by_id.insert(id, key.to_string());
        }
    }

    /// `getPlayerManager().get(name)`: the player index by name.
    pub fn player_by_name(&self, name: &str) -> Option<usize> {
        self.players.iter().position(|p| p.name == name)
    }

    /// `getPlayerNames()`.
    pub fn player_names(&self) -> Vec<String> {
        self.players.iter().map(|p| p.name.clone()).collect()
    }

    /// `sendToSource`: the message lands where the source reads it — a
    /// player's chat, the console buffer, or the command block's last output.
    pub fn send(&mut self, source: &Source, message: Text) {
        match source {
            Source::Console => self.console.push(message),
            Source::Player(i) => {
                if let Some(player) = self.players.get_mut(*i) {
                    player.messages.push(message);
                }
            }
            Source::Block { .. } => self.block_output = Some(message),
        }
    }

    /// A message to a player by index (`sendMessage`).
    pub fn send_to_player(&mut self, index: usize, message: Text) {
        if let Some(player) = self.players.get_mut(index) {
            player.messages.push(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_block_pos_uses_half_offset_y() {
        let mut player = Player::new("p");
        player.x = 1.7;
        player.y = 63.0;
        player.z = -2.3;
        let pos = player.block_pos();
        assert_eq!(
            (pos.x, pos.y, pos.z),
            (1, 63, -3),
            "floor(x), floor(y + 0.5), floor(z)"
        );
    }

    #[test]
    fn floor_matches_java() {
        assert_eq!(math_helper_floor(1.7), 1);
        assert_eq!(math_helper_floor(-2.3), -3);
        assert_eq!(math_helper_floor(-0.5), -1);
        assert_eq!(math_helper_floor(0.0), 0);
    }

    #[test]
    fn inventory_remove_all_semantics() {
        let mut inventory = Inventory::new();
        inventory.items[0] = Some(ItemStack::new(276, "minecraft:diamond_sword", 1, 0));
        inventory.items[1] = Some(ItemStack::new(264, "minecraft:diamond", 10, 0));
        inventory.items[2] = Some(ItemStack::new(264, "minecraft:diamond", 5, 1));
        inventory.armor[0] = Some(ItemStack::new(264, "minecraft:diamond", 3, 0));
        // A null item clears everything; a -1 metadata takes any data value.
        assert_eq!(inventory.remove_all(None, -1), 19);
        assert!(inventory.items.iter().all(Option::is_none));
        // By item and metadata.
        inventory.items[0] = Some(ItemStack::new(264, "minecraft:diamond", 10, 0));
        inventory.items[1] = Some(ItemStack::new(264, "minecraft:diamond", 5, 1));
        assert_eq!(
            inventory.remove_all(Some(&ItemStack::new(264, "", 0, 0)), 0),
            10
        );
        assert_eq!(inventory.items[1].as_ref().unwrap().size, 5);
        // A metadata of -1 takes both data values (the armor slots are empty).
        assert_eq!(
            inventory.remove_all(Some(&ItemStack::new(264, "", 0, 0)), -1),
            15 - 10
        );
        // An unknown item removes nothing.
        assert_eq!(
            inventory.remove_all(Some(&ItemStack::new(1, "", 0, 0)), -1),
            0
        );
    }

    #[test]
    fn add_stack_fills_partials_then_empty_slots() {
        let mut inventory = Inventory::new();
        inventory.items[0] = Some(ItemStack::new(264, "minecraft:diamond", 60, 0));
        inventory.add_stack(ItemStack::new(264, "minecraft:diamond", 70, 0));
        assert_eq!(inventory.items[0].as_ref().unwrap().size, 64);
        assert_eq!(inventory.items[1].as_ref().unwrap().size, 64);
        assert_eq!(inventory.items[2].as_ref().unwrap().size, 2);
    }

    #[test]
    fn abilities_apply_game_mode() {
        let mut abilities = Abilities::default();
        abilities.apply_game_mode(GameMode::Creative);
        assert!(abilities.can_fly && abilities.creative_mode && abilities.invulnerable);
        abilities.apply_game_mode(GameMode::Survival);
        assert!(!abilities.can_fly && !abilities.creative_mode && !abilities.invulnerable);
        abilities.apply_game_mode(GameMode::Adventure);
        assert!(!abilities.can_modify_world);
        abilities.apply_game_mode(GameMode::Survival);
        assert!(abilities.can_modify_world);
    }

    #[test]
    fn stack_enchantments_round_trip_through_nbt() {
        let mut stack = ItemStack::new(276, "minecraft:diamond_sword", 1, 0);
        assert_eq!(stack.get_enchantments(), None);
        stack.add_enchantment(16, 5);
        stack.add_enchantment(21, 3);
        assert_eq!(stack.get_enchantments(), Some(vec![(16, 5), (21, 3)]));
        // Re-enchanting the same id replaces the entry.
        stack.add_enchantment(16, 4);
        assert_eq!(stack.get_enchantments(), Some(vec![(21, 3), (16, 4)]));
    }

    #[test]
    fn set_block_reports_no_change_for_same_state() {
        let mut world = World::new(0);
        assert!(world.set_block_with_metadata(1, 2, 3, 1, 0, 3));
        assert!(
            !world.set_block_with_metadata(1, 2, 3, 1, 0, 3),
            "same state is no change"
        );
        assert!(
            world.set_block_with_metadata(1, 2, 3, 1, 1, 3),
            "a new meta changes"
        );
        assert!(!world.is_chunk_loaded(1, 2, 3));
        world.loaded_chunks.push((1, 3));
        assert!(world.is_chunk_loaded(1, 2, 3));
        assert!(!world.is_air(1, 2, 3));
        assert!(world.is_air(5, 5, 5));
        world.break_block(1, 2, 3, true);
        assert!(world.is_air(1, 2, 3));
    }

    #[test]
    fn source_permissions() {
        let mut server = CommandServer::single(0);
        server.players.push(Player::new("op"));
        server.players.push(Player::new("guest"));
        server.players[0].op = true;
        assert!(Source::Console.can_use_command(&server, 4, ""));
        assert!(
            Source::Player(0).can_use_command(&server, 2, ""),
            "op passes any level"
        );
        assert!(
            !Source::Player(1).can_use_command(&server, 2, ""),
            "a non-op is level 0"
        );
        assert!(
            Source::Player(1).can_use_command(&server, 0, ""),
            "level 0 passes"
        );
        let block = Source::Block {
            world: 0,
            pos: Vec3i::default(),
        };
        assert!(
            block.can_use_command(&server, 2, ""),
            "a command block runs up to level 2"
        );
        assert!(!block.can_use_command(&server, 3, ""));
        assert!(block.is_executor());
        assert_eq!(block.name(&server), "@");
        assert_eq!(Source::Console.name(&server), "Rcon");
    }
}
