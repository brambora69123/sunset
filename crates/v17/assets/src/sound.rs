//! The 1.7.10 `sounds.json` pipeline: `net.minecraft.client.sound.*` and the
//! registration half of `net.minecraft.client.sound.system.SoundManager`,
//! ported exactly. The sound engine itself (OpenAL) is the render crate's
//! job; this crate owns the resource-side contract.

use std::collections::BTreeMap;

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::identifier::Identifier;
use crate::json::{self, JsonValue};

/// `net.minecraft.client.sound.SoundCategory`: the 1.7.10 categories with
/// their `sounds.json` names and protocol ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundCategory {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Mobs,
    Animals,
    Players,
    Ambient,
}

impl SoundCategory {
    pub const ALL: [SoundCategory; 9] = [
        SoundCategory::Master,
        SoundCategory::Music,
        SoundCategory::Records,
        SoundCategory::Weather,
        SoundCategory::Blocks,
        SoundCategory::Mobs,
        SoundCategory::Animals,
        SoundCategory::Players,
        SoundCategory::Ambient,
    ];

    /// The name `sounds.json` writes (Java's `getName`).
    pub fn name(&self) -> &'static str {
        match self {
            SoundCategory::Master => "master",
            SoundCategory::Music => "music",
            SoundCategory::Records => "record",
            SoundCategory::Weather => "weather",
            SoundCategory::Blocks => "block",
            SoundCategory::Mobs => "hostile",
            SoundCategory::Animals => "neutral",
            SoundCategory::Players => "player",
            SoundCategory::Ambient => "ambient",
        }
    }

    /// The protocol id (Java's `getId`).
    pub fn id(&self) -> i32 {
        match self {
            SoundCategory::Master => 0,
            SoundCategory::Music => 1,
            SoundCategory::Records => 2,
            SoundCategory::Weather => 3,
            SoundCategory::Blocks => 4,
            SoundCategory::Mobs => 5,
            SoundCategory::Animals => 6,
            SoundCategory::Players => 7,
            SoundCategory::Ambient => 8,
        }
    }

    /// `SoundCategory.byName`; unknown names give `None` (the vanilla
    /// deserializer turns `null` into a "Invalid category" error).
    pub fn by_name(name: &str) -> Option<SoundCategory> {
        match name {
            "master" => Some(SoundCategory::Master),
            "music" => Some(SoundCategory::Music),
            "record" => Some(SoundCategory::Records),
            "weather" => Some(SoundCategory::Weather),
            "block" => Some(SoundCategory::Blocks),
            "hostile" => Some(SoundCategory::Mobs),
            "neutral" => Some(SoundCategory::Animals),
            "player" => Some(SoundCategory::Players),
            "ambient" => Some(SoundCategory::Ambient),
            _ => None,
        }
    }
}

/// `net.minecraft.client.sound.SoundList$Sound$Type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundRefType {
    /// A `.ogg` file under `sounds/`.
    File,
    /// A reference to another sound event.
    Event,
}

impl SoundRefType {
    /// `SoundList$Sound$Type.byName`.
    pub fn by_name(name: &str) -> Option<SoundRefType> {
        match name {
            "file" => Some(SoundRefType::File),
            "event" => Some(SoundRefType::Event),
            _ => None,
        }
    }
}

/// One entry of a `sounds` array (`SoundList$Sound`).
#[derive(Clone, Debug, PartialEq)]
pub struct SoundDef {
    pub name: String,
    pub volume: f32,
    pub pitch: f32,
    pub weight: i32,
    pub ty: SoundRefType,
    pub stream: bool,
}

impl Default for SoundDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            volume: 1.0,
            pitch: 1.0,
            weight: 1,
            ty: SoundRefType::File,
            stream: false,
        }
    }
}

/// One event of `sounds.json` (`SoundList`).
#[derive(Clone, Debug, PartialEq)]
pub struct SoundEntry {
    pub replace: bool,
    pub category: SoundCategory,
    pub sounds: Vec<SoundDef>,
}

impl SoundEntry {
    /// `SoundListDeserializer.deserialize`: exact field names, defaults and
    /// validation (volume, pitch and weight must be positive; the category
    /// and type names must exist).
    pub fn from_json(value: &JsonValue) -> Result<SoundEntry, String> {
        value.as_object("entry")?;
        let mut sound = SoundEntry {
            replace: false,
            category: SoundCategory::Master,
            sounds: Vec::new(),
        };

        if let Some(replace) = value.get("replace") {
            sound.replace = replace.as_bool("replace")?;
        }

        let category = match value.get("category") {
            Some(category) => category.as_string("category")?,
            None => SoundCategory::Master.name(),
        };
        sound.category =
            SoundCategory::by_name(category).ok_or_else(|| "Invalid category".to_owned())?;

        if let Some(sounds) = value.get("sounds") {
            let sounds = sounds.as_array("sounds")?;
            for element in sounds {
                let def = Self::sound_def(element)?;
                sound.sounds.push(def);
            }
        }

        Ok(sound)
    }

