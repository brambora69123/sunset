//! The era's GUI item path: the cube/flat rule, the cube's GUI geometry, its
//! tints and the animated sprites' frames.

use super::*;

/// `Item.icon` lookups: the registry names the engine and the creative tabs use.
#[test]
fn the_item_registry_names_the_eras_items() {
    assert_eq!(item_id("minecraft:stick"), Some(280));
    assert_eq!(item_id("minecraft:stone"), Some(1));
    assert_eq!(item_name(331), Some("minecraft:redstone"));
    assert_eq!(item_name(175), Some("minecraft:double_plant"));
    assert_eq!(item_id("minecraft:not_an_era_item"), None);
    assert_eq!(item_name(0), None);
    // The registry is sorted by id, and every block item keeps its block's id.
    assert!(ITEMS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(item_id("minecraft:wool"), Some(35));
    assert_eq!(item_id("minecraft:record_wait"), Some(2267));
}

/// `BlockRenderer.isItem3d`'s cube shapes, as far as the icon painter covers
/// them: full cubes draw the block, every other shape its `BlockItem` icon.
#[test]
fn full_cubes_draw_as_blocks_and_other_shapes_as_icons() {
    let cube = |id: u16| drawn_as_block(block_of(id).expect("a block item"));
    assert!(cube(1), "stone is a cube");
    assert!(cube(2), "grass");
    assert!(cube(17), "log");
    assert!(cube(18), "leaves");
    assert!(cube(20), "glass");
    assert!(cube(79), "ice");
    assert!(cube(155), "quartz");
    assert!(!cube(44), "a slab keeps its flat icon");
    assert!(!cube(53), "stairs keep their flat icon");
    assert!(!cube(31), "tall grass is a crossed sprite");
    assert!(!cube(145), "an anvil is no full cube");
    assert!(!cube(78), "a snow layer");
    assert!(!cube(81), "a cactus");
    assert!(!cube(6), "a sapling is a crossed sprite");
    // A plain item places no block, so it has no cube path at all.
    assert!(block_of(280).is_none(), "stick");
    assert!(block_of(331).is_none(), "redstone");
}

/// `ItemRenderer.renderGuiItem`'s block branch: three faces of the era's iso
/// cube are visible and the cube fills the 16x16 slot at the era's scale.
#[test]
fn the_gui_cube_shows_the_top_south_and_west_faces() {
    let matrix = gui_block_matrix(0.0, 0.0);
    let visible: Vec<usize> = (0..FACES.len())
        .filter(|&face| transform_normal(&matrix, FACE_NORMAL[face])[2] > 0.0)
        .collect();
    assert_eq!(visible, [1, 3, 4], "top, south and west face the viewer");

    // The projected cube in a slot at (0, 0): 14.142 x 15.731 pixels,
    // from (x + 0.929, y + 0.134) to (x + 15.071, y + 15.866). The values come
    // from the transform above with the era's own numbers, not from the painter.
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for corner in 0..8 {
        let point = transform_point(
            &matrix,
            [
                (corner & 1) as f32,
                (corner >> 1 & 1) as f32,
                (corner >> 2 & 1) as f32,
            ],
        );
        for axis in 0..2 {
            lo[axis] = lo[axis].min(point[axis]);
            hi[axis] = hi[axis].max(point[axis]);
        }
    }
    let round = |v: f32| (v * 1000.0).round() / 1000.0;
    assert_eq!(round(lo[0]), 0.929);
    assert_eq!(round(hi[0]), 15.071);
    assert_eq!(round(lo[1]), 0.134);
    assert_eq!(round(hi[1]), 15.866);
}

/// `Block.getColor(metadata)`: grass and leaves carry the era's fixed colours,
/// everything else draws the texture as it is.
#[test]
fn the_cube_takes_the_blocks_fixed_colours() {
    let tint = |id: u16, meta: u16| {
        let block = block_of(id).expect("a block item");
        cube_tint(block, id, block_metadata(id, meta))
    };
    assert_eq!(
        tint(2, 0),
        0xFF_FF_FF,
        "grass draws white in `renderAsItem`"
    );
    assert_eq!(
        tint(18, 0),
        0x48_B5_18,
        "oak leaves, `FoliageColors` default"
    );
    assert_eq!(tint(18, 1), 0x61_99_61, "spruce leaves");
    assert_eq!(tint(18, 2), 0x80_A7_55, "birch leaves");
    assert_eq!(
        tint(161, 1),
        0x48_B5_18,
        "`Leaves2Block` keeps the default foliage colour"
    );
    assert_eq!(tint(1, 0), 0xFF_FF_FF, "stone");
    // A `ColoredBlockItem` stores the dye value; the block metadata is inverted.
    assert_eq!(block_metadata(35, 0), 15, "white wool");
    assert_eq!(block_metadata(35, 15), 0, "black wool");
    assert_eq!(
        block_metadata(61, 2),
        3,
        "a furnace always draws facing south"
    );
}

/// The flat icons come from the era's own sprite table: an item from
/// `textures/items`, a non-cube block item from the block's top sprite.
#[test]
fn flat_icons_are_the_eras_sprites() {
    let sprite = |id: i32, meta: i32| {
        let texture = sunset17_assets::item_texture(id, meta).expect("a sprite");
        sprite_name(&texture)
    };
    assert_eq!(sprite(280, 0), "minecraft:items/stick");
    assert_eq!(sprite(331, 0), "minecraft:items/redstone_dust");
    assert_eq!(sprite(351, 14), "minecraft:items/dye_powder_orange");
    assert_eq!(sprite(44, 0), "minecraft:blocks/stone_slab_side");
    assert_eq!(sprite(31, 1), "minecraft:blocks/tallgrass");
}

/// `TextureAtlasSprite`'s frames: the era's two animated item sprites are
/// vertical strips of 16x16 frames, one per tick.
#[test]
fn an_animated_sprite_shows_one_frame_per_tick() {
    let strip = Rect {
        u0: 0.0,
        v0: 0.5,
        u1: 0.25,
        v1: 1.0,
        w: 16,
        h: 512,
    };
    let frame = frame_rect(strip, 0);
    assert_eq!((frame.u0, frame.u1), (0.0, 0.25));
    assert_eq!((frame.v0, frame.v1), (0.5, 0.5 + 0.5 / 32.0));
    let frame = frame_rect(strip, 33);
    assert_eq!(frame.v0, 0.5 + 0.5 / 32.0);
    assert_eq!(frame.v1, 0.5 + 2.0 * (0.5 / 32.0));
    // A still sprite is its whole rect.
    let still = Rect {
        u0: 0.0,
        v0: 0.0,
        u1: 0.25,
        v1: 0.25,
        w: 16,
        h: 16,
    };
    assert_eq!(frame_rect(still, 7), still);
}
