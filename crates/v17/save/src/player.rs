//! `playerdata/<uuid>.dat`: one gzip member of an NBT compound holding the
//! player, with the field set 1.7.10's `PlayerEntity` writes — its own
//! fields, the living and shared entity ones under them, all in
//! `writeNbtWithoutId`'s order. Containers the gameplay crates own (the
//! inventory's stacks, attribute modifiers, status effects, the ender chest)
//! are carried as raw NBT: this module owns the layout, not their content.
//!
//! Reading reproduces the read chain's coercions: velocity over 10 a side is
//! zeroed (`readNbt`'s clamp), `HealF` is the health when present with
//! `Health` as its fallback, `SpawnX`/`SpawnY`/`SpawnZ` only count when all
//! three are there, hunger reads only when `foodLevel` is, and abilities'
//! `flySpeed`/`walkSpeed` update together.

use std::{fs, io, io::Write, path::Path};
use sunrise_core::nbt::{self, Compound, JavaString, Tag};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// `PlayerAbilities`: the five flags and the two speeds, with the Java
/// class's defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Abilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub may_fly: bool,
    pub instabuild: bool,
    pub may_build: bool,
    pub fly_speed: f32,
    pub walk_speed: f32,
}
impl Default for Abilities {
    fn default() -> Self {
        Self {
            invulnerable: false,
            flying: false,
            may_fly: false,
            instabuild: false,
            may_build: true,
            fly_speed: 0.05,
            walk_speed: 0.1,
        }
    }
}
impl Abilities {
    /// `PlayerAbilities.readNbt`: the flags read directly, the speeds update
    /// together when `flySpeed` is there, and `mayBuild` reads only when
    /// present.
    /// `PlayerAbilities.readNbt`, field by field, as Java reads into `this`.
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_nbt(player: &Compound) -> Self {
        let mut abilities = Self::default();
        let Some(Tag::Compound(nbt)) = player.get(&JavaString::from("abilities")) else {
            return abilities;
        };
        let get = |key: &str| nbt.get(&JavaString::from(key));
        if let Some(tag) = get("invulnerable") {
            abilities.invulnerable = bool_of(tag);
        }
        if let Some(tag) = get("flying") {
            abilities.flying = bool_of(tag);
        }
        if let Some(tag) = get("mayfly") {
            abilities.may_fly = bool_of(tag);
        }
        if let Some(tag) = get("instabuild") {
            abilities.instabuild = bool_of(tag);
        }
        if contains_typed(nbt, "flySpeed", 99) {
            abilities.fly_speed = float_of(get("flySpeed"));
            abilities.walk_speed = float_of(get("walkSpeed"));
        }
        if let Some(tag) = get("mayBuild") {
            abilities.may_build = bool_of(tag);
        }
        abilities
    }

    /// `PlayerAbilities.writeNbt`: the abilities compound.
    pub fn to_nbt(&self) -> Compound {
        let mut nbt = Compound::default();
        let mut put = |key: &str, value: Tag| nbt.insert(JavaString::from(key), value);
        put("invulnerable", Tag::Byte(self.invulnerable as i8));
        put("flying", Tag::Byte(self.flying as i8));
        put("mayfly", Tag::Byte(self.may_fly as i8));
        put("instabuild", Tag::Byte(self.instabuild as i8));
        put("mayBuild", Tag::Byte(self.may_build as i8));
        put("flySpeed", Tag::Float(self.fly_speed.to_bits()));
        put("walkSpeed", Tag::Float(self.walk_speed.to_bits()));
        nbt
    }
}

