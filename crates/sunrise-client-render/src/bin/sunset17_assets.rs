//! Opens a prepared 1.7.10 content root and prints what the renderer resolves from it: the
//! stitched sprite array, a handful of block sprites with their pixel sizes, the GUI atlas, the
//! language keys and the bitmap font.
//!
//! Usage: `sunset17_assets [ROOT]` (`SUNSET_1_7_CONTENT` or `.local/sunrise-client/1.7.10` by
//! default).

use std::path::PathBuf;
use sunrise_client_render::{
    assets::RenderAssets,
    hud::HudAtlas,
    resource_source::{ResourceRoot, ResourceSource},
    sprites::Sprites,
    sunset17_assets::{self, FACE_BOTTOM, FACE_EAST, FACE_NORTH, FACE_SOUTH, FACE_TOP, FACE_WEST},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SUNSET_1_7_CONTENT").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(sunset17_assets::DEFAULT_DIR));
    if !sunset17_assets::detect(&root) {
        return Err(format!(
            "{} is not a prepared 1.7.10 root: no {}",
            root.display(),
            sunset17_assets::MANIFEST
        )
        .into());
    }

    let assets = sunset17_assets::Assets::load(&root)?;
    let source = ResourceSource::directory(root.clone());
    let mut sprites = Sprites::new(source.clone());
    assets.insert_sprites(&mut sprites)?;
    sprites.finish();

    println!("1.7.10 assets from {}", root.display());
    println!(
        "  manifest: {} files, {} block ids ({} textured, {} textureless: {:?})",
        assets.files().len(),
        assets.blocks().len(),
        assets.textured_blocks(),
        assets.textureless().len(),
        assets.textureless()
    );
    println!(
        "  atlas: {} layers at {} px (smallest {}, largest {}), {} block textures, {} item textures, {} animations",
        sprites.count(),
        sprites.size,
        sprites.minimum_size,
        sprites
            .layers
            .values()
            .filter_map(|&layer| sprites.native_size(layer))
            .max()
            .unwrap_or(0),
        assets.block_texture_files().count(),
        assets.item_texture_files().count(),
        sprites.animations.len()
    );

    // A handful of blocks across the era's shapes: full cube, grass, wool, a log axis, stairs,
    // the two liquids (still and flowing), a torch and tall grass.
    let cases: &[(u16, u8, u8, &str)] = &[
        (1, 0, FACE_NORTH, "cube"),
        (2, 0, FACE_TOP, "grass top"),
        (2, 0, FACE_NORTH, "grass side"),
        (35, 14, FACE_EAST, "wool (red)"),
        (17, 4, FACE_WEST, "log axis"),
        (17, 4, FACE_NORTH, "log side"),
        (53, 0, FACE_SOUTH, "stairs"),
        (9, 0, FACE_TOP, "water still"),
        (9, 0, FACE_NORTH, "water flowing"),
        (11, 0, FACE_TOP, "lava still"),
        (11, 0, FACE_NORTH, "lava flowing"),
        (50, 0, FACE_BOTTOM, "torch"),
        (31, 1, FACE_SOUTH, "tall grass"),
    ];
    println!("  block sprites:");
    for &(id, metadata, face, label) in cases {
        let Some(texture) = sunset17_assets::block_texture(id, metadata, face) else {
            println!("    {id}:{metadata} face {face} ({label}): no sprite");
            continue;
        };
        let name = sunset17_assets::sprite_name(&texture);
        let layer = sunset17_assets::sprite_layer(&sprites, &texture)
            .ok_or_else(|| format!("{name} did not reach the atlas"))?;
        println!(
            "    {label:<14} block {id:>3}:{metadata:<2} face {face} -> {:<14} {name:<28} layer {layer:<4} {} px (atlas {} px)",
            texture.sprite(),
            sprites.native_size(layer).unwrap_or(0),
            sprites.size
        );
    }

    // The renderer's own entry point for the same root: same layers, stitched into the store
    // the game's renderer consumes.
    let render = RenderAssets::load_1_7_10(&ResourceRoot::directory(root.clone()))?;
    println!(
        "  RenderAssets: {} layers at {} px, {} animations, no block models (1.7.10 has none)",
        render.texture_layers,
        render.sprite_size,
        render.animations.len()
    );

    // GUI atlas: the era's screens and font sheets, packed by the HUD's own atlas.
    let hud = HudAtlas::build(&source)?;
    let gui = hud
        .sprites
        .iter()
        .filter(|(name, _)| name.starts_with("gui/"))
        .count();
    println!(
        "  GUI atlas: {}x{} px, {gui} gui textures, gui/icons {}x{} GUI px, {} font sheets",
        hud.width,
        hud.height,
        hud.sprites["gui/icons"].w,
        hud.sprites["gui/icons"].h,
        hud.sprites
            .keys()
            .filter(|name| name.starts_with("font/"))
            .count()
    );
    println!(
        "  font: A advance {}, space {}, U+4E00 advance {}, glyph height {}",
        hud.font.glyph('A').advance,
        hud.font.glyph(' ').advance,
        hud.font.glyph('\u{4e00}').advance,
        hud.font.glyph('A').bottom - hud.font.glyph('A').top
    );

    let language = assets.language()?;
    println!(
        "  language: {} keys; tile.stone.name={:?}, item.stick.name={:?}, gui.done={:?}",
        language.len(),
        language.get("tile.stone.name"),
        language.get("item.stick.name"),
        language.get("gui.done")
    );
    Ok(())
}
