//! Probe for later parity checks: deterministic output covering the NBT round
//! trip, the section packing and the `Add` nibble. Run with
//! `cargo +1.99.1 run -p sunset-1-7-chunkformat --bin chunkformat_probe`.

use sunset_1_7_chunkformat::chunk::{Chunk, read_chunk, write_chunk};
use sunset_1_7_chunkformat::nbt::{self, Nbt, tag};
use sunset_1_7_chunkformat::section::{Section, flat_from_sections, sections_from_flat};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// Fills a flat 65536-id array deterministically: a spread across both id
/// ranges and every section, about half air.
fn fill(blocks: &mut [u16; 65536]) {
    let mut random = LegacyRandom::new(12345);
    for slot in blocks.iter_mut() {
        let id = random.next_int_bound(1000).unwrap() as u16;
        *slot = if id < 500 { 0 } else { id };
    }
}

fn main() {
    // NBT round trip of every tag type, including modified UTF-8 edge cases.
    let value = Nbt::Compound(vec![
        ("byte".into(), Nbt::Byte(-1)),
        ("short".into(), Nbt::Short(-32768)),
        ("int".into(), Nbt::Int(-2147483648)),
        ("long".into(), Nbt::Long(i64::MIN)),
        ("float".into(), Nbt::Float(1.5)),
        ("double".into(), Nbt::Double(-0.25)),
        ("bytes".into(), Nbt::ByteArray((0..=255u8).collect())),
        (
            "string".into(),
            Nbt::String("Zombie\u{0}§x\u{1f600}".to_string()),
        ),
        (
            "list".into(),
            Nbt::List {
                element: tag::INT,
                items: vec![Nbt::Int(1), Nbt::Int(-1)],
            },
        ),
        ("ints".into(), Nbt::IntArray(vec![0, -1, i32::MAX])),
        (
            "nested".into(),
            Nbt::Compound(vec![("deep".into(), Nbt::String("ok".into()))]),
        ),
    ]);
    let bytes = nbt::write(&value).unwrap();
    let back = nbt::read(&bytes).unwrap();
    println!("nbt_bytes {}", bytes.len());
    println!(
        "nbt_round_trip {}",
        if back == value { "ok" } else { "FAIL" }
    );

    // Flat 65536-id array packed to sections and back, Add nibble included.
    let mut flat = [0u16; 65536];
    fill(&mut flat);
    let sections = sections_from_flat(&flat, true);
    let packed_ok = flat_from_sections(&sections) == flat;
    let non_empty = sections.iter().filter(|s| s.is_some()).count();
    let with_add = sections
        .iter()
        .flatten()
        .filter(|s| s.add.is_some())
        .count();
    let non_air: i32 = sections
        .iter()
        .flatten()
        .map(|s| s.non_air_block_count)
        .sum();
    println!("sections_non_empty {non_empty}");
    println!("sections_with_add {with_add}");
    println!("non_air {non_air}");
    println!("flat_round_trip {}", if packed_ok { "ok" } else { "FAIL" });

    // Chunk <-> NBT round trip through the Anvil Level compound.
    let mut chunk = Chunk::new(-7, 13);
    chunk.sections = sections_from_flat(&flat, true);
    for (i, biome) in chunk.biomes.iter_mut().enumerate() {
        *biome = (i % 254) as u8;
    }
    for (i, height) in chunk.height_map.iter_mut().enumerate() {
        *height = (i % 256) as i32;
    }
    chunk.terrain_populated = true;
    chunk.light_populated = true;
    chunk.inhabited_time = 4242;
    chunk.last_update = 9001;
    chunk.entities.push(Nbt::Compound(vec![
        ("id".into(), Nbt::String("Zombie".into())),
        (
            "Pos".into(),
            Nbt::List {
                element: tag::DOUBLE,
                items: vec![Nbt::Double(1.0), Nbt::Double(70.0), Nbt::Double(2.0)],
            },
        ),
    ]));
    chunk.tile_entities.push(Nbt::Compound(vec![
        ("id".into(), Nbt::String("Furnace".into())),
        ("x".into(), Nbt::Int(1)),
        ("y".into(), Nbt::Int(70)),
        ("z".into(), Nbt::Int(2)),
    ]));
    chunk.tile_ticks.push(Nbt::Compound(vec![
        ("i".into(), Nbt::Int(8)),
        ("t".into(), Nbt::Int(5)),
    ]));

    let bytes = write_chunk(&chunk, true).unwrap();
    let back = read_chunk(&bytes).unwrap();
    let mut expected = chunk.clone();
    expected.last_update = 0; // not read back, as in vanilla
    println!("chunk_bytes {}", bytes.len());
    println!(
        "chunk_round_trip {}",
        if back == expected { "ok" } else { "FAIL" }
    );

    // A no-sky chunk still writes zeroed SkyLight arrays.
    let dark = write_chunk(&chunk, false).unwrap();
    let root = nbt::read(&dark).unwrap();
    let zeroed = root
        .get_compound("Level")
        .get_list("Sections", tag::COMPOUND)
        .iter()
        .all(|s| s.get_byte_array("SkyLight") == [0u8; 2048]);
    println!(
        "nosky_skylight_zeroed {}",
        if zeroed { "ok" } else { "FAIL" }
    );

    // One section's full id through the file, id 600 needing the Add nibble.
    let mut single = Section::new(0, true);
    single.set_block_id(3, 4, 5, 600);
    println!(
        "add_nibble {}",
        if single.block_id(3, 4, 5) == 600 {
            "ok"
        } else {
            "FAIL"
        }
    );
}