fn bool_of(tag: &Tag) -> bool {
    long_of(tag) != 0
}
fn float_of(tag: Option<&Tag>) -> f32 {
    match tag {
        Some(Tag::Float(v)) => f32::from_bits(*v),
        Some(Tag::Double(v)) => f64::from_bits(*v) as f32,
        Some(tag) => long_of(tag) as f32,
        None => 0.0,
    }
}
fn long_of(tag: &Tag) -> i64 {
    match tag {
        Tag::Byte(v) => i64::from(*v),
        Tag::Short(v) => i64::from(*v),
        Tag::Int(v) => i64::from(*v),
        Tag::Long(v) => *v,
        Tag::Float(v) => f32::from_bits(*v) as i64,
        Tag::Double(v) => f64::from_bits(*v) as i64,
        _ => 0,
    }
}

fn get_long(compound: &Compound, key: &str) -> i64 {
    compound.get(&JavaString::from(key)).map_or(0, long_of)
}
fn get_int(compound: &Compound, key: &str) -> i32 {
    get_long(compound, key) as i32
}
fn get_short(compound: &Compound, key: &str) -> i16 {
    get_long(compound, key) as i16
}
fn get_float(compound: &Compound, key: &str) -> f32 {
    match compound.get(&JavaString::from(key)) {
        Some(Tag::Float(v)) => f32::from_bits(*v),
        Some(Tag::Double(v)) => f64::from_bits(*v) as f32,
        Some(tag) => long_of(tag) as f32,
        None => 0.0,
    }
}
fn get_boolean(compound: &Compound, key: &str) -> bool {
    get_long(compound, key) != 0
}
fn contains_typed(compound: &Compound, key: &str, ty: u8) -> bool {
    let found = compound
        .get(&JavaString::from(key))
        .map_or(0, |tag| match tag {
            Tag::End => 0,
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(_) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        });
    found == ty || (ty == 99 && (1..=6).contains(&found))
}
/// A list of NBT values, typed 9 on disk, as the compound keeps them.
fn list_of(compound: &Compound, key: &str) -> Vec<Tag> {
    match compound.get(&JavaString::from(key)) {
        Some(Tag::List(items)) => items.clone(),
        _ => Vec::new(),
    }
}
fn doubles_of(compound: &Compound, key: &str) -> [f64; 3] {
    let get = |i: usize| match compound.get(&JavaString::from(key)) {
        Some(Tag::List(items)) => match items.get(i) {
            Some(Tag::Double(v)) => f64::from_bits(*v),
            Some(Tag::Float(v)) => f32::from_bits(*v) as f64,
            Some(tag) => long_of(tag) as f64,
            None => 0.0,
        },
        _ => 0.0,
    };
    [get(0), get(1), get(2)]
}
fn floats_of(compound: &Compound, key: &str) -> [f32; 2] {
    let get = |i: usize| match compound.get(&JavaString::from(key)) {
        Some(Tag::List(items)) => match items.get(i) {
            Some(Tag::Float(v)) => f32::from_bits(*v),
            Some(Tag::Double(v)) => f64::from_bits(*v) as f32,
            Some(tag) => long_of(tag) as f32,
            None => 0.0,
        },
        _ => 0.0,
    };
    [get(0), get(1)]
}