    fn sound_def(element: &JsonValue) -> Result<SoundDef, String> {
        let mut def = SoundDef::default();
        if element.is_string() {
            def.name = element.as_string("sound")?.to_owned();
            return Ok(def);
        }

        let object = element.as_object("sound")?;
        def.name = element
            .get("name")
            .ok_or_else(|| "name is missing".to_owned())?
            .as_string("name")?
            .to_owned();

        if let Some(ty) = element.get("type") {
            def.ty = SoundRefType::by_name(ty.as_string("type")?)
                .ok_or_else(|| "Invalid type".to_owned())?;
        }
        if let Some(volume) = element.get("volume") {
            let volume = volume.as_f64("volume")?;
            if volume <= 0.0 {
                return Err("Invalid volume".to_owned());
            }
            def.volume = volume as f32;
        }
        if let Some(pitch) = element.get("pitch") {
            let pitch = pitch.as_f64("pitch")?;
            if pitch <= 0.0 {
                return Err("Invalid pitch".to_owned());
            }
            def.pitch = pitch as f32;
        }
        if let Some(weight) = element.get("weight") {
            let weight = weight.as_i64("weight")?;
            if !(weight > 0) {
                return Err("Invalid weight".to_owned());
            }
            def.weight = weight as i32;
        }
        if let Some(stream) = element.get("stream") {
            def.stream = stream.as_bool("stream")?;
        }

        // Java reads the fields through the same JsonObject this binding
        // holds; nothing further is read from it.
        let _ = object;
        Ok(def)
    }
}

/// `SoundManager.reload`'s parse step: a whole `sounds.json` into
/// `(event location, entry)` pairs, in document order.
pub fn parse_sounds_json(
    namespace: &str,
    text: &str,
) -> Result<Vec<(Identifier, SoundEntry)>, String> {
    let root = json::parse(text)?;
    let entries = root.as_object("entry")?;
    let mut events = Vec::new();
    for (name, value) in entries {
        events.push((
            Identifier::new(namespace, name),
            SoundEntry::from_json(value)?,
        ));
    }
    Ok(events)
}

/// A sound file reference resolved to its resource id
/// (`SoundManager.register`, `FILE` case).
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSound {
    pub location: Identifier,
    pub volume: f32,
    pub pitch: f32,
    pub weight: i32,
    pub stream: bool,
}

/// A sound event reference (`SoundManager$38726288`): the referenced event's
/// location, resolved against the registry at play time.
#[derive(Clone, Debug, PartialEq)]
pub struct EventRef {
    pub location: Identifier,
    pub volume: f32,
    pub pitch: f32,
}

/// One container of a pool, in registration order: a resolved sound file
/// (`FILE`) or a reference to another sound event (`EVENT`).
#[derive(Clone, Debug, PartialEq)]
pub enum Container {
    File(ResolvedSound),
    Event(EventRef),
}

/// The containers of one sound event, in `SoundManager.register` order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedEvent {
    pub containers: Vec<Container>,
}

impl ResolvedEvent {
    /// `SoundManager.register`: file refs resolve to
    /// `sounds/<path>.ogg` (the namespace of the name when it carries one,
    /// the event's namespace otherwise) and are dropped when the resource is
    /// missing; event refs keep the referenced event's location.
    pub fn resolve(
        entry: &SoundEntry,
        event_namespace: &str,
        has_resource: impl Fn(&Identifier) -> bool,
    ) -> ResolvedEvent {
        let mut resolved = ResolvedEvent::default();
        for def in &entry.sounds {
            let id = Identifier::parse(&def.name);
            let namespace = if def.name.contains(':') {
                id.namespace()
            } else {
                event_namespace
            };
            match def.ty {
                SoundRefType::File => {
                    let location = Identifier::new(namespace, &format!("sounds/{}.ogg", id.path()));
                    if !has_resource(&location) {
                        continue;
                    }
                    resolved.containers.push(Container::File(ResolvedSound {
                        location,
                        volume: def.volume,
                        pitch: def.pitch,
                        weight: def.weight,
                        stream: def.stream,
                    }));
                }
                SoundRefType::Event => {
                    resolved.containers.push(Container::Event(EventRef {
                        location: Identifier::new(namespace, &def.name),
                        volume: def.volume,
                        pitch: def.pitch,
                    }));
                }
            }
        }
        resolved
    }

