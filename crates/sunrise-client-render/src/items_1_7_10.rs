//! Minecraft 1.7.10's item icons in the GUI: `ItemRenderer.renderGuiItem`.
//!
//! The era draws a stack in two ways (`net/minecraft/client/render/entity/ItemRenderer.java:278`):
//!
//! - a **block item whose block `BlockRenderer.isItem3d` renders as a block**
//!   (`renderAsItem`'s cube branch) is drawn as an isometric cube of its six face
//!   sprites, under the era's own GUI transform,
//!   `translate(x - 2, y + 3, -3) scale(10) translate(1, 0.5, 1) scale(1, 1, -1)
//!   rotateX(210) rotateY(45) rotateY(-90)`, with `rotateY(90) translate(-0.5, -0.5, -0.5)`
//!   from `renderAsItem`'s cube branch and the face shades of the flat-lit pass
//!   (bottom 0.5, top 1.0, north/south 0.8, west/east 0.6);
//! - **every other item** is one flat 16x16 quad of its sprite at `(x, y)`
//!   (`ItemRenderer.drawTexture`), tinted by the item's display colour.
//!
//! A block item's sprite is `BlockItem.getSprite` — the block's top sprite, or the
//! class's own icon (`sunrise_prepare::assets_1_7_10::item_texture`), which is what
//! the era's flat path draws. Which blocks take the cube path is the block's shape:
//! a full cube drawn as a block, `Kind::Cube` and the cube-shaped specialists.
//! The shapes `renderAsItem` gives their own geometry (slabs, stairs, fences, snow,
//! cactus, the crossed plants) keep the flat icon for now.
//!
//! The sprites are the era's own files: [`crate::hud::HudAtlas`] packs
//! `textures/blocks` and `textures/items` under the era's names (`blocks/stone`,
//! `items/stick`), so an icon samples exactly the pixels the mesher's atlas holds.

use crate::hud::{HudAtlas, HudBatch, Rect, rgba};
use crate::mesher::sunset17::blocks::{BlockDef, Kind};
use crate::sunset17_assets::{self, sprite_name};
use std::sync::OnceLock;

/// The pixels one item fills in a GUI slot (`ItemRenderer.drawTexture`'s 16x16).
pub const ICON_SIZE: i32 = 16;

/// The era's flat top face of an item model: `ItemModelGenerator`'s 1/16 plane.
const FLAT_Z: f32 = 7.5 / 16.0;

/// The face shades of the flat-lit pass: bottom, top, north, south, west, east
/// (`BlockRenderer.tesselate*Face`).
const FACE_SHADE: [f32; 6] = [0.5, 1.0, 0.8, 0.8, 0.6, 0.6];

/// One face of the unit cube, in `BlockRenderer`'s order (bottom, top, north,
/// south, west, east): four corners in CCW outward winding (`(p1-p0) x (p2-p0)`
/// matches [`FACE_NORMAL`]).
const FACE_CORNERS: [[[f32; 3]; 4]; 6] = [
    // bottom (-Y)
    [
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
    ],
    // top (+Y)
    [
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
    ],
    // north (-Z)
    [
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ],
    // south (+Z)
    [
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
    ],
    // west (-X)
    [
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
    ],
    // east (+X)
    [
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
    ],
];

/// `Block.getRenderType`'s outward normal per face, in [`FACE_CORNERS`] order.
const FACE_NORMAL: [[f32; 3]; 6] = [
    [0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, 1.0],
    [-1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
];

/// `ColorizerGrass.getGrassColor(0.5, 1.0)`: `BlockGrass.getBlockColor()` in 1.7.10.
pub const GRASS_COLOR: u32 = 0x7C_BD_6B;
/// `ColorizerFoliage.getDefaultColor()`: `AbstractLeavesBlock.getColor(metadata)`,
/// the colour oak and jungle leaves draw in the icon (and both leaves of id 161).
pub const DEFAULT_LEAVES_COLOR: u32 = 0x48_B5_18;
/// `ColorizerFoliage.getSpruceColor()` (`LeavesBlock.getColor` metadata 1).
pub const SPRUCE_LEAVES_COLOR: u32 = 0x61_99_61;
/// `ColorizerFoliage.getBirchColor()` (`LeavesBlock.getColor` metadata 2).
pub const BIRCH_LEAVES_COLOR: u32 = 0x80_A7_55;
/// `BlockLilyPad.getBlockColor()` in 1.7.10.
pub const WATERLILY_COLOR: u32 = 0x20_80_30;
/// `ItemArmor` default brown leather colour in 1.7.10.
pub const LEATHER_DEFAULT_COLOR: u32 = 0xA0_65_40;

/// Upright sprite UV `[u, v]` (0..1, v=0 top of texture) for a point `p` inside `[0..1]^3`
/// on `face` (matching `RenderBlocks.renderFace*` cropping for sub-cube shapes).
fn face_uv(face: usize, p: [f32; 3]) -> [f32; 2] {
    match face {
        0 | 1 => [p[0], p[2]],
        2 => [1.0 - p[0], 1.0 - p[1]],
        3 => [p[0], 1.0 - p[1]],
        4 => [p[2], 1.0 - p[1]],
        _ => [1.0 - p[2], 1.0 - p[1]],
    }
}

/// The item id the era registers under `name` (`minecraft:stick`), if any.
pub fn item_id(name: &str) -> Option<u16> {
    ITEMS
        .iter()
        .find_map(|&(id, item)| (item == name).then_some(id))
}

/// The item name of one era id (`minecraft:stick`), if any.
pub fn item_name(id: u16) -> Option<&'static str> {
    ITEMS
        .iter()
        .find_map(|&(item_id, name)| (item_id == id).then_some(name))
}

/// Whether `id` renders as a 3D block shape (cube, slab, stairs, fence, ...) in 1.7.10.
pub fn is_3d_block_item(id: u16) -> bool {
    block_of(id).is_some_and(|block| !item_boxes(id, block).is_empty())
}

/// Whether `id` is held like a handheld tool/weapon/rod (`Item.isHandheld()` / `bFull3D`) in 1.7.10.
pub fn is_handheld(id: u16) -> bool {
    matches!(
        id,
        256..=258
            | 267..=279
            | 280
            | 283..=286
            | 290..=294
            | 346
            | 352
            | 398
    )
}

/// Whether `id` rotates 180° in hand (`Item.shouldRotate()`: fishing rod, carrot on a stick).
pub fn should_rotate(id: u16) -> bool {
    matches!(id, 346 | 398)
}

/// Whether `id` is the 1.7.10 bow item (`Items.BOW`, id 261).
pub fn is_bow(id: u16) -> bool {
    id == 261
}