/// The player: 1.7.10's `PlayerEntity.writeNbtWithoutId` field set. Position,
/// motion and rotation are the raw values the NBT carries; what the game
/// computes before saving (the sneaking eye-height offset on `Pos` y, for
/// one) is the caller's.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerData {
    /// `Pos`, doubles.
    pub pos: [f64; 3],
    /// `Motion`, doubles; loading zeroes a component over 10 in magnitude.
    pub motion: [f64; 3],
    /// `Rotation`, yaw then pitch, floats.
    pub rotation: [f32; 2],
    pub fall_distance: f32,
    pub fire: i16,
    /// `Air`, the breath ticks left.
    pub air: i16,
    pub on_ground: bool,
    pub dimension: i32,
    pub invulnerable: bool,
    pub portal_cooldown: i32,
    /// `UUIDMost` / `UUIDLeast`.
    pub uuid: [i64; 2],
    /// `HealF` on write (the health as a float) and the read chain's
    /// effective health: `HealF` when present, else `Health`, else 20.
    pub health: f32,
    pub hurt_time: i16,
    pub death_time: i16,
    pub attack_time: i16,
    /// `AbsorptionAmount`.
    pub absorption: f32,
    /// `Attributes`, raw: `EntityAttributes.toNbt`'s compounds.
    pub attributes: Vec<Tag>,
    /// `ActiveEffects`, raw: `StatusEffectInstance.toNbt`'s compounds.
    pub active_effects: Vec<Tag>,
    /// `Inventory`, raw: stack compounds with `Slot` (0..36 main, 100..104
    /// armour), `id`, `Count`, `Damage` and an optional `tag`.
    pub inventory: Vec<Tag>,
    pub selected_item_slot: i32,
    pub sleeping: bool,
    pub sleep_timer: i16,
    /// `XpP`, the experience progress.
    pub xp_progress: f32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub score: i32,
    /// `SpawnX`/`SpawnY`/`SpawnZ`, the bed spawn, when all three are there.
    pub spawn: Option<[i32; 3]>,
    pub spawn_forced: bool,
    pub food_level: i32,
    pub food_tick_timer: i32,
    pub food_saturation: f32,
    pub food_exhaustion: f32,
    pub abilities: Abilities,
    /// `EnderItems`, raw: the ender chest's stack compounds.
    pub ender_items: Vec<Tag>,
}

impl Default for PlayerData {
    /// A fresh player: vanilla's field defaults where the write always puts
    /// a tag (Fire -1, Air 300), and the living defaults (`Health` 20).
    fn default() -> Self {
        Self {
            pos: [0.0; 3],
            motion: [0.0; 3],
            rotation: [0.0; 2],
            fall_distance: 0.0,
            fire: -1,
            air: 300,
            on_ground: false,
            dimension: 0,
            invulnerable: false,
            portal_cooldown: 0,
            uuid: [0, 0],
            health: 20.0,
            hurt_time: 0,
            death_time: 0,
            attack_time: 0,
            absorption: 0.0,
            attributes: Vec::new(),
            active_effects: Vec::new(),
            inventory: Vec::new(),
            selected_item_slot: 0,
            sleeping: false,
            sleep_timer: 0,
            xp_progress: 0.0,
            xp_level: 0,
            xp_total: 0,
            score: 0,
            spawn: None,
            spawn_forced: false,
            food_level: 20,
            food_tick_timer: 0,
            food_saturation: 5.0,
            food_exhaustion: 0.0,
            abilities: Abilities::default(),
            ender_items: Vec::new(),
        }
    }
}

