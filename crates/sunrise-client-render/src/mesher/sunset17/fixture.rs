//! The 1.7.10 sample world of the probe and the unit tests.
//!
//! Every block and atlas cell mirrors `tools/javaoracle_1_7_10/RenderOracle.java`:
//! the same 256x256 atlas of 16x16 cells, the same blocks and the same light, so
//! the vertex stream the era renderer produces can be compared with the real
//! 1.7.10 client's line by line.

use super::State17;
use super::StateTable17;
use super::blocks::{
    BlockDef, FACE_BOTTOM, FACE_EAST, FACE_NORTH, FACE_SOUTH, FACE_TOP, FACE_WEST,
};
use super::render::DoublePlantSprites;
use super::sprite::Sprite;
use super::world::WorldView;

/// The shared atlas size of the fixture sprites.
pub const ATLAS: i32 = 256;

/// The fixture's grass colour, the one `RenderOracle` fills `GrassColors` with.
pub const GRASS_TINT: i32 = 0x91BD59;

const SIZE: i32 = 16;

fn cell(column: i32, row: i32) -> Sprite {
    Sprite::atlas(0, column * 16, row * 16, ATLAS, ATLAS)
}

/// One fixture state: the era id and metadata, the block name and its sprites.
struct Row {
    id: u16,
    meta: u8,
    name: &'static str,
    /// Bottom, top, north, south, west, east.
    sprites: [Sprite; 6],
    overlay: Option<Sprite>,
}

fn same(sprite: Sprite) -> [Sprite; 6] {
    [sprite; 6]
}

/// The states the oracle's cases use, with the era's own ids.
fn rows() -> Vec<Row> {
    let dirt = cell(1, 0);
    let planks = cell(2, 2);
    let log_side = cell(0, 1);
    let log_top = cell(1, 1);
    let grass_top = cell(2, 0);
    let grass_side = cell(3, 0).marked(true);
    let grass_overlay = cell(5, 0);
    let slab_top = cell(0, 3);
    let slab_side = cell(1, 3);
    let tallgrass = cell(2, 3);
    let fern = cell(3, 3);
    let water_still = cell(5, 3);
    let water_flow = cell(6, 3);
    let snow = cell(7, 3);
    let mut rows = vec![
        Row {
            id: 1,
            meta: 0,
            name: "stone",
            sprites: same(cell(0, 0)),
            overlay: None,
        },
        Row {
            id: 2,
            meta: 0,
            name: "grass",
            sprites: [
                dirt, grass_top, grass_side, grass_side, grass_side, grass_side,
            ],
            overlay: Some(grass_overlay),
        },
        Row {
            id: 3,
            meta: 0,
            name: "dirt",
            sprites: same(dirt),
            overlay: None,
        },
        Row {
            id: 17,
            meta: 0,
            name: "log",
            sprites: [log_top, log_top, log_side, log_side, log_side, log_side],
            overlay: None,
        },
        Row {
            id: 17,
            meta: 4,
            name: "log",
            sprites: [log_side, log_side, log_side, log_side, log_top, log_top],
            overlay: None,
        },
        Row {
            id: 17,
            meta: 8,
            name: "log",
            sprites: [log_side, log_side, log_top, log_top, log_side, log_side],
            overlay: None,
        },
        Row {
            id: 20,
            meta: 0,
            name: "glass",
            sprites: same(cell(0, 2)),
            overlay: None,
        },
        Row {
            id: 18,
            meta: 0,
            name: "leaves",
            sprites: same(cell(0, 4)),
            overlay: None,
        },
        Row {
            id: 79,
            meta: 0,
            name: "ice",
            sprites: same(cell(1, 4)),
            overlay: None,
        },
        Row {
            id: 95,
            meta: 0,
            name: "stained_glass",
            sprites: same(cell(2, 4)),
            overlay: None,
        },
        Row {
            id: 50,
            meta: 0,
            name: "torch",
            sprites: same(cell(1, 2)),
            overlay: None,
        },
        Row {
            id: 50,
            meta: 1,
            name: "torch",
            sprites: same(cell(1, 2)),
            overlay: None,
        },
        Row {
            id: 44,
            meta: 0,
            name: "stone_slab",
            sprites: [
                slab_top, slab_top, slab_side, slab_side, slab_side, slab_side,
            ],
            overlay: None,
        },
        Row {
            id: 44,
            meta: 8,
            name: "stone_slab",
            sprites: [
                slab_top, slab_top, slab_side, slab_side, slab_side, slab_side,
            ],
            overlay: None,
        },
        Row {
            id: 53,
            meta: 0,
            name: "oak_stairs",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 53,
            meta: 1,
            name: "oak_stairs",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 53,
            meta: 2,
            name: "oak_stairs",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 53,
            meta: 3,
            name: "oak_stairs",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 53,
            meta: 4,
            name: "oak_stairs",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 85,
            meta: 0,
            name: "fence",
            sprites: same(planks),
            overlay: None,
        },
        Row {
            id: 31,
            meta: 1,
            name: "tallgrass",
            sprites: same(tallgrass),
            overlay: None,
        },
        Row {
            id: 31,
            meta: 2,
            name: "tallgrass",
            sprites: same(fern),
            overlay: None,
        },
        Row {
            id: 9,
            meta: 0,
            name: "water",
            sprites: [
                water_still,
                water_still,
                water_flow,
                water_flow,
                water_flow,
                water_flow,
            ],
            overlay: None,
        },
        Row {
            id: 8,
            meta: 1,
            name: "flowing_water",
            sprites: [
                water_still,
                water_still,
                water_flow,
                water_flow,
                water_flow,
                water_flow,
            ],
            overlay: None,
        },
        Row {
            id: 81,
            meta: 0,
            name: "cactus",
            sprites: [
                cell(0, 5),
                cell(1, 5),
                cell(2, 5),
                cell(2, 5),
                cell(2, 5),
                cell(2, 5),
            ],
            overlay: None,
        },
        Row {
            id: 78,
            meta: 3,
            name: "snow_layer",
            sprites: same(snow),
            overlay: None,
        },
    ];
    double_plant_rows(&mut rows);
    rows
}