/// First-person view-model transform from 1.7.10 `ItemInHandRenderer.renderHand` + `render`,
/// operating directly on centered `[-0.5..0.5]^3` blocks and `[-0.5..0.5, -0.5..0.5, -0.03125..0.03125]`
/// extruded item meshes.
pub fn first_person_item_matrix(
    id: u16,
    left: bool,
    inverse_height: f32,
    attack: f32,
    block: bool,
    bow_ticks: Option<f32>,
    eat: Option<(f32, f32)>,
) -> glam::Mat4 {
    use glam::{Mat4, Vec3};
    let deg = |d: f32| d.to_radians();
    let pi = std::f32::consts::PI;

    let mut m = Mat4::IDENTITY;
    if let Some((remaining, duration)) = eat {
        let f = 1.0 - (remaining / duration.max(1.0)).clamp(0.0, 1.0);
        let inv = 1.0 - f;
        let inv9 = inv * inv * inv;
        let inv9 = inv9 * inv9 * inv9;
        let pull = 1.0 - inv9;
        let bob = if f <= 0.2 {
            0.0
        } else {
            ((remaining / 4.0 * pi).cos() * 0.1).abs()
        };
        m *= Mat4::from_translation(Vec3::new(0.0, bob, 0.0));
        m *= Mat4::from_translation(Vec3::new(pull * 0.6, -pull * 0.5, 0.0));
        m *= Mat4::from_rotation_y(deg(pull * 90.0));
        m *= Mat4::from_rotation_x(deg(pull * 10.0));
        m *= Mat4::from_rotation_z(deg(pull * 30.0));
    } else {
        let f = (attack * pi).sin();
        let f1 = (attack.sqrt() * pi).sin();
        let f2 = (attack.sqrt() * pi * 2.0).sin();
        m *= Mat4::from_translation(Vec3::new(-0.4 * f1, 0.2 * f2, -0.2 * f));
    }

    // renderHand(partial): T(0.56, -0.52 - (1 - handHeight) * 0.6, -0.72), Ry(45), swing, S(0.4).
    let swing = if eat.is_some() { 0.0 } else { attack };
    let sf = (swing * swing * pi).sin();
    let sf1 = (swing.sqrt() * pi).sin();
    m *= Mat4::from_translation(Vec3::new(0.56, -0.52 - inverse_height * 0.6, -0.72));
    m *= Mat4::from_rotation_y(deg(45.0));
    m *= Mat4::from_rotation_y(deg(-20.0 * sf));
    m *= Mat4::from_rotation_z(deg(-20.0 * sf1));
    m *= Mat4::from_rotation_x(deg(-80.0 * sf1));
    m *= Mat4::from_scale(Vec3::splat(0.4));

    if block {
        m *= Mat4::from_translation(Vec3::new(-0.5, 0.2, 0.0));
        m *= Mat4::from_rotation_y(deg(30.0));
        m *= Mat4::from_rotation_x(deg(-80.0));
        m *= Mat4::from_rotation_y(deg(60.0));
    } else if let Some(ticks_used) = bow_ticks {
        m *= Mat4::from_rotation_z(deg(-18.0));
        m *= Mat4::from_rotation_y(deg(-12.0));
        m *= Mat4::from_rotation_x(deg(-8.0));
        m *= Mat4::from_translation(Vec3::new(-0.9, 0.2, 0.0));
        let f = (ticks_used / 20.0).min(1.0);
        let power = ((f * f + f * 2.0) / 3.0).min(1.0);
        if power > 0.1 {
            m *= Mat4::from_translation(Vec3::new(
                0.0,
                ((ticks_used - 0.1) * 1.3).sin() * 0.01 * (power - 0.1),
                0.0,
            ));
        }
        m *= Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.1));
        m *= Mat4::from_rotation_z(deg(-335.0));
        m *= Mat4::from_rotation_y(deg(-50.0));
        m *= Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0));
        m *= Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2));
        m *= Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
        m *= Mat4::from_rotation_y(deg(50.0));
        m *= Mat4::from_rotation_z(deg(335.0));
    }

    if should_rotate(id) {
        m *= Mat4::from_rotation_y(deg(180.0));
    }

    if !is_3d_block_item(id) {
        // ItemInHandRenderer.render: T(0, -0.3, 0) * S(1.5) * Ry(50) * Rz(335) * T(-0.9375, -0.0625, 0)
        // followed by T(0.5, 0.5, -0.03125) to map centered [-0.5..0.5] to [0..1, 0..1, -0.0625..0].
        m *= Mat4::from_translation(Vec3::new(0.0, -0.3, 0.0));
        m *= Mat4::from_scale(Vec3::splat(1.5));
        m *= Mat4::from_rotation_y(deg(50.0));
        m *= Mat4::from_rotation_z(deg(335.0));
        m *= Mat4::from_translation(Vec3::new(-0.9375, -0.0625, 0.0));
        m *= Mat4::from_translation(Vec3::new(0.5, 0.5, -0.03125));
    }

    if left {
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        mirror * m * mirror
    } else {
        m
    }
}

/// Third-person held item transform relative to the arm bone (`PlayerRenderer.renderMore` lines
/// 222-271 + `ItemInHandRenderer.render` lines 84-90).
pub fn third_person_item_matrix(id: u16, left: bool, blocking: bool) -> glam::Mat4 {
    use glam::{Mat4, Vec3};
    let deg = |d: f32| d.to_radians();
    let mut m = Mat4::from_translation(Vec3::new(-0.0625, 0.4375, 0.0625));

    if is_3d_block_item(id) {
        m *= Mat4::from_translation(Vec3::new(0.0, 0.1875, -0.3125));
        m *= Mat4::from_rotation_x(deg(20.0));
        m *= Mat4::from_rotation_y(deg(45.0));
        m *= Mat4::from_scale(Vec3::new(-0.375, -0.375, 0.375));
    } else {
        if is_bow(id) {
            m *= Mat4::from_translation(Vec3::new(0.0, 0.125, 0.3125));
            m *= Mat4::from_rotation_y(deg(-20.0));
            m *= Mat4::from_scale(Vec3::new(0.625, -0.625, 0.625));
            m *= Mat4::from_rotation_x(deg(-100.0));
            m *= Mat4::from_rotation_y(deg(45.0));
        } else if is_handheld(id) {
            if should_rotate(id) {
                m *= Mat4::from_rotation_z(deg(180.0));
                m *= Mat4::from_translation(Vec3::new(0.0, -0.125, 0.0));
            }
            if blocking {
                m *= Mat4::from_translation(Vec3::new(0.05, 0.0, -0.1));
                m *= Mat4::from_rotation_y(deg(-50.0));
                m *= Mat4::from_rotation_x(deg(-10.0));
                m *= Mat4::from_rotation_z(deg(-60.0));
            }
            m *= Mat4::from_translation(Vec3::new(0.0, 0.1875, 0.0));
            m *= Mat4::from_scale(Vec3::new(0.625, -0.625, 0.625));
            m *= Mat4::from_rotation_x(deg(-100.0));
            m *= Mat4::from_rotation_y(deg(45.0));
        } else {
            m *= Mat4::from_translation(Vec3::new(0.25, 0.1875, -0.1875));
            m *= Mat4::from_scale(Vec3::splat(0.375));
            m *= Mat4::from_rotation_z(deg(60.0));
            m *= Mat4::from_rotation_x(deg(-90.0));
            m *= Mat4::from_rotation_z(deg(20.0));
        }

        // ItemInHandRenderer.render on flat items:
        m *= Mat4::from_translation(Vec3::new(0.0, -0.3, 0.0));
        m *= Mat4::from_scale(Vec3::splat(1.5));
        m *= Mat4::from_rotation_y(deg(50.0));
        m *= Mat4::from_rotation_z(deg(335.0));
        m *= Mat4::from_translation(Vec3::new(-0.9375, -0.0625, 0.0));
        m *= Mat4::from_translation(Vec3::new(0.5, 0.5, -0.03125));
    }

    if left {
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        mirror * m * mirror
    } else {
        m
    }
}

