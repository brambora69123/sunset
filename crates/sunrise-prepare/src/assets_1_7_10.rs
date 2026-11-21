//! Minecraft 1.7.10's client assets, extracted from the player's own client JAR, and the
//! id → texture lookup the 1.7.10 renderer reads.
//!
//! The JAR the game imports as `.local/minecraft/1.7.10/named/client.jar` holds the era's block
//! and item textures, the GUI atlas and screens, `en_US.lang`, the bitmap fonts and the title
//! panorama. [`prepare`] copies those into `.local/sunrise-client/1.7.10/assets/minecraft/...`,
//! the layout the 1.7.10 resource pack format reads, and writes [`MANIFEST`] with every file's
//! source and SHA-256 so a later run re-extracts only what changed.
//!
//! The JAR does not hold the era's sounds: 1.7.10 keeps them in the launcher's asset object store
//! (`assets/indexes/` and `assets/objects/`). [`import_sounds`] reads the store's `1.7.10.json`
//! index, checks every object against its own SHA-1 and copies `minecraft/sounds.json` and the
//! `.ogg` files under `minecraft/sounds/` into the prepared folder, where the engine's sound
//! loader reads them; the manifest records each one's hash and every object it skipped with the
//! reason. Nothing is downloaded and nothing is invented.
//!
//! [`block_texture`], [`block_textures`] and [`item_texture`] answer the renderer's question:
//! which sprite does 1.7.10 register for this id and metadata, on this face. The mapping is
//! `Block.getSprite`/`Item.getSprite` and the client classes' overrides, ported in
//! `sunset-1-7-assets`; this module adds the prepared path and the coverage report. Blocks with
//! no sprite at all (1.7.10's air) are listed by [`textureless_blocks`] and in the manifest,
//! never guessed.

use crate::pyjson::Value;
use crate::{digest, hold_lock, read_json, sha256_hex, write_if_changed, Log};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use sunset_1_7_assets::sprites;

/// The Minecraft version this module prepares.
pub const VERSION: &str = "1.7.10";

/// The deobfuscated 1.7.10 client JAR, relative to the Sunrise folder.
pub const DEFAULT_JAR: &str = ".local/minecraft/1.7.10/named/client.jar";

/// The prepared 1.7.10 resource folder, relative to the Sunrise folder.
pub const DEFAULT_DIR: &str = ".local/sunrise-client/1.7.10";

/// The extraction's manifest, beside the content export in the prepared folder.
pub const MANIFEST: &str = "assets-1.7.10.json";

/// [`MANIFEST`]'s layout: bump it when a field's meaning changes, so a later run re-extracts
/// instead of trusting an older file.
pub const MANIFEST_SCHEMA: i64 = 1;

/// The asset index a launcher keeps for the era, whose `objects` name the sounds.
pub const SOUNDS_INDEX: &str = "1.7.10.json";

/// Vanilla's face ids: 0 bottom, 1 top, 2 north, 3 south, 4 west, 5 east.
pub const FACE_BOTTOM: u8 = 0;
pub const FACE_TOP: u8 = 1;
pub const FACE_NORTH: u8 = 2;
pub const FACE_SOUTH: u8 = 3;
pub const FACE_WEST: u8 = 4;
pub const FACE_EAST: u8 = 5;

/// Every face, in vanilla's order.
pub const FACES: [u8; 6] = [
    FACE_BOTTOM,
    FACE_TOP,
    FACE_NORTH,
    FACE_SOUTH,
    FACE_WEST,
    FACE_EAST,
];

/// The two 1.7.10 texture atlases: `TextureAtlas.blocks` and `TextureAtlas.items`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Atlas {
    Blocks,
    Items,
}

impl Atlas {
    /// The atlas-relative folder 1.7.10 registers its sprites under.
    pub fn path(self) -> &'static str {
        match self {
            Atlas::Blocks => sprites::BLOCKS_ATLAS_PATH,
            Atlas::Items => sprites::ITEMS_ATLAS_PATH,
        }
    }

    /// `TextureAtlas`'s numeric atlas id: 0 blocks, 1 items.
    fn atlas_type(self) -> u8 {
        match self {
            Atlas::Blocks => 0,
            Atlas::Items => 1,
        }
    }
}

/// One sprite the renderer draws, with the file it comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    atlas: Atlas,
    sprite: String,
}

impl Texture {
    /// A named sprite on `atlas` (`TextureRegistry.registerSprite`): the renderer's way to ask
    /// for the extra sprites a block or item class registers beside its id's base icon (a
    /// double plant's top, a bow's pulling frames, ...). `None` for an invalid name and for
    /// 1.7.10's `MISSING_ICON_*` placeholder, which is not a file.
    pub fn from_sprite(atlas: Atlas, sprite: impl Into<String>) -> Option<Self> {
        let sprite = sprite.into();
        if !sprites::sprite_name_is_valid(&sprite) || sprite.starts_with("MISSING_ICON_") {
            return None;
        }
        Some(Self { atlas, sprite })
    }

    /// The atlas this sprite belongs to.
    pub fn atlas(&self) -> Atlas {
        self.atlas
    }

    /// The sprite name as 1.7.10 registers it (`grass_top`, `wool_colored_red`).
    pub fn sprite(&self) -> &str {
        &self.sprite
    }

    /// The resource location (`minecraft:textures/blocks/grass_top.png`).
    pub fn location(&self) -> String {
        sprites::texture_location(self.atlas.atlas_type(), &self.sprite).to_string()
    }

    /// The file under the prepared resource folder
    /// (`assets/minecraft/textures/blocks/grass_top.png`).
    pub fn path(&self) -> String {
        let location = sprites::texture_location(self.atlas.atlas_type(), &self.sprite);
        format!("assets/{}/{}", location.namespace(), location.path())
    }
}

/// The sprite 1.7.10 registers for block `id` at `metadata` on `face`, or `None` when the block
/// draws nothing through the block atlas (air, the only such block 1.7.10 has).
pub fn block_texture(id: u16, metadata: u8, face: u8) -> Option<Texture> {
    if face > FACE_EAST {
        return None;
    }
    Texture::from_sprite(Atlas::Blocks, block_sprite(id, metadata, face)?)
}

/// `Block.getSprite(face, metadata)` for the classes the shared sprite table does not cover
/// (its port predates them) or where the table keeps the renderer's view instead of the
/// metadata-only one. Every arm is `net.minecraft.block.*Block.getSprite`, 1.7.10.
fn block_sprite(id: u16, metadata: u8, face: u8) -> Option<String> {
    match id {
        26 => Some(bed_sprite(metadata, face)),
        29 | 33 => Some(piston_base_sprite(id, metadata, face)),
        34 => Some(piston_head_sprite(metadata, face)),
        46 => Some(
            match face {
                FACE_BOTTOM => "tnt_bottom",
                FACE_TOP => "tnt_top",
                _ => "tnt_side",
            }
            .to_owned(),
        ),
        47 => Some(
            if face == FACE_TOP || face == FACE_BOTTOM {
                "planks_oak"
            } else {
                "bookshelf"
            }
            .to_owned(),
        ),
        58 => Some(
            match face {
                FACE_TOP => "crafting_table_top",
                FACE_BOTTOM => "planks_oak",
                FACE_NORTH | FACE_WEST => "crafting_table_front",
                _ => "crafting_table_side",
            }
            .to_owned(),
        ),
        // SignBlock and FenceGateBlock: `Blocks.PLANKS.getSprite(face)`, whose metadata-0
        // sprite is the oak planks one on every face.
        63 | 68 | 107 => Some("planks_oak".to_owned()),
        // PressurePlateBlock's step sound is the sprite name it registers.
        70 | 77 => Some("stone".to_owned()),
        72 | 143 => Some("planks_oak".to_owned()),
        115 => Some(format!(
            "nether_wart_stage_{}",
            match metadata {
                0 => 0,
                1 | 2 => 1,
                _ => 2,
            }
        )),
        118 => Some(
            match face {
                FACE_TOP => "cauldron_top",
                FACE_BOTTOM => "cauldron_bottom",
                _ => "cauldron_side",
            }
            .to_owned(),
        ),
        141 => Some(format!("carrots_stage_{}", crop_stage(metadata))),
        142 => Some(format!("potatoes_stage_{}", crop_stage(metadata))),
        154 => Some(
            if face == FACE_TOP {
                "hopper_top"
            } else {
                "hopper_outside"
            }
            .to_owned(),
        ),
        155 => Some(quartz_sprite(metadata, face)),
        161 => Some(
            if metadata & 3 == 1 {
                "leaves_big_oak"
            } else {
                "leaves_acacia"
            }
            .to_owned(),
        ),
        _ => sprites::block_texture(i32::from(id), i32::from(metadata), face),
    }
}

/// `BedBlock.getSprite`: the foot or head half's top, side or end sprite, oak planks underneath.
fn bed_sprite(metadata: u8, face: u8) -> String {
    if face == FACE_BOTTOM {
        return "planks_oak".to_owned();
    }
    const FACINGS: [[usize; 6]; 4] = [
        [1, 0, 3, 2, 5, 4],
        [1, 0, 5, 4, 2, 3],
        [1, 0, 2, 3, 4, 5],
        [1, 0, 4, 5, 3, 2],
    ];
    let relative = FACINGS[usize::from(metadata & 3)][usize::from(face)];
    let half = if metadata & 8 != 0 { "head" } else { "feet" };
    // The foot half's south end and the head half's north end show their end sprite.
    let end = (metadata & 8 != 0 && relative == 2) || (metadata & 8 == 0 && relative == 3);
    if end {
        format!("bed_{half}_end")
    } else if relative == FACE_WEST as usize || relative == FACE_EAST as usize {
        format!("bed_{half}_side")
    } else {
        format!("bed_{half}_top")
    }
}

