//! The Rust side of a later parity check: build a fresh world's `level.dat`
//! and a stored chunk the way 1.7.10 does, read them back, and print a
//! deterministic digest of the bytes written. A Java oracle (1.7.10's
//! `AnvilWorldStorage.saveData` and `AnvilChunkStorage.saveChunk` over the
//! same inputs) must answer with the same digests when the JDK's zlib
//! matches `sunrise-deflate`'s.

use std::{
    collections::hash_map::DefaultHasher,
    fmt::Write as _,
    fs,
    hash::{Hash, Hasher},
};
use sunrise_core::nbt::{Compound, JavaString, Tag};
use sunset_1_7_save::{
    level_dat::{self, GameMode, WorldData},
    player::PlayerData,
    region::region_path,
    region_io::RegionIo,
};

fn digest(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{} bytes, {:016x}", bytes.len(), hasher.finish())
}

fn main() {
    let dir = std::env::temp_dir().join(format!("sunset-1-7-save-probe-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let now = 1_700_000_000_000i64;

    // A fresh world, as `new WorldData(settings, name)` and the first save
    // make it.
    let mut world = WorldData {
        seed: -4172829371415413273,
        name: "Probe".into(),
        game_mode: GameMode::Survival,
        map_features: true,
        allow_commands: false,
        initialized: false,
        ..WorldData::default()
    };
    // The single player, as `PlayerEntity.writeNbtWithoutId` writes them.
    let player = PlayerData {
        pos: [-192.5, 64.0, 208.25],
        rotation: [137.5, -2.0],
        fire: -1,
        air: 300,
        on_ground: true,
        uuid: [-6140401986334597049, -6899682100637659250],
        ..PlayerData::default()
    };

    let mut out = String::new();
    // level.dat: the first save leaves level.dat and no level.dat_old yet.
    level_dat::save(&dir, &mut world, now).unwrap();
    let bytes = fs::read(dir.join("level.dat")).unwrap();
    let _ = writeln!(out, "level.dat: {}", digest(&bytes));
    let read = WorldData::from_nbt(&level_dat::load(&dir).unwrap());
    let _ = writeln!(
        out,
        "level.dat roundtrip: seed {} name {} spawn {} {} {} time {} mode {} version {}",
        read.seed,
        read.name,
        read.spawn_x,
        read.spawn_y,
        read.spawn_z,
        read.time,
        read.game_mode.id(),
        read.version,
    );

    // A stored chunk: a compound wrapping a Level, as saveChunk writes one.
    let mut level = Compound::default();
    level.insert(JavaString::from("V"), Tag::Byte(1));
    level.insert(JavaString::from("xPos"), Tag::Int(5));
    level.insert(JavaString::from("zPos"), Tag::Int(-3));
    level.insert(JavaString::from("LastUpdate"), Tag::Long(1_234));
    let mut root = Compound::default();
    root.insert(JavaString::from("Level"), Tag::Compound(level));
    let raw = sunrise_core::nbt::write_named(
        &sunrise_core::nbt::NamedTag {
            name: JavaString::default(),
            tag: Tag::Compound(root),
        },
        sunrise_core::nbt::Limits::saved(),
    )
    .unwrap();

    let mut io = RegionIo::new();
    io.write_chunk(&dir, 5, -3, &raw, 1_700_000_000).unwrap();
    let region = fs::read(region_path(&dir, 5, -3)).unwrap();
    let _ = writeln!(out, "region: {}", digest(&region));
    let chunk = io.chunk_data(&dir, 5, -3).unwrap().unwrap();
    let _ = writeln!(
        out,
        "chunk roundtrip: {} bytes, timestamp {}",
        chunk.decompress(1 << 20).unwrap().len(),
        chunk.timestamp,
    );

    // playerdata: <uuid>.dat with the same layout.
    let uuid = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
    sunset_1_7_save::player::save(&dir, uuid, &player).unwrap();
    let bytes = fs::read(dir.join(format!("playerdata/{uuid}.dat"))).unwrap();
    let _ = writeln!(out, "player.dat: {}", digest(&bytes));
    let read = sunset_1_7_save::player::load(&dir, uuid).unwrap().unwrap();
    let _ = writeln!(
        out,
        "player roundtrip: pos {} {} {} health {} food {} fly {}",
        read.pos[0],
        read.pos[1],
        read.pos[2],
        read.health,
        read.food_level,
        read.abilities.fly_speed,
    );

    println!("{out}");
    let _ = fs::remove_dir_all(&dir);
}