/// Draw `id`'s icon for `meta` at `(x, y)` in GUI pixels: `ItemRenderer.renderGuiItem`.
/// `tick` picks the frame of an animated sprite (`items/compass`, `items/clock`).
pub fn paint(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    id: u16,
    meta: u16,
    x: f32,
    y: f32,
    tick: u64,
) {
    if let Some(block) = block_of(id) {
        let boxes = item_boxes(id, block);
        if !boxes.is_empty() {
            paint_block(batch, atlas, block, &boxes, id, meta, x, y);
            return;
        }
    }
    paint_flat_item(batch, atlas, id, meta, x, y, tick);
}

/// The era block of a block item id: `Block.getBlockFromItem`, `None` for an item
/// that places no block. The table is the era's own `BlockDef` of each id, built
/// once from the registry names (`BlockRenderer`'s facts).
fn block_of(id: u16) -> Option<&'static BlockDef> {
    /// The highest block id 1.7.10 registers.
    const MAX_ID: u16 = 175;
    static BLOCKS: OnceLock<Vec<Option<BlockDef>>> = OnceLock::new();
    let blocks = BLOCKS.get_or_init(|| {
        // The era's block registry names (`stone_slab`), which the renderer's
        // block facts are keyed by; `sprites::BLOCKS` holds `setKey`'s
        // translation keys (`stoneSlab`) instead.
        (0..=MAX_ID)
            .map(|id| {
                let block = sunrise_prepare::assets_1_7_10::blocks()
                    .iter()
                    .find(|block| block.id == id)?;
                Some(BlockDef::new(block.name))
            })
            .collect()
    });
    blocks.get(usize::from(id)).and_then(|block| block.as_ref())
}

/// The 3D boxes `[min_x, min_y, min_z, max_x, max_y, max_z]` that `RenderBlocks.renderBlockAsItem`
/// draws for `id`, or empty when the item renders as a flat 2D sprite.
fn item_boxes(id: u16, block: &BlockDef) -> smallvec::SmallVec<[[f32; 6]; 4]> {
    use smallvec::smallvec;
    match id {
        // Single slabs.
        44 | 126 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]],
        // Snow layer (2/16 high in inventory).
        78 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.125, 1.0]],
        // Carpet (1/16 high).
        171 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.0625, 1.0]],
        // Cactus (1/16 inset horizontally).
        81 => smallvec![[0.0625, 0.0, 0.0625, 0.9375, 1.0, 0.9375]],
        // Daylight detector (6/16 high).
        151 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.375, 1.0]],
        // Enchanting table (12/16 high).
        116 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.75, 1.0]],
        // End portal frame (13/16 high).
        120 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.8125, 1.0]],
        // Farmland (15/16 high).
        60 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.9375, 1.0]],
        // Trapdoor (3/16 high).
        96 => smallvec![[0.0, 0.0, 0.0, 1.0, 0.1875, 1.0]],
        // Pressure plates.
        70 | 72 | 147 | 148 => smallvec![[0.0625, 0.0, 0.0625, 0.9375, 0.0625, 0.9375]],
        // Buttons.
        77 | 143 => smallvec![[0.3125, 0.375, 0.375, 0.6875, 0.625, 0.625]],
        // Fences (two posts and two rails).
        85 | 113 => smallvec![
            [0.375, 0.0, 0.0, 0.625, 1.0, 0.25],
            [0.375, 0.0, 0.75, 0.625, 1.0, 1.0],
            [0.4375, 0.375, 0.0, 0.5625, 0.5625, 1.0],
            [0.4375, 0.75, 0.0, 0.5625, 0.9375, 1.0],
        ],
        // Fence gate.
        107 => smallvec![
            [0.0, 0.3125, 0.4375, 0.125, 1.0, 0.5625],
            [0.875, 0.3125, 0.4375, 1.0, 1.0, 0.5625],
            [0.125, 0.375, 0.4375, 0.875, 0.9375, 0.5625],
        ],
        // Cobblestone wall.
        139 => smallvec![
            [0.25, 0.0, 0.25, 0.75, 1.0, 0.75],
            [0.3125, 0.0, 0.0, 0.6875, 0.8125, 1.0],
        ],
        // Anvil (base, stem, top).
        145 => smallvec![
            [0.125, 0.0, 0.125, 0.875, 0.25, 0.875],
            [0.25, 0.25, 0.3125, 0.75, 0.625, 0.6875],
            [0.0, 0.625, 0.1875, 1.0, 1.0, 0.8125],
        ],
        // Chests & pistons.
        54 | 130 | 146 => smallvec![[0.0625, 0.0, 0.0625, 0.9375, 0.875, 0.9375]],
        29 | 33 => smallvec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
        _ if block.kind == Kind::Stairs => smallvec![
            [0.0, 0.0, 0.0, 1.0, 0.5, 1.0],
            [0.0, 0.5, 0.0, 0.5, 1.0, 1.0],
        ],
        _ if drawn_as_block(block) => smallvec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
        _ => smallvec![],
    }
}

/// Whether `renderAsItem` draws the block as a full cube.
fn drawn_as_block(block: &BlockDef) -> bool {
    matches!(
        block.kind,
        Kind::Cube
            | Kind::Grass
            | Kind::Log
            | Kind::Quartz
            | Kind::Glass
            | Kind::Ice
            | Kind::Leaves
            | Kind::Slab { double: true }
    ) && block.bounds.is_full_cube()
}

/// Flat-item sprite layers `(sprite_name, rgb_tint)` for a 2D item in 1.7.10.
fn flat_layers(id: u16, meta: u16) -> smallvec::SmallVec<[(String, u32); 2]> {
    use smallvec::smallvec;
    // Potions: tinted liquid overlay under the bottle outline.
    if id == 373 {
        let bottle = if meta & 0x4000 != 0 {
            "minecraft:items/potion_bottle_splash"
        } else {
            "minecraft:items/potion_bottle_drinkable"
        };
        return smallvec![
            (
                "minecraft:items/potion_overlay".to_owned(),
                potion_color_17(meta)
            ),
            (bottle.to_owned(), 0xFF_FF_FF),
        ];
    }
    // Spawn eggs: primary colour on base, secondary colour on spots overlay.
    if id == 383 {
        let (primary, secondary) = spawn_egg_colors_17(meta);
        return smallvec![
            ("minecraft:items/spawn_egg".to_owned(), primary),
            ("minecraft:items/spawn_egg_overlay".to_owned(), secondary),
        ];
    }
    // Leather armour: brown base + untinted overlay.
    if let Some((base, overlay)) = match id {
        298 => Some((
            "minecraft:items/leather_helmet",
            "minecraft:items/leather_helmet_overlay",
        )),
        299 => Some((
            "minecraft:items/leather_chestplate",
            "minecraft:items/leather_chestplate_overlay",
        )),
        300 => Some((
            "minecraft:items/leather_leggings",
            "minecraft:items/leather_leggings_overlay",
        )),
        301 => Some((
            "minecraft:items/leather_boots",
            "minecraft:items/leather_boots_overlay",
        )),
        _ => None,
    } {
        return smallvec![
            (base.to_owned(), LEATHER_DEFAULT_COLOR),
            (overlay.to_owned(), 0xFF_FF_FF),
        ];
    }
    let Some(texture) = sunset17_assets::item_texture(i32::from(id), i32::from(meta)) else {
        return smallvec![];
    };
    let tint = match id {
        31 if matches!(meta, 1 | 2) => GRASS_COLOR,
        175 if matches!(meta & 7, 2 | 3) => GRASS_COLOR,
        106 => DEFAULT_LEAVES_COLOR,
        111 => WATERLILY_COLOR,
        _ => 0xFF_FF_FF,
    };
    smallvec![(sprite_name(&texture), tint)]
}

