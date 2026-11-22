//! Minecraft 1.7.10's own resources for the renderer: the era's individual block and item
//! textures, its GUI screens and fonts, and its `.lang` file.
//!
//! The prepared 1.7.10 folder (`sunrise-prepare`, `.local/sunrise-client/1.7.10`) keeps the
//! client JAR's `assets/minecraft/` tree exactly as the era laid it out: each block texture is a
//! file under `textures/blocks/`, each item texture under `textures/items/`, and the GUI, font
//! and language files beside them. That is a different shape from 26.3's model-driven atlases,
//! which is why this module exists: [`Assets`] reads the extraction's manifest
//! (the extraction's manifest), [inserts every texture the era's atlases stitched][Assets::insert_sprites]
//! into the renderer's shared sprite array under names of the era's own folders
//! (`minecraft:blocks/stone`, `minecraft:items/stick`), and answers the id → sprite question
//! through the ported `Block.getSprite`/`Item.getSprite` table.
//!
//! The content root's [`Profile`](sunrise_core::profile::Profile) decides: a
//! prepared 1.7.10 folder carries the extraction's manifest, and a layered pack
//! stack over it keeps that profile. No flag switches this on, so
//! `--1-7-10-terrain` and its export always load the same resources.
//!
//! The GUI atlas (`HudAtlas`) and the font (`Font`) read the same profile from
//! their resource source: [`HudAtlas::build`](crate::hud::HudAtlas::build) then
//! packs `textures/gui` and `textures/font`, and the font loader reads
//! `ascii.png`, the unicode pages and `glyph_sizes.bin` instead of the modern
//! `font/*.json` definitions.

use crate::{Result, sprites::Sprites};
use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
};
use sunrise_client_core::lang::Language;

/// The era's two texture atlases.
pub use sunrise_prepare::assets_1_7_10::Atlas;
/// The extraction's manifest, beside the prepared assets.
pub use sunrise_prepare::assets_1_7_10::MANIFEST;
/// The manifest's layout.
pub use sunrise_prepare::assets_1_7_10::MANIFEST_SCHEMA;
/// One sprite the era registers, with the atlas it belongs to.
pub use sunrise_prepare::assets_1_7_10::Texture;
/// The version this module reads.
pub use sunrise_prepare::assets_1_7_10::VERSION;
/// Vanilla's face ids, in its order.
pub use sunrise_prepare::assets_1_7_10::{
    FACE_BOTTOM, FACE_EAST, FACE_NORTH, FACE_SOUTH, FACE_TOP, FACE_WEST, FACES,
};
/// The block and item id → sprite table, `Block.getSprite`/`Item.getSprite` ported.
pub use sunrise_prepare::assets_1_7_10::{
    block_has_texture, block_texture, block_textures, item_texture, textureless_blocks,
};
/// The era's entity models and types, transcribed from its `Model*` and entity classes.
pub use sunrise_prepare::entities_1_7_10::{
    Bone, Cube, EntityType, Model, TextureName, bake_models, bake_types, entity_types,
    model_space_bounds, models, textures,
};
/// The baked part trees the engine loads with the era's entity types.
pub use sunrise_prepare::entities_1_7_10::{
    MODELS as ENTITY_MODELS_FILE, TYPES as ENTITY_TYPES_FILE,
};

/// Where the 1.7.10 export lives under the Sunrise folder, as the preparer writes it.
pub const DEFAULT_DIR: &str = sunrise_prepare::assets_1_7_10::DEFAULT_DIR;

/// The block textures' folder in the era's resource tree.
pub const BLOCKS_TEXTURES: &str = "assets/minecraft/textures/blocks/";
/// The item textures' folder in the era's resource tree.
pub const ITEMS_TEXTURES: &str = "assets/minecraft/textures/items/";
/// The GUI textures' folder.
pub const GUI_TEXTURES: &str = "assets/minecraft/textures/gui/";
/// The font sheets' folder.
pub const FONT_TEXTURES: &str = "assets/minecraft/textures/font/";
/// The entity sheets' folder: every model's texture, and the era's player skin.
pub const ENTITY_TEXTURES: &str = "assets/minecraft/textures/entity/";
/// `TextRenderer`'s glyph-width table.
pub const GLYPH_SIZES: &str = "assets/minecraft/font/glyph_sizes.bin";
/// The era's English language file.
pub const ENGLISH_LANG: &str = "assets/minecraft/lang/en_US.lang";

/// One manifest block row: the id, its registry name and every sprite any metadata or face
/// registers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRow {
    pub id: u16,
    pub name: String,
    /// Every block-atlas sprite this block registers across metadata 0..=15 and all six faces.
    pub sprites: Vec<String>,
    /// 1.7.10 registers no block-atlas sprite for it (air is the only one).
    pub textureless: bool,
}

/// The prepared 1.7.10 folder's manifest: the extracted files and the block sprite coverage.
#[derive(Clone, Debug)]
pub struct Assets {
    root: PathBuf,
    files: Vec<String>,
    blocks: Vec<BlockRow>,
}

#[derive(Deserialize)]
struct Manifest {
    schema: i64,
    minecraft: String,
    files: Vec<FileRow>,
    blocks: Vec<BlockRowJson>,
}
#[derive(Deserialize)]
struct FileRow {
    path: String,
}
#[derive(Deserialize)]
struct BlockRowJson {
    id: u16,
    name: String,
    #[serde(default)]
    textureless: bool,
    #[serde(default)]
    sprites: Vec<String>,
}