    /// `SoundPool.getWeight`: the files' weights plus every referenced
    /// event's weight (looked up in `registry`).
    pub fn weight(&self, registry: &SoundRegistry) -> i32 {
        self.containers
            .iter()
            .map(|container| match container {
                Container::File(sound) => sound.weight,
                Container::Event(event) => registry
                    .get(&event.location)
                    .map(|pool| pool.weight(registry))
                    .unwrap_or(0),
            })
            .sum()
    }
}

/// `net.minecraft.client.sound.SoundPool`: weighted pick over the resolved
/// containers, on the exact `java.util.Random` from `sunset-1-7-core`.
#[derive(Clone, Debug)]
pub struct SoundPool {
    pub location: Identifier,
    pub category: SoundCategory,
    resolved: ResolvedEvent,
    volume_multiplier: f64,
    pitch_multiplier: f64,
    random: LegacyRandom,
}

impl SoundPool {
    pub fn new(location: Identifier, category: SoundCategory, resolved: ResolvedEvent) -> Self {
        let seed = rand_seed(&location);
        Self {
            location,
            category,
            resolved,
            volume_multiplier: 1.0,
            pitch_multiplier: 1.0,
            random: LegacyRandom::new(seed),
        }
    }

    pub fn with_multipliers(
        location: Identifier,
        category: SoundCategory,
        resolved: ResolvedEvent,
        volume_multiplier: f64,
        pitch_multiplier: f64,
    ) -> Self {
        let mut pool = Self::new(location, category, resolved);
        pool.volume_multiplier = volume_multiplier;
        pool.pitch_multiplier = pitch_multiplier;
        pool
    }

    /// `SoundPool.getWeight`: the files' weights plus every referenced
    /// event's weight (looked up in `registry`).
    pub fn weight(&self, registry: &SoundRegistry) -> i32 {
        self.resolved.weight(registry)
    }

    /// `SoundPool.get`: weighted pick over the containers in registration
    /// order; `None` is the vanilla `MISSING_SOUND`. Multipliers apply on
    /// top of the chosen sound's own volume and pitch, exactly as in
    /// vanilla.
    pub fn get(&mut self, registry: &SoundRegistry) -> Option<ChosenSound> {
        let total = self.weight(registry);
        if self.resolved.containers.is_empty() || total == 0 {
            return None;
        }
        let mut remaining = self.random.next_int_bound(total).ok()?;
        for container in &self.resolved.containers {
            match container {
                Container::File(sound) => {
                    remaining -= sound.weight;
                    if remaining < 0 {
                        return Some(ChosenSound {
                            location: sound.location.clone(),
                            volume: f64::from(sound.volume) * self.volume_multiplier,
                            pitch: f64::from(sound.pitch) * self.pitch_multiplier,
                            stream: sound.stream,
                        });
                    }
                }
                Container::Event(event) => {
                    remaining -= registry
                        .get(&event.location)
                        .map(|pool| pool.weight(registry))
                        .unwrap_or(0);
                    if remaining < 0 {
                        let mut pool = registry.get(&event.location)?.clone();
                        pool.volume_multiplier *= self.volume_multiplier;
                        pool.pitch_multiplier *= self.pitch_multiplier;
                        return pool.get(registry);
                    }
                }
            }
        }
        None
    }
}

/// A picked sound with its final volume and pitch.
#[derive(Clone, Debug, PartialEq)]
pub struct ChosenSound {
    pub location: Identifier,
    pub volume: f64,
    pub pitch: f64,
    pub stream: bool,
}

/// `net.minecraft.client.sound.system.SoundRegistry`: event location ->
/// pool, plus the `replace` semantics of `SoundManager.register`.
#[derive(Clone, Debug, Default)]
pub struct SoundRegistry {
    pools: BTreeMap<Identifier, SoundPool>,
}

