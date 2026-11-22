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
use crate::sunset17_assets::{self, Texture, sprite_name};
use std::sync::OnceLock;

/// The pixels one item fills in a GUI slot (`ItemRenderer.drawTexture`'s 16x16).
pub const ICON_SIZE: i32 = 16;

/// The era's flat top face of an item model: `ItemModelGenerator`'s 1/16 plane.
const FLAT_Z: f32 = 7.5 / 16.0;

/// The face shades of the flat-lit pass: bottom, top, north, south, west, east
/// (`BlockRenderer.tesselate*Face`).
const FACE_SHADE: [f32; 6] = [0.5, 1.0, 0.8, 0.8, 0.6, 0.6];

/// One face of the unit cube, in `BlockRenderer`'s order (bottom, top, north,
/// south, west, east): four corners and the sprite fraction each samples, `u`
/// and `v` running 0..1 from the sprite's `uMin`/`vMin` (the atlas cell's left
/// and top edge) to its `uMax`/`vMax`.
const FACES: [[([f32; 3], [f32; 2]); 4]; 6] = [
    // bottom
    [
        ([0.0, 0.0, 1.0], [0.0, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 1.0]),
        ([1.0, 0.0, 0.0], [1.0, 1.0]),
        ([1.0, 0.0, 1.0], [1.0, 0.0]),
    ],
    // top
    [
        ([1.0, 1.0, 1.0], [1.0, 0.0]),
        ([1.0, 1.0, 0.0], [1.0, 1.0]),
        ([0.0, 1.0, 0.0], [0.0, 1.0]),
        ([0.0, 1.0, 1.0], [0.0, 0.0]),
    ],
    // north
    [
        ([0.0, 1.0, 0.0], [1.0, 1.0]),
        ([1.0, 1.0, 0.0], [0.0, 1.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 0.0, 0.0], [1.0, 0.0]),
    ],
    // south
    [
        ([0.0, 1.0, 1.0], [0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 1.0]),
        ([1.0, 0.0, 1.0], [1.0, 0.0]),
        ([1.0, 1.0, 1.0], [0.0, 0.0]),
    ],
    // west
    [
        ([0.0, 1.0, 1.0], [1.0, 1.0]),
        ([0.0, 1.0, 0.0], [0.0, 1.0]),
        ([0.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0]),
    ],
    // east
    [
        ([0.0, 0.0, 1.0], [0.0, 0.0]),
        ([0.0, 0.0, 0.0], [1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 1.0]),
        ([0.0, 1.0, 1.0], [0.0, 1.0]),
    ],
];

/// `Block.getRenderType`'s outward normal per face, in [`FACES`] order.
const FACE_NORMAL: [[f32; 3]; 6] = [
    [0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, -1.0],
    [0.0, 0.0, 1.0],
    [-1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
];

/// `ColorizerFoliage.getDefaultColor()`: `AbstractLeavesBlock.getColor(metadata)`,
/// the colour oak and jungle leaves draw in the icon (and both leaves of id 161).
pub const DEFAULT_LEAVES_COLOR: u32 = 0x48_B5_18;
/// `ColorizerFoliage.getSpruceColor()` (`LeavesBlock.getColor` metadata 1).
pub const SPRUCE_LEAVES_COLOR: u32 = 0x61_99_61;
/// `ColorizerFoliage.getBirchColor()` (`LeavesBlock.getColor` metadata 2).
pub const BIRCH_LEAVES_COLOR: u32 = 0x80_A7_55;

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
    match block_of(id) {
        Some(block) if drawn_as_block(block) => paint_block(batch, atlas, block, id, meta, x, y),
        _ => {
            let Some(texture) = sunset17_assets::item_texture(i32::from(id), i32::from(meta))
            else {
                return;
            };
            paint_flat(batch, atlas, &texture, x, y, tick);
        }
    }
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

/// Whether `renderAsItem` draws the block as a cube: `BlockRenderer.isItem3d`'s
/// cube shapes, here the full-cube blocks. The era gives slabs, stairs, fences,
/// snow layers, cacti and the crossed plants their own geometry; those keep the
/// flat icon.
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
    ) && block.bounds.is_full_cube()
}

/// The flat path: one 16x16 quad of the item's sprite, the era's
/// `ItemRenderer.drawTexture`.
fn paint_flat(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    texture: &Texture,
    x: f32,
    y: f32,
    tick: u64,
) {
    let Some(rect) = atlas.gui_rect(&sprite_name(texture)).copied() else {
        return;
    };
    let rect = frame_rect(rect, tick);
    batch.rect_uv(x, y, ICON_SIZE as f32, ICON_SIZE as f32, rect, u32::MAX);
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

/// The block path: the visible faces of the era's GUI cube, shaded and tinted
/// (`BlockRenderer.renderAsItem`'s cube branch).
fn paint_block(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    block: &BlockDef,
    id: u16,
    meta: u16,
    x: f32,
    y: f32,
) {
    let block_meta = block_metadata(id, meta);
    let tint = cube_tint(block, id, block_meta);
    let matrix = gui_block_matrix(x, y);
    // Only the faces turned toward the viewer draw: the era leaves back-face
    // culling on for this pass. Painter's order between them, farthest first.
    let mut faces: Vec<(f32, [([f32; 3], [f32; 2]); 4], usize)> = Vec::with_capacity(3);
    for (face, corners) in FACES.into_iter().enumerate() {
        let normal = transform_normal(&matrix, FACE_NORMAL[face]);
        if normal[2] <= 0.0 {
            continue;
        }
        let depth = corners
            .iter()
            .map(|&(corner, _)| transform_point(&matrix, corner)[2])
            .sum::<f32>()
            / 4.0;
        faces.push((depth, corners, face));
    }
    faces.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, corners, face) in faces {
        let Some(texture) = sunset17_assets::block_texture(id, block_meta, face as u8) else {
            continue;
        };
        let Some(rect) = atlas.gui_rect(&sprite_name(&texture)).copied() else {
            continue;
        };
        let shade = FACE_SHADE[face];
        let color = rgba(
            (f32::from((tint >> 16 & 0xFF) as u8) * shade) as u8,
            (f32::from((tint >> 8 & 0xFF) as u8) * shade) as u8,
            (f32::from((tint & 0xFF) as u8) * shade) as u8,
            0xFF,
        );
        let mut positions = [[0.0; 2]; 4];
        let mut uvs = [[0.0; 2]; 4];
        for (i, &(corner, uv)) in corners.iter().enumerate() {
            let p = transform_point(&matrix, corner);
            positions[i] = [p[0], p[1]];
            uvs[i] = [
                rect.u0 + (rect.u1 - rect.u0) * uv[0],
                rect.v0 + (rect.v1 - rect.v0) * uv[1],
            ];
        }
        batch.quad(positions, uvs, color);
    }
}

/// `Block.getColor(metadata)`, the tint the cube is drawn with. `renderAsItem`
/// draws grass white; the leaves answer with the era's foliage colours, and
/// every other block keeps its texture.
fn cube_tint(block: &BlockDef, id: u16, meta: u8) -> u32 {
    match block.kind {
        Kind::Grass => 0xFF_FF_FF,
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
/// their metadata-3 orientation; a `ColoredBlockItem` stores the dye value,
/// whose block metadata is `~item & 15`.
fn block_metadata(id: u16, meta: u16) -> u8 {
    match id {
        35 | 95 | 159 | 160 | 171 => (!(meta as u8)) & 15,
        23 | 61 | 158 => 3,
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