/// 1.7.10 `PotionHelper.calcPotionLiquidColor` for the standard creative/brewing damage bits.
fn potion_color_17(meta: u16) -> u32 {
    if meta == 0 {
        return 0x38_5D_C6;
    }
    match meta & 0x0F {
        1 => 0xCD_5C_AB,  // Regeneration
        2 => 0x7C_AF_C6,  // Swiftness
        3 => 0xE4_9A_3A,  // Fire Resistance
        4 => 0x4E_93_31,  // Poison
        5 => 0xF8_24_23,  // Healing
        6 => 0x1F_1F_A1,  // Night Vision
        8 => 0x48_4D_48,  // Weakness
        9 => 0x93_24_23,  // Strength
        10 => 0x5A_6C_81, // Slowness
        12 => 0x43_0A_09, // Harming
        13 => 0x2E_52_99, // Water Breathing
        14 => 0x7F_83_92, // Invisibility
        _ => 0x38_5D_C6,
    }
}

/// 1.7.10 `EntityList` spawn egg `(primary, secondary)` colours by entity id.
fn spawn_egg_colors_17(entity_id: u16) -> (u32, u32) {
    match entity_id {
        50 => (0x0D_A7_0B, 0x00_00_00),  // Creeper
        51 => (0xC1_C1_C1, 0x49_49_49),  // Skeleton
        52 => (0x34_2D_27, 0xA8_0E_0E),  // Spider
        54 => (0x00_AF_AF, 0x79_9C_65),  // Zombie
        55 => (0x51_A0_3E, 0x7E_BF_6E),  // Slime
        56 => (0xF9_F9_F9, 0xBC_BC_BC),  // Ghast
        57 => (0xEA_93_93, 0x4C_71_29),  // Zombie Pigman
        58 => (0x16_1616, 0x00_00_00),   // Enderman
        59 => (0x0C_42_4E, 0xA8_0E_0E),  // Cave Spider
        60 => (0x6E_6E_6E, 0x30_30_30),  // Silverfish
        61 => (0xF6_B2_01, 0xFF_F8_7E),  // Blaze
        62 => (0x34_00_00, 0xFC_FC_00),  // Magma Cube
        65 => (0x4C_3E_30, 0x0F_0F_0F),  // Bat
        66 => (0x34_00_00, 0x51_A0_3E),  // Witch
        90 => (0xF0_A5_A2, 0xDB_63_5F),  // Pig
        91 => (0xE7_E7_E7, 0xFF_B5_B5),  // Sheep
        92 => (0x44_36_26, 0xA1_A1_A1),  // Cow
        93 => (0xA1_A1_A1, 0xFF_00_00),  // Chicken
        94 => (0x22_3B_4D, 0x70_88_99),  // Squid
        95 => (0xD7_D3_D3, 0xCE_AF_96),  // Wolf
        96 => (0xA0_0F_10, 0xB7_B7_B7),  // Mooshroom
        98 => (0xEF_DE_7D, 0x56_44_34),  // Ocelot
        100 => (0xC0_9E_7D, 0xEE_E5_00), // Horse
        120 => (0x56_3C_33, 0xBD_8B_72), // Villager
        _ => (0xFF_FF_FF, 0xA0_A0_A0),
    }
}

/// The flat path: one or two tinted 16x16 quads at `(x, y)` (`ItemRenderer.drawTexture`).
fn paint_flat_item(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    id: u16,
    meta: u16,
    x: f32,
    y: f32,
    tick: u64,
) {
    for (name, tint) in flat_layers(id, meta) {
        let Some(rect) = atlas.gui_rect(&name).copied() else {
            continue;
        };
        let rect = frame_rect(rect, tick);
        let color = rgba(
            (tint >> 16 & 0xFF) as u8,
            (tint >> 8 & 0xFF) as u8,
            (tint & 0xFF) as u8,
            0xFF,
        );
        batch.rect_uv(x, y, ICON_SIZE as f32, ICON_SIZE as f32, rect, color);
    }
}

/// `TextureAtlasSprite`'s animation: the file is a vertical strip of
/// `height / width` frames, one per tick (the era's `{"animation": {}}` sprites
/// carry the default `frametime` of 1), and the icon shows the frame `tick`
/// falls in.
fn frame_rect(rect: Rect, tick: u64) -> Rect {
    let frames = (rect.h / rect.w.max(1)).max(1);
    if frames == 1 {
        return rect;
    }
    let frame = (tick % u64::from(frames)) as f32;
    let dv = (rect.v1 - rect.v0) / frames as f32;
    Rect {
        v0: rect.v0 + dv * frame,
        v1: rect.v0 + dv * (frame + 1.0),
        ..rect
    }
}

/// The block path: the visible faces of the era's GUI block boxes, shaded,
/// cropped to each box's bounds and sorted back-to-front.
#[allow(clippy::too_many_arguments)]
fn paint_block(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    block: &BlockDef,
    boxes: &[[f32; 6]],
    id: u16,
    meta: u16,
    x: f32,
    y: f32,
) {
    let block_meta = block_metadata(id, meta);
    let matrix = gui_block_matrix(x, y);
    let mut draws: Vec<(f32, [[f32; 2]; 4], [[f32; 2]; 4], usize)> =
        Vec::with_capacity(boxes.len() * 3);
    for &[x0, y0, z0, x1, y1, z1] in boxes {
        for (face, unit_corners) in FACE_CORNERS.into_iter().enumerate() {
            let normal = transform_normal(&matrix, FACE_NORMAL[face]);
            if normal[2] <= 0.0 {
                continue;
            }
            let mut depth = 0.0;
            let mut positions = [[0.0; 2]; 4];
            let mut uvs = [[0.0; 2]; 4];
            for (i, corner) in unit_corners.into_iter().enumerate() {
                let p = [
                    x0 + (x1 - x0) * corner[0],
                    y0 + (y1 - y0) * corner[1],
                    z0 + (z1 - z0) * corner[2],
                ];
                let q = transform_point(&matrix, p);
                depth += q[2];
                positions[i] = [q[0], q[1]];
                uvs[i] = face_uv(face, p);
            }
            draws.push((depth / 4.0, positions, uvs, face));
        }
    }
    draws.sort_by(|a, b| a.0.total_cmp(&b.0));
    let shade_color = |tint: u32, shade: f32| {
        rgba(
            (f32::from((tint >> 16 & 0xFF) as u8) * shade) as u8,
            (f32::from((tint >> 8 & 0xFF) as u8) * shade) as u8,
            (f32::from((tint & 0xFF) as u8) * shade) as u8,
            0xFF,
        )
    };
    let map_uvs = |rect: Rect, frac: [[f32; 2]; 4]| {
        frac.map(|uv| {
            [
                rect.u0 + (rect.u1 - rect.u0) * uv[0],
                rect.v0 + (rect.v1 - rect.v0) * uv[1],
            ]
        })
    };
    for (_, positions, frac_uvs, face) in draws {
        let Some(texture) = sunset17_assets::block_texture(id, block_meta, face as u8) else {
            continue;
        };
        let Some(rect) = atlas.gui_rect(&sprite_name(&texture)).copied() else {
            continue;
        };
        let shade = FACE_SHADE[face];
        let tint = face_tint(block, id, block_meta, face);
        batch.quad(positions, map_uvs(rect, frac_uvs), shade_color(tint, shade));
        // 1.7.10 fancy grass block: tint `grass_side_overlay` over the untinted side.
        if block.kind == Kind::Grass
            && (2..=5).contains(&face)
            && let Some(overlay) = atlas
                .gui_rect("minecraft:blocks/grass_side_overlay")
                .copied()
        {
            batch.quad(
                positions,
                map_uvs(overlay, frac_uvs),
                shade_color(GRASS_COLOR, shade),
            );
        }
    }
}