/// `DoublePlantBlock`'s sprites at the oracle's atlas cells: bottom and top by
/// variant, then the sunflower's face front and back.
pub fn double_plant_sprites() -> DoublePlantSprites {
    let bottom = [
        cell(4, 4),
        cell(8, 4),
        cell(0, 4),
        cell(2, 4),
        cell(10, 4),
        cell(12, 4),
    ];
    let top = [
        cell(5, 4),
        cell(9, 4),
        cell(1, 4),
        cell(3, 4),
        cell(11, 4),
        cell(13, 4),
    ];
    DoublePlantSprites {
        bottom,
        top,
        sunflower: [cell(6, 4), cell(7, 4)],
    }
}

/// The double plant's sixteen metadata values: the six variants as the lower
/// half, and the upper half (bit 8) with the rotation bits a placed plant may
/// carry.
fn double_plant_rows(rows: &mut Vec<Row>) {
    let sprites = double_plant_sprites();
    for meta in 0..16u8 {
        let variant = usize::from(meta & 7).min(5);
        rows.push(Row {
            id: 175,
            meta,
            name: "double_plant",
            sprites: same(if meta & 8 != 0 {
                sprites.top[variant]
            } else {
                sprites.bottom[variant]
            }),
            overlay: None,
        });
    }
}

/// The fixture states, in the probe's state order.
pub fn states() -> StateTable17 {
    let rows = rows();
    let mut states = vec![State17::air()];
    let mut pairs = vec![(0_u16, 0_u8)];
    for row in rows {
        let block = BlockDef::new(row.name).with_key(row.id);
        states.push(State17 {
            // Every fixture block's emitted light is the era's.
            light: light_of(row.name),
            block,
            meta: row.meta,
            sprites: row.sprites,
            overlay: row.overlay,
        });
        pairs.push((row.id, row.meta));
    }
    StateTable17 { states, pairs }
}

/// `Block.getLight()` of the fixture blocks that emit.
fn light_of(name: &str) -> u8 {
    match name {
        "torch" => 14,
        _ => 0,
    }
}

/// The era id of a fixture block name.
pub fn id(name: &str) -> u16 {
    match name {
        "stone" => 1,
        "grass" => 2,
        "dirt" => 3,
        "flowing_water" => 8,
        "water" => 9,
        "log" => 17,
        "glass" => 20,
        "leaves" => 18,
        "ice" => 79,
        "stained_glass" => 95,
        "tallgrass" => 31,
        "double_plant" => 175,
        "stone_slab" => 44,
        "torch" => 50,
        "oak_stairs" => 53,
        "cactus" => 81,
        "snow_layer" => 78,
        "fence" => 85,
        _ => 0,
    }
}

/// The cell each face of the fixture sprites names, for the layer checks.
pub const FACE_CELLS: [(usize, i32, i32); 6] = [
    (FACE_BOTTOM as usize, 0, 0),
    (FACE_TOP as usize, 2, 0),
    (FACE_NORTH as usize, 3, 0),
    (FACE_SOUTH as usize, 3, 0),
    (FACE_WEST as usize, 3, 0),
    (FACE_EAST as usize, 3, 0),
];