/// `PistonBaseBlock.getSprite`: the top or inner sprite on the facing side (the inner one once
/// extended), planks-like bottom on the opposite side, the side sprite elsewhere.
fn piston_base_sprite(id: u16, metadata: u8, face: u8) -> String {
    const OPPOSITE: [u8; 6] = [1, 0, 3, 2, 5, 4];
    let facing = (metadata & 7).min(5);
    if face == facing {
        return if metadata & 8 != 0 {
            "piston_inner"
        } else if id == 29 {
            "piston_top_sticky"
        } else {
            "piston_top_normal"
        }
        .to_owned();
    }
    if face == OPPOSITE[usize::from(facing)] {
        "piston_bottom".to_owned()
    } else {
        "piston_side".to_owned()
    }
}

/// `PistonHeadBlock.getSprite` with no forced sprite (the moving-block renderer sets one while
/// a piston extends; the persisted block uses this).
fn piston_head_sprite(metadata: u8, face: u8) -> String {
    const OPPOSITE: [u8; 6] = [1, 0, 3, 2, 5, 4];
    let facing = (metadata & 7).min(5);
    if face == facing {
        return if metadata & 8 != 0 {
            "piston_top_sticky"
        } else {
            "piston_top_normal"
        }
        .to_owned();
    }
    if face == OPPOSITE[usize::from(facing)] {
        "piston_top_normal".to_owned()
    } else {
        "piston_side".to_owned()
    }
}

/// `WheatBlock`'s growth stage helper, shared by `CarrotsBlock` and `PotatoesBlock`: metadata
/// 6 shows stage 5's texture, 7 and above the last one.
fn crop_stage(metadata: u8) -> u8 {
    if metadata >= 7 {
        3
    } else {
        let stage = if metadata == 6 { 5 } else { metadata };
        stage >> 1
    }
}

/// `QuartzBlock.getSprite`: the default, chiselled and lines sprites, with the pillar's axis in
/// metadata 3 and 4 turning the two axis faces to the lines-top sprite.
fn quartz_sprite(metadata: u8, face: u8) -> String {
    match metadata {
        0 | 5..=u8::MAX => match face {
            FACE_TOP => "quartz_block_top".to_owned(),
            FACE_BOTTOM => "quartz_block_bottom".to_owned(),
            _ => "quartz_block_side".to_owned(),
        },
        1 => match face {
            FACE_TOP | FACE_BOTTOM => "quartz_block_chiseled_top".to_owned(),
            _ => "quartz_block_chiseled".to_owned(),
        },
        2 => match face {
            FACE_TOP | FACE_BOTTOM => "quartz_block_lines_top".to_owned(),
            _ => "quartz_block_lines".to_owned(),
        },
        3 => match face {
            FACE_WEST | FACE_EAST => "quartz_block_lines_top".to_owned(),
            _ => "quartz_block_lines".to_owned(),
        },
        _ => match face {
            FACE_NORTH | FACE_SOUTH => "quartz_block_lines_top".to_owned(),
            _ => "quartz_block_lines".to_owned(),
        },
    }
}

/// The block's six sprites in [`FACES`] order: what a cube renderer uploads for this id and
/// metadata. `None` on a face the block does not texture.
pub fn block_textures(id: u16, metadata: u8) -> [Option<Texture>; 6] {
    FACES.map(|face| block_texture(id, metadata, face))
}

/// The sprite 1.7.10 registers for item `id` at `metadata` (`damage`), or `None` when the item
/// draws nothing. A block item resolves through the block's top sprite at metadata 0, or through
/// its own item rule (slabs, stairs, wool, logs, ...), on whichever atlas 1.7.10 registers it.
pub fn item_texture(id: i32, metadata: i32) -> Option<Texture> {
    if sprites::item(id).is_some() {
        return named_item_texture(id, metadata);
    }
    block_item_texture(id, metadata)
}

/// `Item.getSprite(metadata)` and the item classes' overrides, for an id `Item.init` registers
/// as an item rather than a block item.
fn named_item_texture(id: i32, metadata: i32) -> Option<Texture> {
    let entry = sprites::item(id)?;
    let sprite = match id {
        // BowItem.registerSprites: the standby sprite is the inventory icon; the pulling frames
        // are the hand renderer's.
        261 => "bow_standby".to_owned(),
        // CoalItem.getSprite: metadata 1 is charcoal.
        263 => match metadata {
            1 => "charcoal".to_owned(),
            _ => "coal".to_owned(),
        },
        // FishingRodItem.registerSprites: the uncast sprite is the inventory icon.
        346 => "fishing_rod_uncast".to_owned(),
        // FishItem.getSprite: the type by metadata, cooked where the type can be cooked.
        349 | 350 => {
            let kind = sprites::FISH_TYPES[(metadata & 3) as usize];
            if id == 350 && kind.2 {
                format!("fish_{}_cooked", kind.1)
            } else {
                format!("fish_{}_raw", kind.1)
            }
        }
        // DyeItem.getSprite.
        351 => format!(
            "dye_powder_{}",
            sprites::DYE_COLOR_NAMES[metadata.clamp(0, 15) as usize]
        ),
        // PotionItem.getSprite: bit 16384 is the splash potion.
        373 => match metadata & 16384 {
            0 => "potion_bottle_drinkable".to_owned(),
            _ => "potion_bottle_splash".to_owned(),
        },
        // GlassBottleItem.getSprite: the potion's empty bottle icon.
        374 => "potion_bottle_drinkable".to_owned(),
        // SkullItem.getSprite.
        397 => format!(
            "skull_{}",
            sprites::SKULL_TYPES[metadata.clamp(0, 4) as usize]
        ),
        _ => entry.sprite?.to_owned(),
    };
    Texture::from_sprite(Atlas::Items, sprite)
}

/// `Item.init` gives no block item to these blocks (`var1` in the init, plus air).
const BLOCKS_WITHOUT_ITEM: [i32; 27] = [
    0, 26, 34, 36, 55, 59, 63, 64, 68, 71, 74, 75, 83, 92, 93, 94, 104, 105, 115, 117, 118, 124,
    132, 140, 144, 149, 150,
];

/// The `ColoredBlockItem` blocks: item metadata inverted to block metadata.
const COLORED_ITEM_BLOCKS: [i32; 5] = [35, 95, 159, 160, 171];

/// The `VariantBlockItem` blocks (`spriteProvider` is the block itself, the north face).
const VARIANT_ITEM_BLOCKS: [i32; 13] = [3, 5, 6, 12, 17, 24, 37, 38, 97, 98, 139, 155, 162];

/// The `LeavesItem` blocks (the bottom face).
const LEAVES_ITEM_BLOCKS: [i32; 2] = [18, 161];

/// The `SlabItem` blocks (the north face).
const SLAB_ITEM_BLOCKS: [i32; 4] = [43, 44, 125, 126];

/// The `PlantBlockItem` blocks (the bottom face).
const PLANT_ITEM_BLOCKS: [i32; 2] = [31, 106];

/// `BlockItem.getSprite(metadata)` and the block item classes' overrides: the blocks atlas for
/// every block item except the hopper, whose item icon lives on the items atlas.
fn block_item_texture(id: i32, metadata: i32) -> Option<Texture> {
    let block = sprites::block(id)?;
    if BLOCKS_WITHOUT_ITEM.contains(&id) {
        return None;
    }
    // `HopperBlock.getItemSpriteName` is the only non-null one of 1.7.10's block items.
    if id == 154 {
        return Texture::from_sprite(Atlas::Items, "hopper");
    }
    let metadata = (metadata & 15) as u8;
    let sprite = if COLORED_ITEM_BLOCKS.contains(&id) {
        block_sprite(block.id as u16, !metadata & 15, FACE_NORTH)?
    } else if VARIANT_ITEM_BLOCKS.contains(&id) || id == 145 {
        block_sprite(block.id as u16, metadata, FACE_NORTH)?
    } else if LEAVES_ITEM_BLOCKS.contains(&id) {
        block_sprite(block.id as u16, metadata, FACE_BOTTOM)?
    } else if SLAB_ITEM_BLOCKS.contains(&id) {
        block_sprite(block.id as u16, metadata, FACE_NORTH)?
    } else if PLANT_ITEM_BLOCKS.contains(&id) {
        block_sprite(block.id as u16, metadata, FACE_BOTTOM)?
    } else if id == 175 {
        // BushItem.getSprite: the sunflower's front, the other variants' top.
        let variant = metadata & 7;
        if variant == 0 {
            "double_plant_sunflower_front".to_owned()
        } else {
            format!(
                "double_plant_{}_top",
                sprites::DOUBLE_PLANT_TEXTURES[usize::from(variant.min(5))]
            )
        }
    } else {
        // `BlockItem.getSprite`: the block's top sprite at metadata 0.
        block_sprite(block.id as u16, 0, FACE_TOP)?
    };
    Texture::from_sprite(Atlas::Blocks, sprite)
}

/// Every block id the 1.7.10 content export emits, in registry order.
pub fn blocks() -> &'static [sunset_1_7_blocks::BlockDef] {
    sunset_1_7_blocks::BLOCKS
}