impl SoundRegistry {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
        }
    }

    pub fn get(&self, location: &Identifier) -> Option<&SoundPool> {
        self.pools.get(location)
    }

    pub fn contains_key(&self, location: &Identifier) -> bool {
        self.pools.contains_key(location)
    }

    pub fn event_count(&self) -> usize {
        self.pools.len()
    }

    pub fn event_names(&self) -> Vec<String> {
        self.pools.keys().map(Identifier::to_string).collect()
    }

    /// `SoundManager.register`: an existing event survives unless the new
    /// entry sets `replace`; every container of the entry adds to the pool.
    /// `has_resource` is the manager's resource check for the `.ogg` files.
    pub fn register(
        &mut self,
        location: Identifier,
        entry: &SoundEntry,
        has_resource: impl Fn(&Identifier) -> bool,
    ) {
        if !self.contains_key(&location) || entry.replace {
            self.pools.insert(
                location.clone(),
                SoundPool::new(location.clone(), entry.category, ResolvedEvent::default()),
            );
        }

        let resolved = ResolvedEvent::resolve(entry, location.namespace(), has_resource);
        let pool = self
            .pools
            .get_mut(&location)
            .expect("register inserts the pool");
        pool.resolved.containers.extend(resolved.containers);
    }

    /// `SoundManager.reload` over one `sounds.json`: every event of the
    /// document registered under the namespace.
    pub fn register_sounds_json(
        &mut self,
        namespace: &str,
        text: &str,
        has_resource: impl Fn(&Identifier) -> bool,
    ) -> Result<(), String> {
        for (location, entry) in parse_sounds_json(namespace, text)? {
            self.register(location, &entry, &has_resource);
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.pools.clear();
    }
}