impl Assets {
    /// Read `root`'s extraction manifest and check it agrees with the extracted files.
    pub fn load(root: &Path) -> Result<Self> {
        let bytes = std::fs::read(root.join(MANIFEST))?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        if manifest.schema != MANIFEST_SCHEMA {
            return Err(format!(
                "{} has manifest schema {}, not {MANIFEST_SCHEMA}: extract the assets again",
                root.join(MANIFEST).display(),
                manifest.schema
            )
            .into());
        }
        if manifest.minecraft != VERSION {
            return Err(format!(
                "{} is for Minecraft {}, not {VERSION}",
                root.join(MANIFEST).display(),
                manifest.minecraft
            )
            .into());
        }
        let files: Vec<String> = manifest.files.into_iter().map(|file| file.path).collect();
        let blocks: Vec<BlockRow> = manifest
            .blocks
            .into_iter()
            .map(|row| BlockRow {
                id: row.id,
                name: row.name,
                sprites: row.sprites,
                textureless: row.textureless,
            })
            .collect();
        let present: BTreeSet<&str> = files.iter().map(String::as_str).collect();
        for row in &blocks {
            for sprite in &row.sprites {
                let path = block_sprite_path(sprite);
                if !present.contains(path.as_str()) {
                    return Err(format!(
                        "{} names the block sprite {sprite} but the extraction has no {path}",
                        root.join(MANIFEST).display()
                    )
                    .into());
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            files,
            blocks,
        })
    }

    /// The prepared folder this manifest came from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every extracted file, relative to the folder and in the preparer's order.
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// The block ids the content export has, in registry order.
    pub fn blocks(&self) -> &[BlockRow] {
        &self.blocks
    }

    /// The block ids with no block-atlas sprite (1.7.10's air).
    pub fn textureless(&self) -> Vec<u16> {
        self.blocks
            .iter()
            .filter(|row| row.textureless)
            .map(|row| row.id)
            .collect()
    }

    /// How many block ids register at least one sprite.
    pub fn textured_blocks(&self) -> usize {
        self.blocks.iter().filter(|row| !row.textureless).count()
    }

    /// The block textures the extraction has, by stem (`stone`, `grass_top`).
    pub fn block_texture_files(&self) -> impl Iterator<Item = &str> {
        texture_stems(&self.files, BLOCKS_TEXTURES)
    }

    /// The item textures the extraction has, by stem (`stick`).
    pub fn item_texture_files(&self) -> impl Iterator<Item = &str> {
        texture_stems(&self.files, ITEMS_TEXTURES)
    }

    /// The GUI textures the extraction has.
    pub fn gui_texture_files(&self) -> impl Iterator<Item = &str> {
        png_paths(&self.files, GUI_TEXTURES)
    }

    /// The font sheets the extraction has.
    pub fn font_texture_files(&self) -> impl Iterator<Item = &str> {
        png_paths(&self.files, FONT_TEXTURES)
    }

    /// The entity sheets the extraction has (the player's skin, mob textures, overlays).
    pub fn entity_texture_files(&self) -> impl Iterator<Item = &str> {
        png_paths(&self.files, ENTITY_TEXTURES)
    }

    /// Stitch every block and item texture of the era's atlases into the renderer's shared
    /// sprite array.
    ///
    /// 1.7.10's `TextureMap` walks `textures/blocks` and `textures/items` and registers every
    /// file it finds, not only the ones a block names, so both folders go in whole. A name is
    /// the era's folder below `textures/` (`minecraft:blocks/stone`), which is also the file the
    /// renderer's bare-source lookup reads (`assets/minecraft/textures/blocks/stone.png`).
    /// Metadata variants need no separate entry: the lookup names the same sprite the table
    /// gives, and that sprite is in the array.
    pub fn insert_sprites(&self, sprites: &mut Sprites) -> Result<()> {
        for stem in self.block_texture_files() {
            sprites.try_insert(&format!("minecraft:blocks/{stem}"))?;
        }
        for stem in self.item_texture_files() {
            sprites.try_insert(&format!("minecraft:items/{stem}"))?;
        }
        Ok(())
    }

    /// Load the era's language (`assets/minecraft/lang/en_US.lang`) into the engine's language
    /// type. 1.7.10 has one language file, `en_US`; its values come from the ported `Locale`
    /// parser, which rewrites `%d`/`%f` placeholders the way the era's formatter did.
    pub fn language(&self) -> Result<Language> {
        let bytes = std::fs::read(self.root.join(ENGLISH_LANG))?;
        let mut locale = sunset_1_7_assets::lang::Locale::new();
        locale.load_bytes(&bytes);
        let entries = parse_lang(&bytes).into_iter().filter_map(|key| {
            let value = locale.get(&key)?.to_owned();
            Some((key, value))
        });
        Ok(Language::of("en_us", entries))
    }
}

/// The atlas name of a sprite: `minecraft:blocks/<sprite>` or `minecraft:items/<sprite>`.
///
/// This is the key [`Sprites::try_insert`] registers it under and the one
/// [`sprite_layer`] finds it by.
pub fn sprite_name(texture: &Texture) -> String {
    let folder = match texture.atlas() {
        Atlas::Blocks => "blocks",
        Atlas::Items => "items",
    };
    format!("minecraft:{folder}/{}", texture.sprite())
}

/// The renderer's array layer for a sprite, once [`Assets::insert_sprites`] added it.
pub fn sprite_layer(sprites: &Sprites, texture: &Texture) -> Option<u32> {
    sprites.layers.get(&sprite_name(texture)).copied()
}

/// The file a block sprite is extracted to under the prepared folder.
pub fn block_sprite_path(sprite: &str) -> String {
    format!("{BLOCKS_TEXTURES}{sprite}.png")
}

/// The HUD atlas's name for an era entity file: `textures/entity/pig/pig.png` is packed as
/// `entity/pig/pig` (the path below `textures/`, without the extension).
pub fn entity_sprite(path: &str) -> Option<String> {
    path.strip_prefix("textures/")?
        .strip_suffix(".png")
        .map(str::to_owned)
}

/// Give the era's entity sheets the 26.3 names the engine's species table asks for.
///
/// The HUD atlas packs `textures/entity` under the era's own names (`entity/steve`,
/// `entity/pig/pig`), while a mob's `MobType` names its sheet the way 26.3 lays it out
/// (`entity/player/wide/steve`, `entity/pig/pig_temperate`). This points each 26.3 name at
/// the era's rect, so the species table stays the 26.3 one. Names the era already matches
/// need no alias, and every one of 26.3's default player skin names draws the era's one skin.
pub fn alias_entity_textures(sprites: &mut HashMap<String, crate::hud::Rect>) {
    for texture in textures() {
        let Some(sprite) = entity_sprite(texture.path) else {
            continue;
        };
        if sprite == texture.name {
            continue;
        }
        if let Some(rect) = sprites.get(&sprite).copied() {
            sprites.insert(texture.name.to_owned(), rect);
        }
    }
    // 1.7.10 has one default player skin. 26.3's `DefaultPlayerSkin` names eighteen (wide and
    // slim, nine names each); a player whose offline profile picks any of them draws the era's
    // sheet. The first-person hand already falls back to `entity/player/wide/steve`, but the
    // body uses the profile's name, so every one needs the rect.
    let player_sheet = textures()
        .iter()
        .find(|texture| texture.name == "entity/player/wide/steve")
        .and_then(|texture| entity_sprite(texture.path))
        .and_then(|sprite| sprites.get(&sprite).copied());
    if let Some(rect) = player_sheet {
        for name in crate::tab_list::DEFAULT_SKINS {
            sprites.insert(name.to_owned(), rect);
        }
    }
}

/// Synthesised 1.7.10 UI textures packed into [`crate::hud::HudAtlas`] so shared screens
/// (title, pause, options, world select, lists, text fields) draw with the era's palette
/// instead of missing 26.3 sprite files.
pub(crate) fn era_ui_images(
    root: &crate::resource_source::ResourceSource,
) -> Vec<(String, u32, u32, Vec<u8>)> {
    let mut out = Vec::new();
    // `GuiScreen.drawBackground`: `options_background.png` tiled at (64,64,64), and
    // `GuiSlot` list backgrounds at (32,32,32).
    if let Ok(bytes) = root.read("assets/minecraft/textures/gui/options_background.png")
        && let Ok((w, h, rgba)) = crate::hud::decode_bytes(&bytes)
    {
        let tint = |scale: u32| -> Vec<u8> {
            rgba.as_chunks::<4>()
                .0
                .iter()
                .flat_map(|p| {
                    [
                        ((u32::from(p[0]) * scale) / 255) as u8,
                        ((u32::from(p[1]) * scale) / 255) as u8,
                        ((u32::from(p[2]) * scale) / 255) as u8,
                        p[3],
                    ]
                })
                .collect()
        };
        let menu_bg = tint(64);
        let list_bg = tint(32);
        out.push(("gui/menu_background".to_owned(), w, h, menu_bg));
        out.push(("gui/menu_list_background".to_owned(), w, h, list_bg.clone()));
        out.push(("gui/inworld_menu_list_background".to_owned(), w, h, list_bg));
    }
    let solid = |w: u32, h: u32, px: [u8; 4]| -> Vec<u8> {
        std::iter::repeat_n(px, (w * h) as usize)
            .flatten()
            .collect()
    };
    out.push((
        "gui/inworld_menu_background".to_owned(),
        4,
        4,
        solid(4, 4, [16, 16, 16, 192]),
    ));
    for name in [
        "gui/header_separator",
        "gui/footer_separator",
        "gui/inworld_header_separator",
        "gui/inworld_footer_separator",
    ] {
        out.push((name.to_owned(), 2, 2, solid(2, 2, [0, 0, 0, 220])));
    }
    let bordered = |w: u32, h: u32, fill: [u8; 4], border: [u8; 4]| -> Vec<u8> {
        let mut buf = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
                buf.extend_from_slice(if edge { &border } else { &fill });
            }
        }
        buf
    };
    // `GuiTextField`: black fill with #A0A0A0 border (or white when focused).
    out.push((
        "era_ui/text_field".to_owned(),
        16,
        16,
        bordered(16, 16, [0, 0, 0, 255], [160, 160, 160, 255]),
    ));
    out.push((
        "era_ui/text_field_highlighted".to_owned(),
        16,
        16,
        bordered(16, 16, [0, 0, 0, 255], [255, 255, 255, 255]),
    ));
    // `GuiSlot` scrollbar: black track and grey thumb with light bevel.
    out.push((
        "era_ui/scroller_background".to_owned(),
        4,
        4,
        solid(4, 4, [0, 0, 0, 255]),
    ));
    let mut scroller = Vec::with_capacity(6 * 6 * 4);
    for y in 0..6 {
        for x in 0..6 {
            let px = if x == 5 || y == 5 {
                [128, 128, 128, 255]
            } else if x == 0 || y == 0 {
                [192, 192, 192, 255]
            } else {
                [160, 160, 160, 255]
            };
            scroller.extend_from_slice(&px);
        }
    }
    out.push(("era_ui/scroller".to_owned(), 6, 6, scroller));
    // Checkboxes: 17x17 dark box with crisp border and white checkmark when selected.
    for (name, checked, lit) in [
        ("era_ui/checkbox", false, false),
        ("era_ui/checkbox_highlighted", false, true),
        ("era_ui/checkbox_selected", true, false),
        ("era_ui/checkbox_selected_highlighted", true, true),
    ] {
        let border = if lit {
            [255, 255, 255, 255]
        } else {
            [160, 160, 160, 255]
        };
        let mut buf = bordered(17, 17, [32, 32, 32, 255], border);
        if checked {
            let pts: &[(u32, u32)] = &[
                (3, 8),
                (4, 9),
                (5, 10),
                (6, 11),
                (7, 12),
                (8, 11),
                (9, 10),
                (10, 9),
                (11, 8),
                (12, 7),
                (13, 6),
                (13, 5),
                (4, 8),
                (5, 9),
                (6, 10),
                (7, 11),
                (8, 10),
                (9, 9),
                (10, 8),
                (11, 7),
                (12, 6),
            ];
            for &(x, y) in pts {
                let i = ((y * 17 + x) * 4) as usize;
                buf[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        out.push((name.to_owned(), 17, 17, buf));
    }
    // Classic 15x15 monochrome UI icons so TitleScreen's Accessibility and Friends buttons
    // do not render empty boxes when modern `gui/sprites/icon/*` sheets are absent.
    let icon_from_bitmap = |rows: [&str; 15]| -> Vec<u8> {
        let mut buf = Vec::with_capacity(15 * 15 * 4);
        for row in rows {
            for ch in row.chars().take(15) {
                let px = match ch {
                    '#' => [255, 255, 255, 255],
                    'o' => [200, 200, 200, 255],
                    _ => [0, 0, 0, 0],
                };
                buf.extend_from_slice(&px);
            }
        }
        buf
    };
    out.push((
        "era_ui/icon_accessibility".to_owned(),
        15,
        15,
        icon_from_bitmap([
            ".......#.......",
            "......###......",
            ".......#.......",
            "...............",
            "..###########..",
            "......###......",
            "......###......",
            "......###......",
            "......###......",
            ".....##.##.....",
            ".....##.##.....",
            "....##...##....",
            "....##...##....",
            "...##.....##...",
            "...............",
        ]),
    ));
    out.push((
        "era_ui/icon_friends".to_owned(),
        15,
        15,
        icon_from_bitmap([
            "...............",
            "...##.....##...",
            "..####...####..",
            "..####...####..",
            "...##.....##...",
            "...............",
            "..####...####..",
            ".######.######.",
            ".######.######.",
            ".######.######.",
            ".######.######.",
            "...............",
            "...............",
            "...............",
            "...............",
        ]),
    ));
    out
}

/// Register 1.7.10's widget rects (`gui/widgets.png` buttons, sliders, tabs and the
/// synthesised controls from [`era_ui_images`]) under the 26.3 GUI sprite ids shared menus
/// read (`widget/button`, `gui/sprites/widget/button`, ...).
pub(crate) fn register_era_ui_sprites(
    sprites: &mut HashMap<String, crate::hud::Rect>,
    gui: &mut HashMap<String, crate::hud::GuiSprite>,
) {
    use crate::hud::{GuiSprite, Rect};
    use crate::ui::NineSlice;
    let sub = |base: Rect, x: f32, y: f32, w: f32, h: f32| -> Rect {
        let (du, dv) = ((base.u1 - base.u0) / 256.0, (base.v1 - base.v0) / 256.0);
        Rect {
            u0: base.u0 + x * du,
            v0: base.v0 + y * dv,
            u1: base.u0 + (x + w) * du,
            v1: base.v0 + (y + h) * dv,
            w: w as u32,
            h: h as u32,
        }
    };
    let put = |sprites: &mut HashMap<String, Rect>,
               gui: &mut HashMap<String, GuiSprite>,
               id: &str,
               rect: Rect,
               scaling: Option<NineSlice>| {
        let entry = GuiSprite::Present { rect, scaling };
        gui.insert(id.to_owned(), entry);
        gui.insert(format!("gui/sprites/{id}"), entry);
        sprites.insert(format!("gui/sprites/{id}"), rect);
    };
    if let Some(widgets) = sprites.get("gui/widgets").copied() {
        let btn_slice = Some(NineSlice::new(200, 20, [3, 3, 3, 3], false));
        let disabled = sub(widgets, 0.0, 46.0, 200.0, 20.0);
        let normal = sub(widgets, 0.0, 66.0, 200.0, 20.0);
        let hover = sub(widgets, 0.0, 86.0, 200.0, 20.0);
        put(sprites, gui, "widget/button_disabled", disabled, btn_slice);
        put(sprites, gui, "widget/button", normal, btn_slice);
        put(sprites, gui, "widget/button_highlighted", hover, btn_slice);
        // 1.7.10 `GuiOptionSlider`: disabled button track, normal/hover button ends as handle.
        put(sprites, gui, "widget/slider", disabled, btn_slice);
        put(
            sprites,
            gui,
            "widget/slider_highlighted",
            disabled,
            btn_slice,
        );
        put(sprites, gui, "widget/slider_handle", normal, btn_slice);
        put(
            sprites,
            gui,
            "widget/slider_handle_highlighted",
            hover,
            btn_slice,
        );
        put(sprites, gui, "widget/tab", disabled, btn_slice);
        put(sprites, gui, "widget/tab_highlighted", hover, btn_slice);
        put(sprites, gui, "widget/tab_selected", normal, btn_slice);
        put(
            sprites,
            gui,
            "widget/tab_selected_highlighted",
            hover,
            btn_slice,
        );
        put(
            sprites,
            gui,
            "widget/slot_frame",
            sub(widgets, 0.0, 22.0, 24.0, 22.0),
            Some(NineSlice::new(24, 22, [3, 3, 3, 3], false)),
        );
        put(
            sprites,
            gui,
            "icon/language",
            sub(widgets, 0.0, 106.0, 20.0, 20.0),
            None,
        );
    }
    let field_slice = Some(NineSlice::new(16, 16, [1, 1, 1, 1], true));
    for (src, dst, slice) in [
        ("era_ui/text_field", "widget/text_field", field_slice),
        (
            "era_ui/text_field_highlighted",
            "widget/text_field_highlighted",
            field_slice,
        ),
        ("era_ui/checkbox", "widget/checkbox", None),
        (
            "era_ui/checkbox_highlighted",
            "widget/checkbox_highlighted",
            None,
        ),
        ("era_ui/checkbox_selected", "widget/checkbox_selected", None),
        (
            "era_ui/checkbox_selected_highlighted",
            "widget/checkbox_selected_highlighted",
            None,
        ),
        (
            "era_ui/scroller_background",
            "widget/scroller_background",
            None,
        ),
        (
            "era_ui/scroller",
            "widget/scroller",
            Some(NineSlice::new(6, 6, [1, 1, 1, 1], true)),
        ),
        ("era_ui/icon_accessibility", "icon/accessibility", None),
        ("era_ui/icon_friends", "friends/friends", None),
    ] {
        if let Some(rect) = sprites.get(src).copied() {
            put(sprites, gui, dst, rect, slice);
        }
    }
}

/// English fallbacks for modern UI and command keys that 1.7.10's `en_US.lang` does not carry,
/// layered underneath `en_US.lang` so the era's own strings always win.
pub fn modern_ui_fallbacks() -> &'static [(&'static str, &'static str)] {
    &[
        ("options.generic_value", "%s: %s"),
        ("options.online", "Online..."),
        ("options.online.title", "Online Options"),
        ("options.skinCustomisation", "Skin Customization..."),
        ("options.skinCustomisation.title", "Skin Customization"),
        ("options.chat", "Chat Settings..."),
        ("options.chat.title", "Chat Settings"),
        ("options.accessibility", "Accessibility Settings..."),
        ("options.accessibility.title", "Accessibility Settings"),
        (
            "options.credits_and_attribution",
            "Credits & Attribution...",
        ),
        ("options.chunks", "%s chunks"),
        ("options.blocks", "%s blocks"),
        ("options.framerate", "%s fps"),
        ("options.gamma.default", "Default"),
        ("options.clouds.fast", "Fast"),
        ("options.clouds.fancy", "Fancy"),
        ("options.entityShadows", "Entity Shadows"),
        ("options.autoJump", "Auto-Jump"),
        ("options.attackIndicator", "Attack Indicator"),
        ("options.attack.crosshair", "Crosshair"),
        ("options.attack.hotbar", "Hotbar"),
        ("options.mainHand", "Main Hand"),
        ("options.mainHand.left", "Left"),
        ("options.mainHand.right", "Right"),
        ("options.biomeBlendRadius", "Biome Blend"),
        ("options.entityDistanceScaling", "Entity Distance"),
        ("options.improvedTransparency", "Improved Transparency"),
        ("options.cutoutLeaves", "Cutout Leaves"),
        ("options.glintSpeed", "Glint Speed"),
        ("options.glintStrength", "Glint Strength"),
        ("options.fovEffectScale", "FOV Effects"),
        ("options.damageTiltStrength", "Damage Tilt"),
        ("options.darknessEffectScale", "Darkness Pulsing"),
        ("options.hideLightningFlashes", "Hide Lightning Flashes"),
        ("options.hideSplashTexts", "Hide Splash Texts"),
        ("options.autosaveIndicator", "Autosave Indicator"),
        ("options.key.hold", "Hold"),
        ("options.key.toggle", "Toggle"),
        ("options.invertMouseX", "Invert X"),
        ("options.invertMouseY", "Invert Y"),
        ("options.discrete_mouse_scroll", "Discrete Scrolling"),
        ("options.audioDevice", "Device"),
        ("options.audioDevice.default", "System Default"),
        ("options.directionalAudio", "Directional Audio"),
        ("options.graphics.preset", "Graphics Preset"),
        ("options.graphicsApi", "Graphics API"),
        ("options.graphicsApi.default", "Default"),
        ("options.graphicsApi.vulkan", "Vulkan"),
        ("options.graphicsApi.opengl", "OpenGL"),
        ("options.maxAnisotropy", "Anisotropic Filtering"),
        ("options.chunkFade", "Chunk Fade"),
        ("options.chunkFade.none", "Off"),
        ("options.chunkFade.seconds", "%ss"),
        ("options.inactivityFpsLimit", "Reduce FPS When"),
        ("options.inactivityFpsLimit.minimized", "Minimized"),
        ("options.inactivityFpsLimit.afk", "AFK"),
        ("options.exclusiveFullscreen", "Exclusive Fullscreen"),
        ("options.exclusiveFullscreen.on", "ON"),
        ("options.exclusiveFullscreen.off", "OFF"),
        ("options.fullscreen.current", "Current"),
        ("options.fullscreen.unavailable", "Unavailable"),
        (
            "options.accessibility.menu_background_blurriness",
            "Menu Background Blur",
        ),
        (
            "options.accessibility.text_background_opacity",
            "Text Background Opacity",
        ),
        ("options.accessibility.text_background", "Text Background"),
        ("options.accessibility.text_background.chat", "Chat"),
        (
            "options.accessibility.text_background.everywhere",
            "Everywhere",
        ),
        ("options.accessibility.high_contrast", "High Contrast"),
        (
            "options.accessibility.high_contrast_block_outline",
            "High Contrast Block Outline",
        ),
        (
            "options.accessibility.panorama_speed",
            "Panorama Scroll Speed",
        ),
        ("options.accessibility.narrator_hotkey", "Narrator Hotkey"),
        ("options.chat.line_spacing", "Line Spacing"),
        ("options.chat.delay", "Chat Delay"),
        ("options.chat.drafts", "Save Chat Drafts"),
        ("options.allowServerListing", "Allow Server Listings"),
        ("options.autoSuggestCommands", "Command Suggestions"),
        ("options.allowCursorChanges", "Allow Cursor Changes"),
        (
            "options.ctrlClickEmulatesRightClick",
            "Ctrl+Click Right-Click",
        ),
        (
            "options.darkMojangStudiosBackgroundColor",
            "Monochrome Logo",
        ),
        ("options.language.title", "Language"),
        ("options.font.title", "Font Settings"),
        ("controls.keybinds", "Key Binds..."),
        ("controls.keybinds.title", "Key Binds"),
        ("gui.advancements", "Achievements"),
        ("gui.continue", "Continue"),
        ("gui.proceed", "Proceed"),
        ("gui.acknowledge", "OK"),
        ("gui.friends.open", "Friends"),
        ("gui.selectWorld.search", "Search..."),
        ("options.worldOptions.button", "World Options..."),
        ("options.worldOptions.title", "World Options"),
        ("options.worldOptions.general.title", "General"),
        ("options.worldOptions.multiplayer.title", "Multiplayer"),
        ("options.worldOptions.game_mode", "Default Game Mode"),
        (
            "options.worldOptions.personal_game_mode",
            "Personal Game Mode",
        ),
        (
            "options.worldOptions.guest.command_access",
            "Guest Command Access",
        ),
        (
            "options.worldOptions.guest.force_game_mode",
            "Force Game Mode",
        ),
        ("menu.multiplayerOptions.lan", "Open to LAN"),
        ("lanServer.port", "Port Number"),
        ("menu.sendFeedback", "Send Feedback"),
        ("menu.reportBugs", "Report Bugs"),
        ("menu.playerReporting", "Player Reporting"),
        ("menu.server_links", "Server Links..."),
        ("chat.square_brackets", "[%s]"),
        ("chat.coordinates.tooltip", "Click to teleport"),
        ("chat.copy.click", "Click to Copy to Clipboard"),
        (
            "accessibility.onboarding.screen.title",
            "Welcome to Minecraft!\n\nWould you like to enable the Narrator or visit the Accessibility Settings?",
        ),
        (
            "accessibility.onboarding.accessibility.button",
            "Accessibility Settings...",
        ),
        ("options.narrator", "Narrator"),
        ("options.narrator.notavailable", "Not Available"),
        (
            "multiplayerWarning.header",
            "Caution: Third-Party Online Play",
        ),
        ("multiplayerWarning.check", "Do not show this screen again"),
        ("selectWorld.version", "Version:"),
        ("selectWorld.versionUnknown", "unknown"),
        ("selectWorld.commands", "Commands"),
        ("selectWorld.edit", "Edit"),
        ("selectWorld.edit.title", "Edit World"),
        ("selectWorld.edit.save", "Save"),
        ("selectWorld.edit.backup", "Make Backup"),
        ("selectWorld.edit.openFolder", "Open World Folder"),
        ("selectWorld.edit.resetIcon", "Reset Icon"),
        ("selectWorld.edit.optimize", "Optimize World"),
        (
            "controls.keybinds.duplicateKeybinds",
            "Key is bound to multiple actions",
        ),
        ("options.percent_value", "%s: %s%%"),
        ("options.pixel_value", "%s: %spx"),
        ("options.chat.delay_none", "None"),
        ("pack.openFolder", "Open Pack Folder"),
        ("pack.nameAndSource", "%s (%s)"),
        ("createWorld.tab.game.title", "Game"),
        ("createWorld.tab.world.title", "World"),
        ("createWorld.tab.more.title", "More"),
        ("createWorld.preparing", "Preparing for world creation..."),
        (
            "commands.give.failed.toomanyitems",
            "Cannot give more than %s items",
        ),
        ("commands.give.success.single", "Gave %s %s to %s"),
        ("commands.give.success.multiple", "Gave %s %s to %s players"),
        ("commands.gamemode.success.self", "Set own game mode to %s"),
        (
            "commands.gamemode.success.other",
            "Set %s's game mode to %s",
        ),
        ("commands.time.set", "Set the time to %s"),
        (
            "commands.tp.success.coordinates",
            "Teleported %s to %s, %s, %s",
        ),
        (
            "commands.teleport.success.location.single",
            "Teleported %s to %s, %s, %s",
        ),
        (
            "commands.teleport.success.entity.single",
            "Teleported %s to %s",
        ),
        (
            "commands.effect.give.success.single",
            "Applied effect %s to %s",
        ),
        (
            "commands.effect.clear.everything.success.single",
            "Removed every effect from %s",
        ),
        (
            "commands.enchant.success.single",
            "Applied enchantment %s to %s",
        ),
        ("commands.kill.success.single", "Killed %s"),
    ]
}

fn texture_stems<'a>(files: &'a [String], folder: &'a str) -> impl Iterator<Item = &'a str> {
    files
        .iter()
        .filter_map(move |path| path.strip_prefix(folder))
        .filter_map(|rest| rest.strip_suffix(".png"))
        // A texture beside the folder's files (an animated strip's mcmeta) is no sprite.
        .filter(|rest| !rest.contains('.'))
}

fn png_paths<'a>(files: &'a [String], folder: &'a str) -> impl Iterator<Item = &'a str> {
    files
        .iter()
        .filter(move |path| path.starts_with(folder) && path.ends_with(".png"))
        .map(String::as_str)
}

/// `Locale.loadBytes`' lines: UTF-8, `#` comments and empty lines skipped, the first `=`
/// splits the key from the value, a `\r` before the newline dropped. The values are the ported
/// locale's; this only names the keys so a `Language` can take them.
fn parse_lang(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .split('\n')
        .filter_map(|line| {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, _) = line.split_once('=')?;
            Some(key.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets::RenderAssets,
        hud::HudAtlas,
        resource_source::{ResourceRoot, ResourceSource},
        sprites::Sprites,
    };
    use sunrise_core::profile::Profile;

    /// The prepared 1.7.10 folder, when this machine has one (CI does not).
    fn root() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/sunrise-client")
            .join(VERSION);
        (Profile::of_resource_root(&root).ok() == Some(Profile::V1_7_10)).then_some(root)
    }

    #[test]
    fn the_era_tables_load_without_the_26_3_manifest() {
        let Some(root) = root() else { return };
        let assets = crate::assets::Assets::load(&root).unwrap();
        let state = |name: &str| assets.registry.default_state(name).unwrap();

        // The content export's rows, with the era's material names for the
        // test scene's six layers.
        assert_eq!(assets.registry.len(), 2379);
        let material = |layer: usize| {
            assets
                .registry
                .state(assets.material_states[layer])
                .unwrap()
                .name
                .clone()
        };
        assert_eq!(material(0), "minecraft:bedrock");
        assert_eq!(material(1), "minecraft:dirt");
        assert_eq!(material(2), "minecraft:stone");
        assert_eq!(material(3), "minecraft:sand");
        assert_eq!(material(4), "minecraft:planks");
        assert_eq!(material(5), "minecraft:brick_block");

        // 1.7.10's heightmap counts every non-air block and its fluid flow stops
        // at every non-air, non-fluid one.
        assert!(assets.blocks_fluid_flow.contains("minecraft:stone"));
        assert!(!assets.blocks_fluid_flow.contains("minecraft:water"));
        assert!(!assets.blocks_fluid_flow.contains("minecraft:air"));

        // The era's source liquids take a `level` so the modern fluid code reads
        // them; the flowing blocks stay as the export wrote them.
        let water = &assets
            .registry
            .state(state("minecraft:water"))
            .unwrap()
            .properties;
        assert_eq!(water.get("level").map(String::as_str), Some("0"));
        let flowing = &assets
            .registry
            .state(state("minecraft:flowing_water"))
            .unwrap()
            .properties;
        assert_eq!(flowing.get("level"), None);
        let ladder = &assets
            .registry
            .state(state("minecraft:ladder"))
            .unwrap()
            .properties;
        assert_eq!(ladder.get("facing").map(String::as_str), Some("north"));

        // The render set is the era's atlas, not a model catalog.
        let render = RenderAssets::load_1_7_10(&ResourceRoot::directory(root.clone())).unwrap();
        assert_eq!(render.sprite_size, 32);
        assert!(!render.pixels.is_empty());
        assert!(render.model_placements.is_empty() && render.model_gallery.is_empty());
        assert!(render.sprite_layers.contains_key("minecraft:blocks/stone"));
        assert!(
            render
                .sprite_layers
                .contains_key("minecraft:block/water_still"),
            "the era's fluid sprites answer under 26.3's names too"
        );
    }

    #[test]
    fn the_era_biomes_and_environment_stand_in_for_the_missing_data_files() {
        let Some(root) = root() else { return };
        let source = ResourceSource::directory(root.clone());
        let biomes = crate::biome::Biomes::load(&ResourceRoot::new(
            root.clone(),
            root.clone(),
            source.clone(),
        ))
        .unwrap();
        assert_eq!(biomes.list.len(), 61);
        assert!(biomes.is_era());
        assert_eq!(biomes.id("minecraft:plains"), 1);
        assert_eq!(biomes.era_id(2), biomes.id("minecraft:desert"));
        assert_eq!(biomes.get(biomes.era_id(2)).name, "minecraft:desert");
        assert_eq!(biomes.get(biomes.era_id(2)).temperature, 2.0);
        assert_eq!(
            biomes.era_id(40),
            biomes.era_id(1),
            "an id the era's layers never produce reads as plains"
        );
        assert_eq!(biomes.get(biomes.era_id(9)).name, "minecraft:sky");

        for dimension in ["overworld", "the_nether", "the_end"] {
            let environment =
                crate::environment::Environment::load_dimension(&source, dimension).unwrap();
            // The layers build; the era's dimensions supply no timelines, so a
            // sample reads the constant dimension colours.
            let _ = environment;
        }
    }

    /// 1.7.10 gives the flat and thin decorations no collision box; the client
    /// must predict the same, or walking through tall grass catches.
    #[test]
    fn the_era_decorations_have_no_collision() {
        let Some(root) = root() else { return };
        let assets = crate::assets::Assets::load(&root).unwrap();
        let collisions = crate::collisions::load_1_7_10(&root, assets.registry).unwrap();
        for name in [
            "minecraft:tallgrass",
            "minecraft:yellow_flower",
            "minecraft:red_flower",
            "minecraft:sapling",
            "minecraft:deadbush",
            "minecraft:reeds",
            "minecraft:torch",
            "minecraft:redstone_wire",
            "minecraft:vine",
            "minecraft:snow_layer",
            "minecraft:waterlily",
            "minecraft:double_plant",
        ] {
            let state = assets.registry.default_state(name).unwrap();
            let shape = collisions
                .static_shape(state)
                .unwrap()
                .expect("a state with a static shape");
            assert!(
                shape.is_empty(),
                "{name} must not collide, its shape is {:?}",
                shape.coordinates(sunrise_core::geometry::Axis::Y)
            );
        }
        for name in ["minecraft:grass", "minecraft:dirt", "minecraft:stone"] {
            let state = assets.registry.default_state(name).unwrap();
            assert!(
                collisions
                    .static_shape(state)
                    .unwrap()
                    .expect("a static shape")
                    .is_cube(),
                "{name} is a full cube"
            );
        }
        let slab = assets
            .registry
            .default_state("minecraft:stone_slab")
            .unwrap();
        let steps = collisions
            .static_shape(slab)
            .unwrap()
            .expect("a static shape")
            .coordinates(sunrise_core::geometry::Axis::Y)
            .len();
        assert!(steps > 2, "a slab is a half block, not a cube");
    }

    /// 1.7.10's selection outline is the block's `setShape` box, which the era
    /// decorations still have where their collision shape is null: tall grass
    /// outlines as a 0.8-tall, 0.8-wide box that hugs the plant, not as a
    /// full cell whose top edge floats above it.
    #[test]
    fn the_era_crossed_plants_outline_their_set_shape_box() {
        let Some(root) = root() else { return };
        let assets = crate::assets::Assets::load(&root).unwrap();
        let collisions = crate::collisions::load_1_7_10(&root, assets.registry).unwrap();
        let cached = collisions.cached_shapes();
        let outline = |name: &str| {
            let state = assets.registry.default_state(name).unwrap();
            cached
                .context_outline_shape(
                    state,
                    sunrise_world::collision_context::EntityContextKind::Player,
                )
                .cloned()
        };
        let bounds = |shape: &sunrise_core::geometry::VoxelShape| {
            let bounds = shape.bounds().expect("a non-empty shape");
            [
                bounds.min(sunrise_core::geometry::Axis::X),
                bounds.min(sunrise_core::geometry::Axis::Y),
                bounds.min(sunrise_core::geometry::Axis::Z),
                bounds.max(sunrise_core::geometry::Axis::X),
                bounds.max(sunrise_core::geometry::Axis::Y),
                bounds.max(sunrise_core::geometry::Axis::Z),
            ]
        };
        // `TallPlantBlock`'s constructor: 0.5 +- 0.4, 0.8 tall.
        let tallgrass = outline("minecraft:tallgrass").expect("tall grass has an outline");
        assert_eq!(
            bounds(&tallgrass),
            [
                f64::from(0.1_f32),
                0.0,
                f64::from(0.1_f32),
                f64::from(0.9_f32),
                f64::from(0.8_f32),
                f64::from(0.9_f32),
            ]
        );
        // `FlowerBlock` keeps `PlantBlock`'s 0.2 inset and 0.6 height.
        let flower = outline("minecraft:yellow_flower").expect("a flower has an outline");
        assert_eq!(
            bounds(&flower),
            [
                f64::from(0.3_f32),
                0.0,
                f64::from(0.3_f32),
                f64::from(0.7_f32),
                f64::from(0.6_f32),
                f64::from(0.7_f32),
            ]
        );
        // `DoublePlantBlock.updateShape` is the full cube, so no row is needed
        // and the client's default cube stays.
        assert!(outline("minecraft:double_plant").is_none());
    }

    /// The era table serialises each shape's occupied cells: a slab is the
    /// bottom half, not the full cube an all-ones mask would make it.
    #[test]
    fn the_era_table_keeps_a_shapes_occupancy() {
        let Some(root) = root() else { return };
        let assets = crate::assets::Assets::load(&root).unwrap();
        let collisions = crate::collisions::load_1_7_10(&root, assets.registry).unwrap();
        let slab = assets
            .registry
            .default_state("minecraft:stone_slab")
            .unwrap();
        let shape = collisions
            .static_shape(slab)
            .unwrap()
            .expect("a state with a static shape");
        let bounds = shape.bounds().expect("a slab has a box");
        assert_eq!(bounds.max(sunrise_core::geometry::Axis::Y), 0.5);
        assert_eq!(bounds.max(sunrise_core::geometry::Axis::X), 1.0);
    }

    #[test]
    fn the_era_collisions_stand_on_solid_blocks_and_not_on_water() {
        let Some(root) = root() else { return };
        let assets = crate::assets::Assets::load(&root).unwrap();
        let collisions = crate::collisions::load_1_7_10(&root, assets.registry).unwrap();
        let stone = assets.registry.default_state("minecraft:stone").unwrap();
        let water = assets.registry.default_state("minecraft:water").unwrap();
        assert!(
            collisions
                .static_shape(stone)
                .unwrap()
                .is_some_and(|shape| shape.is_block())
        );
        assert!(
            collisions
                .static_shape(water)
                .unwrap()
                .is_some_and(|shape| shape.is_empty())
        );
    }

    /// A texture's own edge before the atlas resamples every layer to one size.
    fn native(sprites: &Sprites, texture: &Texture) -> u32 {
        let layer = sprite_layer(sprites, texture).expect("sprite has a layer");
        sprites.native_size(layer).expect("layer has a native size")
    }

    #[test]
    fn a_source_keeps_the_profile_it_was_opened_with() {
        use crate::resource_source::Resource;
        // A flat index is built over a prepared root's files: the profile comes
        // from that root, not from whichever fonts the index happens to hold.
        let files = [
            ("assets/minecraft/font/glyph_sizes.bin", Vec::new()),
            ("assets/minecraft/textures/font/ascii.png", Vec::new()),
        ]
        .into_iter()
        .map(|(name, bytes)| (name.to_owned(), Resource::bytes(bytes)))
        .collect();
        let era = ResourceSource::indexed(files).with_profile(Profile::V1_7_10);
        assert_eq!(era.profile(), Profile::V1_7_10);
        assert_eq!(
            ResourceSource::indexed(Default::default()).profile(),
            Profile::Modern
        );
    }

    #[test]
    fn sprite_lookups_match_the_era() {
        let Some(root) = root() else { return };
        let assets = Assets::load(&root).unwrap();
        let mut sprites = Sprites::new(ResourceSource::directory(root));
        assets.insert_sprites(&mut sprites).unwrap();
        sprites.finish();
        assert_eq!(
            sprites.size, 32,
            "1.7.10's flowing water and lava are 32x32 frames"
        );

        let cases: &[(u16, u8, u8, &str)] = &[
            (1, 0, FACE_NORTH, "stone"),
            (2, 0, FACE_TOP, "grass_top"),
            (2, 0, FACE_NORTH, "grass_side"),
            (2, 0, FACE_BOTTOM, "dirt"),
            (35, 0, FACE_NORTH, "wool_colored_white"),
            (35, 14, FACE_NORTH, "wool_colored_red"),
            (35, 5, FACE_NORTH, "wool_colored_lime"),
            (17, 0, FACE_TOP, "log_oak_top"),
            (17, 0, FACE_NORTH, "log_oak"),
            // The axis in metadata bits 2-3 turns the axis faces to the top sprite.
            (17, 4, FACE_EAST, "log_oak_top"),
            (17, 4, FACE_NORTH, "log_oak"),
            (53, 0, FACE_NORTH, "planks_oak"),
            (9, 0, FACE_TOP, "water_still"),
            (9, 0, FACE_NORTH, "water_flow"),
            (8, 0, FACE_TOP, "water_still"),
            (8, 0, FACE_NORTH, "water_flow"),
            (11, 0, FACE_TOP, "lava_still"),
            (11, 0, FACE_NORTH, "lava_flow"),
            (10, 0, FACE_TOP, "lava_still"),
            (10, 0, FACE_NORTH, "lava_flow"),
            (50, 0, FACE_NORTH, "torch_on"),
            (31, 0, FACE_NORTH, "deadbush"),
            (31, 1, FACE_NORTH, "tallgrass"),
            (31, 2, FACE_NORTH, "fern"),
        ];
        for &(id, metadata, face, sprite) in cases {
            let texture = block_texture(id, metadata, face)
                .unwrap_or_else(|| panic!("block {id}:{metadata} face {face} has no sprite"));
            assert_eq!(
                texture.sprite(),
                sprite,
                "block {id}:{metadata} face {face}"
            );
            assert_eq!(texture.atlas(), Atlas::Blocks);
            assert!(
                sprite_layer(&sprites, &texture).is_some(),
                "{} is not in the atlas",
                sprite_name(&texture)
            );
        }
        let stone = block_texture(1, 0, FACE_NORTH).unwrap();
        assert_eq!(native(&sprites, &stone), 16);
        assert_eq!(sprites.size, 32, "the atlas holds the 32x32 liquid frames");
        // Item icons come from the era's items atlas, a separate folder.
        let stick = item_texture(280, 0).unwrap();
        assert_eq!(
            (stick.atlas(), stick.sprite()),
            (Atlas::Items, "stick"),
            "item 280 is the stick"
        );
        assert!(sprite_layer(&sprites, &stick).is_some());
    }

    #[test]
    fn atlas_holds_every_sprite_the_export_references() {
        let Some(root) = root() else { return };
        let assets = Assets::load(&root).unwrap();
        assert_eq!(assets.blocks().len(), 171, "every 1.7.10 block id");
        assert_eq!(assets.textured_blocks(), 170);
        assert_eq!(
            assets.textureless(),
            vec![0],
            "air is the only textureless one"
        );
        assert_eq!(textureless_blocks(), &[0]);
        assert!(block_has_texture(1));
        assert!(!block_has_texture(0));

        // The renderer's real entry point, over the same folder.
        let root = ResourceRoot::directory(root);
        let render = RenderAssets::load_1_7_10(&root).unwrap();
        let block_files = assets.block_texture_files().count();
        let item_files = assets.item_texture_files().count();
        assert!(block_files > 0 && item_files > 0);
        let era_names = render
            .sprite_layers
            .keys()
            .filter(|name| {
                name.starts_with("minecraft:blocks/") || name.starts_with("minecraft:items/")
            })
            .count();
        assert_eq!(
            era_names,
            block_files + item_files,
            "every extracted block and item texture names a layer"
        );
        // 26.3's fluid and breaking renderers ask for `block/<name>`; the era's
        // names share their layer rather than adding a second one.
        let layer = |name: &str| render.sprite_layers[name];
        for name in ["water_still", "water_flow", "lava_still", "lava_flow"] {
            assert_eq!(
                layer(&format!("minecraft:block/{name}")),
                layer(&format!("minecraft:blocks/{name}")),
                "{name}"
            );
        }
        assert_eq!(
            render.sprite_layers["minecraft:block/water_overlay"],
            render.sprite_layers["minecraft:blocks/water_still"],
            "the still sheet stands in for the missing overlay"
        );
        assert!(
            (render.texture_layers as usize) <= block_files + item_files,
            "identical still textures share a layer; names never outnumber files"
        );
        for row in assets.blocks().iter().filter(|row| !row.textureless) {
            for sprite in &row.sprites {
                let name = format!("minecraft:blocks/{sprite}");
                assert!(
                    render.sprite_layers.contains_key(&name),
                    "block {} ({}) names {name}, which is not in the atlas",
                    row.name,
                    row.id
                );
            }
        }
        // The liquids' animated strips make the atlas 32 px: the smaller sprites are
        // nearest-resampled up, as 26.3 does for any bigger texture.
        assert_eq!(render.sprite_size, 32);
        assert!(!render.pixels.is_empty());
        assert!(
            !render.animations.is_empty(),
            "water, lava and fire animate"
        );
        let stone = block_texture(1, 0, FACE_NORTH).unwrap();
        let layer = render.sprite_layers[&sprite_name(&stone)];
        let texel = layer as usize * (render.sprite_size * render.sprite_size * 4) as usize;
        assert!(
            render.pixels.get()[texel..texel + 4] != [248, 0, 248, 255],
            "stone is the era's texture, not the missing one"
        );
    }

    #[test]
    fn language_loads_the_eras_keys() {
        let Some(root) = root() else { return };
        let language = Assets::load(&root).unwrap().language().unwrap();
        assert!(language.len() > 1000, "1.7.10's en_US.lang is large");
        assert_eq!(language.get("tile.stone.name"), Some("Stone"));
        assert_eq!(language.get("item.stick.name"), Some("Stick"));
        assert_eq!(language.get("container.inventory"), Some("Inventory"));
        assert_eq!(language.get("gui.done"), Some("Done"));
    }

    /// The era's own entity sheets are in the HUD atlas under both their era names and the
    /// 26.3 names the engine's species table asks for. The player's skin and the pig/cow/
    /// chicken sheets are the ones whose names differ.
    #[test]
    fn the_era_entity_sheets_answer_the_engines_names() {
        let Some(root) = root() else { return };
        let source = ResourceSource::directory(root);
        let atlas = HudAtlas::build(&source).unwrap();
        let rect = |name: &str| {
            atlas
                .sprites
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("{name} is not in the HUD atlas"))
        };
        assert_eq!(rect("entity/steve").w, 64);
        assert_eq!(rect("entity/steve").h, 32);
        // The engine asks for 26.3's names; the alias points at the era's file.
        for (engine, era) in [
            ("entity/player/wide/steve", "entity/steve"),
            ("entity/player/slim/steve", "entity/steve"),
            ("entity/player/wide/alex", "entity/steve"),
            ("entity/player/slim/zuri", "entity/steve"),
            ("entity/pig/pig_temperate", "entity/pig/pig"),
            ("entity/cow/cow_temperate", "entity/cow/cow"),
            ("entity/chicken/chicken_temperate", "entity/chicken"),
            ("entity/sheep/sheep_wool", "entity/sheep/sheep_fur"),
        ] {
            assert_eq!(rect(engine), rect(era), "{engine} does not alias {era}");
        }
        // The era's own names that already match stay as they are.
        for name in [
            "entity/zombie/zombie",
            "entity/skeleton/skeleton",
            "entity/creeper/creeper",
            "entity/spider/spider",
        ] {
            assert!(atlas.sprites.contains_key(name), "{name}");
        }
        assert_eq!(
            entity_sprite("textures/entity/pig/pig.png"),
            Some("entity/pig/pig".into())
        );
        assert_eq!(
            entity_sprite("textures/gui/icons.png"),
            Some("gui/icons".into())
        );
        assert_eq!(entity_sprite("textures/entity/steve"), None);
    }

    #[test]
    fn font_loads_the_eras_sheets() {
        let Some(root) = root() else { return };
        let assets = Assets::load(&root).unwrap();
        let source = ResourceSource::directory(root);
        let atlas = HudAtlas::build(&source).unwrap();
        // Every extracted font sheet is packed: ascii, the SGD sheet and the unicode pages.
        let sheets = assets.font_texture_files().count();
        let packed = atlas
            .sprites
            .keys()
            .filter(|name| name.starts_with("font/"))
            .count();
        assert_eq!(packed, sheets, "every font sheet is packed");
        assert_eq!(atlas.sprites["font/ascii"].w, 128);
        assert_eq!(atlas.sprites["font/ascii"].h, 128);
        assert!(atlas.sprites.contains_key("font/unicode_page_4e"));
        // The era's GUI screens are packed under their own paths, so a screen can draw
        // `gui/icons` and `gui/widgets` from the same atlas.
        assert_eq!(atlas.sprites["gui/icons"].w, 256);
        assert_eq!(atlas.sprites["gui/icons"].h, 256);
        assert_eq!(atlas.sprites["gui/widgets"].w, 256);
        assert_eq!(atlas.sprites["gui/widgets"].h, 256);
        assert!(atlas.sprites.contains_key("gui/container/inventory"));

        // `TextRenderer.compute_charWidths`' values for the era's ascii.png: the cell is
        // 8x8, and the scan leaves one pixel between a glyph and the next.
        let font = &atlas.font;
        assert_eq!(font.glyph('A').advance, 6.0);
        assert_eq!(font.glyph('W').advance, 6.0);
        assert_eq!(font.glyph('i').advance, 2.0);
        assert_eq!(font.glyph('l').advance, 3.0);
        // A space draws nothing and advances 4 (`drawGlyph`'s first branch).
        let space = font.glyph(' ');
        assert!(space.rect.is_none());
        assert_eq!(space.advance, 4.0);
        // A glyph is 8 pixels tall, however the GUI is scaled.
        let a = font.glyph('A');
        assert_eq!(a.bottom - a.top, 7.99);
        assert!(a.rect.is_some());
        assert_eq!(a.rect.unwrap().texels((atlas.width, atlas.height)), (6, 8));

        // `drawUnicodeGlyph`: the page's 16x16 cell at half scale, the width and advance from
        // `glyph_sizes.bin`. U+4E00 is 0xF there: columns 0..15, advance (16)/2 + 1 = 9.
        let cjk = font.glyph('\u{4e00}');
        assert_eq!(cjk.advance, 9.0);
        assert_eq!(cjk.bottom - cjk.top, 7.99);
        let cell = cjk
            .rect
            .expect("page 4e is packed")
            .texels((atlas.width, atlas.height));
        assert_eq!(cell.1, 16, "the page cell is 16 texels tall");
        assert_eq!(cell.0, 16, "columns 0..=15 of the page");
    }
}