/// The block ids with no sprite of their own in 1.7.10: air, the only one. [`block_texture`]
/// returns `None` for every face and metadata value of these ids, and the manifest names each
/// one with the reason instead of guessing a texture.
pub fn textureless_blocks() -> &'static [u16] {
    static IDS: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        blocks()
            .iter()
            .filter(|block| {
                !(0..=15u8).any(|metadata| {
                    FACES
                        .into_iter()
                        .any(|face| block_texture(block.id, metadata, face).is_some())
                })
            })
            .map(|block| block.id)
            .collect()
    })
}

/// Whether block `id` registers at least one block-atlas sprite in 1.7.10. False for air and
/// for an id 1.7.10 does not have.
pub fn block_has_texture(id: u16) -> bool {
    blocks().iter().any(|block| block.id == id) && !textureless_blocks().contains(&id)
}

/// What one [`extract`] wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Assets in the manifest.
    pub files: usize,
    /// Files written this run.
    pub written: usize,
    /// Files already on disk with the JAR's bytes.
    pub unchanged: usize,
    /// Bytes the manifest accounts for.
    pub bytes: u64,
    /// Block ids the manifest covers.
    pub blocks: usize,
    /// Block ids that register at least one sprite.
    pub textured_blocks: usize,
    /// Block ids with no sprite of their own.
    pub textureless_blocks: usize,
}

struct FileEntry {
    path: String,
    sha256: String,
    bytes: u64,
}

struct BlockRow {
    id: u16,
    name: String,
    sprites: BTreeSet<String>,
}

/// Extracts the 1.7.10 assets from `jar` into `dir`, writes [`MANIFEST`] beside them, and
/// returns what the folder now holds. Files that already match the JAR are left untouched; the
/// previous manifest lets a run skip a file it already verified, and every destination whose
/// bytes changed is rewritten.
pub fn extract(jar: &Path, dir: &Path, log: Log) -> Result<Summary, String> {
    let mut archive = crate::open_zip(jar)?;
    let names = crate::zip_names(&archive)?;
    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|name| !names.iter().any(|present| present == name))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "{} is not a 1.7.10 client JAR: it has no {}",
            jar.display(),
            missing.join(", ")
        ));
    }

    let jar_sha256 = digest(jar, "sha256")?;
    let jar_bytes = fs::metadata(jar)
        .map_err(|e| format!("{}: {e}", jar.display()))?
        .len();
    let previous = previous_files(dir, &jar_sha256);
    let imported_sounds = previous_sounds(dir, &jar_sha256);
    let mut wanted: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| is_asset(name))
        .collect();
    wanted.sort_unstable();
    wanted.dedup();

    let lock = hold_lock(dir, log)?;
    let mut files = Vec::with_capacity(wanted.len());
    let (mut written, mut unchanged, mut bytes) = (0, 0, 0u64);
    for name in wanted {
        let relative = crate::member_path(name, "Invalid asset path in the client JAR")?;
        let target = dir.join(relative);
        if let Some((sha256, size)) = previous.get(name) {
            if target.is_file() && digest(&target, "sha256")? == *sha256 {
                files.push(FileEntry {
                    path: name.to_owned(),
                    sha256: sha256.clone(),
                    bytes: *size,
                });
                unchanged += 1;
                bytes += size;
                continue;
            }
        }
        let data = crate::zip_bytes(&mut archive, name)?;
        let sha256 = sha256_hex(&data);
        let size = data.len() as u64;
        if target.is_file() && digest(&target, "sha256")? == sha256 {
            unchanged += 1;
        } else {
            write_if_changed(&target, &data)?;
            written += 1;
        }
        files.push(FileEntry {
            path: name.to_owned(),
            sha256,
            bytes: size,
        });
        bytes += size;
    }

    let rows = block_rows();
    let textured = rows.iter().filter(|row| !row.sprites.is_empty()).count();
    let summary = Summary {
        files: files.len(),
        written,
        unchanged,
        bytes,
        blocks: rows.len(),
        textured_blocks: textured,
        textureless_blocks: rows.len() - textured,
    };
    let manifest = crate::pyjson::dumps(
        &manifest(
            jar,
            &jar_sha256,
            jar_bytes,
            &files,
            &rows,
            &names,
            imported_sounds,
        ),
        &crate::pyjson::INDENT2,
    ) + "\n";
    write_if_changed(&dir.join(MANIFEST), manifest.as_bytes())?;
    drop(lock);
    log(&format!(
        "{} files ({} written, {} unchanged, {} bytes) from {}; {} blocks, {} with textures, \
         {} textureless",
        summary.files,
        summary.written,
        summary.unchanged,
        summary.bytes,
        jar.display(),
        summary.blocks,
        summary.textured_blocks,
        summary.textureless_blocks
    ));
    Ok(summary)
}

/// [`extract`] with the preparer's defaults: the imported 1.7.10 client JAR at [`DEFAULT_JAR`]
/// and the prepared folder at [`DEFAULT_DIR`], then [`import_sounds`] from `sounds_from` or the
/// launcher store [`default_sounds_store`] finds. Returns the folder written.
pub fn prepare(
    root: &Path,
    output: Option<&Path>,
    jar: Option<&Path>,
    sounds_from: Option<&Path>,
    log: Log,
) -> Result<PathBuf, String> {
    let jar = jar.map_or_else(|| root.join(DEFAULT_JAR), Path::to_path_buf);
    let dir = crate::resources_dir(root, VERSION, output);
    extract(&jar, &dir, log)?;
    let store = match sounds_from {
        Some(store) => Some(store.to_path_buf()),
        None => default_sounds_store(),
    };
    match store {
        Some(store) => {
            import_sounds(&store, &dir, log)?;
        }
        None => log(&format!(
            "1.7.10 sounds not imported: no launcher asset store with indexes/{SOUNDS_INDEX} \
             was found; pass --sounds-from <store>"
        )),
    }
    Ok(dir)
}

/// What one [`import_sounds`] did with the era's sound objects.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoundSummary {
    /// Objects the index names as sounds (`sounds.json` and the `.ogg` files).
    pub indexed: usize,
    /// Objects present in the store.
    pub found: usize,
    /// Objects whose bytes match the index's SHA-1.
    pub verified: usize,
    /// Objects written to the prepared folder this run.
    pub copied: usize,
    /// Objects already there with the index's bytes.
    pub unchanged: usize,
    /// Objects missing from the store or not matching the index, skipped.
    pub skipped: usize,
    /// Bytes of the verified objects.
    pub bytes: u64,
}

struct SoundFile {
    path: String,
    sha1: String,
    bytes: u64,
}

struct SkippedSound {
    path: String,
    reason: String,
}

/// The stores [`default_sounds_store`] looks in, most specific first: Prism Launcher's (its
/// standard data folder, then its Flatpak one) and MultiMC's, each with the `assets` folder the
/// launcher keeps its index and objects in.
fn sounds_store_candidates() -> Vec<PathBuf> {
    let home = crate::artifacts::home();
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|d| !d.is_empty())
        .map_or_else(|| home.join(".local/share"), PathBuf::from);
    if cfg!(windows) {
        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.clone());
        return vec![app_data.join("PrismLauncher/assets")];
    }
    if cfg!(target_os = "macos") {
        return vec![home.join("Library/Application Support/PrismLauncher/assets")];
    }
    vec![
        data.join("PrismLauncher/assets"),
        home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/assets"),
        data.join("multimc/assets"),
    ]
}

/// The launcher asset store holding the era's index, when one of the folders [`prepare`] knows
/// holds it: the player's own install, never a download.
pub fn default_sounds_store() -> Option<PathBuf> {
    sounds_store_candidates()
        .into_iter()
        .find(|store| store.join("indexes").join(SOUNDS_INDEX).is_file())
}

/// Whether an asset index entry is one of the era's sounds: its `sounds.json` table or a sound
/// file under `sounds/`.
fn is_sound_asset(name: &str) -> bool {
    name == "minecraft/sounds.json"
        || (name.starts_with("minecraft/sounds/") && name.ends_with(".ogg"))
}

