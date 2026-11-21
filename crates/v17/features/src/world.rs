//! The world surface features generate into: reads and block writes.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::biome;
use crate::block::Block;
use sunset_1_7_core::RandomSource;

/// Placement flags, as vanilla's `setBlock` layer flags.
pub const FLAG_NO_NEIGHBOR_NOTIFY: u8 = 2;
pub const FLAG_NOTIFY: u8 = 3;
/// Neither neighbours nor clients: vanilla's plain `4`.
pub const FLAG_SILENT: u8 = 4;

/// A world during generation. Reads outside the generating area answer with
/// the implementation's default (the game reads generated neighbour chunks;
/// probes back one chunk and read stone there).
pub trait GenWorld {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block;
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> u8;
    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
        flags: u8,
    );
    fn get_biome_id(&self, x: i32, z: i32) -> i32;
    /// `World.getHeight`: highest non-air block + 1.
    fn get_height(&self, x: i32, z: i32) -> i32;
    /// `World.getSurfaceHeight`: highest blocks-movement non-leaves block + 1.
    fn get_surface_height(&self, x: i32, z: i32) -> i32;
    /// `World.getPrecipitationHeight`: highest solid-or-liquid block + 1.
    fn get_precipitation_height(&self, x: i32, z: i32) -> i32;
    /// Block light at a position; 0 during chunk population.
    fn get_block_light(&self, _x: i32, _y: i32, _z: i32) -> i32 {
        0
    }
    /// Sky light at a position. During population the light engine has only
    /// sky light, and a column is lit down to its height map, so above the
    /// height map it is 15 and below it 0 (no propagation into caves), which
    /// is what the vanilla checks during generation see.
    fn get_sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        if y >= self.get_height(x, z) { 15 } else { 0 }
    }
    /// `World.getActualLight`: the brighter of block and sky light.
    fn get_actual_light(&self, x: i32, y: i32, z: i32) -> i32 {
        self.get_block_light(x, y, z)
            .max(self.get_sky_light(x, y, z))
    }
    /// `world.dimension.noSky` is false for the overworld.
    fn has_sky(&self) -> bool {
        true
    }

    fn is_air(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y, z) == Block::Air
    }

    /// `Biome.getTemperature`: flat below y 65; the altitude term above. The
    /// `TEMPERATURE_NOISE` term is omitted (it only cools very high columns).
    fn get_temperature(&self, x: i32, y: i32, z: i32) -> f32 {
        let base = biome::biome(self.get_biome_id(x, z)).temperature;
        if y > 64 {
            base - (y as f32 - 64.0) * 0.05 / 30.0
        } else {
            base
        }
    }

    /// `World.canFreeze(x, y, z)`: the populate call passes
    /// `needsAdjacentNonWaterBlock = false`. Block light is 0 during
    /// population, so the `< 10` guard passes.
    fn can_freeze(&self, x: i32, y: i32, z: i32) -> bool {
        if self.get_temperature(x, y, z) > 0.15 {
            return false;
        }
        if (0..256).contains(&y) && self.get_block_light(x, y, z) < 10 {
            let block = self.get_block(x, y, z);
            if (block == Block::Water || block == Block::FlowingWater)
                && self.get_block_metadata(x, y, z) == 0
            {
                return true;
            }
        }
        false
    }

    /// `World.canSnowFall(x, y, z, true)`. Block light is 0 during population;
    /// the snow layer's own placement check (`SnowLayerBlock.canBePlaced`)
    /// wants air with a solid block below, leaves, or a full snow layer.
    fn can_snow(&self, x: i32, y: i32, z: i32) -> bool {
        if self.get_temperature(x, y, z) > 0.15 {
            return false;
        }
        if (0..256).contains(&y) && self.get_block_light(x, y, z) < 10 {
            let below = self.get_block(x, y - 1, z);
            let can_place = if matches!(below, Block::Ice | Block::PackedIce) {
                false
            } else if below.is_leaves() {
                true
            } else if below == Block::SnowLayer {
                self.get_block_metadata(x, y - 1, z) & 7 == 7
            } else {
                below.is_opaque() && below.blocks_movement()
            };
            if self.get_block(x, y, z).is_air_material() && can_place {
                return true;
            }
        }
        false
    }

    /// `FlowerBlock.canSurvive` / `PlantBlock.canSurvive` for the plant about
    /// to be placed: flowers stand on grass or dirt, a mushroom also on
    /// mycelium or podzol and on any opaque block in light below 13, and a
    /// dead bush on sand or the mesa clays.
    fn plant_can_survive(&self, plant: Block, x: i32, y: i32, z: i32) -> bool {
        let below = self.get_block(x, y - 1, z);
        match plant {
            Block::BrownMushroom | Block::RedMushroom => {
                below == Block::Mycelium
                    || (below == Block::Dirt && self.get_block_metadata(x, y - 1, z) == 2)
                    || (self.get_actual_light(x, y, z) < 13 && below.is_opaque())
            }
            Block::DeadBush => matches!(
                below,
                Block::Sand | Block::HardenedClay | Block::StainedHardenedClay | Block::Dirt
            ),
            _ => below.plant_can_survive(),
        }
    }

    /// `Blocks.CACTUS.canSurvive`: cactus stands on sand or cactus, with no
    /// solid block beside it.
    fn cactus_can_survive(&self, x: i32, y: i32, z: i32) -> bool {
        let block = self.get_block(x, y, z);
        if !block.is_air_material() {
            return false;
        }
        let below = self.get_block(x, y - 1, z);
        if below != Block::Sand && below != Block::Cactus {
            return false;
        }
        self.get_block(x - 1, y, z).blocks_movement()
            || self.get_block(x + 1, y, z).blocks_movement()
            || self.get_block(x, y, z - 1).blocks_movement()
            || self.get_block(x, y, z + 1).blocks_movement()
    }

    /// `Blocks.REEDS.canSurvive`: reeds stand on reeds, or on grass/dirt with
    /// water beside the base.
    fn reeds_can_survive(&self, x: i32, y: i32, z: i32) -> bool {
        let block = self.get_block(x, y, z);
        if !block.is_air_material() {
            return false;
        }
        let below = self.get_block(x, y - 1, z);
        if below == Block::Reeds {
            return true;
        }
        if !matches!(below, Block::Grass | Block::Dirt) {
            return false;
        }
        self.get_block(x - 1, y - 1, z).is_liquid()
            || self.get_block(x + 1, y - 1, z).is_liquid()
            || self.get_block(x, y - 1, z - 1).is_liquid()
            || self.get_block(x, y - 1, z + 1).is_liquid()
    }

    /// `Blocks.LILY_PAD.canBePlaced`: a lily pad floats on still water.
    fn lily_pad_can_be_placed(&self, x: i32, y: i32, z: i32) -> bool {
        self.get_block(x, y - 1, z) == Block::Water && self.get_block_metadata(x, y - 1, z) == 0
    }
}