/// Emit 3D world-space quad vertices (4 per quad) for a 1.7.10 item (first-person hand,
/// dropped item entity, or third-person mob/player held item) under `pose`.
#[allow(clippy::too_many_arguments)]
pub fn emit_world_item(
    atlas: &HudAtlas,
    id: u16,
    meta: u16,
    pose: glam::Mat4,
    light: u32,
    lit: bool,
    tick: u64,
    out: &mut Vec<crate::mesher::Vertex>,
) -> bool {
    if let Some(block) = block_of(id) {
        let boxes = item_boxes(id, block);
        if !boxes.is_empty() {
            return emit_world_block(atlas, block, &boxes, id, meta, pose, light, lit, out);
        }
    }
    emit_world_flat(atlas, id, meta, pose, light, lit, tick, out)
}

#[allow(clippy::too_many_arguments)]
fn emit_world_block(
    atlas: &HudAtlas,
    block: &BlockDef,
    boxes: &[[f32; 6]],
    id: u16,
    meta: u16,
    pose: glam::Mat4,
    light: u32,
    lit: bool,
    out: &mut Vec<crate::mesher::Vertex>,
) -> bool {
    let block_meta = block_metadata(id, meta);
    let before = out.len();
    let normal_w = if lit { -127 } else { 0 };
    let mut push_face = |pts: [[f32; 3]; 4],
                         frac_uvs: [[f32; 2]; 4],
                         rect: Rect,
                         n: glam::Vec3,
                         tint: u32,
                         shade: f32| {
        let s = if lit { 1.0 } else { shade };
        let color = rgba(
            (f32::from((tint >> 16 & 0xFF) as u8) * s) as u8,
            (f32::from((tint >> 8 & 0xFF) as u8) * s) as u8,
            (f32::from((tint & 0xFF) as u8) * s) as u8,
            0xFF,
        );
        let packed_n = [
            (n.x * 127.0).round() as i8,
            (n.y * 127.0).round() as i8,
            (n.z * 127.0).round() as i8,
            normal_w,
        ];
        for (p, uv) in pts.into_iter().zip(frac_uvs) {
            out.push(crate::mesher::Vertex {
                pos: p,
                uv: [
                    rect.u0 + (rect.u1 - rect.u0) * uv[0],
                    rect.v0 + (rect.v1 - rect.v0) * uv[1],
                ],
                layer: 0,
                color,
                light,
                normal: packed_n,
                glint_uv: [0.0, 0.0],
            });
        }
    };
    for &[x0, y0, z0, x1, y1, z1] in boxes {
        for (face, unit_corners) in FACE_CORNERS.into_iter().enumerate() {
            let Some(texture) = sunset17_assets::block_texture(id, block_meta, face as u8) else {
                continue;
            };
            let Some(rect) = atlas.gui_rect(&sprite_name(&texture)).copied() else {
                continue;
            };
            let n = pose
                .transform_vector3(glam::Vec3::from(FACE_NORMAL[face]))
                .normalize_or_zero();
            let mut pts = [[0.0; 3]; 4];
            let mut frac_uvs = [[0.0; 2]; 4];
            for (i, corner) in unit_corners.into_iter().enumerate() {
                let local = [
                    x0 + (x1 - x0) * corner[0],
                    y0 + (y1 - y0) * corner[1],
                    z0 + (z1 - z0) * corner[2],
                ];
                let centered = glam::Vec3::new(local[0] - 0.5, local[1] - 0.5, local[2] - 0.5);
                pts[i] = pose.transform_point3(centered).to_array();
                frac_uvs[i] = face_uv(face, local);
            }
            let tint = face_tint(block, id, block_meta, face);
            push_face(pts, frac_uvs, rect, n, tint, FACE_SHADE[face]);
            if block.kind == Kind::Grass
                && (2..=5).contains(&face)
                && let Some(overlay) = atlas
                    .gui_rect("minecraft:blocks/grass_side_overlay")
                    .copied()
            {
                let lifted = pts.map(|p| [p[0] + n.x * 1e-3, p[1] + n.y * 1e-3, p[2] + n.z * 1e-3]);
                push_face(lifted, frac_uvs, overlay, n, GRASS_COLOR, FACE_SHADE[face]);
            }
        }
    }
    out.len() > before
}

