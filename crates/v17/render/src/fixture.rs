//! Sample blocks and a small test world for the unit tests and the probe.
//!
//! The block table mirrors the Java render oracle
//! (`tools/javaoracle_1_7_10/RenderOracle.java`): the same 256x256 atlas
//! cells and the same block properties, so the two vertex streams can be
//! compared line by line.

use crate::block::{BlockDef, CrossKind, Kind, Sprites};
use crate::material::Material;
use crate::sprite::Sprite;
use crate::world::WorldView;

/// The shared atlas size of the fixture sprites.
pub const ATLAS: i32 = 256;

fn icon(name: &'static str, column: i32, row: i32) -> Sprite {
    Sprite::atlas(name, column * 16, row * 16, ATLAS, ATLAS)
}

/// The blocks the fixture world carries.
pub struct Blocks {
    pub air: BlockDef,
    pub stone: BlockDef,
    pub grass: BlockDef,
    pub dirt: BlockDef,
    pub log: BlockDef,
    pub glass: BlockDef,
    pub torch: BlockDef,
    pub slab: BlockDef,
    pub stairs: BlockDef,
    pub fence: BlockDef,
    pub tallgrass: BlockDef,
    pub fern: BlockDef,
    pub water: BlockDef,
    pub snow: BlockDef,
}

impl Blocks {
    pub fn new() -> Blocks {
        let dirt_sprite = icon("dirt", 1, 0);
        let planks = icon("planks_oak", 2, 2);
        Blocks {
            air: BlockDef::new(
                "air",
                Kind::Air,
                Material::AIR,
                Sprites::Single(icon("air", 0, 0)),
            ),
            stone: BlockDef::new(
                "stone",
                Kind::Cube,
                Material::STONE,
                Sprites::Single(icon("stone", 0, 0)),
            ),
            grass: BlockDef::new(
                "grass",
                Kind::Grass,
                Material::GRASS,
                Sprites::Grass {
                    top: icon("grass_top", 2, 0),
                    side: icon("grass_side", 3, 0),
                    side_overlay: icon("grass_side_overlay", 5, 0),
                    bottom: dirt_sprite,
                },
            ),
            dirt: BlockDef::new(
                "dirt",
                Kind::Cube,
                Material::DIRT,
                Sprites::Single(dirt_sprite),
            ),
            log: BlockDef::new(
                "log",
                Kind::Log,
                Material::WOOD,
                Sprites::Log {
                    side: icon("log_oak", 0, 1),
                    top: icon("log_oak_top", 1, 1),
                },
            ),
            glass: BlockDef::new(
                "glass",
                Kind::Glass,
                Material::GLASS,
                Sprites::Single(icon("glass", 0, 2)),
            ),
            torch: BlockDef::new(
                "torch",
                Kind::Torch,
                Material::PLANT,
                Sprites::Single(icon("torch_on", 1, 2)),
            )
            .with_light(14),
            slab: BlockDef::new(
                "stone_slab",
                Kind::Slab,
                Material::STONE,
                Sprites::Slab {
                    top: icon("stone_slab_top", 0, 3),
                    side: icon("stone_slab_side", 1, 3),
                },
            ),
            stairs: BlockDef::new(
                "oak_stairs",
                Kind::Stairs,
                Material::WOOD,
                Sprites::Single(planks),
            ),
            fence: BlockDef::new(
                "oak_fence",
                Kind::Fence,
                Material::WOOD,
                Sprites::Single(planks),
            ),
            tallgrass: BlockDef::new(
                "tallgrass",
                Kind::Cross(CrossKind::TallGrass),
                Material::PLANT,
                Sprites::Single(icon("tallgrass", 2, 3)),
            )
            .with_tint(0x91BD59),
            fern: BlockDef::new(
                "fern",
                Kind::Cross(CrossKind::TallGrass),
                Material::PLANT,
                Sprites::Single(icon("fern", 3, 3)),
            )
            .with_tint(0x91BD59),
            water: BlockDef::new(
                "water",
                Kind::Liquid,
                Material::WATER,
                Sprites::Liquid {
                    still: icon("water_still", 5, 3),
                    flow: icon("water_flow", 6, 3),
                },
            ),
            snow: BlockDef::new(
                "snow",
                Kind::Snow,
                Material::SNOW_LAYER,
                Sprites::Single(icon("snow", 7, 3)),
            ),
        }
    }

    /// Block ids of the fixture world.
    pub fn id(&self, name: &str) -> usize {
        match name {
            "air" => 0,
            "stone" => 1,
            "grass" => 2,
            "dirt" => 3,
            "log" => 4,
            "glass" => 5,
            "torch" => 6,
            "stone_slab" => 7,
            "oak_stairs" => 8,
            "oak_fence" => 9,
            "tallgrass" => 10,
            "fern" => 11,
            "water" => 12,
            "snow" => 13,
            _ => 0,
        }
    }
}