impl PlayerData {
    /// The read chain: `PlayerEntity.readNbt` over `LivingEntity.readNbt`
    /// over `Entity.readNbt`, with each class's coercions and defaults.
    /// Field by field, as the Java chain reads into `this` one by one.
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_nbt(player: &Compound) -> Self {
        let mut data = Self::default();
        // Entity.readNbt: Motion first, with the 10-a-side clamp, then Pos.
        data.motion = doubles_of(player, "Motion");
        for velocity in &mut data.motion {
            if velocity.abs() > 10.0 {
                *velocity = 0.0;
            }
        }
        data.pos = doubles_of(player, "Pos");
        data.rotation = floats_of(player, "Rotation");
        data.fall_distance = get_float(player, "FallDistance");
        data.fire = get_short(player, "Fire");
        data.air = get_short(player, "Air");
        data.on_ground = get_boolean(player, "OnGround");
        data.dimension = get_int(player, "Dimension");
        data.invulnerable = get_boolean(player, "Invulnerable");
        data.portal_cooldown = get_int(player, "PortalCooldown");
        if contains_typed(player, "UUIDMost", 4) && contains_typed(player, "UUIDLeast", 4) {
            data.uuid = [get_long(player, "UUIDMost"), get_long(player, "UUIDLeast")];
        }
        // LivingEntity.readNbt: absorption, attributes, effects, health.
        data.absorption = get_float(player, "AbsorptionAmount");
        data.attributes = list_of(player, "Attributes");
        data.active_effects = list_of(player, "ActiveEffects");
        data.health = if contains_typed(player, "HealF", 99) {
            get_float(player, "HealF")
        } else {
            match player.get(&JavaString::from("Health")) {
                Some(Tag::Float(v)) => f32::from_bits(*v),
                Some(Tag::Double(v)) => f64::from_bits(*v) as f32,
                Some(Tag::Short(v)) => f32::from(*v),
                Some(tag) => long_of(tag) as f32,
                None => 20.0, // getMaxHealth: a player's 20.
            }
        };
        data.hurt_time = get_short(player, "HurtTime");
        data.death_time = get_short(player, "DeathTime");
        data.attack_time = get_short(player, "AttackTime");
        // PlayerEntity.readCustomNbt.
        data.inventory = list_of(player, "Inventory");
        data.selected_item_slot = get_int(player, "SelectedItemSlot");
        data.sleeping = get_boolean(player, "Sleeping");
        data.sleep_timer = get_short(player, "SleepTimer");
        data.xp_progress = get_float(player, "XpP");
        data.xp_level = get_int(player, "XpLevel");
        data.xp_total = get_int(player, "XpTotal");
        data.score = get_int(player, "Score");
        if contains_typed(player, "SpawnX", 99)
            && contains_typed(player, "SpawnY", 99)
            && contains_typed(player, "SpawnZ", 99)
        {
            data.spawn = Some([
                get_int(player, "SpawnX"),
                get_int(player, "SpawnY"),
                get_int(player, "SpawnZ"),
            ]);
            data.spawn_forced = get_boolean(player, "SpawnForced");
        }
        // HungerManager.readNbt: only when foodLevel is there.
        if contains_typed(player, "foodLevel", 99) {
            data.food_level = get_int(player, "foodLevel");
            data.food_tick_timer = get_int(player, "foodTickTimer");
            data.food_saturation = get_float(player, "foodSaturationLevel");
            data.food_exhaustion = get_float(player, "foodExhaustionLevel");
        }
        data.abilities = Abilities::from_nbt(player);
        data.ender_items = list_of(player, "EnderItems");
        data
    }

    /// `writeNbtWithoutId` over `writeCustomNbt` over `PlayerEntity`'s: every
    /// tag in the write order, containers always written.
    pub fn to_nbt(&self) -> Compound {
        let mut nbt = Compound::default();
        let mut put = |key: &str, value: Tag| nbt.insert(JavaString::from(key), value);
        // Entity.writeNbtWithoutId.
        put("Pos", list_of_doubles(&self.pos));
        put("Motion", list_of_doubles(&self.motion));
        put(
            "Rotation",
            Tag::List(
                self.rotation
                    .iter()
                    .map(|v| Tag::Float(v.to_bits()))
                    .collect(),
            ),
        );
        put("FallDistance", Tag::Float(self.fall_distance.to_bits()));
        put("Fire", Tag::Short(self.fire));
        put("Air", Tag::Short(self.air));
        put("OnGround", Tag::Byte(self.on_ground as i8));
        put("Dimension", Tag::Int(self.dimension));
        put("Invulnerable", Tag::Byte(self.invulnerable as i8));
        put("PortalCooldown", Tag::Int(self.portal_cooldown));
        put("UUIDMost", Tag::Long(self.uuid[0]));
        put("UUIDLeast", Tag::Long(self.uuid[1]));
        // LivingEntity.writeCustomNbt: HealF and the short Health (the ceil),
        // hurt, death and attack timers, absorption, attributes, effects.
        put("HealF", Tag::Float(self.health.to_bits()));
        put("Health", Tag::Short(self.health.ceil() as i16));
        put("HurtTime", Tag::Short(self.hurt_time));
        put("DeathTime", Tag::Short(self.death_time));
        put("AttackTime", Tag::Short(self.attack_time));
        put("AbsorptionAmount", Tag::Float(self.absorption.to_bits()));
        put("Attributes", Tag::List(self.attributes.clone()));
        if !self.active_effects.is_empty() {
            put("ActiveEffects", Tag::List(self.active_effects.clone()));
        }
        // PlayerEntity.writeCustomNbt.
        put("Inventory", Tag::List(self.inventory.clone()));
        put("SelectedItemSlot", Tag::Int(self.selected_item_slot));
        put("Sleeping", Tag::Byte(self.sleeping as i8));
        put("SleepTimer", Tag::Short(self.sleep_timer));
        put("XpP", Tag::Float(self.xp_progress.to_bits()));
        put("XpLevel", Tag::Int(self.xp_level));
        put("XpTotal", Tag::Int(self.xp_total));
        put("Score", Tag::Int(self.score));
        if let Some(spawn) = &self.spawn {
            put("SpawnX", Tag::Int(spawn[0]));
            put("SpawnY", Tag::Int(spawn[1]));
            put("SpawnZ", Tag::Int(spawn[2]));
            put("SpawnForced", Tag::Byte(self.spawn_forced as i8));
        }
        put("foodLevel", Tag::Int(self.food_level));
        put("foodTickTimer", Tag::Int(self.food_tick_timer));
        put(
            "foodSaturationLevel",
            Tag::Float(self.food_saturation.to_bits()),
        );
        put(
            "foodExhaustionLevel",
            Tag::Float(self.food_exhaustion.to_bits()),
        );
        put("abilities", Tag::Compound(self.abilities.to_nbt()));
        put("EnderItems", Tag::List(self.ender_items.clone()));
        nbt
    }
}

