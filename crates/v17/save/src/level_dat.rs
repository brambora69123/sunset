//! `level.dat`: the world's settings, one gzip member of an NBT compound
//! wrapping a `Data` compound. The field set is 1.7.10's `WorldData`
//! (`net.minecraft.world.WorldData`), written in its `writeNbt` order —
//! there is no `DataVersion`, `LevelData` `Dimension` encoding or anything
//! later. Player data rides inside as the `Player` compound when the save
//! holds a single player (1.7.10's single-player `level.dat`), with the same
//! layout `playerdata/<uuid>.dat` carries.
//!
//! Reading reproduces `WorldData(NbtCompound)`'s defaults: absent weather
//! times are 0, `MapFeatures` defaults true, `initialized` defaults true,
//! `allowCommands` defaults to the game mode being creative, `DayTime`
//! falls back to `Time`, and a missing generator name is the default
//! generator. Compound access coercion is `NbtCompound`'s: a number tag of
//! any width reads through `NbtElement$Number`'s narrowing casts, and a
//! wrong-type read yields the default, not an error.
//!
//! Storage layout (`AnvilWorldStorage.saveData`): write to `level.dat_new`,
//! delete `level.dat_old`, rename `level.dat` to `level.dat_old`, rename
//! `level.dat_new` to `level.dat`. Loading tries `level.dat`, then
//! `level.dat_old`.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use sunrise_core::nbt::{self, Compound, JavaString, Tag};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// The world generator, `WorldGeneratorType`'s id, key and version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratorType {
    pub id: i32,
    pub key: &'static str,
    pub version: i32,
    /// `isVersioned`: only `default` (id 0) is, so `generatorVersion` 0 on
    /// it means the pre-1.2 generator, `default_1_1`.
    pub versioned: bool,
}

/// `WorldGeneratorType`'s constants, in its table.
pub const GENERATOR_TYPES: [GeneratorType; 5] = [
    GeneratorType {
        id: 0,
        key: "default",
        version: 1,
        versioned: true,
    },
    GeneratorType {
        id: 1,
        key: "flat",
        version: 0,
        versioned: false,
    },
    GeneratorType {
        id: 2,
        key: "largeBiomes",
        version: 0,
        versioned: false,
    },
    GeneratorType {
        id: 3,
        key: "amplified",
        version: 0,
        versioned: false,
    },
    GeneratorType {
        id: 8,
        key: "default_1_1",
        version: 0,
        versioned: false,
    },
];

const DEFAULT: usize = 0;

impl GeneratorType {
    /// `getTypeForVersion`: `default` at version 0 is `default_1_1`.
    pub fn for_version(self, version: i32) -> Self {
        if self.id == 0 && version == 0 {
            GENERATOR_TYPES[4]
        } else {
            self
        }
    }
}

/// `WorldGeneratorType.byKey`: case-insensitive over the key.
fn generator_by_key(key: &str) -> Option<usize> {
    GENERATOR_TYPES
        .iter()
        .position(|g| g.key.eq_ignore_ascii_case(key))
}

/// `WorldSettings$GameMode`: survival, creative, adventure. `byId` maps
/// anything else to survival.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
}
impl GameMode {
    pub fn id(self) -> i32 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
        }
    }
    /// `WorldSettings$GameMode.byId`: out of range is survival.
    pub fn by_id(id: i32) -> Self {
        match id {
            1 => Self::Creative,
            2 => Self::Adventure,
            _ => Self::Survival,
        }
    }
}

/// `GameRules`' defaults, in the order the Java constructor adds them. The
/// compound on disk holds every rule as a string, and reads keep unknown
/// rules, as `GameRules.readNbt` does.
pub const DEFAULT_GAME_RULES: [(&str, &str); 9] = [
    ("doFireTick", "true"),
    ("mobGriefing", "true"),
    ("keepInventory", "false"),
    ("doMobSpawning", "true"),
    ("doMobLoot", "true"),
    ("doTileDrops", "true"),
    ("commandBlockOutput", "true"),
    ("naturalRegeneration", "true"),
    ("doDaylightCycle", "true"),
];

