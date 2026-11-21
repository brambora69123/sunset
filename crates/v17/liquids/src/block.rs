//! The block facts the 1.7.10 liquid rules read, and the metadata helpers.

/// `net.minecraft.block.material.Material`, as far as liquids look at it.
///
/// Only the liquid distinction, air, and the portal rule need their own
/// variant; everything else shares [`Material::Other`] and carries its
/// `isSolid`, `blocksMovement` and `isFlammable` answers as fields of
/// [`Block`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Material {
    Air,
    Water,
    Lava,
    Portal,
    Other,
}

impl Material {
    /// `Material.isLiquid`.
    pub fn is_liquid(self) -> bool {
        matches!(self, Material::Water | Material::Lava)
    }
}

/// A block id with the facts the liquid classes ask of `Block`.
///
/// The world supplies these in [`crate::LiquidWorld::get_block`]; the flow
/// rules never look a block up anywhere else. `id` is the 1.7.10 numeric id,
/// which is what `Block.getId`, `Block.byId` and `Block.areEqual` compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub id: u16,
    /// `Block.getMaterial`.
    pub material: Material,
    /// `Material.isSolid`.
    pub solid: bool,
    /// `Material.blocksMovement`.
    pub blocks_movement: bool,
    /// `Material.isFlammable`, which lava's fire spreading reads.
    pub flammable: bool,
}

impl Block {
    pub const fn new(
        id: u16,
        material: Material,
        solid: bool,
        blocks_movement: bool,
        flammable: bool,
    ) -> Self {
        Block {
            id,
            material,
            solid,
            blocks_movement,
            flammable,
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        AIR
    }
}

/// `Blocks.AIR`'s id.
pub const AIR_ID: u16 = 0;
/// `Blocks.STONE`'s id.
pub const STONE_ID: u16 = 1;
/// `Blocks.COBBLESTONE`'s id.
pub const COBBLESTONE_ID: u16 = 4;
/// `Blocks.FLOWING_WATER`'s id.
pub const FLOWING_WATER_ID: u16 = 8;
/// `Blocks.WATER`'s id.
pub const WATER_ID: u16 = 9;
/// `Blocks.FLOWING_LAVA`'s id.
pub const FLOWING_LAVA_ID: u16 = 10;
/// `Blocks.LAVA`'s id.
pub const LAVA_ID: u16 = 11;
/// `Blocks.OBSIDIAN`'s id.
pub const OBSIDIAN_ID: u16 = 49;
/// `Blocks.FIRE`'s id.
pub const FIRE_ID: u16 = 51;

/// `Blocks.AIR`.
pub const AIR: Block = Block::new(AIR_ID, Material::Air, false, false, false);
/// `Blocks.STONE`.
pub const STONE: Block = Block::new(STONE_ID, Material::Other, true, true, false);
/// `Blocks.COBBLESTONE`.
pub const COBBLESTONE: Block = Block::new(COBBLESTONE_ID, Material::Other, true, true, false);
/// `Blocks.OBSIDIAN`.
pub const OBSIDIAN: Block = Block::new(OBSIDIAN_ID, Material::Other, true, true, false);
/// `Blocks.FIRE`, which lava's source tick places.
pub const FIRE: Block = Block::new(FIRE_ID, Material::Other, false, false, false);
/// `Blocks.FLOWING_WATER`: the flowing water block class.
pub const FLOWING_WATER: Block = Block::new(FLOWING_WATER_ID, Material::Water, false, false, false);
/// `Blocks.WATER`: the still water block class.
pub const WATER: Block = Block::new(WATER_ID, Material::Water, false, false, false);
/// `Blocks.FLOWING_LAVA`: the flowing lava block class.
pub const FLOWING_LAVA: Block = Block::new(FLOWING_LAVA_ID, Material::Lava, false, false, false);
/// `Blocks.LAVA`: the still lava block class.
pub const LAVA: Block = Block::new(LAVA_ID, Material::Lava, false, false, false);

/// `Blocks.WOODEN_DOOR`.
pub const WOODEN_DOOR: u16 = 64;
/// `Blocks.IRON_DOOR`.
pub const IRON_DOOR: u16 = 71;
/// `Blocks.STANDING_SIGN`.
pub const STANDING_SIGN: u16 = 63;
/// `Blocks.LADDER`.
pub const LADDER: u16 = 65;
/// `Blocks.REEDS`.
pub const REEDS: u16 = 83;

/// `LiquidBlock.getHeightLoss`: the water surface drop for a metadata level.
pub fn height_loss(state: i32) -> f32 {
    let state = if state >= 8 { 0 } else { state };
    (state + 1) as f32 / 9.0
}

/// `LiquidBlock.getLiquidDepth`'s metadata half: the falling bit reads as
/// level 0.
pub fn liquid_depth(metadata: i32) -> i32 {
    if metadata >= 8 { 0 } else { metadata }
}

/// `Block.byId(Block.getId(flowing) + 1)`: the still block of a flowing block.
pub fn source_of(block: Block) -> Block {
    match block.id {
        FLOWING_WATER_ID => WATER,
        FLOWING_LAVA_ID => LAVA,
        _ => block,
    }
}

/// `Block.byId(Block.getId(source) - 1)`: the flowing block of a still block.
pub fn flowing_of(block: Block) -> Block {
    match block.id {
        WATER_ID => FLOWING_WATER,
        LAVA_ID => FLOWING_LAVA,
        _ => block,
    }
}