#[allow(clippy::too_many_arguments)]
fn emit_world_flat(
    atlas: &HudAtlas,
    id: u16,
    meta: u16,
    pose: glam::Mat4,
    light: u32,
    lit: bool,
    tick: u64,
    out: &mut Vec<crate::mesher::Vertex>,
) -> bool {
    let before = out.len();
    let normal_w = if lit { -127 } else { 0 };
    let pack = |local_n: glam::Vec3| {
        let n = pose.transform_vector3(local_n).normalize_or_zero();
        [
            (n.x * 127.0).round() as i8,
            (n.y * 127.0).round() as i8,
            (n.z * 127.0).round() as i8,
            normal_w,
        ]
    };
    let n_front = pack(glam::Vec3::Z);
    let n_back = pack(glam::Vec3::NEG_Z);
    let n_left = pack(glam::Vec3::NEG_X);
    let n_right = pack(glam::Vec3::X);
    let n_top = pack(glam::Vec3::Y);
    let n_bottom = pack(glam::Vec3::NEG_Y);

    let mut push_quad = |corners: [([f32; 3], [f32; 2]); 4], color: u32, normal: [i8; 4]| {
        for (p, uv) in corners {
            out.push(crate::mesher::Vertex {
                pos: pose.transform_point3(glam::Vec3::from(p)).to_array(),
                uv,
                layer: 0,
                color,
                light,
                normal,
                glint_uv: [0.0, 0.0],
            });
        }
    };

    for (layer_idx, (name, tint)) in flat_layers(id, meta).into_iter().enumerate() {
        let Some(rect) = atlas.gui_rect(&name).copied() else {
            continue;
        };
        let rect = frame_rect(rect, tick);
        let color = rgba(
            (tint >> 16 & 0xFF) as u8,
            (tint >> 8 & 0xFF) as u8,
            (tint & 0xFF) as u8,
            0xFF,
        );
        let z_pad = layer_idx as f32 * 0.001;
        let z1 = 0.03125 + z_pad;
        let z0 = -0.03125 - z_pad;
        // Match 1.7.10 `ItemInHandRenderer.draw`: centered in [-0.5..0.5]^2, thickness 1/16,
        // with pixel-slice edge quads whose transparent texels are alpha-discarded in entity.wgsl.
        push_quad(
            [
                ([-0.5, -0.5, z1], [rect.u1, rect.v1]),
                ([0.5, -0.5, z1], [rect.u0, rect.v1]),
                ([0.5, 0.5, z1], [rect.u0, rect.v0]),
                ([-0.5, 0.5, z1], [rect.u1, rect.v0]),
            ],
            color,
            n_front,
        );
        push_quad(
            [
                ([-0.5, 0.5, z0], [rect.u1, rect.v0]),
                ([0.5, 0.5, z0], [rect.u0, rect.v0]),
                ([0.5, -0.5, z0], [rect.u0, rect.v1]),
                ([-0.5, -0.5, z0], [rect.u1, rect.v1]),
            ],
            color,
            n_back,
        );
        let w = rect.w.clamp(1, 64);
        let h = rect.h.clamp(1, 64);
        let du = rect.u1 - rect.u0;
        let dv = rect.v1 - rect.v0;
        for k in 0..w {
            let u = rect.u1 - du * ((k as f32 + 0.5) / w as f32);
            let x_l = k as f32 / w as f32 - 0.5;
            push_quad(
                [
                    ([x_l, -0.5, z0], [u, rect.v1]),
                    ([x_l, -0.5, z1], [u, rect.v1]),
                    ([x_l, 0.5, z1], [u, rect.v0]),
                    ([x_l, 0.5, z0], [u, rect.v0]),
                ],
                color,
                n_left,
            );
            let x_r = (k + 1) as f32 / w as f32 - 0.5;
            push_quad(
                [
                    ([x_r, 0.5, z0], [u, rect.v0]),
                    ([x_r, 0.5, z1], [u, rect.v0]),
                    ([x_r, -0.5, z1], [u, rect.v1]),
                    ([x_r, -0.5, z0], [u, rect.v1]),
                ],
                color,
                n_right,
            );
        }
        for k in 0..h {
            let v = rect.v1 - dv * ((k as f32 + 0.5) / h as f32);
            let y_t = (k + 1) as f32 / h as f32 - 0.5;
            push_quad(
                [
                    ([-0.5, y_t, z1], [rect.u1, v]),
                    ([0.5, y_t, z1], [rect.u0, v]),
                    ([0.5, y_t, z0], [rect.u0, v]),
                    ([-0.5, y_t, z0], [rect.u1, v]),
                ],
                color,
                n_top,
            );
            let y_b = k as f32 / h as f32 - 0.5;
            push_quad(
                [
                    ([0.5, y_b, z1], [rect.u0, v]),
                    ([-0.5, y_b, z1], [rect.u1, v]),
                    ([-0.5, y_b, z0], [rect.u1, v]),
                    ([0.5, y_b, z0], [rect.u0, v]),
                ],
                color,
                n_bottom,
            );
        }
    }
    out.len() > before
}

/// Per-face tint for a 3D block item: grass tints its top face (`grass_top`),
/// leaves tint all faces with the era's foliage colours, everything else stays white.
fn face_tint(block: &BlockDef, id: u16, meta: u8, face: usize) -> u32 {
    match block.kind {
        Kind::Grass if face == 1 => GRASS_COLOR,
        Kind::Leaves => cube_tint(block, id, meta),
        _ => 0xFF_FF_FF,
    }
}

/// `Block.getColor(metadata)`, the block's default tint colour.
fn cube_tint(block: &BlockDef, id: u16, meta: u8) -> u32 {
    match block.kind {
        Kind::Grass => GRASS_COLOR,
        Kind::Leaves if id == 18 => match meta & 3 {
            1 => SPRUCE_LEAVES_COLOR,
            2 => BIRCH_LEAVES_COLOR,
            _ => DEFAULT_LEAVES_COLOR,
        },
        Kind::Leaves => DEFAULT_LEAVES_COLOR,
        _ => 0xFF_FF_FF,
    }
}

/// `renderAsItem`'s block metadata: the dispenser, dropper and furnace draw
/// their metadata-3 (south-facing) orientation; `ItemCloth` passes item damage
/// straight through (`0 = white .. 15 = black`).
fn block_metadata(id: u16, meta: u16) -> u8 {
    match id {
        23 | 61 | 62 | 158 => 3,
        _ => meta as u8 & 15,
    }
}

/// The GUI item cube's transform, `ItemRenderer.renderGuiItem`'s block branch as
/// 4x4 matrices: `translate(x - 2, y + 3, -3) scale(10) translate(1, 0.5, 1)
/// scale(1, 1, -1) rotateX(210) rotateY(45) rotateY(-90)` and `renderAsItem`'s
/// `rotateY(90) translate(-0.5, -0.5, -0.5)`; the three Y rotations combine to
/// 45 degrees.
fn gui_block_matrix(x: f32, y: f32) -> [[f32; 4]; 4] {
    let mut m = translate(x - 2.0, y + 3.0, -3.0);
    m = multiply(m, scale(10.0, 10.0, 10.0));
    m = multiply(m, translate(1.0, 0.5, 1.0));
    m = multiply(m, scale(1.0, 1.0, -1.0));
    m = multiply(m, rotate_x(210.0));
    m = multiply(m, rotate_y(45.0));
    multiply(m, translate(-0.5, -0.5, -0.5))
}

fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (column, row) in out.iter_mut().enumerate() {
        for (i, value) in row.iter_mut().enumerate() {
            *value = (0..4).map(|k| a[k][i] * b[column][k]).sum();
        }
    }
    out
}

/// Column-major translation, as OpenGL stores it.
fn translate(x: f32, y: f32, z: f32) -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [x, y, z, 1.0],
    ]
}

