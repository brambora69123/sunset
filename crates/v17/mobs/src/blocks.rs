//! A minimal solid/liquid block model, just enough for 1.7.10 pathfinding and spawning.
//!
//! Only the properties `PathFinder.getBlockingType`, `PathNavigation` and
//! `NaturalSpawner.isValidSpawnPos` consult are modeled; real block tables live in
//! the blocks crate. Every block has a [`Material`], and a handful carry the
//! special flags the 1.7.10 pathfinder tests (`Blocks.WATER`, `TRAPDOOR`,
//! `WOODEN_DOOR`, `FENCE_GATE`, bedrock, and `canWalkThrough`).

use self::Material::*;

/// `net.minecraft.block.material.Material`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Material {
    Air,
    Grass,
    Ground,
    Wood,
    Rock,
    Water,
    Lava,
    Leaves,
    Plant,
    ReplaceablePlant,
    Sponge,
    Cloth,
    Fire,
    Sand,
    Circuits,
    Glass,
    RedstoneLight,
    TNT,
    Ice,
    Snow,
    CraftedSnow,
    Cactus,
    Clay,
    Vegetable,
    Portal,
    Cake,
    Piston,
    IceSolid,
}

impl Material {
    /// `Material.isLiquid()`.
    pub fn is_liquid(self) -> bool {
        matches!(self, Water | Lava)
    }

    /// `Material.isSolid()`.
    pub fn is_solid(self) -> bool {
        !matches!(
            self,
            Air | Fire | Water | Lava | Plant | ReplaceablePlant | Snow | Circuits
        )
    }

    /// `Material.blocksMovement()`.
    pub fn blocks_movement(self) -> bool {
        !matches!(
            self,
            Air | Fire | Water | Lava | Plant | ReplaceablePlant | Snow | Circuits
        )
    }
}

/// `net.minecraft.block.Block`, reduced to the surface the AI reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Block {
    /// The block's material.
    pub material: Material,
    /// `Block.getRenderType()`: 9 is liquid render, 11/32 are special crosses.
    pub render_type: i32,
    /// `Block.canWalkThrough(world, x, y, z)` — passable without blocking movement.
    pub can_walk_through: bool,
    /// Identifies `Blocks.WATER`/`Blocks.FLOWING_WATER`.
    pub is_water: bool,
    /// `Blocks.TRAPDOOR`.
    pub is_trapdoor: bool,
    /// `Blocks.WOODEN_DOOR`.
    pub is_wooden_door: bool,
    /// `Blocks.FENCE_GATE`.
    pub is_fence_gate: bool,
    /// `Blocks.BEDROCK`.
    pub is_bedrock: bool,
    /// `Blocks.GRASS`.
    pub is_grass: bool,
    /// The block's id, for spawning tables and debugging.
    pub id: u16,
}

impl Block {
    /// A block with the given material and default pathfinding flags.
    pub const fn new(id: u16, material: Material) -> Self {
        Block {
            material,
            render_type: 0,
            can_walk_through: false,
            is_water: false,
            is_trapdoor: false,
            is_wooden_door: false,
            is_fence_gate: false,
            is_bedrock: false,
            is_grass: false,
            id,
        }
    }

    /// `Blocks.AIR`.
    pub const AIR: Block = Block::new(0, Air);
    /// `Blocks.STONE`.
    pub const STONE: Block = Block::new(1, Rock);
    /// `Blocks.GRASS`.
    pub const GRASS: Block = Block {
        is_grass: true,
        id: 2,
        ..Block::new(2, Grass)
    };
    /// `Blocks.DIRT`.
    pub const DIRT: Block = Block::new(3, Ground);
    /// `Blocks.WATER` — still water, render type 9.
    pub const WATER: Block = Block {
        render_type: 9,
        can_walk_through: true,
        is_water: true,
        id: 9,
        ..Block::new(9, Water)
    };
    /// `Blocks.FLOWING_WATER`, same surface as still water.
    pub const FLOWING_WATER: Block = Block {
        render_type: 9,
        can_walk_through: true,
        is_water: true,
        id: 10,
        ..Block::new(10, Water)
    };
    /// `Blocks.LAVA`.
    pub const LAVA: Block = Block {
        render_type: 9,
        can_walk_through: true,
        id: 11,
        ..Block::new(11, Lava)
    };
    /// `Blocks.BEDROCK`.
    pub const BEDROCK: Block = Block {
        is_bedrock: true,
        id: 7,
        ..Block::new(7, Rock)
    };
    /// `Blocks.SAND`.
    pub const SAND: Block = Block::new(12, Sand);
    /// `Blocks.WOODEN_DOOR` (the block part).
    pub const WOODEN_DOOR: Block = Block {
        is_wooden_door: true,
        id: 64,
        ..Block::new(64, Wood)
    };
    /// `Blocks.TRAPDOOR`.
    pub const TRAPDOOR: Block = Block {
        is_trapdoor: true,
        id: 96,
        ..Block::new(96, Wood)
    };
    /// `Blocks.FENCE_GATE`.
    pub const FENCE_GATE: Block = Block {
        is_fence_gate: true,
        id: 107,
        ..Block::new(107, Wood)
    };
    /// `Blocks.TALL_GRASS` — a replaceable plant.
    pub const TALL_GRASS: Block = Block {
        can_walk_through: true,
        id: 31,
        ..Block::new(31, ReplaceablePlant)
    };
    /// `Blocks.GLASS`.
    pub const GLASS: Block = Block::new(20, Glass);

    /// `Block.getMaterial()`.
    pub fn get_material(&self) -> Material {
        self.material
    }

    /// `Block.getRenderType()`.
    pub fn get_render_type(&self) -> i32 {
        self.render_type
    }

    /// `Block.canWalkThrough(world, x, y, z)`.
    pub fn can_walk_through(&self) -> bool {
        self.can_walk_through
    }
}

/// The handful of real 1.7.10 blocks the AI distinguishes, by id.
///
/// Unknown ids fall back to a plain solid rock block, matching how a
/// pathfinding probe over a synthetic world only needs solid/air/liquid.
pub fn block_by_id(id: u16) -> Block {
    match id {
        0 => Block::AIR,
        1 => Block::STONE,
        2 => Block::GRASS,
        3 => Block::DIRT,
        7 => Block::BEDROCK,
        8 | 9 => Block::WATER,
        10 | 11 => Block::LAVA,
        12 => Block::SAND,
        20 => Block::GLASS,
        31 => Block::TALL_GRASS,
        64 => Block::WOODEN_DOOR,
        96 => Block::TRAPDOOR,
        107 => Block::FENCE_GATE,
        _ => Block::STONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquids_are_liquid_and_not_solid() {
        assert!(Block::WATER.get_material().is_liquid());
        assert!(Block::LAVA.get_material().is_liquid());
        assert!(!Block::WATER.get_material().is_solid());
        assert!(!Block::AIR.get_material().is_solid());
        assert!(Block::STONE.get_material().is_solid());
    }

    #[test]
    fn air_blocks_no_movement() {
        assert!(!Material::Air.blocks_movement());
        assert!(Material::Rock.blocks_movement());
        assert!(Material::Glass.blocks_movement());
    }
}