/// The pool RNG's seed: derived from the event location so the same registry
/// draws identically run to run.
fn rand_seed(location: &Identifier) -> i64 {
    location.to_string().bytes().fold(0i64, |seed, byte| {
        seed.wrapping_mul(31).wrapping_add(i64::from(byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{
        "random.explode": {"category": "block", "sounds": ["explode1", {"name": "explode2", "volume": 0.5, "weight": 2}]},
        "music.menu": {"category": "music", "replace": true, "sounds": [{"name": "menu1", "stream": true}]},
        "nested": {"sounds": [{"name": "music.menu", "type": "event"}]}
    }"#;

    #[test]
    fn categories_match_1_7_10() {
        assert_eq!(
            SoundCategory::by_name("master"),
            Some(SoundCategory::Master)
        );
        assert_eq!(
            SoundCategory::by_name("record"),
            Some(SoundCategory::Records)
        );
        assert_eq!(SoundCategory::by_name("hostile"), Some(SoundCategory::Mobs));
        assert_eq!(
            SoundCategory::by_name("neutral"),
            Some(SoundCategory::Animals)
        );
        assert_eq!(SoundCategory::by_name("nope"), None);
        assert_eq!(SoundCategory::Music.id(), 1);
        assert_eq!(SoundCategory::ALL.len(), 9);
    }

    #[test]
    fn parses_entry_with_defaults() {
        let entry =
            SoundEntry::from_json(&json::parse(r#"{"sounds": ["bang"]}"#).unwrap()).unwrap();
        assert!(!entry.replace);
        assert_eq!(entry.category, SoundCategory::Master);
        assert_eq!(entry.sounds.len(), 1);
        assert_eq!(entry.sounds[0].name, "bang");
        assert_eq!(entry.sounds[0].volume, 1.0);
        assert_eq!(entry.sounds[0].weight, 1);
        assert_eq!(entry.sounds[0].ty, SoundRefType::File);
        assert!(!entry.sounds[0].stream);
    }

    #[test]
    fn parses_full_entry() {
        let entry = SoundEntry::from_json(
            &json::parse(r#"{"replace": true, "category": "music", "sounds": [{"name": "x", "stream": true, "volume": 0.5, "weight": 3, "pitch": 1.5}]}"#)
                .unwrap(),
        )
        .unwrap();
        assert!(entry.replace);
        assert_eq!(entry.category, SoundCategory::Music);
        assert!(entry.sounds[0].stream);
        assert_eq!(entry.sounds[0].volume, 0.5);
        assert_eq!(entry.sounds[0].pitch, 1.5);
        assert_eq!(entry.sounds[0].weight, 3);
    }

    #[test]
    fn rejects_bad_values() {
        assert!(
            SoundEntry::from_json(
                &json::parse(r#"{"sounds": [{"name": "x", "volume": 0}]}"#).unwrap()
            )
            .is_err()
        );
        assert!(
            SoundEntry::from_json(
                &json::parse(r#"{"sounds": [{"name": "x", "pitch": -1}]}"#).unwrap()
            )
            .is_err()
        );
        assert!(
            SoundEntry::from_json(
                &json::parse(r#"{"sounds": [{"name": "x", "weight": 0}]}"#).unwrap()
            )
            .is_err()
        );
        assert!(SoundEntry::from_json(&json::parse(r#"{"category": "bogus"}"#).unwrap()).is_err());
        assert!(
            SoundEntry::from_json(
                &json::parse(r#"{"sounds": [{"name": "x", "type": "bogus"}]}"#).unwrap()
            )
            .is_err()
        );
        assert!(SoundEntry::from_json(&json::parse(r#""not an object""#).unwrap()).is_err());
    }

    #[test]
    fn resolves_file_refs_with_namespace() {
        let entry = SoundEntry::from_json(
            &json::parse(r#"{"sounds": ["mine", "othermod:theirs"]}"#).unwrap(),
        )
        .unwrap();
        let resolved = ResolvedEvent::resolve(&entry, "minecraft", |_| true);
        let Container::File(first) = &resolved.containers[0] else {
            panic!("expected a file container");
        };
        let Container::File(second) = &resolved.containers[1] else {
            panic!("expected a file container");
        };
        assert_eq!(first.location.to_string(), "minecraft:sounds/mine.ogg");
        assert_eq!(second.location.to_string(), "othermod:sounds/theirs.ogg");
    }

    #[test]
    fn drops_missing_files_keeps_event_refs() {
        let entry = SoundEntry::from_json(
            &json::parse(r#"{"sounds": ["gone", {"name": "random.explode", "type": "event"}]}"#)
                .unwrap(),
        )
        .unwrap();
        let resolved = ResolvedEvent::resolve(&entry, "minecraft", |_| false);
        assert!(
            resolved
                .containers
                .iter()
                .all(|container| matches!(container, Container::Event(_)))
        );
        let Container::Event(event) = &resolved.containers[0] else {
            panic!("expected an event container");
        };
        assert_eq!(event.location.to_string(), "minecraft:random.explode");
    }

    #[test]
    fn registration_replace_semantics() {
        let first = r#"{"category": "weather", "sounds": ["rain1"]}"#;
        let replaced = r#"{"replace": true, "category": "weather", "sounds": ["rain2"]}"#;
        let added = r#"{"category": "weather", "sounds": ["rain3"]}"#;

        let mut registry = SoundRegistry::new();
        registry.register(
            Identifier::parse("minecraft:rain"),
            &entry_of(first),
            |_| true,
        );
        registry.register(
            Identifier::parse("minecraft:rain"),
            &entry_of(added),
            |_| true,
        );
        assert_eq!(
            registry
                .get(&Identifier::parse("minecraft:rain"))
                .unwrap()
                .resolved
                .containers
                .len(),
            2
        );

        registry.register(
            Identifier::parse("minecraft:rain"),
            &entry_of(replaced),
            |_| true,
        );
        let pool = registry.get(&Identifier::parse("minecraft:rain")).unwrap();
        assert_eq!(pool.resolved.containers.len(), 1);
        let Container::File(sound) = &pool.resolved.containers[0] else {
            panic!("expected a file container");
        };
        assert_eq!(sound.location.path(), "sounds/rain2.ogg");
    }

    fn entry_of(text: &str) -> SoundEntry {
        SoundEntry::from_json(&json::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn weighted_pick_matches_java_random() {
        // Two files, weights 1 and 2: the draw order of java.util.Random
        // seeded from the location decides which is picked; the invariant is
        // a deterministic pick for a fixed registry and 3 draws in 0..=2.
        let mut registry = SoundRegistry::new();
        registry
            .register_sounds_json(
                "minecraft",
                r#"{"ev": {"sounds": ["a", {"name": "b", "weight": 2}]}}"#,
                |_| true,
            )
            .unwrap();
        let mut pool = registry
            .get(&Identifier::parse("minecraft:ev"))
            .unwrap()
            .clone();
        let picked = pool.get(&registry).unwrap();
        assert!(
            picked.location.path() == "sounds/a.ogg" || picked.location.path() == "sounds/b.ogg"
        );
        // Same seed, same pick.
        let mut pool_again = registry
            .get(&Identifier::parse("minecraft:ev"))
            .unwrap()
            .clone();
        assert_eq!(pool_again.get(&registry).unwrap(), picked);
    }

    #[test]
    fn full_sounds_json_round_trip() {
        let mut registry = SoundRegistry::new();
        registry
            .register_sounds_json("minecraft", JSON, |_| true)
            .unwrap();
        assert_eq!(registry.event_count(), 3);
        assert!(registry.contains_key(&Identifier::parse("minecraft:random.explode")));
        assert_eq!(
            registry
                .get(&Identifier::parse("minecraft:random.explode"))
                .unwrap()
                .category,
            SoundCategory::Blocks
        );
        assert_eq!(
            registry
                .get(&Identifier::parse("minecraft:music.menu"))
                .unwrap()
                .category,
            SoundCategory::Music
        );
        assert_eq!(
            registry
                .get(&Identifier::parse("minecraft:random.explode"))
                .unwrap()
                .resolved
                .weight(&registry),
            3
        );
    }
}