fn list_of_doubles(values: &[f64; 3]) -> Tag {
    Tag::List(values.iter().map(|v| Tag::Double(v.to_bits())).collect())
}

/// A player file's root, as `writeNbtWithoutId` + `NbtIo.writeCompressed`
/// produce them.
pub fn write_bytes(data: &PlayerData) -> io::Result<Vec<u8>> {
    let bytes = nbt::write_named(
        &nbt::NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(data.to_nbt()),
        },
        nbt::Limits {
            max_depth: nbt::DISK_WRITE_NESTING_GUARD,
            ..nbt::Limits::saved()
        },
    )
    .map_err(|e| invalid(e.to_string()))?;
    Ok(crate::java_zip::gzip(&bytes))
}

/// `NbtIo.readCompressed`, from bytes, as the player compound.
pub fn read_bytes(bytes: &[u8]) -> io::Result<PlayerData> {
    let raw = crate::java_zip::gunzip(bytes)?;
    let (named, _) =
        nbt::read_named(&raw, nbt::Limits::saved()).map_err(|e| invalid(e.to_string()))?;
    match named.tag {
        Tag::Compound(player) => Ok(PlayerData::from_nbt(&player)),
        _ => Err(invalid("player.dat root is not a compound")),
    }
}

/// `AlphaWorldStorage.savePlayerData`: write `<uuid>.dat.tmp`, then rename it
/// over `<uuid>.dat`.
pub fn save(world_dir: &Path, uuid: &str, data: &PlayerData) -> io::Result<()> {
    let dir = world_dir.join("playerdata");
    fs::create_dir_all(&dir)?;
    let bytes = write_bytes(data)?;
    let temporary = dir.join(format!("{uuid}.dat.tmp"));
    let file = dir.join(format!("{uuid}.dat"));
    {
        let mut handle = fs::File::create(&temporary)?;
        handle.write_all(&bytes)?;
        handle.sync_all()?;
    }
    if file.exists() {
        fs::remove_file(&file)?;
    }
    fs::rename(&temporary, &file)
}

/// `AlphaWorldStorage.loadPlayerData`: the player compound from
/// `<uuid>.dat`, `None` for a player with no file.
pub fn load(world_dir: &Path, uuid: &str) -> io::Result<Option<PlayerData>> {
    let path = world_dir.join("playerdata").join(format!("{uuid}.dat"));
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(read_bytes(&fs::read(path)?)?))
}