/// Copies the era's sounds from the launcher asset store at `store` into `dir`, checking every
/// object against the SHA-1 its index names and skipping (never accepting) a missing or
/// mismatched one. The files land where the engine's sound loader reads them
/// (`assets/minecraft/sounds.json`, `assets/minecraft/sounds/**.ogg`) and the manifest records
/// each verified file and every skip with its reason.
pub fn import_sounds(store: &Path, dir: &Path, log: Log) -> Result<SoundSummary, String> {
    let index_path = store.join("indexes").join(SOUNDS_INDEX);
    if !index_path.is_file() {
        return Err(format!(
            "{} is not a launcher asset store for 1.7.10: no {}; pass --sounds-from <store>",
            store.display(),
            index_path.display()
        ));
    }
    let index_bytes =
        fs::read(&index_path).map_err(|e| format!("{}: {e}", index_path.display()))?;
    let index_sha1 = crate::hex(&<sha1::Sha1 as sha1::Digest>::digest(&index_bytes));
    let index_text =
        std::str::from_utf8(&index_bytes).map_err(|e| format!("{}: {e}", index_path.display()))?;
    let index =
        crate::pyjson::loads(index_text).map_err(|e| format!("{}: {e}", index_path.display()))?;
    let objects = index
        .get("objects")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{} has no objects", index_path.display()))?;
    if !objects
        .iter()
        .any(|(name, _)| name == "minecraft/sounds.json")
    {
        return Err(format!(
            "{} is not the 1.7.10 asset index: it names no minecraft/sounds.json",
            index_path.display()
        ));
    }

    let lock = hold_lock(dir, log)?;
    let mut summary = SoundSummary::default();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut wanted: Vec<(&String, &Value)> = objects
        .iter()
        .filter(|(name, _)| is_sound_asset(name))
        .map(|(name, info)| (name, info))
        .collect();
    wanted.sort_unstable_by(|a, b| a.0.cmp(b.0));
    for (name, info) in wanted {
        summary.indexed += 1;
        let skip = |reason: String, skipped: &mut Vec<SkippedSound>| {
            log(&format!("1.7.10 sound {name} skipped: {reason}"));
            skipped.push(SkippedSound {
                path: name.clone(),
                reason,
            });
        };
        let Some(hash) = info.get("hash").and_then(Value::as_str) else {
            skip("the index gives no hash".into(), &mut skipped);
            summary.skipped += 1;
            continue;
        };
        let Some(size) = info
            .get("size")
            .and_then(Value::as_i64)
            .and_then(|size| u64::try_from(size).ok())
        else {
            skip("the index gives no size".into(), &mut skipped);
            summary.skipped += 1;
            continue;
        };
        let Some(relative) = crate::artifacts::safe_relative(name) else {
            skip("unsafe asset name in the index".into(), &mut skipped);
            summary.skipped += 1;
            continue;
        };
        let Ok(source) = crate::sounds::object_path(store, hash) else {
            skip("the index gives no usable SHA-1".into(), &mut skipped);
            summary.skipped += 1;
            continue;
        };
        if !source.is_file() {
            skip(format!("missing object {}", source.display()), &mut skipped);
            summary.skipped += 1;
            continue;
        }
        summary.found += 1;
        let actual = match digest(&source, "sha1") {
            Ok(actual) => actual,
            Err(error) => {
                skip(error, &mut skipped);
                summary.skipped += 1;
                continue;
            }
        };
        if actual != hash {
            skip(
                format!("the object's bytes are sha1 {actual}, the index says {hash}"),
                &mut skipped,
            );
            summary.skipped += 1;
            continue;
        }
        let destination = dir.join("assets").join(relative);
        let up_to_date =
            destination.is_file() && digest(&destination, "sha1").is_ok_and(|sha| sha == hash);
        if up_to_date {
            summary.unchanged += 1;
        } else {
            match crate::files::Import::stage(&source, &destination, hash, "sha1", false)
                .and_then(crate::files::Import::commit)
            {
                Ok(_) => summary.copied += 1,
                Err(error) => {
                    skip(
                        format!("could not write {}: {error}", destination.display()),
                        &mut skipped,
                    );
                    summary.skipped += 1;
                    continue;
                }
            }
        }
        summary.verified += 1;
        summary.bytes += size;
        files.push(SoundFile {
            path: destination
                .strip_prefix(dir)
                .unwrap_or(&destination)
                .to_string_lossy()
                .replace('\\', "/"),
            sha1: hash.to_owned(),
            bytes: size,
        });
    }

    record_sounds(dir, store, &index_sha1, &summary, &files, &skipped)?;
    drop(lock);
    log(&format!(
        "1.7.10 sound assets ready: {} objects ({} verified, {} copied, {} unchanged, {} skipped), \
         {:.1} MB from {}",
        summary.indexed,
        summary.verified,
        summary.copied,
        summary.unchanged,
        summary.skipped,
        summary.bytes as f64 / 1e6,
        store.display()
    ));
    Ok(summary)
}

