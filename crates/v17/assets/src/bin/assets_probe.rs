//! Deterministic probe of the 1.7.10 asset pipeline, for later parity
//! checks: registry counts, sample textures, lang formatting, font widths
//! and `sounds.json` parsing. Same output run to run.

use sunset_1_7_assets::font;
use sunset_1_7_assets::identifier::Identifier;
use sunset_1_7_assets::lang::{FormatArg, Locale};
use sunset_1_7_assets::resource::{
    MemoryResourcePack, PackMetadata, SimpleReloadableResourceManager,
};
use sunset_1_7_assets::sound::{ResolvedEvent, SoundRegistry, parse_sounds_json};
use sunset_1_7_assets::sprites;

fn main() {
    println!("== registries ==");
    println!("blocks {}", sprites::block_count());
    println!("items {}", sprites::item_count());

    println!("== block textures ==");
    for (id, meta, face) in [
        (1i32, 0i32, 1u8),
        (2, 0, 1),
        (2, 0, 2),
        (3, 2, 1),
        (5, 2, 1),
        (6, 5, 1),
        (17, 4, 4),
        (18, 2, 1),
        (24, 2, 2),
        (35, 0, 1),
        (35, 15, 1),
        (44, 1, 1),
        (61, 2, 2),
        (86, 3, 5),
        (98, 3, 1),
        (99, 14, 1),
        (155, 2, 1),
        (175, 12, 1),
    ] {
        println!(
            "block {id}/{meta}/{face} = {}",
            sprites::block_texture(id, meta, face)
                .as_deref()
                .unwrap_or("(none)")
        );
    }

    println!("== item textures ==");
    for (id, meta) in [
        (256i32, 0i32),
        (263, 1),
        (349, 1),
        (350, 1),
        (351, 15),
        (373, 16384),
        (397, 3),
        (2256, 0),
        (5, 2),
        (35, 0),
        (154, 0),
    ] {
        println!(
            "item {id}/{meta} = {}",
            sprites::item_texture(id, meta)
                .as_deref()
                .unwrap_or("(none)")
        );
    }

    println!("== lang ==");
    let mut locale = Locale::new();
    locale
        .load_bytes(b"tile.stone.name=Stone\nitems.count=%s items\npotion.prefix.gone=Format %d\n");
    println!("{}", locale.translate("tile.stone.name", &[]));
    println!(
        "{}",
        locale.translate("items.count", &[FormatArg::Text("three")])
    );
    println!("{}", locale.translate("tile.missing.name", &[]));

    println!("== font ==");
    let widths = font::compute_char_widths(&font::solid_sheet(), 128, 128);
    println!("space width {}", widths[32]);
    println!("unicode page 0 {}", font::unicode_page_location(0));
    println!("unicode page 10 {}", font::unicode_page_location(10));

    println!("== sounds.json ==");
    let text = r#"{"random.explode": {"category": "block", "sounds": ["explode1", {"name": "explode2", "volume": 0.5, "weight": 2}]}}"#;
    let events = parse_sounds_json("minecraft", text).unwrap();
    for (location, entry) in &events {
        println!(
            "event {location} category {} sounds {}",
            location,
            entry.sounds.len()
        );
    }
    let resolved = ResolvedEvent::resolve(&events[0].1, "minecraft", |_| true);
    for container in &resolved.containers {
        if let sunset_1_7_assets::sound::Container::File(sound) = container {
            println!(
                "file {} volume {} weight {}",
                sound.location, sound.volume, sound.weight
            );
        }
    }

    println!("== resource manager ==");
    let mut pack = MemoryResourcePack::new("test");
    pack.add("minecraft", "lang/en_US.lang", b"key=value".to_vec());
    let mut manager = SimpleReloadableResourceManager::new();
    manager.add_pack(std::rc::Rc::new(pack), &["minecraft"]);
    let resource = manager
        .get_resource(&Identifier::parse("minecraft:lang/en_US.lang"))
        .unwrap();
    println!("resource {} bytes", resource.data.len());

    let metadata =
        PackMetadata::parse(br#"{"pack":{"pack_format":1,"description":"1.7.10"}}"#).unwrap();
    println!(
        "pack_format {} matches_game {}",
        metadata.pack_format,
        metadata.matches_game()
    );

    let mut registry = SoundRegistry::new();
    registry
        .register_sounds_json("minecraft", text, |_| true)
        .unwrap();
    println!("registered events {}", registry.event_count());
}