/// A `Compound` from a `Tag::compound` value, for the tests.
#[cfg(test)]
fn as_compound(tag: Tag) -> Compound {
    match tag {
        Tag::Compound(compound) => compound,
        _ => panic!("not a compound"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: PlayerData) -> PlayerData {
        let bytes = write_bytes(&data).unwrap();
        read_bytes(&bytes).unwrap()
    }

    #[test]
    fn player_dat_roundtrips() {
        let mut stack = Compound::default();
        stack.insert(JavaString::from("id"), Tag::Short(278));
        stack.insert(JavaString::from("Count"), Tag::Byte(1));
        stack.insert(JavaString::from("Damage"), Tag::Short(0));
        let mut slot = Compound::default();
        slot.insert(JavaString::from("Slot"), Tag::Byte(0));
        for (key, value) in stack.iter() {
            slot.insert(key.clone(), value.clone());
        }
        let mut effect = Compound::default();
        effect.insert(JavaString::from("id"), Tag::Byte(1));
        effect.insert(JavaString::from("Amplifier"), Tag::Byte(0));
        effect.insert(JavaString::from("Duration"), Tag::Int(1200));
        let data = PlayerData {
            pos: [-5.5, 64.0, 12.25],
            motion: [0.0, -0.0784, 0.0],
            rotation: [190.5, -12.0],
            fall_distance: 3.5,
            fire: -1,
            air: 300,
            on_ground: true,
            dimension: 0,
            invulnerable: false,
            portal_cooldown: 40,
            uuid: [-6140401986334597049, -6899682100637659250],
            health: 13.0,
            hurt_time: 3,
            death_time: 0,
            attack_time: 0,
            absorption: 0.0,
            attributes: vec![Tag::Compound(Compound::from([(
                JavaString::from("Name"),
                Tag::String(JavaString::from("generic.maxHealth")),
            )]))],
            active_effects: vec![Tag::Compound(effect)],
            inventory: vec![Tag::Compound(slot)],
            selected_item_slot: 0,
            sleeping: false,
            sleep_timer: 0,
            xp_progress: 0.35,
            xp_level: 12,
            xp_total: 240,
            score: 355,
            spawn: Some([-16, 65, 240]),
            spawn_forced: false,
            food_level: 17,
            food_tick_timer: 0,
            food_saturation: 3.0,
            food_exhaustion: 0.4,
            abilities: Abilities {
                may_fly: false,
                ..Abilities::default()
            },
            ender_items: Vec::new(),
        };
        let read = roundtrip(data.clone());
        assert_eq!(read, data, "every field roundtrips through gzip NBT");
    }

    #[test]
    fn reading_matches_the_chain_coercions() {
        // Motion over 10 a side zeroes, Health without HealF, the short read.
        let player = Tag::compound([
            (
                "Motion",
                Tag::List(vec![
                    Tag::Double((-25.0f64).to_bits()),
                    Tag::Double(0f64.to_bits()),
                    Tag::Double(0f64.to_bits()),
                ]),
            ),
            ("Health", Tag::Short(9)),
            ("Air", Tag::Short(150)),
            ("Inventory", Tag::List(Vec::new())),
        ]);
        let data = PlayerData::from_nbt(&as_compound(player));
        assert_eq!(data.motion[0], 0.0, "the 10 clamp zeroes the component");
        assert_eq!(data.health, 9.0, "Health short reads when HealF is absent");
        assert_eq!(data.air, 150);
        assert_eq!(
            data.food_level, 20,
            "hunger keeps its default without foodLevel"
        );
    }

    #[test]
    fn healf_wins_over_health() {
        let player = Tag::compound([
            ("HealF", Tag::Float(7.0f32.to_bits())),
            ("Health", Tag::Short(20)),
            ("Inventory", Tag::List(Vec::new())),
        ]);
        let data = PlayerData::from_nbt(&as_compound(player));
        assert_eq!(data.health, 7.0);
    }

    #[test]
    fn absent_health_is_the_player_max() {
        let player = Tag::compound([("Inventory", Tag::List(Vec::new()))]);
        let data = PlayerData::from_nbt(&as_compound(player));
        assert_eq!(data.health, 20.0);
    }

    #[test]
    fn spawn_reads_only_when_all_three_are_there() {
        let partial = Tag::compound([
            ("SpawnX", Tag::Int(-16)),
            ("SpawnZ", Tag::Int(240)),
            ("Inventory", Tag::List(Vec::new())),
        ]);
        let data = PlayerData::from_nbt(&as_compound(partial));
        assert_eq!(data.spawn, None);
        let full = Tag::compound([
            ("SpawnX", Tag::Int(-16)),
            ("SpawnY", Tag::Int(65)),
            ("SpawnZ", Tag::Int(240)),
            ("SpawnForced", Tag::Byte(1)),
            ("Inventory", Tag::List(Vec::new())),
        ]);
        let data = PlayerData::from_nbt(&as_compound(full));
        assert_eq!(data.spawn, Some([-16, 65, 240]));
        assert!(data.spawn_forced);
    }

    #[test]
    fn abilities_read_with_their_defaults() {
        let none = PlayerData::from_nbt(&as_compound(Tag::compound([(
            "Inventory",
            Tag::List(Vec::new()),
        )])));
        assert_eq!(none.abilities, Abilities::default());
        let creative = Tag::compound([
            (
                "abilities",
                Tag::Compound(Compound::from([
                    (JavaString::from("mayfly"), Tag::Byte(1)),
                    (JavaString::from("instabuild"), Tag::Byte(1)),
                    (JavaString::from("flySpeed"), Tag::Float(0.5f32.to_bits())),
                ])),
            ),
            ("Inventory", Tag::List(Vec::new())),
        ]);
        let data = PlayerData::from_nbt(&as_compound(creative));
        assert!(data.abilities.may_fly);
        assert!(data.abilities.instabuild);
        assert_eq!(data.abilities.fly_speed, 0.5);
        assert_eq!(
            data.abilities.walk_speed, 0.0,
            "walkSpeed reads with flySpeed, 0 for an absent tag (vanilla's getFloat)"
        );
        assert!(data.abilities.may_build, "mayBuild defaults true");
    }

    #[test]
    fn active_effects_only_write_when_there_are_any() {
        let empty = PlayerData::default().to_nbt();
        assert!(empty.get(&JavaString::from("ActiveEffects")).is_none());
        assert!(empty.get(&JavaString::from("Attributes")).is_some());
        assert!(empty.get(&JavaString::from("EnderItems")).is_some());
    }

    #[test]
    fn health_writes_as_healf_and_a_short_ceil() {
        let nbt = PlayerData {
            health: 13.5,
            ..PlayerData::default()
        }
        .to_nbt();
        let healf = nbt.get(&JavaString::from("HealF"));
        let health = nbt.get(&JavaString::from("Health"));
        assert_eq!(healf, Some(&Tag::Float(13.5f32.to_bits())));
        assert_eq!(health, Some(&Tag::Short(14)), "Health is the ceil");
    }

    #[test]
    fn the_file_rotation_leaves_one_player_dat() {
        let dir = std::env::temp_dir().join(format!("sunset-player-dat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        let data = PlayerData {
            pos: [1.0, 2.0, 3.0],
            ..PlayerData::default()
        };
        save(&dir, uuid, &data).unwrap();
        assert!(dir.join("playerdata").join(format!("{uuid}.dat")).is_file());
        assert!(
            !dir.join("playerdata")
                .join(format!("{uuid}.dat.tmp"))
                .exists()
        );
        let read = load(&dir, uuid).unwrap().unwrap();
        assert_eq!(read.pos, [1.0, 2.0, 3.0]);
        assert!(
            load(&dir, "00000000-0000-0000-0000-000000000000")
                .unwrap()
                .is_none()
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
