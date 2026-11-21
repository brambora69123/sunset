//! 1.7.10's `sounds.json` asked for by the engine's event ids.
//!
//! The era's events are named after its `Block$Sounds` keys (`dig.stone`, `step.grass`), while
//! the game asks for the ids in its block facts (`block.stone.break`, `block.grass.step`). This
//! module is the table between them: for every engine id whose era counterpart the loaded
//! `sounds.json` holds, [`SoundRegistry::add_event_reference`] adds an event that plays the
//! era pool, so the era entry's own files, weights, volume and pitch decide what is heard.
//!
//! The table is 1.7.10's `Block$Sounds` itself: `getBreaking` is `dig.<key>`, `getStepping` is
//! `step.<key>` and `getPlacing` is `getBreaking`, with three subclasses — glass
//! (`dig.glass`, placing `step.stone`), the ladder (`dig.wood`, `step.ladder`) and the anvil
//! (`dig.stone`, placing `random.anvil_land`, stepping `step.anvil`, which the era's
//! `sounds.json` does not define and so plays nothing, as in vanilla). The engine's five block
//! causes all land on those three events: a break and a placement on `getBreaking` and
//! `getPlacing`, a step, a mining hit and a fall on `getStepping` (the engine applies vanilla's
//! volume and pitch factors for each cause).
//!
//! A few non-block events the 26.3 ids renamed are listed too, each by the 1.7.10 class that
//! plays it. Nothing is guessed: an engine id with no entry here, and every era event whose
//! files the import could not copy, plays nothing.

use crate::sound_events::SoundRegistry;

/// One 1.7.10 `Block$Sounds` value under the family name the block facts give its states, with
/// the era's own overrides for the events it renames.
struct BlockSounds {
    family: &'static str,
    /// The `key` its events are built from.
    key: &'static str,
    /// `getBreaking`, when it is not `dig.<key>`.
    breaking: Option<&'static str>,
    /// `getStepping`, when it is not `step.<key>`.
    stepping: Option<&'static str>,
    /// `getPlacing`, when it is not `getBreaking`.
    placing: Option<&'static str>,
}

impl BlockSounds {
    /// `getBreaking`, `getStepping` and `getPlacing`, without the namespace.
    fn events(&self) -> (String, String, String) {
        let breaking = self
            .breaking
            .map_or_else(|| format!("dig.{}", self.key), str::to_owned);
        let stepping = self
            .stepping
            .map_or_else(|| format!("step.{}", self.key), str::to_owned);
        let placing = self.placing.map_or_else(|| breaking.clone(), str::to_owned);
        (breaking, stepping, placing)
    }
}

/// `Block.DEFAULT_SOUNDS`, `WOOD_SOUNDS`, `GRAVEL_SOUNDS`, `GRASS_SOUNDS`, `STONE_SOUNDS`,
/// `CLOTH_SOUNDS`, `SAND_SOUNDS`, `SNOW_SOUNDS`, `GLASS_SOUNDS` (`Block$96978149`),
/// `LADDER_SOUNDS` (`Block$48198598`) and `ANVIL_SOUNDS` (`Block$23487049`).
const BLOCK_SOUNDS: [BlockSounds; 10] = [
    BlockSounds {
        family: "stone",
        key: "stone",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "grass",
        key: "grass",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "gravel",
        key: "gravel",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "sand",
        key: "sand",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "snow",
        key: "snow",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "wood",
        key: "wood",
        breaking: None,
        stepping: None,
        placing: None,
    },
    // `CLOTH_SOUNDS`: the era's `cloth` key, which the engine calls wool.
    BlockSounds {
        family: "wool",
        key: "cloth",
        breaking: None,
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "glass",
        key: "stone",
        breaking: Some("dig.glass"),
        stepping: None,
        placing: Some("step.stone"),
    },
    BlockSounds {
        family: "ladder",
        key: "ladder",
        breaking: Some("dig.wood"),
        stepping: None,
        placing: None,
    },
    BlockSounds {
        family: "anvil",
        key: "anvil",
        breaking: Some("dig.stone"),
        stepping: None,
        placing: Some("random.anvil_land"),
    },
];

/// The engine's block-sound causes: `break`, `place`, `step`, `hit` and `fall`, plus the
/// anvil's `land`, which the facts use for its placement (`random.anvil_land`).
const CAUSES: [&str; 6] = ["break", "place", "step", "hit", "fall", "land"];

