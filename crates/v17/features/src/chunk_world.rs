//! One generated chunk as a `GenWorld`, for tests and probes.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

/// Blocks are stored column-major, like vanilla's `Block[]`:
/// `index = (x * 16 + z) * 256 + y`.
#[derive(Clone)]
pub struct ChunkWorld {
    pub origin_x: i32,
    pub origin_z: i32,
    blocks: Vec<Block>,
    metadata: Vec<u8>,
    biome_ids: Vec<i32>,
    /// Reads outside the chunk answer with this (stone, like generated
    /// terrain below the surface).
    pub out_of_chunk_block: Block,
}

impl ChunkWorld {
    pub fn new(origin_x: i32, origin_z: i32) -> ChunkWorld {
        ChunkWorld {
            origin_x,
            origin_z,
            blocks: vec![Block::Air; 65_536],
            metadata: vec![0; 65_536],
            biome_ids: vec![0; 256],
            out_of_chunk_block: Block::Stone,
        }
    }

    pub fn set_biome(&mut self, x: i32, z: i32, id: i32) {
        self.biome_ids[((x & 15) + (z & 15) * 16) as usize] = id;
    }

    fn in_chunk(&self, x: i32, z: i32) -> bool {
        x >= self.origin_x && x < self.origin_x + 16 && z >= self.origin_z && z < self.origin_z + 16
    }

    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        (((x - self.origin_x) * 16 + (z - self.origin_z)) * 256 + y) as usize
    }
}

impl GenWorld for ChunkWorld {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        if self.in_chunk(x, z) {
            self.blocks[self.index(x, y, z)]
        } else {
            self.out_of_chunk_block
        }
    }

    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_chunk(x, z) && (0..256).contains(&y) {
            self.metadata[self.index(x, y, z)]
        } else {
            0
        }
    }

    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
        _flags: u8,
    ) {
        // Vanilla writes into generated neighbour chunks; the probe world
        // holds one chunk, so out-of-chunk writes are dropped.
        if !self.in_chunk(x, z) || !(0..256).contains(&y) {
            return;
        }
        let index = self.index(x, y, z);
        let old = self.blocks[index];
        let old_metadata = self.metadata[index];
        if old == block && old_metadata == metadata {
            return;
        }
        // `WorldChunk.setBlockWithMetadataAt` orders these three steps: the
        // block goes in, the replaced block's `onRemoved` runs (still seeing
        // the old metadata), then the new metadata is written.
        self.blocks[index] = block;
        self.on_removed(x, y, z, old);
        self.metadata[index] = metadata;
    }

    fn get_biome_id(&self, x: i32, z: i32) -> i32 {
        self.biome_ids[((x & 15) + (z & 15) * 16) as usize]
    }

    fn get_height(&self, x: i32, z: i32) -> i32 {
        for y in (0..256).rev() {
            if self.get_block(x, y, z) != Block::Air {
                return y + 1;
            }
        }
        0
    }

    fn get_surface_height(&self, x: i32, z: i32) -> i32 {
        for y in (0..256).rev() {
            let block = self.get_block(x, y, z);
            if block.blocks_movement() && !matches!(block, Block::Leaves | Block::Leaves2) {
                return y + 1;
            }
        }
        -1
    }

    fn get_precipitation_height(&self, x: i32, z: i32) -> i32 {
        for y in (0..256).rev() {
            let block = self.get_block(x, y, z);
            if block.is_solid() || block.is_liquid() {
                return y + 1;
            }
        }
        0
    }

    fn has_sky(&self) -> bool {
        true
    }
}

impl ChunkWorld {
    /// The `onRemoved` hooks that write metadata: `AbstractLeavesBlock` flags
    /// its 3x3x3 leaf neighbours with the decay bit, `AbstractLogBlock` flags
    /// every still-unflagged leaf in its 9x9x9 box.
    fn on_removed(&mut self, x: i32, y: i32, z: i32, old: Block) {
        let (radius, only_unflagged) = match old {
            Block::Leaves | Block::Leaves2 => (1, false),
            Block::Log | Block::Log2 => (4, true),
            _ => return,
        };
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    let (bx, by, bz) = (x + dx, y + dy, z + dz);
                    if !self.in_chunk(bx, bz) || !(0..256).contains(&by) {
                        continue;
                    }
                    if !self.get_block(bx, by, bz).is_leaves() {
                        continue;
                    }
                    let metadata = self.get_block_metadata(bx, by, bz);
                    if only_unflagged && metadata & 8 != 0 {
                        continue;
                    }
                    let index = self.index(bx, by, bz);
                    self.metadata[index] = metadata | 8;
                }
            }
        }
    }

    /// Fill a flat stone floor from y 0 to `top` so features have terrain.
    pub fn fill(&mut self, from: i32, to: i32, block: Block) {
        for x in 0..16 {
            for z in 0..16 {
                for y in from..=to {
                    let index = ((x * 16 + z) * 256 + y) as usize;
                    self.blocks[index] = block;
                }
            }
        }
    }

    pub fn block_count(&self, block: Block) -> usize {
        self.blocks.iter().filter(|b| **b == block).count()
    }

    /// The whole chunk as `(x, y, z, block, metadata)` rows, y-major, for
    /// deterministic probe output.
    pub fn rows(&self) -> impl Iterator<Item = (i32, i32, i32, Block, u8)> + '_ {
        self.blocks
            .iter()
            .enumerate()
            .filter_map(move |(index, block)| {
                if *block == Block::Air {
                    return None;
                }
                let y = (index % 256) as i32;
                let column = index / 256;
                let x = self.origin_x + (column / 16) as i32;
                let z = self.origin_z + (column % 16) as i32;
                Some((x, y, z, *block, self.metadata[index]))
            })
    }
}

const _: () = {
    assert!(FLAG_NO_NEIGHBOR_NOTIFY == 2);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_reads_columns() {
        let mut world = ChunkWorld::new(4, 8);
        world.set_block_with_metadata(6, 30, 10, Block::Log, 2, FLAG_NO_NEIGHBOR_NOTIFY);
        assert_eq!(world.get_block(6, 30, 10), Block::Log);
        assert_eq!(world.get_block_metadata(6, 30, 10), 2);
        assert_eq!(world.get_block(6, 31, 10), Block::Air);
        assert_eq!(world.get_height(6, 10), 31);
    }

    #[test]
    fn out_of_chunk_reads_default_and_writes_drop() {
        let mut world = ChunkWorld::new(0, 0);
        assert_eq!(world.get_block(-1, 20, 0), Block::Stone);
        world.set_block_with_metadata(-1, 20, 0, Block::GoldOre, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        assert_eq!(world.get_block(-1, 20, 0), Block::Stone);
    }

    #[test]
    fn surface_and_precipitation_heights() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Stone);
        for x in 0..16 {
            for z in 0..16 {
                world.set_block_with_metadata(x, 41, z, Block::Water, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            }
        }
        assert_eq!(world.get_surface_height(3, 4), 41);
        assert_eq!(world.get_precipitation_height(3, 4), 42);
    }
}