/// Writes the import into the era manifest, keeping its files and blocks: the store, the index's
/// SHA-1, the counts, each verified file's hash and every skip with its reason.
fn record_sounds(
    dir: &Path,
    store: &Path,
    index_sha1: &str,
    summary: &SoundSummary,
    files: &[SoundFile],
    skipped: &[SkippedSound],
) -> Result<(), String> {
    let mut manifest = read_json(&dir.join(MANIFEST))?;
    let in_client_jar = manifest
        .get("sounds")
        .and_then(|sounds| sounds.get("in_client_jar"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    // What the folder holds, not what this run did: `copied` and `unchanged` differ between a
    // first import and an idempotent one, and a manifest that changed for that reason would not
    // be the same record of the same content.
    let counts = Value::Object(vec![
        ("indexed".into(), Value::Int(summary.indexed.to_string())),
        ("found".into(), Value::Int(summary.found.to_string())),
        ("verified".into(), Value::Int(summary.verified.to_string())),
        ("skipped".into(), Value::Int(summary.skipped.to_string())),
        ("bytes".into(), Value::Int(summary.bytes.to_string())),
    ]);
    let files = files
        .iter()
        .map(|file| {
            Value::Object(vec![
                ("path".into(), Value::Str(file.path.clone())),
                ("sha1".into(), Value::Str(file.sha1.clone())),
                ("bytes".into(), Value::Int(file.bytes.to_string())),
            ])
        })
        .collect();
    let skipped = skipped
        .iter()
        .map(|file| {
            Value::Object(vec![
                ("path".into(), Value::Str(file.path.clone())),
                ("reason".into(), Value::Str(file.reason.clone())),
            ])
        })
        .collect();
    manifest.insert(
        "sounds",
        Value::Object(vec![
            ("extracted".into(), Value::Bool(true)),
            ("store".into(), Value::Str(store.display().to_string())),
            ("index".into(), Value::Str(SOUNDS_INDEX.into())),
            ("index_sha1".into(), Value::Str(index_sha1.into())),
            (
                "in_client_jar".into(),
                Value::Int(in_client_jar.to_string()),
            ),
            ("counts".into(), counts),
            ("files".into(), Value::Array(files)),
            ("skipped".into(), Value::Array(skipped)),
            (
                "note".into(),
                Value::Str(
                    "the era's sounds come from the launcher's asset store, each object checked \
                     against the index's SHA-1; see skipped for the objects it could not use"
                        .into(),
                ),
            ),
        ]),
    );
    let text = crate::pyjson::dumps(&manifest, &crate::pyjson::INDENT2) + "\n";
    write_if_changed(&dir.join(MANIFEST), text.as_bytes())
}

/// The assets the era's client JAR carries. A 1.7.10 JAR without this many is not one this
/// extractor understands (the names are the ones the renderer and the font/UI code read first).
const REQUIRED: [&str; 13] = [
    "assets/minecraft/textures/blocks/stone.png",
    "assets/minecraft/textures/blocks/grass_top.png",
    "assets/minecraft/textures/items/stick.png",
    "assets/minecraft/textures/gui/icons.png",
    "assets/minecraft/textures/gui/title/background/panorama_0.png",
    "assets/minecraft/textures/gui/title/background/panorama_1.png",
    "assets/minecraft/textures/gui/title/background/panorama_2.png",
    "assets/minecraft/textures/gui/title/background/panorama_3.png",
    "assets/minecraft/textures/gui/title/background/panorama_4.png",
    "assets/minecraft/textures/gui/title/background/panorama_5.png",
    "assets/minecraft/textures/font/ascii.png",
    "assets/minecraft/font/glyph_sizes.bin",
    "assets/minecraft/lang/en_US.lang",
];

fn is_asset(name: &str) -> bool {
    let texture = |folder: &str| {
        name.strip_prefix(folder)
            .is_some_and(|rest| rest.ends_with(".png") || rest.ends_with(".png.mcmeta"))
    };
    texture("assets/minecraft/textures/blocks/")
        || texture("assets/minecraft/textures/items/")
        || texture("assets/minecraft/textures/gui/")
        || texture("assets/minecraft/textures/font/")
        || texture("assets/minecraft/textures/colormap/")
        // The sky's own pictures: sun, the moon-phase sheet, the vanilla cloud
        // texture, the End sky. The renderer's celestial pass reads them by path.
        || texture("assets/minecraft/textures/environment/")
        || name == "assets/minecraft/font/glyph_sizes.bin"
        || name == "assets/minecraft/lang/en_US.lang"
}

/// The manifest an earlier run wrote for this JAR, when it is still this layout's.
fn previous_manifest(dir: &Path, jar_sha256: &str) -> Option<Value> {
    let manifest = read_json(&dir.join(MANIFEST)).ok()?;
    let same = manifest.get("schema").and_then(Value::as_i64) == Some(MANIFEST_SCHEMA)
        && manifest.get("minecraft").and_then(Value::as_str) == Some(VERSION)
        && manifest
            .get("client_jar")
            .and_then(|jar| jar.get("sha256"))
            .and_then(Value::as_str)
            == Some(jar_sha256);
    same.then_some(manifest)
}

fn previous_files(dir: &Path, jar_sha256: &str) -> HashMap<String, (String, u64)> {
    let Some(manifest) = previous_manifest(dir, jar_sha256) else {
        return HashMap::new();
    };
    let mut files = HashMap::new();
    for entry in manifest
        .get("files")
        .and_then(Value::as_array)
        .unwrap_or_default()
    {
        let (Some(path), Some(sha256), Some(bytes)) = (
            entry.get("path").and_then(Value::as_str),
            entry.get("sha256").and_then(Value::as_str),
            entry.get("bytes").and_then(Value::as_i64),
        ) else {
            continue;
        };
        if let Ok(bytes) = u64::try_from(bytes) {
            files.insert(path.to_owned(), (sha256.to_owned(), bytes));
        }
    }
    files
}

/// The sound import an earlier run recorded, which a re-extraction keeps instead of writing the
/// "not in the JAR" stub over it.
fn previous_sounds(dir: &Path, jar_sha256: &str) -> Option<Value> {
    let manifest = previous_manifest(dir, jar_sha256)?;
    let sounds = manifest.get("sounds")?;
    let imported = matches!(sounds.get("extracted"), Some(Value::Bool(true)));
    imported.then(|| sounds.clone())
}

fn block_rows() -> Vec<BlockRow> {
    blocks()
        .iter()
        .map(|def| {
            let mut sprites = BTreeSet::new();
            for metadata in 0..=15u8 {
                for face in FACES {
                    if let Some(texture) = block_texture(def.id, metadata, face) {
                        sprites.insert(texture.sprite().to_owned());
                    }
                }
            }
            BlockRow {
                id: def.id,
                name: format!("minecraft:{}", def.name),
                sprites,
            }
        })
        .collect()
}

fn block_reason(id: u16) -> &'static str {
    match id {
        0 => "air draws nothing",
        34 | 36 => "piston technical block, never drawn through the block atlas",
        63 | 68 => "sign, drawn from its entity model's sign texture",
        119 => "end portal, drawn by its own render pass",
        _ => "no block-atlas sprite is registered in 1.7.10",
    }
}

fn manifest(
    jar: &Path,
    jar_sha256: &str,
    jar_bytes: u64,
    files: &[FileEntry],
    rows: &[BlockRow],
    names: &[String],
    imported_sounds: Option<Value>,
) -> Value {
    let files = files
        .iter()
        .map(|file| {
            Value::Object(vec![
                ("path".into(), Value::Str(file.path.clone())),
                ("sha256".into(), Value::Str(file.sha256.clone())),
                ("bytes".into(), Value::Int(file.bytes.to_string())),
            ])
        })
        .collect();
    let blocks = rows
        .iter()
        .map(|row| {
            let mut fields = vec![
                ("id".into(), Value::Int(row.id.to_string())),
                ("name".into(), Value::Str(row.name.clone())),
                ("textureless".into(), Value::Bool(row.sprites.is_empty())),
            ];
            if row.sprites.is_empty() {
                fields.push(("reason".into(), Value::Str(block_reason(row.id).into())));
            } else {
                fields.push((
                    "sprites".into(),
                    Value::Array(row.sprites.iter().cloned().map(Value::Str).collect()),
                ));
            }
            Value::Object(fields)
        })
        .collect();
    Value::Object(vec![
        ("schema".into(), Value::Int(MANIFEST_SCHEMA.to_string())),
        ("minecraft".into(), Value::Str(VERSION.into())),
        (
            "client_jar".into(),
            Value::Object(vec![
                ("path".into(), Value::Str(jar.display().to_string())),
                ("sha256".into(), Value::Str(jar_sha256.into())),
                ("bytes".into(), Value::Int(jar_bytes.to_string())),
            ]),
        ),
        ("files".into(), Value::Array(files)),
        ("blocks".into(), Value::Array(blocks)),
        (
            "sounds".into(),
            imported_sounds.unwrap_or_else(|| sounds(names)),
        ),
    ])
}

/// 1.7.10's sounds are not in the client JAR on this machine: the launcher keeps them as the
/// asset index's `minecraft/sounds/...` objects. The manifest records what the JAR had, so the
/// audio step knows it must read the player's asset store and never treats silence as extracted.
fn sounds(names: &[String]) -> Value {
    let ogg = names
        .iter()
        .filter(|name| name.starts_with("assets/minecraft/sounds/") && name.ends_with(".ogg"))
        .count();
    let sounds_json = names
        .iter()
        .any(|name| name == "assets/minecraft/sounds.json");
    let present = ogg > 0 || sounds_json;
    let note = if present {
        "the client JAR carries sound files; this extraction copies textures, language, fonts and \
         the panorama only, so the audio step still imports the sound objects"
    } else {
        "1.7.10 keeps its sounds in the launcher's asset object store (assets/indexes and \
         assets/objects), not in the client JAR; the audio step reads them from the player's \
         install. Nothing was invented here."
    };
    Value::Object(vec![
        ("extracted".into(), Value::Bool(false)),
        ("in_client_jar".into(), Value::Int(ogg.to_string())),
        ("sounds_json".into(), Value::Bool(sounds_json)),
        ("note".into(), Value::Str(note.into())),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PANORAMA: [&str; 6] = [
        "assets/minecraft/textures/gui/title/background/panorama_0.png",
        "assets/minecraft/textures/gui/title/background/panorama_1.png",
        "assets/minecraft/textures/gui/title/background/panorama_2.png",
        "assets/minecraft/textures/gui/title/background/panorama_3.png",
        "assets/minecraft/textures/gui/title/background/panorama_4.png",
        "assets/minecraft/textures/gui/title/background/panorama_5.png",
    ];

    /// A JAR with the entries this extractor requires, a few more of each kind, and members a
    /// 1.7.10 client also carries but this extraction leaves alone.
    fn sample_entries() -> Vec<(&'static str, &'static [u8])> {
        let mut entries = vec![
            (
                "assets/minecraft/textures/blocks/stone.png",
                b"stone".as_slice(),
            ),
            (
                "assets/minecraft/textures/blocks/grass_top.png",
                b"grass top".as_slice(),
            ),
            (
                "assets/minecraft/textures/blocks/wool_colored_white.png",
                b"white wool".as_slice(),
            ),
            (
                "assets/minecraft/textures/blocks/water_still.png.mcmeta",
                b"{\"animation\":{}}".as_slice(),
            ),
            (
                "assets/minecraft/textures/items/stick.png",
                b"stick".as_slice(),
            ),
            (
                "assets/minecraft/textures/items/diamond.png",
                b"diamond".as_slice(),
            ),
            (
                "assets/minecraft/textures/gui/icons.png",
                b"icons".as_slice(),
            ),
            (
                "assets/minecraft/textures/gui/widgets.png",
                b"widgets".as_slice(),
            ),
            (
                "assets/minecraft/textures/font/ascii.png",
                b"ascii".as_slice(),
            ),
            (
                "assets/minecraft/font/glyph_sizes.bin",
                b"glyphs".as_slice(),
            ),
            (
                "assets/minecraft/lang/en_US.lang",
                b"language.name=English\n".as_slice(),
            ),
            (
                "assets/minecraft/textures/colormap/grass.png",
                b"colormap".as_slice(),
            ),
            (
                "assets/minecraft/textures/entity/steve.png",
                b"entity".as_slice(),
            ),
            ("version.json", b"{}".as_slice()),
            ("net/minecraft/world/World.class", b"class".as_slice()),
        ];
        for panorama in PANORAMA {
            entries.push((panorama, b"panorama".as_slice()));
        }
        entries
    }

    fn write_jar(path: &Path, entries: &[(&str, &[u8])]) {
        let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, data) in entries {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(data).unwrap();
        }
        archive.finish().unwrap();
    }

    fn boolean(value: &Value) -> Option<bool> {
        match value {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn scratch(label: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "sunrise-assets-1710-{}-{label}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn files_under(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_under(root, &path, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }

    fn extracted(label: &str) -> (PathBuf, PathBuf, Vec<(&'static str, &'static [u8])>) {
        let root = scratch(label);
        let jar = root.join("client.jar");
        let entries = sample_entries();
        write_jar(&jar, &entries);
        let dir = root.join("resources");
        (jar, dir, entries)
    }

    fn block_path(id: u16, metadata: u8, face: u8) -> String {
        block_texture(id, metadata, face)
            .unwrap_or_else(|| panic!("block {id}/{metadata}/{face} resolved nothing"))
            .path()
    }

    fn item_path(id: i32, metadata: i32) -> String {
        item_texture(id, metadata)
            .unwrap_or_else(|| panic!("item {id}/{metadata} resolved nothing"))
            .path()
    }

    #[test]
    fn extraction_writes_the_assets_and_a_manifest_that_matches_them() {
        let (jar, dir, entries) = extracted("manifest");
        let log = |_: &str| {};
        let summary = extract(&jar, &dir, &log).unwrap();

        let expected: Vec<&str> = entries
            .iter()
            .map(|(name, _)| *name)
            .filter(|name| is_asset(name))
            .collect();
        assert_eq!(summary.files, expected.len());
        assert_eq!(summary.written, summary.files);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(summary.blocks, blocks().len());
        assert_eq!(summary.textureless_blocks, 1);
        assert_eq!(
            summary.bytes,
            entries
                .iter()
                .filter(|(name, _)| is_asset(name))
                .map(|(_, data)| data.len() as u64)
                .sum::<u64>()
        );

        for name in &expected {
            assert!(dir.join(name).is_file(), "{name} was not extracted");
        }
        assert!(!dir
            .join("assets/minecraft/textures/entity/steve.png")
            .exists());
        assert!(!dir.join("version.json").exists());
        assert!(!dir.join("net/minecraft/world/World.class").exists());

        let manifest = crate::read_json(&dir.join(MANIFEST)).unwrap();
        assert_eq!(
            manifest.get("schema").and_then(Value::as_i64),
            Some(MANIFEST_SCHEMA)
        );
        assert_eq!(
            manifest.get("minecraft").and_then(Value::as_str),
            Some(VERSION)
        );
        let jar_sha256 = digest(&jar, "sha256").unwrap();
        assert_eq!(
            manifest
                .get("client_jar")
                .and_then(|jar| jar.get("sha256"))
                .and_then(Value::as_str),
            Some(jar_sha256.as_str())
        );

        let files = manifest.get("files").and_then(Value::as_array).unwrap();
        assert_eq!(files.len(), expected.len());
        let mut listed: Vec<&str> = files
            .iter()
            .map(|file| file.get("path").and_then(Value::as_str).unwrap())
            .collect();
        let sorted = listed.clone();
        listed.sort_unstable();
        assert_eq!(listed, sorted, "the manifest's files are not sorted");
        assert_eq!(listed, {
            let mut expected = expected.clone();
            expected.sort_unstable();
            expected
        });
        for file in files {
            let path = dir.join(file.get("path").and_then(Value::as_str).unwrap());
            let bytes = fs::metadata(&path).unwrap().len();
            let sha256 = digest(&path, "sha256").unwrap();
            assert_eq!(
                file.get("bytes").and_then(Value::as_i64),
                Some(bytes as i64),
                "{}",
                path.display()
            );
            assert_eq!(
                file.get("sha256").and_then(Value::as_str),
                Some(sha256.as_str()),
                "{}",
                path.display()
            );
        }
        let mut on_disk = Vec::new();
        files_under(&dir, &dir.join("assets"), &mut on_disk);
        on_disk.sort_unstable();
        assert_eq!(
            on_disk,
            listed
                .iter()
                .map(|path| (*path).to_owned())
                .collect::<Vec<_>>(),
            "the manifest and the extracted files disagree"
        );

        let blocks = manifest.get("blocks").and_then(Value::as_array).unwrap();
        assert_eq!(blocks.len(), self::blocks().len());
        let air = blocks
            .iter()
            .find(|block| block.get("id").and_then(Value::as_i64) == Some(0))
            .unwrap();
        assert_eq!(air.get("textureless").and_then(boolean), Some(true));
        assert!(air.get("reason").and_then(Value::as_str).is_some());
        let stone = blocks
            .iter()
            .find(|block| block.get("id").and_then(Value::as_i64) == Some(1))
            .unwrap();
        assert_eq!(stone.get("textureless").and_then(boolean), Some(false));
        assert_eq!(
            stone.get("sprites").and_then(Value::as_array),
            Some(&[Value::Str("stone".into())][..])
        );
        let grass = blocks
            .iter()
            .find(|block| block.get("id").and_then(Value::as_i64) == Some(2))
            .unwrap();
        let sprites: Vec<&str> = grass
            .get("sprites")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        for sprite in ["dirt", "grass_top", "grass_side"] {
            assert!(
                sprites.contains(&sprite),
                "grass has no {sprite}: {sprites:?}"
            );
        }
        for block in blocks {
            let id = block.get("id").and_then(Value::as_i64).unwrap();
            if block.get("textureless").and_then(boolean).unwrap() {
                assert!(
                    block.get("reason").and_then(Value::as_str).is_some(),
                    "block {id} is textureless without saying why"
                );
                assert!(block.get("sprites").is_none(), "block {id} has both");
            } else {
                assert!(
                    !block
                        .get("sprites")
                        .and_then(Value::as_array)
                        .unwrap()
                        .is_empty(),
                    "block {id} lists no sprites"
                );
            }
        }

        let sounds = manifest.get("sounds").unwrap();
        assert_eq!(sounds.get("in_client_jar").and_then(Value::as_i64), Some(0));
        assert_eq!(sounds.get("extracted").and_then(boolean), Some(false));
        assert!(sounds
            .get("note")
            .and_then(Value::as_str)
            .unwrap()
            .contains("asset object store"));
    }

    #[test]
    fn a_second_run_writes_only_what_changed() {
        let (jar, dir, entries) = extracted("idempotent");
        let log = |_: &str| {};
        let first = extract(&jar, &dir, &log).unwrap();
        assert_eq!(first.written, first.files);
        let manifest = fs::read(dir.join(MANIFEST)).unwrap();

        let second = extract(&jar, &dir, &log).unwrap();
        assert_eq!(second.written, 0);
        assert_eq!(second.unchanged, second.files);
        assert_eq!(fs::read(dir.join(MANIFEST)).unwrap(), manifest);

        let stone = dir.join("assets/minecraft/textures/blocks/stone.png");
        fs::write(&stone, b"corrupt").unwrap();
        let repaired = extract(&jar, &dir, &log).unwrap();
        assert_eq!(repaired.written, 1);
        assert_eq!(repaired.unchanged, repaired.files - 1);
        assert_eq!(fs::read(&stone).unwrap(), b"stone");
        assert_eq!(fs::read(dir.join(MANIFEST)).unwrap(), manifest);

        let mut changed = entries.clone();
        changed
            .iter_mut()
            .find(|(name, _)| *name == "assets/minecraft/textures/blocks/stone.png")
            .unwrap()
            .1 = b"new stone";
        write_jar(&jar, &changed);
        let updated = extract(&jar, &dir, &log).unwrap();
        assert_eq!(updated.written, 1);
        assert_eq!(updated.unchanged, updated.files - 1);
        assert_eq!(fs::read(&stone).unwrap(), b"new stone");
        assert_ne!(fs::read(dir.join(MANIFEST)).unwrap(), manifest);
    }

    #[test]
    fn a_jar_without_the_expected_assets_is_refused() {
        let root = scratch("refused");
        let jar = root.join("client.jar");
        let entries: Vec<(&str, &[u8])> = sample_entries()
            .into_iter()
            .filter(|(name, _)| *name != "assets/minecraft/textures/blocks/stone.png")
            .collect();
        write_jar(&jar, &entries);
        let log = |_: &str| {};
        let error = extract(&jar, &root.join("resources"), &log).unwrap_err();
        assert!(error.contains("not a 1.7.10 client JAR"), "{error}");
        assert!(error.contains("stone.png"), "{error}");
    }

    #[test]
    fn known_block_textures_resolve() {
        let blocks = "assets/minecraft/textures/blocks";
        assert_eq!(block_path(1, 0, FACE_TOP), format!("{blocks}/stone.png"));
        assert_eq!(
            block_path(2, 0, FACE_TOP),
            format!("{blocks}/grass_top.png")
        );
        assert_eq!(
            block_path(2, 0, FACE_NORTH),
            format!("{blocks}/grass_side.png")
        );
        assert_eq!(block_path(2, 0, FACE_BOTTOM), format!("{blocks}/dirt.png"));

        assert_eq!(
            block_path(35, 0, FACE_NORTH),
            format!("{blocks}/wool_colored_white.png")
        );
        assert_eq!(
            block_path(35, 4, FACE_NORTH),
            format!("{blocks}/wool_colored_yellow.png")
        );
        assert_eq!(
            block_path(35, 14, FACE_NORTH),
            format!("{blocks}/wool_colored_red.png")
        );
        assert_eq!(
            block_path(35, 15, FACE_NORTH),
            format!("{blocks}/wool_colored_black.png")
        );

        assert_eq!(
            block_path(17, 0, FACE_TOP),
            format!("{blocks}/log_oak_top.png")
        );
        assert_eq!(
            block_path(17, 0, FACE_NORTH),
            format!("{blocks}/log_oak.png")
        );
        assert_eq!(
            block_path(17, 4, FACE_WEST),
            format!("{blocks}/log_oak_top.png")
        );
        assert_eq!(
            block_path(17, 4, FACE_NORTH),
            format!("{blocks}/log_oak.png")
        );
        assert_eq!(
            block_path(17, 8, FACE_SOUTH),
            format!("{blocks}/log_oak_top.png")
        );
        assert_eq!(block_path(17, 8, FACE_TOP), format!("{blocks}/log_oak.png"));
        assert_eq!(
            block_path(162, 1, FACE_TOP),
            format!("{blocks}/log_big_oak_top.png")
        );

        assert_eq!(
            block_path(53, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(67, 0, FACE_NORTH),
            format!("{blocks}/cobblestone.png")
        );
        assert_eq!(
            block_path(134, 0, FACE_NORTH),
            format!("{blocks}/planks_spruce.png")
        );
        assert_eq!(
            block_path(156, 0, FACE_TOP),
            format!("{blocks}/quartz_block_top.png")
        );

        for id in [8, 9] {
            assert_eq!(
                block_path(id, 0, FACE_TOP),
                format!("{blocks}/water_still.png")
            );
            assert_eq!(
                block_path(id, 0, FACE_BOTTOM),
                format!("{blocks}/water_still.png")
            );
            assert_eq!(
                block_path(id, 0, FACE_NORTH),
                format!("{blocks}/water_flow.png")
            );
        }
        for id in [10, 11] {
            assert_eq!(
                block_path(id, 0, FACE_TOP),
                format!("{blocks}/lava_still.png")
            );
            assert_eq!(
                block_path(id, 0, FACE_NORTH),
                format!("{blocks}/lava_flow.png")
            );
        }

        assert_eq!(
            block_path(50, 0, FACE_NORTH),
            format!("{blocks}/torch_on.png")
        );

        assert_eq!(
            block_path(31, 0, FACE_NORTH),
            format!("{blocks}/deadbush.png")
        );
        assert_eq!(
            block_path(31, 1, FACE_NORTH),
            format!("{blocks}/tallgrass.png")
        );
        assert_eq!(block_path(31, 2, FACE_NORTH), format!("{blocks}/fern.png"));

        assert_eq!(
            block_path(26, 0, FACE_BOTTOM),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(26, 0, FACE_TOP),
            format!("{blocks}/bed_feet_top.png")
        );
        assert_eq!(
            block_path(26, 0, FACE_NORTH),
            format!("{blocks}/bed_feet_end.png")
        );
        assert_eq!(
            block_path(26, 0, FACE_WEST),
            format!("{blocks}/bed_feet_side.png")
        );
        assert_eq!(
            block_path(26, 8, FACE_TOP),
            format!("{blocks}/bed_head_top.png")
        );
        assert_eq!(
            block_path(26, 8, FACE_SOUTH),
            format!("{blocks}/bed_head_end.png")
        );
        assert_eq!(
            block_path(26, 8, FACE_EAST),
            format!("{blocks}/bed_head_side.png")
        );
        assert_eq!(
            block_path(26, 2, FACE_SOUTH),
            format!("{blocks}/bed_feet_end.png")
        );
        assert_eq!(
            block_path(26, 3, FACE_WEST),
            format!("{blocks}/bed_feet_end.png")
        );
        assert_eq!(
            block_path(26, 10, FACE_NORTH),
            format!("{blocks}/bed_head_end.png")
        );

        assert_eq!(
            block_path(58, 0, FACE_TOP),
            format!("{blocks}/crafting_table_top.png")
        );
        assert_eq!(
            block_path(58, 0, FACE_BOTTOM),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(58, 0, FACE_NORTH),
            format!("{blocks}/crafting_table_front.png")
        );
        assert_eq!(
            block_path(58, 0, FACE_WEST),
            format!("{blocks}/crafting_table_front.png")
        );
        assert_eq!(
            block_path(58, 0, FACE_EAST),
            format!("{blocks}/crafting_table_side.png")
        );

        assert_eq!(block_path(46, 0, FACE_TOP), format!("{blocks}/tnt_top.png"));
        assert_eq!(
            block_path(46, 0, FACE_BOTTOM),
            format!("{blocks}/tnt_bottom.png")
        );
        assert_eq!(
            block_path(46, 0, FACE_NORTH),
            format!("{blocks}/tnt_side.png")
        );

        assert_eq!(
            block_path(47, 0, FACE_TOP),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(47, 0, FACE_NORTH),
            format!("{blocks}/bookshelf.png")
        );

        assert_eq!(
            block_path(33, 1, FACE_TOP),
            format!("{blocks}/piston_top_normal.png")
        );
        assert_eq!(
            block_path(33, 9, FACE_TOP),
            format!("{blocks}/piston_inner.png")
        );
        assert_eq!(
            block_path(29, 1, FACE_TOP),
            format!("{blocks}/piston_top_sticky.png")
        );
        assert_eq!(
            block_path(34, 1, FACE_TOP),
            format!("{blocks}/piston_top_normal.png")
        );
        assert_eq!(
            block_path(34, 9, FACE_TOP),
            format!("{blocks}/piston_top_sticky.png")
        );
        assert_eq!(
            block_path(34, 0, FACE_NORTH),
            format!("{blocks}/piston_side.png")
        );

        assert_eq!(
            block_path(155, 1, FACE_TOP),
            format!("{blocks}/quartz_block_chiseled_top.png")
        );
        assert_eq!(
            block_path(155, 2, FACE_TOP),
            format!("{blocks}/quartz_block_lines_top.png")
        );
        assert_eq!(
            block_path(155, 3, FACE_WEST),
            format!("{blocks}/quartz_block_lines_top.png")
        );
        assert_eq!(
            block_path(155, 3, FACE_NORTH),
            format!("{blocks}/quartz_block_lines.png")
        );

        assert_eq!(
            block_path(118, 0, FACE_TOP),
            format!("{blocks}/cauldron_top.png")
        );
        assert_eq!(
            block_path(118, 0, FACE_BOTTOM),
            format!("{blocks}/cauldron_bottom.png")
        );
        assert_eq!(
            block_path(118, 0, FACE_NORTH),
            format!("{blocks}/cauldron_side.png")
        );
        assert_eq!(
            block_path(154, 0, FACE_TOP),
            format!("{blocks}/hopper_top.png")
        );
        assert_eq!(
            block_path(154, 0, FACE_NORTH),
            format!("{blocks}/hopper_outside.png")
        );

        assert_eq!(
            block_path(115, 0, FACE_NORTH),
            format!("{blocks}/nether_wart_stage_0.png")
        );
        assert_eq!(
            block_path(115, 2, FACE_NORTH),
            format!("{blocks}/nether_wart_stage_1.png")
        );
        assert_eq!(
            block_path(115, 3, FACE_NORTH),
            format!("{blocks}/nether_wart_stage_2.png")
        );
        assert_eq!(
            block_path(141, 2, FACE_NORTH),
            format!("{blocks}/carrots_stage_1.png")
        );
        assert_eq!(
            block_path(141, 7, FACE_NORTH),
            format!("{blocks}/carrots_stage_3.png")
        );
        assert_eq!(
            block_path(142, 2, FACE_NORTH),
            format!("{blocks}/potatoes_stage_1.png")
        );

        assert_eq!(
            block_path(63, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(68, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(
            block_path(107, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(block_path(70, 0, FACE_NORTH), format!("{blocks}/stone.png"));
        assert_eq!(
            block_path(72, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );
        assert_eq!(block_path(77, 0, FACE_NORTH), format!("{blocks}/stone.png"));
        assert_eq!(
            block_path(143, 0, FACE_NORTH),
            format!("{blocks}/planks_oak.png")
        );

        assert_eq!(
            block_path(161, 1, FACE_NORTH),
            format!("{blocks}/leaves_big_oak.png")
        );
        assert_eq!(
            block_path(161, 2, FACE_NORTH),
            format!("{blocks}/leaves_acacia.png")
        );

        let texture = block_texture(1, 0, FACE_TOP).unwrap();
        assert_eq!(texture.atlas(), Atlas::Blocks);
        assert_eq!(texture.sprite(), "stone");
        assert_eq!(texture.location(), "minecraft:textures/blocks/stone.png");
    }

    #[test]
    fn known_item_textures_resolve() {
        let blocks = "assets/minecraft/textures/blocks";
        let items = "assets/minecraft/textures/items";
        assert_eq!(item_path(256, 0), format!("{items}/iron_shovel.png"));
        assert_eq!(item_path(263, 0), format!("{items}/coal.png"));
        assert_eq!(item_path(263, 1), format!("{items}/charcoal.png"));
        assert_eq!(item_path(346, 0), format!("{items}/fishing_rod_uncast.png"));
        assert_eq!(item_path(349, 1), format!("{items}/fish_salmon_raw.png"));
        assert_eq!(item_path(350, 1), format!("{items}/fish_salmon_cooked.png"));
        assert_eq!(item_path(351, 15), format!("{items}/dye_powder_white.png"));
        assert_eq!(
            item_path(373, 0),
            format!("{items}/potion_bottle_drinkable.png")
        );
        assert_eq!(
            item_path(373, 16384),
            format!("{items}/potion_bottle_splash.png")
        );
        assert_eq!(
            item_path(374, 0),
            format!("{items}/potion_bottle_drinkable.png")
        );
        assert_eq!(item_path(397, 3), format!("{items}/skull_steve.png"));

        assert_eq!(item_path(35, 0), format!("{blocks}/wool_colored_black.png"));
        assert_eq!(
            item_path(35, 15),
            format!("{blocks}/wool_colored_white.png")
        );
        assert_eq!(item_path(5, 2), format!("{blocks}/planks_birch.png"));
        assert_eq!(item_path(58, 0), format!("{blocks}/crafting_table_top.png"));
        assert_eq!(item_path(46, 0), format!("{blocks}/tnt_top.png"));
        assert_eq!(item_path(47, 0), format!("{blocks}/planks_oak.png"));
        assert_eq!(item_path(145, 0), format!("{blocks}/anvil_base.png"));
        assert_eq!(item_path(44, 1), format!("{blocks}/sandstone_normal.png"));
        assert_eq!(
            item_path(175, 0),
            format!("{blocks}/double_plant_sunflower_front.png")
        );
        assert_eq!(
            item_path(175, 1),
            format!("{blocks}/double_plant_syringa_top.png")
        );

        let hopper = item_texture(154, 0).unwrap();
        assert_eq!(hopper.atlas(), Atlas::Items);
        assert_eq!(hopper.path(), format!("{items}/hopper.png"));

        assert_eq!(item_texture(26, 0), None, "a bed has no block item");
        assert_eq!(item_texture(63, 0), None, "a sign has no block item");
        assert_eq!(item_texture(70, 0).unwrap().atlas(), Atlas::Blocks);

        let shovel = item_texture(256, 0).unwrap();
        assert_eq!(shovel.atlas(), Atlas::Items);
        assert_eq!(
            shovel.location(),
            "minecraft:textures/items/iron_shovel.png"
        );
    }

    #[test]
    fn every_exported_block_resolves_or_is_textureless() {
        let mut textureless = Vec::new();
        for block in blocks() {
            let mut resolved = false;
            for metadata in 0..=15u8 {
                for face in FACES {
                    if let Some(texture) = block_texture(block.id, metadata, face) {
                        resolved = true;
                        assert_eq!(texture.atlas(), Atlas::Blocks);
                        assert!(
                            texture
                                .path()
                                .starts_with("assets/minecraft/textures/blocks/"),
                            "{}",
                            texture.path()
                        );
                        assert!(texture.path().ends_with(".png"), "{}", texture.path());
                    }
                }
            }
            if !resolved {
                textureless.push((block.id, block.name));
            }
        }
        assert_eq!(
            textureless,
            vec![(0, "air")],
            "only air has no sprite in 1.7.10; the manifest names every textureless block"
        );
        assert!(blocks().iter().any(|block| block.name == "stone"));
        assert!(block_has_texture(1));
        assert!(!block_has_texture(0));
        assert!(!block_has_texture(165), "1.7.10 has no block 165");
    }

    /// The era's sound table and a few `.ogg` objects in a launcher store, indexed by
    /// `indexes/1.7.10.json` with the hashes the objects actually have.
    fn sound_store(root: &Path) -> (PathBuf, Vec<(String, Vec<u8>)>) {
        let store = root.join("store");
        let sounds_json = br#"{
            "dig.stone": {"category": "block", "sounds": ["dig/stone1", "dig/stone2"]},
            "step.grass": {"category": "neutral", "sounds": ["step/grass1"]},
            "dig.wood": {"category": "block", "sounds": ["dig/wood1"]}
        }"#;
        let objects: Vec<(String, Vec<u8>)> = vec![
            ("minecraft/sounds.json".into(), sounds_json.to_vec()),
            (
                "minecraft/sounds/dig/stone1.ogg".into(),
                b"OggS stone one".to_vec(),
            ),
            (
                "minecraft/sounds/dig/stone2.ogg".into(),
                b"OggS stone two".to_vec(),
            ),
            (
                "minecraft/sounds/step/grass1.ogg".into(),
                b"OggS grass".to_vec(),
            ),
            (
                "minecraft/sounds/dig/wood1.ogg".into(),
                b"OggS wood".to_vec(),
            ),
            // Not a sound: the store holds every asset, and the import leaves these alone.
            (
                "minecraft/textures/blocks/stone.png".into(),
                b"PNG stone".to_vec(),
            ),
        ];
        for (_, data) in &objects {
            let hash = crate::hex(&<sha1::Sha1 as sha1::Digest>::digest(data));
            let path = store.join("objects").join(&hash[..2]).join(&hash);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, data).unwrap();
        }
        let mut index = String::from("{\"objects\": {");
        for (position, (name, data)) in objects.iter().enumerate() {
            let hash = crate::hex(&<sha1::Sha1 as sha1::Digest>::digest(data));
            let comma = if position == 0 { "" } else { ", " };
            index += &format!(
                "{comma}\"{name}\": {{\"hash\": \"{hash}\", \"size\": {}}}",
                data.len()
            );
        }
        index += "}}";
        let indexes = store.join("indexes");
        fs::create_dir_all(&indexes).unwrap();
        fs::write(indexes.join(SOUNDS_INDEX), index).unwrap();
        (store, objects)
    }

    fn sound_manifest(dir: &Path) -> Value {
        let manifest = read_json(&dir.join(MANIFEST)).unwrap();
        manifest.get("sounds").unwrap().clone()
    }

    #[test]
    fn sounds_are_verified_and_copied_where_the_engine_reads_them() {
        let (jar, dir, _) = extracted("sounds-import");
        extract(&jar, &dir, &|_| {}).unwrap();
        let (store, objects) = sound_store(jar.parent().unwrap());
        let summary = import_sounds(&store, &dir, &|_| {}).unwrap();
        // Five sounds: the table and the four `.ogg` files; the stone texture is left alone.
        assert_eq!(summary.indexed, 5);
        assert_eq!(summary.found, 5);
        assert_eq!(summary.verified, 5);
        assert_eq!(summary.copied, 5);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(
            summary.bytes,
            objects[..5]
                .iter()
                .map(|(_, data)| data.len() as u64)
                .sum::<u64>()
        );
        for (name, data) in &objects[..5] {
            let relative = name.strip_prefix("minecraft/").unwrap();
            let path = dir.join("assets/minecraft").join(relative);
            assert_eq!(&fs::read(&path).unwrap(), data, "{}", path.display());
        }
        // The store's texture object is not a sound: the JAR's copy stays as extracted.
        assert_eq!(
            fs::read(dir.join("assets/minecraft/textures/blocks/stone.png")).unwrap(),
            b"stone"
        );

        let sounds = sound_manifest(&dir);
        assert_eq!(
            sounds.get("extracted"),
            Some(&Value::Bool(true)),
            "the manifest does not say the sounds were imported"
        );
        assert_eq!(
            sounds.get("index").and_then(Value::as_str),
            Some(SOUNDS_INDEX)
        );
        assert_eq!(
            sounds
                .get("counts")
                .and_then(|c| c.get("verified"))
                .and_then(Value::as_i64),
            Some(5)
        );
        let files = sounds.get("files").and_then(Value::as_array).unwrap();
        assert_eq!(files.len(), 5);
        let mut paths: Vec<&str> = files
            .iter()
            .map(|file| file.get("path").and_then(Value::as_str).unwrap())
            .collect();
        let sorted = paths.clone();
        paths.sort_unstable();
        assert_eq!(paths, sorted, "the manifest's sound files are not sorted");
        for file in files {
            let path = file.get("path").and_then(Value::as_str).unwrap();
            let data = &objects
                .iter()
                .find(|(name, _)| format!("assets/{name}") == path)
                .unwrap()
                .1;
            assert_eq!(
                file.get("sha1").and_then(Value::as_str),
                Some(crate::hex(&<sha1::Sha1 as sha1::Digest>::digest(data)).as_str())
            );
            assert_eq!(
                file.get("bytes").and_then(Value::as_i64),
                Some(data.len() as i64)
            );
        }
        assert!(sounds
            .get("skipped")
            .and_then(Value::as_array)
            .unwrap()
            .is_empty());
        // The extraction's own records are still there.
        let manifest = read_json(&dir.join(MANIFEST)).unwrap();
        assert_eq!(
            manifest
                .get("blocks")
                .and_then(Value::as_array)
                .unwrap()
                .len(),
            blocks().len()
        );
    }

    #[test]
    fn a_second_sound_import_copies_nothing_and_an_extraction_keeps_it() {
        let (jar, dir, _) = extracted("sounds-idempotent");
        extract(&jar, &dir, &|_| {}).unwrap();
        let (store, _) = sound_store(jar.parent().unwrap());
        import_sounds(&store, &dir, &|_| {}).unwrap();
        let manifest = fs::read(dir.join(MANIFEST)).unwrap();
        let second = import_sounds(&store, &dir, &|_| {}).unwrap();
        assert_eq!(second.copied, 0);
        assert_eq!(second.unchanged, second.indexed);
        assert_eq!(fs::read(dir.join(MANIFEST)).unwrap(), manifest);
        // Re-extracting the JAR rewrites the manifest but keeps the imported sounds' record.
        extract(&jar, &dir, &|_| {}).unwrap();
        let sounds = sound_manifest(&dir);
        assert_eq!(sounds.get("extracted"), Some(&Value::Bool(true)));
        assert_eq!(
            sounds
                .get("counts")
                .and_then(|c| c.get("verified"))
                .and_then(Value::as_i64),
            Some(5)
        );
        assert_eq!(
            fs::read(dir.join("assets/minecraft/sounds/dig/stone1.ogg")).unwrap(),
            b"OggS stone one"
        );
    }

    #[test]
    fn missing_and_mismatched_objects_are_skipped_and_say_why() {
        let (jar, dir, _) = extracted("sounds-skipped");
        extract(&jar, &dir, &|_| {}).unwrap();
        let (store, objects) = sound_store(jar.parent().unwrap());
        // One object is gone from the store, another no longer hashes to its index entry.
        let hash = |data: &[u8]| crate::hex(&<sha1::Sha1 as sha1::Digest>::digest(data));
        fs::remove_file(
            store
                .join("objects")
                .join(&hash(&objects[1].1)[..2])
                .join(hash(&objects[1].1)),
        )
        .unwrap();
        fs::write(
            store
                .join("objects")
                .join(&hash(&objects[2].1)[..2])
                .join(hash(&objects[2].1)),
            b"not the object",
        )
        .unwrap();
        let lines = std::cell::RefCell::new(Vec::new());
        let summary = import_sounds(&store, &dir, &|line| {
            lines.borrow_mut().push(line.to_string())
        })
        .unwrap();
        assert_eq!(summary.indexed, 5);
        assert_eq!(summary.verified, 3);
        assert_eq!(summary.copied, 3);
        assert_eq!(summary.skipped, 2);
        assert!(lines
            .borrow()
            .iter()
            .any(|line| line.contains("dig/stone1.ogg") && line.contains("missing object")));
        assert!(lines
            .borrow()
            .iter()
            .any(|line| line.contains("dig/stone2.ogg") && line.contains("sha1")));
        assert!(!dir.join("assets/minecraft/sounds/dig/stone1.ogg").exists());
        assert!(!dir.join("assets/minecraft/sounds/dig/stone2.ogg").exists());
        let sounds = sound_manifest(&dir);
        assert_eq!(
            sounds
                .get("counts")
                .and_then(|c| c.get("skipped"))
                .and_then(Value::as_i64),
            Some(2)
        );
        let skipped: Vec<(&str, &str)> = sounds
            .get("skipped")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry.get("path").and_then(Value::as_str).unwrap(),
                    entry.get("reason").and_then(Value::as_str).unwrap(),
                )
            })
            .collect();
        assert_eq!(skipped.len(), 2);
        assert!(skipped
            .iter()
            .any(|(path, reason)| *path == "minecraft/sounds/dig/stone1.ogg"
                && reason.contains("missing object")));
        assert!(skipped
            .iter()
            .any(|(path, reason)| *path == "minecraft/sounds/dig/stone2.ogg"
                && reason.contains("sha1")));
    }

    #[test]
    fn a_store_without_the_eras_index_is_refused() {
        let (jar, dir, _) = extracted("sounds-no-index");
        let empty = jar.parent().unwrap().join("empty");
        fs::create_dir_all(&empty).unwrap();
        let error = import_sounds(&empty, &dir, &|_| {}).unwrap_err();
        assert!(error.contains("no "), "{error}");
        assert!(error.contains(SOUNDS_INDEX), "{error}");
        let (store, _) = sound_store(jar.parent().unwrap());
        fs::write(
            store.join("indexes").join(SOUNDS_INDEX),
            "{\"objects\": {}}",
        )
        .unwrap();
        let error = import_sounds(&store, &dir, &|_| {}).unwrap_err();
        assert!(error.contains("no minecraft/sounds.json"), "{error}");
    }
}