/// Events the era names differently, each with the 1.7.10 class that plays it.
const RENAMES: [(&str, &str); 7] = [
    // `MusicManager$Music`: CREATIVE, CREDITS, END_BOSS, END and NETHER.
    ("music.creative", "music.game.creative"),
    ("music.credits", "music.game.end.credits"),
    ("music.dragon", "music.game.end.dragon"),
    ("music.end", "music.game.end"),
    ("music.nether", "music.game.nether"),
    // `World`'s cave ambience.
    ("ambient.cave", "ambient.cave.cave"),
    // `ButtonWidget.playClickSound`.
    ("ui.button.click", "gui.button.press"),
];

/// The era event an engine sound id asks for, as a `minecraft:` id, or `None` when the era has
/// no counterpart (the id then plays nothing unless its own name is a registered era event).
pub fn era_event(engine: &str) -> Option<String> {
    let path = engine.strip_prefix("minecraft:").unwrap_or(engine);
    if let Some((family, cause)) = path
        .strip_prefix("block.")
        .and_then(|rest| rest.rsplit_once('.'))
    {
        let sounds = BLOCK_SOUNDS.iter().find(|sounds| sounds.family == family)?;
        let (breaking, stepping, placing) = sounds.events();
        let event = match cause {
            "break" => breaking,
            "place" | "land" => placing,
            "step" | "hit" | "fall" => stepping,
            _ => return None,
        };
        return Some(format!("minecraft:{event}"));
    }
    RENAMES
        .iter()
        .find(|(engine, _)| *engine == path)
        .map(|(_, era)| format!("minecraft:{era}"))
}

/// Every engine id a block sound can ask for, with the era event [`era_event`] maps it to.
pub fn block_events() -> impl Iterator<Item = (String, String)> {
    BLOCK_SOUNDS.iter().flat_map(|sounds| {
        CAUSES.iter().map(move |cause| {
            let engine = format!("block.{}.{cause}", sounds.family);
            let era = era_event(&engine).expect("every family and cause maps");
            (format!("minecraft:{engine}"), era)
        })
    })
}