/// A dense 16x16x16 world of fixture states with metadata and light storage.
pub struct TestWorld {
    table: StateTable17,
    state: Vec<u16>,
    sky: Vec<u8>,
    blocklight: Vec<u8>,
    double_plant: DoublePlantSprites,
    pub grass_tint: i32,
}

impl TestWorld {
    pub fn new() -> TestWorld {
        TestWorld {
            table: states(),
            state: vec![0; (SIZE * SIZE * SIZE) as usize],
            sky: vec![15; (SIZE * SIZE * SIZE) as usize],
            blocklight: vec![0; (SIZE * SIZE * SIZE) as usize],
            double_plant: double_plant_sprites(),
            grass_tint: GRASS_TINT,
        }
    }

    /// The fixture states, for `id(name)`.
    pub fn table(&self) -> &StateTable17 {
        &self.table
    }

    /// The fixture state list: era id and metadata pairs by state index.
    pub fn blocks(&self) -> &StateTable17 {
        &self.table
    }

    fn index(x: i32, y: i32, z: i32) -> usize {
        ((y * SIZE + z) * SIZE + x) as usize
    }

    /// `World.setBlock` with a fixture (id, metadata).
    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u8, meta: u8) {
        if !(0..SIZE).contains(&x) || !(0..SIZE).contains(&y) || !(0..SIZE).contains(&z) {
            return; // `World.setBlock` outside the world is a no-op, like vanilla
        }
        let state = self.table.state_of(u16::from(id), meta);
        self.state[Self::index(x, y, z)] = state;
    }

    pub fn set_sky_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        if (0..SIZE).contains(&x) && (0..SIZE).contains(&y) && (0..SIZE).contains(&z) {
            self.sky[Self::index(x, y, z)] = light;
        }
    }

    pub fn set_block_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        if (0..SIZE).contains(&x) && (0..SIZE).contains(&y) && (0..SIZE).contains(&z) {
            self.blocklight[Self::index(x, y, z)] = light;
        }
    }

    pub fn metadata_at(&self, x: i32, y: i32, z: i32) -> u8 {
        self.table.get(self.state[Self::index(x, y, z)]).meta
    }

    fn in_world(x: i32, y: i32, z: i32) -> bool {
        (0..SIZE).contains(&x) && (0..SIZE).contains(&y) && (0..SIZE).contains(&z)
    }
}

impl WorldView for TestWorld {
    fn state(&self, x: i32, y: i32, z: i32) -> &State17 {
        if !Self::in_world(x, y, z) {
            return self.table.get(0);
        }
        self.table.get(self.state[Self::index(x, y, z)])
    }

    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        // `World.getLight` clamps y below the world to its bottom section;
        // above the single chunk's sections a loaded chunk reports the sky
        // default.
        let y = if y < 0 { 0 } else { y };
        if y >= SIZE {
            return 15;
        }
        if !Self::in_world(x, y, z) {
            return 15;
        }
        i32::from(self.sky[Self::index(x, y, z)])
    }

    fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        let y = if y < 0 { 0 } else { y };
        if y >= SIZE {
            return 0;
        }
        if !Self::in_world(x, y, z) {
            return 0;
        }
        i32::from(self.blocklight[Self::index(x, y, z)])
    }

    fn biome_tint(&self, _tint: super::blocks::Tint, _x: i32, _y: i32, _z: i32) -> i32 {
        self.grass_tint
    }

    fn double_plant_sprites(&self) -> Option<&DoublePlantSprites> {
        Some(&self.double_plant)
    }
}

impl Default for TestWorld {
    fn default() -> TestWorld {
        TestWorld::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_world_is_air() {
        let world = TestWorld::new();
        assert_eq!(world.block(-1, 0, 0).kind, super::super::blocks::Kind::Air);
        assert_eq!(world.block(-1, 0, 0).key, 0);
    }

    #[test]
    fn set_and_read_roundtrip() {
        let mut world = TestWorld::new();
        world.set(1, 2, 3, 1, 0);
        assert_eq!(world.block(1, 2, 3).key, 1);
        assert_eq!(world.metadata(1, 2, 3), 0);
    }

    #[test]
    fn the_table_names_every_fixture_state() {
        let world = TestWorld::new();
        let stone = world.table().state_of(1, 0);
        assert_eq!(world.table().get(stone).block.key, 1);
        assert_eq!(world.table().state_of(17, 4), world.table().state_of(17, 4));
        assert_ne!(world.table().state_of(17, 0), world.table().state_of(17, 4));
    }

    #[test]
    fn light_below_the_world_reads_the_bottom_section() {
        // `World.getBrightness` clamps y < 0 to y = 0.
        let mut world = TestWorld::new();
        world.set_sky_light(2, 0, 3, 7);
        assert_eq!(world.sky_light(2, -1, 3), 7);
    }
}