/// The world's settings: 1.7.10's `WorldData` fields.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldData {
    pub seed: i64,
    /// The generator by its row in [`GENERATOR_TYPES`].
    pub generator: usize,
    pub generator_options: String,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    /// `Time`, ticks since the world began.
    pub time: i64,
    /// `DayTime`; loading falls back to `time` when the tag is absent.
    pub time_of_day: i64,
    pub last_played: i64,
    pub size_on_disk: i64,
    /// The single player's data, when the save carries it inside `level.dat`.
    pub player: Option<Compound>,
    /// The dimension the saved player is in, read from their `Dimension`.
    pub dimension: i32,
    pub name: String,
    /// The save format version: 19133 for Anvil.
    pub version: i32,
    pub rain_time: i32,
    pub raining: bool,
    pub thunder_time: i32,
    pub thundering: bool,
    pub game_mode: GameMode,
    pub map_features: bool,
    pub hardcore: bool,
    pub allow_commands: bool,
    pub initialized: bool,
    /// Rule name → value, every value a string, as vanilla keeps them.
    pub game_rules: Vec<(String, String)>,
}

impl Default for WorldData {
    /// `new WorldData(settings, name)`: a fresh world, `initialized` false.
    fn default() -> Self {
        Self {
            seed: 0,
            generator: DEFAULT,
            generator_options: String::new(),
            spawn_x: 0,
            spawn_y: 0,
            spawn_z: 0,
            time: 0,
            time_of_day: 0,
            last_played: 0,
            size_on_disk: 0,
            player: None,
            dimension: 0,
            name: String::new(),
            version: 0,
            rain_time: 0,
            raining: false,
            thunder_time: 0,
            thundering: false,
            game_mode: GameMode::Survival,
            map_features: false,
            hardcore: false,
            allow_commands: false,
            initialized: false,
            game_rules: default_game_rules(),
        }
    }
}