impl Default for Blocks {
    fn default() -> Blocks {
        Blocks::new()
    }
}

/// A dense 16x16x16 world of fixture blocks with metadata and light storage.
pub struct TestWorld {
    blocks: Blocks,
    ids: Vec<u8>,
    meta: Vec<u8>,
    sky: Vec<u8>,
    blocklight: Vec<u8>,
    pub grass_tint: i32,
}

const SIZE: i32 = 16;

impl TestWorld {
    pub fn new() -> TestWorld {
        let blocks = Blocks::new();
        let air_id = blocks.id("air") as u8;
        TestWorld {
            blocks,
            ids: vec![air_id; (SIZE * SIZE * SIZE) as usize],
            meta: vec![0; (SIZE * SIZE * SIZE) as usize],
            sky: vec![15; (SIZE * SIZE * SIZE) as usize],
            blocklight: vec![0; (SIZE * SIZE * SIZE) as usize],
            grass_tint: 0x91BD59,
        }
    }

    pub fn blocks(&self) -> &Blocks {
        &self.blocks
    }

    fn index(x: i32, y: i32, z: i32) -> usize {
        ((y * SIZE + z) * SIZE + x) as usize
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u8, meta: u8) {
        if x < 0 || y < 0 || z < 0 || x >= SIZE || y >= SIZE || z >= SIZE {
            return; // `World.setBlock` outside the world is a no-op, like vanilla
        }
        let index = Self::index(x, y, z);
        self.ids[index] = id;
        self.meta[index] = meta;
    }

    pub fn set_sky_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        if x < 0 || y < 0 || z < 0 || x >= SIZE || y >= SIZE || z >= SIZE {
            return;
        }
        self.sky[Self::index(x, y, z)] = light;
    }

    pub fn set_block_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        if x < 0 || y < 0 || z < 0 || x >= SIZE || y >= SIZE || z >= SIZE {
            return;
        }
        self.blocklight[Self::index(x, y, z)] = light;
    }

    pub fn metadata_at(&self, x: i32, y: i32, z: i32) -> u8 {
        self.meta[Self::index(x, y, z)]
    }

    fn in_world(x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < SIZE && y < SIZE && z < SIZE
    }

    fn block_at(&self, x: i32, y: i32, z: i32) -> &BlockDef {
        if !Self::in_world(x, y, z) {
            return &self.blocks.air;
        }
        let id = self.ids[Self::index(x, y, z)] as usize;
        match id {
            1 => &self.blocks.stone,
            2 => &self.blocks.grass,
            3 => &self.blocks.dirt,
            4 => &self.blocks.log,
            5 => &self.blocks.glass,
            6 => &self.blocks.torch,
            7 => &self.blocks.slab,
            8 => &self.blocks.stairs,
            9 => &self.blocks.fence,
            10 => &self.blocks.tallgrass,
            11 => &self.blocks.fern,
            12 => &self.blocks.water,
            13 => &self.blocks.snow,
            _ => &self.blocks.air,
        }
    }
}

impl WorldView for TestWorld {
    fn block(&self, x: i32, y: i32, z: i32) -> &BlockDef {
        self.block_at(x, y, z)
    }

    fn metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        if !Self::in_world(x, y, z) {
            return 0;
        }
        self.meta[Self::index(x, y, z)] as i32
    }

    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        // `World.getLight`: y < 0 clamps to 0; above the single chunk's
        // sections a loaded chunk reports the sky default.
        let y = if y < 0 { 0 } else { y };
        if y >= SIZE {
            return 15;
        }
        if !Self::in_world(x, y, z) {
            return 15;
        }
        self.sky[Self::index(x, y, z)] as i32
    }

    fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        let y = if y < 0 { 0 } else { y };
        if y >= SIZE {
            return 0;
        }
        if !Self::in_world(x, y, z) {
            return 0;
        }
        self.blocklight[Self::index(x, y, z)] as i32
    }

    fn biome_grass_tint(&self, _x: i32, _z: i32) -> i32 {
        self.grass_tint
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
        assert_eq!(world.block(-1, 0, 0).name, "air");
        assert_eq!(world.block(0, 100, 0).name, "air");
    }

    #[test]
    fn set_and_read_roundtrip() {
        let mut world = TestWorld::new();
        world.set(1, 2, 3, 1, 0);
        assert_eq!(world.block(1, 2, 3).name, "stone");
        assert_eq!(world.metadata(1, 2, 3), 0);
    }

    #[test]
    fn light_below_the_world_reads_the_bottom_section() {
        // `World.getBrightness` clamps y < 0 to y = 0.
        let mut world = TestWorld::new();
        world.set_sky_light(2, 0, 3, 7);
        assert_eq!(world.sky_light(2, -1, 3), 7);
    }
}