fn scale(x: f32, y: f32, z: f32) -> [[f32; 4]; 4] {
    [
        [x, 0.0, 0.0, 0.0],
        [0.0, y, 0.0, 0.0],
        [0.0, 0.0, z, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// `glRotatef(degrees, 1, 0, 0)`.
fn rotate_x(degrees: f32) -> [[f32; 4]; 4] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos, sin, 0.0],
        [0.0, -sin, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// `glRotatef(degrees, 0, 1, 0)`.
fn rotate_y(degrees: f32) -> [[f32; 4]; 4] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [
        [cos, 0.0, -sin, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sin, 0.0, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// A point through a column-major matrix.
fn transform_point(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| m[0][i] * p[0] + m[1][i] * p[1] + m[2][i] * p[2] + m[3][i])
}

/// A direction through a matrix's rotation, translation dropped.
fn transform_normal(m: &[[f32; 4]; 4], n: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| m[0][i] * n[0] + m[1][i] * n[1] + m[2][i] * n[2])
}

/// The flat item's plane, as the item renderer's own quads see it: the era's
/// `ItemModelGenerator` plane 1/16 above the middle, a 1x1 quad.
pub const FLAT_MIN: f32 = -0.5;
pub const FLAT_MAX: f32 = 0.5;
/// The z of [`FLAT_MIN`]'s plane: `7.5 / 16` blocks, [`FLAT_Z`].
pub const FLAT_PLANE: f32 = FLAT_Z;

/// The 1.7.10 item registry, `Item.REGISTRY` in id order: the item's numeric id and its
/// registry name, which is also the engine's item id (`minecraft:<name>`).
pub const ITEMS: &[(u16, &str)] = &[
    (1, "minecraft:stone"),
    (2, "minecraft:grass"),
    (3, "minecraft:dirt"),
    (4, "minecraft:cobblestone"),
    (5, "minecraft:planks"),
    (6, "minecraft:sapling"),
    (7, "minecraft:bedrock"),
    (8, "minecraft:flowing_water"),
    (9, "minecraft:water"),
    (10, "minecraft:flowing_lava"),
    (11, "minecraft:lava"),
    (12, "minecraft:sand"),
    (13, "minecraft:gravel"),
    (14, "minecraft:gold_ore"),
    (15, "minecraft:iron_ore"),
    (16, "minecraft:coal_ore"),
    (17, "minecraft:log"),
    (18, "minecraft:leaves"),
    (19, "minecraft:sponge"),
    (20, "minecraft:glass"),
    (21, "minecraft:lapis_ore"),
    (22, "minecraft:lapis_block"),
    (23, "minecraft:dispenser"),
    (24, "minecraft:sandstone"),
    (25, "minecraft:noteblock"),
    (27, "minecraft:golden_rail"),
    (28, "minecraft:detector_rail"),
    (29, "minecraft:sticky_piston"),
    (30, "minecraft:web"),
    (31, "minecraft:tallgrass"),
    (32, "minecraft:deadbush"),
    (33, "minecraft:piston"),
    (35, "minecraft:wool"),
    (37, "minecraft:yellow_flower"),
    (38, "minecraft:red_flower"),
    (39, "minecraft:brown_mushroom"),
    (40, "minecraft:red_mushroom"),
    (41, "minecraft:gold_block"),
    (42, "minecraft:iron_block"),
    (43, "minecraft:double_stone_slab"),
    (44, "minecraft:stone_slab"),
    (45, "minecraft:brick_block"),
    (46, "minecraft:tnt"),
    (47, "minecraft:bookshelf"),
    (48, "minecraft:mossy_cobblestone"),
    (49, "minecraft:obsidian"),
    (50, "minecraft:torch"),
    (51, "minecraft:fire"),
    (52, "minecraft:mob_spawner"),
    (53, "minecraft:oak_stairs"),
    (54, "minecraft:chest"),
    (56, "minecraft:diamond_ore"),
    (57, "minecraft:diamond_block"),
    (58, "minecraft:crafting_table"),
    (60, "minecraft:farmland"),
    (61, "minecraft:furnace"),
    (62, "minecraft:lit_furnace"),
    (65, "minecraft:ladder"),
    (66, "minecraft:rail"),
    (67, "minecraft:stone_stairs"),
    (69, "minecraft:lever"),
    (70, "minecraft:stone_pressure_plate"),
    (72, "minecraft:wooden_pressure_plate"),
    (73, "minecraft:redstone_ore"),
    (76, "minecraft:redstone_torch"),
    (77, "minecraft:stone_button"),
    (78, "minecraft:snow_layer"),
    (79, "minecraft:ice"),
    (80, "minecraft:snow"),
    (81, "minecraft:cactus"),
    (82, "minecraft:clay"),
    (84, "minecraft:jukebox"),
    (85, "minecraft:fence"),
    (86, "minecraft:pumpkin"),
    (87, "minecraft:netherrack"),
    (88, "minecraft:soul_sand"),
    (89, "minecraft:glowstone"),
    (90, "minecraft:portal"),
    (91, "minecraft:lit_pumpkin"),
    (95, "minecraft:stained_glass"),
    (96, "minecraft:trapdoor"),
    (97, "minecraft:monster_egg"),
    (98, "minecraft:stonebrick"),
    (99, "minecraft:brown_mushroom_block"),
    (100, "minecraft:red_mushroom_block"),
    (101, "minecraft:iron_bars"),
    (102, "minecraft:glass_pane"),
    (103, "minecraft:melon_block"),
    (106, "minecraft:vine"),
    (107, "minecraft:fence_gate"),
    (108, "minecraft:brick_stairs"),
    (109, "minecraft:stone_brick_stairs"),
    (110, "minecraft:mycelium"),
    (111, "minecraft:waterlily"),
    (112, "minecraft:nether_brick"),
    (113, "minecraft:nether_brick_fence"),
    (114, "minecraft:nether_brick_stairs"),
    (116, "minecraft:enchanting_table"),
    (119, "minecraft:end_portal"),
    (120, "minecraft:end_portal_frame"),
    (121, "minecraft:end_stone"),
    (122, "minecraft:dragon_egg"),
    (123, "minecraft:redstone_lamp"),
    (125, "minecraft:double_wooden_slab"),
    (126, "minecraft:wooden_slab"),
    (127, "minecraft:cocoa"),
    (128, "minecraft:sandstone_stairs"),
    (129, "minecraft:emerald_ore"),
    (130, "minecraft:ender_chest"),
    (131, "minecraft:tripwire_hook"),
    (133, "minecraft:emerald_block"),
    (134, "minecraft:spruce_stairs"),
    (135, "minecraft:birch_stairs"),
    (136, "minecraft:jungle_stairs"),
    (137, "minecraft:command_block"),
    (138, "minecraft:beacon"),
    (139, "minecraft:cobblestone_wall"),
    (141, "minecraft:carrots"),
    (142, "minecraft:potatoes"),
    (143, "minecraft:wooden_button"),
    (145, "minecraft:anvil"),
    (146, "minecraft:trapped_chest"),
    (147, "minecraft:light_weighted_pressure_plate"),
    (148, "minecraft:heavy_weighted_pressure_plate"),
    (151, "minecraft:daylight_detector"),
    (152, "minecraft:redstone_block"),
    (153, "minecraft:quartz_ore"),
    (154, "minecraft:hopper"),
    (155, "minecraft:quartz_block"),
    (156, "minecraft:quartz_stairs"),
    (157, "minecraft:activator_rail"),
    (158, "minecraft:dropper"),
    (159, "minecraft:stained_hardened_clay"),
    (160, "minecraft:stained_glass_pane"),
    (161, "minecraft:leaves2"),
    (162, "minecraft:log2"),
    (163, "minecraft:acacia_stairs"),
    (164, "minecraft:dark_oak_stairs"),
    (170, "minecraft:hay_block"),
    (171, "minecraft:carpet"),
    (172, "minecraft:hardened_clay"),
    (173, "minecraft:coal_block"),
    (174, "minecraft:packed_ice"),
    (175, "minecraft:double_plant"),
    (256, "minecraft:iron_shovel"),
    (257, "minecraft:iron_pickaxe"),
    (258, "minecraft:iron_axe"),
    (259, "minecraft:flint_and_steel"),
    (260, "minecraft:apple"),
    (261, "minecraft:bow"),
    (262, "minecraft:arrow"),
    (263, "minecraft:coal"),
    (264, "minecraft:diamond"),
    (265, "minecraft:iron_ingot"),
    (266, "minecraft:gold_ingot"),
    (267, "minecraft:iron_sword"),
    (268, "minecraft:wooden_sword"),
    (269, "minecraft:wooden_shovel"),
    (270, "minecraft:wooden_pickaxe"),
    (271, "minecraft:wooden_axe"),
    (272, "minecraft:stone_sword"),
    (273, "minecraft:stone_shovel"),
    (274, "minecraft:stone_pickaxe"),
    (275, "minecraft:stone_axe"),
    (276, "minecraft:diamond_sword"),
    (277, "minecraft:diamond_shovel"),
    (278, "minecraft:diamond_pickaxe"),
    (279, "minecraft:diamond_axe"),
    (280, "minecraft:stick"),
    (281, "minecraft:bowl"),
    (282, "minecraft:mushroom_stew"),
    (283, "minecraft:golden_sword"),
    (284, "minecraft:golden_shovel"),
    (285, "minecraft:golden_pickaxe"),
    (286, "minecraft:golden_axe"),
    (287, "minecraft:string"),
    (288, "minecraft:feather"),
    (289, "minecraft:gunpowder"),
    (290, "minecraft:wooden_hoe"),
    (291, "minecraft:stone_hoe"),
    (292, "minecraft:iron_hoe"),
    (293, "minecraft:diamond_hoe"),
    (294, "minecraft:golden_hoe"),
    (295, "minecraft:wheat_seeds"),
    (296, "minecraft:wheat"),
    (297, "minecraft:bread"),
    (298, "minecraft:leather_helmet"),
    (299, "minecraft:leather_chestplate"),
    (300, "minecraft:leather_leggings"),
    (301, "minecraft:leather_boots"),
    (302, "minecraft:chainmail_helmet"),
    (303, "minecraft:chainmail_chestplate"),
    (304, "minecraft:chainmail_leggings"),
    (305, "minecraft:chainmail_boots"),
    (306, "minecraft:iron_helmet"),
    (307, "minecraft:iron_chestplate"),
    (308, "minecraft:iron_leggings"),
    (309, "minecraft:iron_boots"),
    (310, "minecraft:diamond_helmet"),
    (311, "minecraft:diamond_chestplate"),
    (312, "minecraft:diamond_leggings"),
    (313, "minecraft:diamond_boots"),
    (314, "minecraft:golden_helmet"),
    (315, "minecraft:golden_chestplate"),
    (316, "minecraft:golden_leggings"),
    (317, "minecraft:golden_boots"),
    (318, "minecraft:flint"),
    (319, "minecraft:porkchop"),
    (320, "minecraft:cooked_porkchop"),
    (321, "minecraft:painting"),
    (322, "minecraft:golden_apple"),
    (323, "minecraft:sign"),
    (324, "minecraft:wooden_door"),
    (325, "minecraft:bucket"),
    (326, "minecraft:water_bucket"),
    (327, "minecraft:lava_bucket"),
    (328, "minecraft:minecart"),
    (329, "minecraft:saddle"),
    (330, "minecraft:iron_door"),
    (331, "minecraft:redstone"),
    (332, "minecraft:snowball"),
    (333, "minecraft:boat"),
    (334, "minecraft:leather"),
    (335, "minecraft:milk_bucket"),
    (336, "minecraft:brick"),
    (337, "minecraft:clay_ball"),
    (338, "minecraft:reeds"),
    (339, "minecraft:paper"),
    (340, "minecraft:book"),
    (341, "minecraft:slime_ball"),
    (342, "minecraft:chest_minecart"),
    (343, "minecraft:furnace_minecart"),
    (344, "minecraft:egg"),
    (345, "minecraft:compass"),
    (346, "minecraft:fishing_rod"),
    (347, "minecraft:clock"),
    (348, "minecraft:glowstone_dust"),
    (349, "minecraft:fish"),
    (350, "minecraft:cooked_fished"),
    (351, "minecraft:dye"),
    (352, "minecraft:bone"),
    (353, "minecraft:sugar"),
    (354, "minecraft:cake"),
    (355, "minecraft:bed"),
    (356, "minecraft:repeater"),
    (357, "minecraft:cookie"),
    (358, "minecraft:filled_map"),
    (359, "minecraft:shears"),
    (360, "minecraft:melon"),
    (361, "minecraft:pumpkin_seeds"),
    (362, "minecraft:melon_seeds"),
    (363, "minecraft:beef"),
    (364, "minecraft:cooked_beef"),
    (365, "minecraft:chicken"),
    (366, "minecraft:cooked_chicken"),
    (367, "minecraft:rotten_flesh"),
    (368, "minecraft:ender_pearl"),
    (369, "minecraft:blaze_rod"),
    (370, "minecraft:ghast_tear"),
    (371, "minecraft:gold_nugget"),
    (372, "minecraft:nether_wart"),
    (373, "minecraft:potion"),
    (374, "minecraft:glass_bottle"),
    (375, "minecraft:spider_eye"),
    (376, "minecraft:fermented_spider_eye"),
    (377, "minecraft:blaze_powder"),
    (378, "minecraft:magma_cream"),
    (379, "minecraft:brewing_stand"),
    (380, "minecraft:cauldron"),
    (381, "minecraft:ender_eye"),
    (382, "minecraft:speckled_melon"),
    (383, "minecraft:spawn_egg"),
    (384, "minecraft:experience_bottle"),
    (385, "minecraft:fire_charge"),
    (386, "minecraft:writable_book"),
    (387, "minecraft:written_book"),
    (388, "minecraft:emerald"),
    (389, "minecraft:item_frame"),
    (390, "minecraft:flower_pot"),
    (391, "minecraft:carrot"),
    (392, "minecraft:potato"),
    (393, "minecraft:baked_potato"),
    (394, "minecraft:poisonous_potato"),
    (395, "minecraft:map"),
    (396, "minecraft:golden_carrot"),
    (397, "minecraft:skull"),
    (398, "minecraft:carrot_on_a_stick"),
    (399, "minecraft:nether_star"),
    (400, "minecraft:pumpkin_pie"),
    (401, "minecraft:fireworks"),
    (402, "minecraft:firework_charge"),
    (403, "minecraft:enchanted_book"),
    (404, "minecraft:comparator"),
    (405, "minecraft:netherbrick"),
    (406, "minecraft:quartz"),
    (407, "minecraft:tnt_minecart"),
    (408, "minecraft:hopper_minecart"),
    (417, "minecraft:iron_horse_armor"),
    (418, "minecraft:golden_horse_armor"),
    (419, "minecraft:diamond_horse_armor"),
    (420, "minecraft:lead"),
    (421, "minecraft:name_tag"),
    (422, "minecraft:command_block_minecart"),
    (2256, "minecraft:record_13"),
    (2257, "minecraft:record_cat"),
    (2258, "minecraft:record_blocks"),
    (2259, "minecraft:record_chirp"),
    (2260, "minecraft:record_far"),
    (2261, "minecraft:record_mall"),
    (2262, "minecraft:record_mellohi"),
    (2263, "minecraft:record_stal"),
    (2264, "minecraft:record_strad"),
    (2265, "minecraft:record_ward"),
    (2266, "minecraft:record_11"),
    (2267, "minecraft:record_wait"),
];

#[cfg(test)]
#[path = "items_1_7_10_tests.rs"]
mod tests;