/// Convenience for the many `random.nextInt(bound)` draws; every call site in
/// the feature code has a positive bound.
pub(crate) fn next_int<R: RandomSource>(random: &mut R, bound: i32) -> i32 {
    random
        .next_int_bound(bound)
        .expect("feature code always draws with a positive bound")
}

/// `WorldChunk`'s height map, which `World.getHeight` reads: one above the
/// highest block whose `Block.getOpacity` is nonzero. Vanilla updates it from
/// `setBlockWithMetadataAt` on every write, so plants and lava never raise it
/// while leaves and water do. A `GenWorld` has no such map — its `get_height`
/// scans for the highest non-air block — so the decorator runs on this view,
/// which derives a column on first use and then keeps vanilla's update rule.
pub struct HeightMapWorld<'a> {
    inner: &'a mut dyn GenWorld,
    heights: RefCell<HashMap<(i32, i32), i32>>,
}

impl<'a> HeightMapWorld<'a> {
    pub fn new(inner: &'a mut dyn GenWorld) -> HeightMapWorld<'a> {
        HeightMapWorld {
            inner,
            heights: RefCell::new(HashMap::new()),
        }
    }

    /// The column's current height map value.
    fn height(&self, x: i32, z: i32) -> i32 {
        if let Some(height) = self.heights.borrow().get(&(x, z)) {
            return *height;
        }
        let height = self.column_height(x, z);
        self.heights.borrow_mut().insert((x, z), height);
        height
    }

    fn set_height(&self, x: i32, z: i32, height: i32) {
        self.heights.borrow_mut().insert((x, z), height);
    }

    /// `populateHeightMap`: one above the highest nonzero-opacity block.
    fn column_height(&self, x: i32, z: i32) -> i32 {
        (1..=256)
            .rev()
            .find(|&y| self.inner.get_block(x, y - 1, z).height_opacity() != 0)
            .unwrap_or(0)
    }

    /// `updateHeightMap`: the downward scan vanilla runs from `y`.
    fn rescan(&self, x: i32, y: i32, z: i32) -> i32 {
        let mut height = y;
        while height > 0 && self.inner.get_block(x, height - 1, z).height_opacity() == 0 {
            height -= 1;
        }
        height
    }
}

impl GenWorld for HeightMapWorld<'_> {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        self.inner.get_block(x, y, z)
    }

    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> u8 {
        self.inner.get_block_metadata(x, y, z)
    }

    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
        flags: u8,
    ) {
        // `World.setBlockWithMetadata` refuses heights outside the chunk, so
        // the height map never sees them.
        if !(0..256).contains(&y) {
            return;
        }
        let old = self.inner.get_block(x, y, z);
        let old_metadata = self.inner.get_block_metadata(x, y, z);
        self.inner
            .set_block_with_metadata(x, y, z, block, metadata, flags);
        // `setBlockWithMetadataAt` returns before touching the height map
        // when the block and its metadata are unchanged.
        if old == block && old_metadata == metadata {
            return;
        }
        let height = self.height(x, z);
        if block.height_opacity() > 0 {
            if y >= height {
                self.set_height(x, z, self.rescan(x, y + 1, z));
            }
        } else if y == height - 1 {
            self.set_height(x, z, self.rescan(x, y + 1, z));
        }
    }

    fn get_biome_id(&self, x: i32, z: i32) -> i32 {
        self.inner.get_biome_id(x, z)
    }

    fn get_height(&self, x: i32, z: i32) -> i32 {
        self.height(x, z)
    }

    fn get_surface_height(&self, x: i32, z: i32) -> i32 {
        self.inner.get_surface_height(x, z)
    }

    fn get_precipitation_height(&self, x: i32, z: i32) -> i32 {
        self.inner.get_precipitation_height(x, z)
    }

    fn get_block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        self.inner.get_block_light(x, y, z)
    }

    fn has_sky(&self) -> bool {
        self.inner.has_sky()
    }
}