/// Adds the era table to a registry loaded from a 1.7.10 root: each mapped engine id plays the
/// era event the way a `{"type": "event"}` entry of `sounds.json` does, and only when that era
/// event is registered (its files were imported and passed the index's check).
pub fn add_engine_events(registry: &mut SoundRegistry) {
    let before = registry.names().count();
    let mapped = block_events()
        .chain(
            RENAMES
                .iter()
                .map(|(engine, era)| (format!("minecraft:{engine}"), format!("minecraft:{era}"))),
        )
        .collect::<Vec<_>>();
    for (engine, era) in &mapped {
        registry.add_event_reference(engine, era);
    }
    let added = registry.names().count() - before;
    if added > 0 {
        log::info!(
            "Sunrise: 1.7.10 sound table: {before} era events from sounds.json, {added} of {} \
             engine ids resolve to them",
            mapped.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_source::ResourceSource;
    use std::fs;
    use sunrise_core::random::LegacyRandom;

    #[test]
    fn the_block_families_name_the_eras_events() {
        // `Block.init`'s registrations, by the family the block facts use.
        for (block, breaking, stepping) in [
            ("stone", "dig.stone", "step.stone"),
            ("grass", "dig.grass", "step.grass"),
            ("gravel", "dig.gravel", "step.gravel"),
            ("sand", "dig.sand", "step.sand"),
            ("snow", "dig.snow", "step.snow"),
            ("wood", "dig.wood", "step.wood"),
            ("wool", "dig.cloth", "step.cloth"),
        ] {
            let event = format!("block.{block}.break");
            assert_eq!(
                era_event(&event).as_deref(),
                Some(format!("minecraft:{breaking}").as_str()),
                "{block} break"
            );
            let event = format!("block.{block}.place");
            assert_eq!(
                era_event(&event).as_deref(),
                Some(format!("minecraft:{breaking}").as_str()),
                "{block} place"
            );
            for cause in ["step", "hit", "fall"] {
                let event = format!("block.{block}.{cause}");
                assert_eq!(
                    era_event(&event).as_deref(),
                    Some(format!("minecraft:{stepping}").as_str()),
                    "{block} {cause}"
                );
            }
        }
        // `Block$96978149`: breaking is the glass sound, stepping and placing the stone ones.
        assert_eq!(
            era_event("block.glass.break").as_deref(),
            Some("minecraft:dig.glass")
        );
        assert_eq!(
            era_event("block.glass.place").as_deref(),
            Some("minecraft:step.stone")
        );
        assert_eq!(
            era_event("block.glass.step").as_deref(),
            Some("minecraft:step.stone")
        );
        // `Block$48198598`: breaking and placing are wood, stepping keeps the ladder's key.
        for cause in ["break", "place"] {
            let event = format!("block.ladder.{cause}");
            assert_eq!(
                era_event(&event).as_deref(),
                Some("minecraft:dig.wood"),
                "ladder {cause}"
            );
        }
        assert_eq!(
            era_event("block.ladder.step").as_deref(),
            Some("minecraft:step.ladder")
        );
        // `Block$23487049`: the anvil's placement has its own event; its step names one the
        // era's sounds.json does not define.
        assert_eq!(
            era_event("block.anvil.break").as_deref(),
            Some("minecraft:dig.stone")
        );
        assert_eq!(
            era_event("block.anvil.land").as_deref(),
            Some("minecraft:random.anvil_land")
        );
        assert_eq!(
            era_event("block.anvil.step").as_deref(),
            Some("minecraft:step.anvil")
        );
        assert_eq!(era_event("block.unknown.break"), None);
        assert_eq!(era_event("block.stone.explode"), None);
        assert_eq!(era_event("entity.zombie.ambient"), None);
    }

    #[test]
    fn renamed_events_keep_the_eras_names() {
        for (engine, era) in [
            ("music.creative", "music.game.creative"),
            ("music.credits", "music.game.end.credits"),
            ("music.dragon", "music.game.end.dragon"),
            ("music.end", "music.game.end"),
            ("music.nether", "music.game.nether"),
            ("ambient.cave", "ambient.cave.cave"),
            ("ui.button.click", "gui.button.press"),
        ] {
            assert_eq!(
                era_event(&format!("minecraft:{engine}")).as_deref(),
                Some(format!("minecraft:{era}").as_str())
            );
        }
    }

    /// A 1.7.10-shaped root: the era's shape markers, its `sounds.json` and a file for every
    /// event the cases play.
    fn era_root(label: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("sunrise-sound-17-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let assets = root.join("assets/minecraft");
        for marker in ["font/glyph_sizes.bin", "textures/font/ascii.png"] {
            let path = assets.join(marker);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"marker").unwrap();
        }
        for sound in [
            "dig/stone1",
            "dig/stone2",
            "step/stone1",
            "dig/grass1",
            "step/grass1",
            "dig/gravel1",
            "step/gravel1",
            "dig/sand1",
            "step/sand1",
            "dig/snow1",
            "step/snow1",
            "dig/wood1",
            "step/wood1",
            "dig/cloth1",
            "step/cloth1",
            "dig/glass1",
            "step/ladder1",
            "random/anvil_land",
            "random/click",
            "music/game/creative1",
            "music/menu/menu1",
            "ambient/cave/cave1",
        ] {
            let path = assets.join(format!("sounds/{sound}.ogg"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"OggS").unwrap();
        }
        fs::write(
            assets.join("sounds.json"),
            br#"{
                "dig.stone": {"category": "block", "sounds": ["dig/stone1", "dig/stone2"]},
                "step.stone": {"category": "neutral", "sounds": ["step/stone1"]},
                "dig.grass": {"category": "block", "sounds": ["dig/grass1"]},
                "step.grass": {"category": "neutral", "sounds": ["step/grass1"]},
                "dig.gravel": {"category": "block", "sounds": ["dig/gravel1"]},
                "step.gravel": {"category": "neutral", "sounds": ["step/gravel1"]},
                "dig.sand": {"category": "block", "sounds": ["dig/sand1"]},
                "step.sand": {"category": "neutral", "sounds": ["step/sand1"]},
                "dig.snow": {"category": "block", "sounds": ["dig/snow1"]},
                "step.snow": {"category": "neutral", "sounds": ["step/snow1"]},
                "dig.wood": {"category": "block", "sounds": ["dig/wood1"]},
                "step.wood": {"category": "neutral", "sounds": ["step/wood1"]},
                "dig.cloth": {"category": "block", "sounds": ["dig/cloth1"]},
                "step.cloth": {"category": "neutral", "sounds": ["step/cloth1"]},
                "dig.glass": {"category": "block", "sounds": ["dig/glass1"]},
                "step.ladder": {"category": "neutral", "sounds": ["step/ladder1"]},
                "random.anvil_land": {"category": "block", "sounds": ["random/anvil_land"]},
                "gui.button.press": {"category": "player", "sounds": ["random/click"]},
                "music.game.creative": {"category": "music", "sounds": ["music/game/creative1"]},
                "music.menu": {"category": "music", "sounds": ["music/menu/menu1"]},
                "ambient.cave.cave": {"category": "ambient", "sounds": ["ambient/cave/cave1"]}
            }"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn the_engine_ids_play_the_eras_files() {
        let root = era_root("files");
        let registry = SoundRegistry::load(&ResourceSource::directory(root.clone()));
        let play = |event: &str| {
            let mut random = LegacyRandom::new(7);
            registry
                .sound(event, &mut random)
                .map(|sound| sound.path().1)
        };
        // 21 era events, plus every mapped engine id whose era event is registered: all 60
        // block ids except the anvil's three step-event ids (`step.anvil` is absent), and 3 of
        // the 7 renames (the era has no music.game.end/nether/credits here).
        assert_eq!(registry.names().count(), 21 + 57 + 3);
        // Stone's break picks one of its two era files, as `WeighedSoundEvents.getSound` does.
        let stone = play("minecraft:block.stone.break").unwrap();
        assert!(
            stone == "sounds/dig/stone1.ogg" || stone == "sounds/dig/stone2.ogg",
            "{stone}"
        );
        assert_eq!(
            play("minecraft:block.grass.step").unwrap(),
            "sounds/step/grass1.ogg"
        );
        // Torch is wood: placing asks for dig.wood.
        assert_eq!(
            play("minecraft:block.wood.place").unwrap(),
            "sounds/dig/wood1.ogg"
        );
        assert_eq!(
            play("minecraft:block.wool.break").unwrap(),
            "sounds/dig/cloth1.ogg"
        );
        assert_eq!(
            play("minecraft:block.wool.step").unwrap(),
            "sounds/step/cloth1.ogg"
        );
        // Glass breaks with its own sound and places/steps on stone.
        assert_eq!(
            play("minecraft:block.glass.break").unwrap(),
            "sounds/dig/glass1.ogg"
        );
        assert_eq!(
            play("minecraft:block.glass.place").unwrap(),
            "sounds/step/stone1.ogg"
        );
        assert_eq!(
            play("minecraft:block.ladder.step").unwrap(),
            "sounds/step/ladder1.ogg"
        );
        assert_eq!(
            play("minecraft:block.anvil.land").unwrap(),
            "sounds/random/anvil_land.ogg"
        );
        // The era has no `step.anvil`, so the engine id has no sound, as in vanilla.
        assert_eq!(play("minecraft:block.anvil.step"), None);
        // Renames and names the era shares.
        assert_eq!(
            play("minecraft:ui.button.click").unwrap(),
            "sounds/random/click.ogg"
        );
        assert_eq!(
            play("minecraft:music.creative").unwrap(),
            "sounds/music/game/creative1.ogg"
        );
        assert_eq!(
            play("minecraft:music.menu").unwrap(),
            "sounds/music/menu/menu1.ogg"
        );
        assert_eq!(
            play("minecraft:ambient.cave").unwrap(),
            "sounds/ambient/cave/cave1.ogg"
        );
        // Nothing mapped: mobs, weather and the rest of 26.3's ids.
        assert_eq!(play("minecraft:entity.zombie.ambient"), None);
        assert_eq!(play("minecraft:weather.rain"), None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn without_the_eras_sounds_json_nothing_is_mapped() {
        let root = era_root("empty");
        fs::remove_file(root.join("assets/minecraft/sounds.json")).unwrap();
        let registry = SoundRegistry::load(&ResourceSource::directory(root.clone()));
        assert!(
            registry.is_empty(),
            "a root with no sounds.json must stay empty"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