fn default_game_rules() -> Vec<(String, String)> {
    DEFAULT_GAME_RULES
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// `NbtCompound.getNumberCoerced`: any numeric tag through
/// `NbtElement$Number`'s narrowing casts.
fn number(tag: &Tag) -> Option<i64> {
    match tag {
        Tag::Byte(v) => Some(i64::from(*v)),
        Tag::Short(v) => Some(i64::from(*v)),
        Tag::Int(v) => Some(i64::from(*v)),
        Tag::Long(v) => Some(*v),
        Tag::Float(v) => Some(f32::from_bits(*v) as i64),
        Tag::Double(v) => Some(f64::from_bits(*v) as i64),
        _ => None,
    }
}

fn get_long(data: &Compound, key: &str) -> i64 {
    data.get(&JavaString::from(key))
        .and_then(number)
        .unwrap_or(0)
}
fn get_int(data: &Compound, key: &str) -> i32 {
    get_long(data, key) as i32
}
fn get_boolean(data: &Compound, key: &str) -> bool {
    get_byte(data, key) != 0
}
fn get_byte(data: &Compound, key: &str) -> i8 {
    get_long(data, key) as i8
}
/// `NbtCompound.getString`: a string tag's value, other tags' `toString`
/// (numbers print as Java prints them), absent is "".
fn get_string(data: &Compound, key: &str) -> String {
    match data.get(&JavaString::from(key)) {
        Some(Tag::String(value)) => value.to_string_lossy(),
        Some(Tag::Byte(v)) => v.to_string(),
        Some(Tag::Short(v)) => v.to_string(),
        Some(Tag::Int(v)) => v.to_string(),
        Some(Tag::Long(v)) => v.to_string(),
        Some(Tag::Float(v)) => java_float_to_string(f32::from_bits(*v)),
        Some(Tag::Double(v)) => java_double_to_string(f64::from_bits(*v)),
        _ => String::new(),
    }
}
/// Java's `Float.toString` for the whole-number values a mis-typed read can
/// meet here; close enough for a diagnostic path, exact for the common ones.
fn java_float_to_string(value: f32) -> String {
    if value == value.trunc() && value.abs() < 1e7 {
        format!("{}.0", value as i64)
    } else {
        value.to_string()
    }
}
fn java_double_to_string(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e7 {
        format!("{}.0", value as i64)
    } else {
        value.to_string()
    }
}

fn contains_typed(data: &Compound, key: &str, ty: u8) -> bool {
    let found = data.get(&JavaString::from(key)).map_or(0, |tag| match tag {
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

/// The `GameRules` compound as `GameRules.toNbt` writes it: every rule a
/// string.
fn game_rules_to_nbt(rules: &[(String, String)]) -> Tag {
    Tag::Compound(
        rules
            .iter()
            .map(|(name, value)| {
                (
                    JavaString::from(name.as_str()),
                    Tag::String(JavaString::from(value.as_str())),
                )
            })
            .collect(),
    )
}

/// `GameRules.readNbt`: known rules update, unknown rules are added, and
/// non-string entries read through `getString`'s coercion.
fn read_game_rules(compound: &Compound) -> Vec<(String, String)> {
    let mut rules = default_game_rules();
    for (key, value) in compound.iter() {
        let name = key.to_string_lossy();
        let value = match value {
            Tag::String(value) => value.to_string_lossy(),
            Tag::Byte(v) => v.to_string(),
            Tag::Short(v) => v.to_string(),
            Tag::Int(v) => v.to_string(),
            Tag::Long(v) => v.to_string(),
            Tag::Float(v) => java_float_to_string(f32::from_bits(*v)),
            Tag::Double(v) => java_double_to_string(f64::from_bits(*v)),
            _ => continue,
        };
        if let Some(slot) = rules.iter_mut().find(|(rule, _)| *rule == name) {
            slot.1 = value;
        } else {
            rules.push((name, value));
        }
    }
    rules
}

impl WorldData {
    /// `new WorldData(nbt)`: the `Data` compound, read with `NbtCompound`'s
    /// defaults and coercions. Field by field, as the Java constructor reads
    /// into `this` one by one.
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_nbt(data: &Compound) -> Self {
        let mut world = Self::default();
        world.seed = get_long(data, "RandomSeed");
        // `byKey`: an unknown name is the default generator, and the version
        // mapping (its `else if`) does not run for it.
        if let Some(index) = generator_by_key(&get_string(data, "generatorName")) {
            world.generator = index;
            let generator = GENERATOR_TYPES[index];
            if generator.versioned {
                let version = if contains_typed(data, "generatorVersion", 99) {
                    get_int(data, "generatorVersion")
                } else {
                    0
                };
                world.generator = GENERATOR_TYPES
                    .iter()
                    .position(|g| *g == generator.for_version(version))
                    .unwrap_or(DEFAULT);
            }
            if contains_typed(data, "generatorOptions", 8) {
                world.generator_options = get_string(data, "generatorOptions");
            }
        }
        world.game_mode = GameMode::by_id(get_int(data, "GameType"));
        world.map_features = if contains_typed(data, "MapFeatures", 99) {
            get_boolean(data, "MapFeatures")
        } else {
            true
        };
        world.spawn_x = get_int(data, "SpawnX");
        world.spawn_y = get_int(data, "SpawnY");
        world.spawn_z = get_int(data, "SpawnZ");
        world.time = get_long(data, "Time");
        world.time_of_day = if contains_typed(data, "DayTime", 99) {
            get_long(data, "DayTime")
        } else {
            world.time
        };
        world.last_played = get_long(data, "LastPlayed");
        world.size_on_disk = get_long(data, "SizeOnDisk");
        world.name = get_string(data, "LevelName");
        world.version = get_int(data, "version");
        world.rain_time = get_int(data, "rainTime");
        world.raining = get_boolean(data, "raining");
        world.thunder_time = get_int(data, "thunderTime");
        world.thundering = get_boolean(data, "thundering");
        world.hardcore = get_boolean(data, "hardcore");
        world.initialized = if contains_typed(data, "initialized", 99) {
            get_boolean(data, "initialized")
        } else {
            true
        };
        world.allow_commands = if contains_typed(data, "allowCommands", 99) {
            get_boolean(data, "allowCommands")
        } else {
            world.game_mode == GameMode::Creative
        };
        if let Some(Tag::Compound(player)) = data.get(&JavaString::from("Player")) {
            world.dimension = get_int(player, "Dimension");
            world.player = Some(player.clone());
        }
        if let Some(Tag::Compound(rules)) = data.get(&JavaString::from("GameRules")) {
            world.game_rules = read_game_rules(rules);
        }
        world
    }

    /// `writeNbt`: the `Data` compound, in vanilla's field order.
    /// `last_played` is the caller's clock (`getTimeMillis()` in vanilla);
    /// pass `now_millis` for it.
    pub fn to_nbt(&self, now_millis: i64) -> Compound {
        let generator = GENERATOR_TYPES[self.generator.min(GENERATOR_TYPES.len() - 1)];
        let mut data = Compound::default();
        let mut put = |key: &str, value: Tag| data.insert(JavaString::from(key), value);
        put("RandomSeed", Tag::Long(self.seed));
        put(
            "generatorName",
            Tag::String(JavaString::from(generator.key)),
        );
        put("generatorVersion", Tag::Int(generator.version));
        put(
            "generatorOptions",
            Tag::String(JavaString::from(self.generator_options.as_str())),
        );
        put("GameType", Tag::Int(self.game_mode.id()));
        put("MapFeatures", Tag::Byte(self.map_features as i8));
        put("SpawnX", Tag::Int(self.spawn_x));
        put("SpawnY", Tag::Int(self.spawn_y));
        put("SpawnZ", Tag::Int(self.spawn_z));
        put("Time", Tag::Long(self.time));
        put("DayTime", Tag::Long(self.time_of_day));
        put("SizeOnDisk", Tag::Long(self.size_on_disk));
        put("LastPlayed", Tag::Long(now_millis));
        put(
            "LevelName",
            Tag::String(JavaString::from(self.name.as_str())),
        );
        put("version", Tag::Int(self.version));
        put("rainTime", Tag::Int(self.rain_time));
        put("raining", Tag::Byte(self.raining as i8));
        put("thunderTime", Tag::Int(self.thunder_time));
        put("thundering", Tag::Byte(self.thundering as i8));
        put("hardcore", Tag::Byte(self.hardcore as i8));
        put("allowCommands", Tag::Byte(self.allow_commands as i8));
        put("initialized", Tag::Byte(self.initialized as i8));
        put("GameRules", game_rules_to_nbt(&self.game_rules));
        if let Some(player) = &self.player {
            put("Player", Tag::Compound(player.clone()));
        }
        data
    }

    /// The save format version an Anvil save carries: `getVersion()`.
    pub fn set_version(&mut self, version: i32) {
        self.version = version;
    }
}

/// `AnvilWorldStorage`'s version stamp.
pub const ANVIL_VERSION: i32 = 19133;

/// The root of a compressed `.dat` file: a named compound holding `Data`.
fn wrap_data(data: Compound) -> Tag {
    Tag::compound([("Data", Tag::Compound(data))])
}

/// `NbtIo.readCompressed` + `getCompound("Data")`, from bytes.
pub fn read_data_compound(bytes: &[u8]) -> io::Result<Compound> {
    let raw = crate::java_zip::gunzip(bytes)?;
    let (named, _) =
        nbt::read_named(&raw, nbt::Limits::saved()).map_err(|e| invalid(e.to_string()))?;
    match named.tag {
        Tag::Compound(root) => match root.get(&JavaString::from("Data")) {
            Some(Tag::Compound(data)) => Ok(data.clone()),
            _ => Err(invalid("level.dat has no Data compound")),
        },
        _ => Err(invalid("level.dat root is not a compound")),
    }
}

/// `writeCompressed(new NbtCompound(){ put("Data", …) })`, to bytes.
pub fn write_data_compound(data: &Compound) -> io::Result<Vec<u8>> {
    let root = wrap_data(data.clone());
    let bytes = nbt::write_named(
        &nbt::NamedTag {
            name: JavaString::default(),
            tag: root,
        },
        nbt::Limits {
            max_depth: nbt::DISK_WRITE_NESTING_GUARD,
            ..nbt::Limits::saved()
        },
    )
    .map_err(|e| invalid(e.to_string()))?;
    Ok(crate::java_zip::gzip(&bytes))
}

/// The paths `saveData` cycles through.
pub struct LevelDatPaths {
    pub new_: PathBuf,
    pub old: PathBuf,
    pub current: PathBuf,
}
impl LevelDatPaths {
    pub fn new(world_dir: &Path) -> Self {
        Self {
            new_: world_dir.join("level.dat_new"),
            old: world_dir.join("level.dat_old"),
            current: world_dir.join("level.dat"),
        }
    }
}

/// `AlphaWorldStorage.loadData`: read `level.dat`, falling back to
/// `level.dat_old`. Returns the `Data` compound.
pub fn load(world_dir: &Path) -> io::Result<Compound> {
    let paths = LevelDatPaths::new(world_dir);
    let mut last = None;
    for path in [&paths.current, &paths.old] {
        match fs::read(path) {
            Ok(bytes) => match read_data_compound(&bytes) {
                Ok(data) => return Ok(data),
                Err(e) => last = Some(e),
            },
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| invalid("no level.dat")))
}

/// `AnvilWorldStorage.saveData`: stamp version 19133, write `level.dat_new`,
/// then rotate `level.dat` → `level.dat_old` → replaced. Returns the world
/// data as written (the caller's copy keeps its old version, as vanilla's
/// `setVersion` runs on the stored copy).
pub fn save(world_dir: &Path, data: &mut WorldData, now_millis: i64) -> io::Result<()> {
    data.set_version(ANVIL_VERSION);
    let paths = LevelDatPaths::new(world_dir);
    let bytes = write_data_compound(&data.to_nbt(now_millis))?;
    // `NbtIo.writeCompressed` into level.dat_new, then the rename dance.
    let mut file = fs::File::create(&paths.new_)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    if paths.old.exists() {
        fs::remove_file(&paths.old)?;
    }
    if paths.current.exists() {
        fs::rename(&paths.current, &paths.old)?;
    }
    fs::rename(&paths.new_, &paths.current)?;
    Ok(())
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

    fn roundtrip(world: WorldData, now: i64) -> WorldData {
        let nbt = world.to_nbt(now);
        let bytes = write_data_compound(&nbt).unwrap();
        let data = read_data_compound(&bytes).unwrap();
        WorldData::from_nbt(&data)
    }

    #[test]
    fn level_dat_roundtrips_through_gzip_nbt() {
        let mut world = WorldData {
            seed: -4172829371415413273,
            name: "New World".into(),
            spawn_x: -192,
            spawn_y: 64,
            spawn_z: 208,
            time: 108_213,
            time_of_day: 96_000,
            rain_time: 51_341,
            raining: true,
            game_mode: GameMode::Survival,
            map_features: true,
            hardcore: false,
            allow_commands: false,
            initialized: true,
            version: 0,
            ..WorldData::default()
        };
        world.game_rules[2] = ("keepInventory".into(), "true".into());
        let now = 1_700_000_123_456;
        let read = roundtrip(world.clone(), now);
        assert_eq!(read.last_played, now, "LastPlayed is the caller's clock");
        let mut world_without_last_played = world.clone();
        world_without_last_played.last_played = read.last_played;
        assert_eq!(read, world_without_last_played);
    }

    #[test]
    fn field_order_is_the_compound_s_jvm_hash_order() {
        // 1.7.10's NbtCompound writes a HashMap's key set, not insertion
        // order; sunrise-core's compound keeps that order. The field set is
        // what this module pins.
        let world = WorldData::default();
        let nbt = world.to_nbt(0);
        let mut keys: Vec<String> = nbt.keys().map(|k| k.to_string_lossy()).collect();
        let mut expected: Vec<&str> = [
            "RandomSeed",
            "generatorName",
            "generatorVersion",
            "generatorOptions",
            "GameType",
            "MapFeatures",
            "SpawnX",
            "SpawnY",
            "SpawnZ",
            "Time",
            "DayTime",
            "SizeOnDisk",
            "LastPlayed",
            "LevelName",
            "version",
            "rainTime",
            "raining",
            "thunderTime",
            "thundering",
            "hardcore",
            "allowCommands",
            "initialized",
            "GameRules",
        ]
        .into();
        keys.sort();
        expected.sort();
        assert_eq!(keys, expected);
        // And the order is stable run to run (a HashMap's order, fixed by
        // the keys and their insertions).
        let again = world.to_nbt(0);
        assert_eq!(
            nbt.keys().map(|k| k.to_string_lossy()).collect::<Vec<_>>(),
            again
                .keys()
                .map(|k| k.to_string_lossy())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn absent_fields_take_vanillas_defaults() {
        // A root without a Data compound is refused, as getCompound's cast
        // would be on a wrong-typed entry.
        let root = Tag::compound([("NotData", Tag::Long(99))]);
        let bytes = {
            let raw = nbt::write_named(
                &nbt::NamedTag {
                    name: JavaString::default(),
                    tag: root,
                },
                nbt::Limits::saved(),
            )
            .unwrap();
            crate::java_zip::gzip(&raw)
        };
        let error = read_data_compound(&bytes).unwrap_err();
        assert!(error.to_string().contains("Data"));
    }

    #[test]
    fn defaults_match_world_data_nbt_constructor() {
        let empty = Compound::default();
        let world = WorldData::from_nbt(&empty);
        assert_eq!(world.seed, 0);
        assert_eq!(world.generator, DEFAULT);
        assert!(world.map_features, "MapFeatures defaults true");
        assert!(world.initialized, "initialized defaults true");
        assert!(
            !world.allow_commands,
            "allowCommands defaults to survival's false"
        );
        assert_eq!(world.game_mode, GameMode::Survival);
        assert_eq!(world.game_rules.len(), DEFAULT_GAME_RULES.len());
        let creative = Tag::compound([("GameType", Tag::Int(1))]);
        let creative = WorldData::from_nbt(&as_compound(creative));
        assert!(
            creative.allow_commands,
            "allowCommands defaults to true for creative"
        );
    }

    #[test]
    fn day_time_falls_back_to_time() {
        let tag = Tag::compound([("Time", Tag::Long(555))]);
        let world = WorldData::from_nbt(&as_compound(tag));
        assert_eq!(world.time_of_day, 555);
    }

    #[test]
    fn default_1_1_generator_reads_from_version_zero() {
        let tag = Tag::compound([
            ("generatorName", Tag::String(JavaString::from("default"))),
            ("generatorVersion", Tag::Int(0)),
        ]);
        let world = WorldData::from_nbt(&as_compound(tag));
        assert_eq!(GENERATOR_TYPES[world.generator].key, "default_1_1");
        // Written back, it keeps default_1_1's key.
        let nbt = world.to_nbt(0);
        let name = get_string(&nbt, "generatorName");
        assert_eq!(name, "default_1_1");
    }

    #[test]
    fn unknown_generator_falls_back_to_default_without_the_version_map() {
        let tag = Tag::compound([(
            "generatorName",
            Tag::String(JavaString::from("superflatxyz")),
        )]);
        let world = WorldData::from_nbt(&as_compound(tag));
        assert_eq!(
            world.generator, DEFAULT,
            "an unknown name is the default; the else-if skips version mapping"
        );
    }

    #[test]
    fn numeric_coercion_reads_any_width() {
        let tag = Tag::compound([("SpawnY", Tag::Byte(64))]);
        let world = WorldData::from_nbt(&as_compound(tag));
        assert_eq!(world.spawn_y, 64, "a byte SpawnY reads like getInt does");
    }

    #[test]
    fn game_rules_keep_unknown_rules_and_update_known() {
        let data = Compound::from([
            (
                JavaString::from("keepInventory"),
                Tag::String(JavaString::from("true")),
            ),
            (
                JavaString::from("doDaylightCycle"),
                Tag::String(JavaString::from("false")),
            ),
            (
                JavaString::from("sunsetRule"),
                Tag::String(JavaString::from("on")),
            ),
        ]);
        let rules = read_game_rules(&data);
        assert_eq!(
            rules.iter().find(|(n, _)| n == "keepInventory").unwrap().1,
            "true"
        );
        assert_eq!(
            rules
                .iter()
                .find(|(n, _)| n == "doDaylightCycle")
                .unwrap()
                .1,
            "false"
        );
        assert_eq!(
            rules.iter().find(|(n, _)| n == "sunsetRule").unwrap().1,
            "on",
            "an unknown rule is kept, as readNbt adds it"
        );
        // The nine defaults keep their order ahead of the added rule.
        assert_eq!(rules[2].0, "keepInventory");
        assert_eq!(rules[8].0, "doDaylightCycle");
        assert_eq!(rules[9].0, "sunsetRule");
    }

    #[test]
    fn the_save_rotation_leaves_level_dat_and_a_backup() {
        let dir = std::env::temp_dir().join(format!("sunset-level-dat-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut world = WorldData {
            name: "rotation".into(),
            ..WorldData::default()
        };
        save(&dir, &mut world, 1).unwrap();
        assert!(dir.join("level.dat").is_file());
        assert!(!dir.join("level.dat_new").exists());
        assert!(!dir.join("level.dat_old").exists());
        let mut world2 = world.clone();
        world2.name = "second".into();
        save(&dir, &mut world2, 2).unwrap();
        let data = load(&dir).unwrap();
        let read = WorldData::from_nbt(&data);
        assert_eq!(read.name, "second");
        assert_eq!(read.version, ANVIL_VERSION, "the save stamps 19133");
        let old = read_data_compound(&fs::read(dir.join("level.dat_old")).unwrap()).unwrap();
        let backup = WorldData::from_nbt(&old);
        assert_eq!(backup.name, "rotation", "the old level.dat moved to _old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_level_dat_written_by_another_tool_reads() {
        // tests/level.dat: one gzip member wrapping a Data compound with the
        // 1.7.10 field set, written by a Python driver of the same layout.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/level.dat");
        let bytes = fs::read(&path).unwrap();
        let compound = read_data_compound(&bytes).unwrap();
        let world = WorldData::from_nbt(&compound);
        use crate::player::PlayerData;
        assert_eq!(world.seed, 123_456_789);
        assert_eq!(GENERATOR_TYPES[world.generator].key, "default");
        assert_eq!(GENERATOR_TYPES[world.generator].version, 1);
        assert_eq!(world.game_mode, GameMode::Survival);
        assert_eq!((world.spawn_x, world.spawn_y, world.spawn_z), (0, 64, 0));
        assert_eq!(world.time, 1_000);
        assert_eq!(world.time_of_day, 1_000);
        assert_eq!(world.name, "World");
        assert_eq!(world.version, 19133);
        assert!(!world.raining);
        assert!(!world.hardcore);
        assert!(!world.allow_commands);
        assert!(world.initialized);
        assert_eq!(world.rain_time, 100);
        assert_eq!(
            world
                .game_rules
                .iter()
                .find(|(n, _)| n == "doFireTick")
                .map(|(_, v)| v.as_str()),
            Some("true")
        );
        // The Player compound rides inside, with the same layout playerdata
        // carries; the dimension comes from it.
        assert!(world.player.is_some());
        assert_eq!(world.dimension, 0);
        let player = PlayerData::from_nbt(world.player.as_ref().unwrap());
        assert_eq!(player.pos, [0.0, 64.5, 0.0]);
        assert_eq!(player.health, 20.0);
        assert_eq!(player.air, 300);
        assert_eq!(player.food_level, 20);
        assert_eq!(player.abilities.walk_speed, 0.1);
    }
}
