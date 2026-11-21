//! Blocks, materials, directions and inventories for the 1.7.10 redstone port.
//!
//! Ported from `net.minecraft.block.Block`, `Material`, `Facings`, `Directions`,
//! `ItemStack` and the inventory helpers the redstone subsystem touches.

/// Six-direction offsets, `net.minecraft.util.math.Directions`.
/// 0=DOWN, 1=UP, 2=NORTH(-Z), 3=SOUTH(+Z), 4=WEST(-X), 5=EAST(+X).
pub mod directions {
    pub const DOWN: i32 = 0;
    pub const UP: i32 = 1;
    pub const NORTH: i32 = 2;
    pub const SOUTH: i32 = 3;
    pub const WEST: i32 = 4;
    pub const EAST: i32 = 5;

    pub const OPPOSITE: [i32; 6] = [1, 0, 3, 2, 5, 4];
    pub const X_OFFSET: [i32; 6] = [0, 0, 0, 0, -1, 1];
    pub const Y_OFFSET: [i32; 6] = [-1, 1, 0, 0, 0, 0];
    pub const Z_OFFSET: [i32; 6] = [0, 0, -1, 1, 0, 0];
    pub const NAMES: [&str; 6] = ["DOWN", "UP", "NORTH", "SOUTH", "WEST", "EAST"];
}

/// Horizontal facings, `net.minecraft.util.math.Facings`.
/// 0=SOUTH(+Z), 1=WEST(-X), 2=NORTH(-Z), 3=EAST(+X).
pub mod facings {
    pub const SOUTH: i32 = 0;
    pub const WEST: i32 = 1;
    pub const NORTH: i32 = 2;
    pub const EAST: i32 = 3;

    pub const X_OFFSET: [i32; 4] = [0, -1, 0, 1];
    pub const Z_OFFSET: [i32; 4] = [1, 0, -1, 0];
    pub const NAMES: [&str; 4] = ["SOUTH", "WEST", "NORTH", "EAST"];
    /// Facing -> six-direction id (`TO_DIR`).
    pub const TO_DIR: [i32; 4] = [3, 4, 2, 5];
    pub const OPPOSITE: [i32; 4] = [2, 3, 0, 1];
    pub const CLOCKWISE: [i32; 4] = [1, 2, 3, 0];
    pub const ANTICLOCKWISE: [i32; 4] = [3, 0, 1, 2];
}

/// The subset of `Material` behavior redstone reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Material {
    /// `isSolid()` + `isSolidBlocking()`: AirMaterial/PlantMaterial/LiquidMaterial override both to false.
    pub solid_blocking: bool,
    /// `blocksMovement()`.
    pub blocks_movement: bool,
    /// `isOpaque()`.
    pub opaque: bool,
    /// `getPistonMoveBehavior()`: 0 none, 1 destroy, 2 block.
    pub piston_move_behavior: i32,
}

impl Material {
    pub const AIR: Material = Material {
        solid_blocking: false,
        blocks_movement: false,
        opaque: false,
        piston_move_behavior: 1,
    };
    pub const LIQUID: Material = Material {
        solid_blocking: false,
        blocks_movement: false,
        opaque: false,
        piston_move_behavior: 1,
    };
    pub const PLANT: Material = Material {
        solid_blocking: false,
        blocks_movement: false,
        opaque: false,
        piston_move_behavior: 1,
    };
    /// `DECORATION`: torches, wire, diodes.
    pub const DECORATION: Material = Material {
        solid_blocking: false,
        blocks_movement: false,
        opaque: false,
        piston_move_behavior: 1,
    };
    pub const NORMAL: Material = Material {
        solid_blocking: true,
        blocks_movement: true,
        opaque: true,
        piston_move_behavior: 0,
    };
    /// `GLASS`, `ICE`: translucent, `isSolidBlocking()` false, `blocksMovement()` true.
    pub const TRANSLUCENT: Material = Material {
        solid_blocking: false,
        blocks_movement: true,
        opaque: true,
        piston_move_behavior: 0,
    };
    /// `ANVIL`, `PORTAL`, `PISTON`: `setBlocksPistonMove()`.
    pub const BLOCKS_PISTON: Material = Material {
        solid_blocking: true,
        blocks_movement: true,
        opaque: true,
        piston_move_behavior: 2,
    };
    /// `LEAVES`: translucent but destroys on piston move.
    pub const LEAVES: Material = Material {
        solid_blocking: false,
        blocks_movement: true,
        opaque: true,
        piston_move_behavior: 1,
    };
    /// `REDSTONE_LAMP`: solid, not translucent, `MapColor.AIR`.
    pub const REDSTONE_LAMP: Material = Material {
        solid_blocking: true,
        blocks_movement: true,
        opaque: true,
        piston_move_behavior: 0,
    };
}

/// Blocks the redstone subsystem distinguishes. Everything the port's tests
/// build circuits from is a variant; anything else is `Solid` or `Other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Air,
    /// Generic full, solid, mineable, non-opaque-irrelevant block (stone, dirt...).
    Solid,
    /// A solid block that blocks pistons' `canMoveBlock` via mining time (bedrock).
    Bedrock,
    Obsidian,
    RedstoneWire,
    RedstoneTorch,
    UnlitRedstoneTorch,
    Repeater,
    PoweredRepeater,
    Comparator,
    PoweredComparator,
    Piston,
    StickyPiston,
    PistonHead,
    MovingPiston,
    Hopper,
    Dispenser,
    Dropper,
    Lever,
    RedstoneBlock,
    RedstoneLamp,
    LitRedstoneLamp,
    /// Glowstone: wire can sit on it without a solid top.
    Glowstone,
    /// Any other block id, with the id it had.
    Other(u16),
    /// A non-full-cube plant-like block (grass, flowers): piston destroys it.
    Plant,
    /// Water/lava.
    Liquid,
}

impl Block {
    /// `Block.isSolid()`: material solid-blocking, a cube, and not a signal source.
    pub fn is_solid(self) -> bool {
        !self.is_signal_source() && self.material().solid_blocking && self.is_cube()
    }

    /// `Material.isSolidBlocking()` on this block's material.
    pub fn is_solid_blocking(self) -> bool {
        self.material().solid_blocking
    }

    /// `Block.isCube()`: full 1x1x1 block.
    pub fn is_cube(self) -> bool {
        !matches!(
            self,
            Block::Air
                | Block::RedstoneWire
                | Block::RedstoneTorch
                | Block::UnlitRedstoneTorch
                | Block::Repeater
                | Block::PoweredRepeater
                | Block::Comparator
                | Block::PoweredComparator
                | Block::Piston
                | Block::StickyPiston
                | Block::PistonHead
                | Block::MovingPiston
                | Block::Hopper
                | Block::Lever
                | Block::Plant
                | Block::Liquid
        )
    }

    /// `Block.isSignalSource()`.
    pub fn is_signal_source(self) -> bool {
        matches!(
            self,
            Block::RedstoneWire
                | Block::RedstoneTorch
                | Block::UnlitRedstoneTorch
                | Block::Repeater
                | Block::PoweredRepeater
                | Block::Comparator
                | Block::PoweredComparator
                | Block::RedstoneBlock
                | Block::Lever
        )
    }

    /// `Block.isAnalogSignalSource()`: containers only in this port.
    pub fn is_analog_signal_source(self) -> bool {
        matches!(self, Block::Hopper | Block::Dispenser | Block::Dropper)
    }

    /// `block instanceof BlockEntityProvider` (`hasBlockEntity`).
    pub fn has_block_entity(self) -> bool {
        matches!(
            self,
            Block::Hopper
                | Block::Dispenser
                | Block::Dropper
                | Block::Comparator
                | Block::PoweredComparator
                | Block::MovingPiston
        )
    }

    /// `Block.isOpaque()` (`opaqueCube`, which `Block`'s constructor takes from
    /// `isSolidRender()`): the decoration and machine blocks that override
    /// `isSolidRender` to false do not count as opaque for a dispenser's facing.
    pub fn is_opaque(self) -> bool {
        !matches!(
            self,
            Block::Air
                | Block::Plant
                | Block::Liquid
                | Block::RedstoneWire
                | Block::RedstoneTorch
                | Block::UnlitRedstoneTorch
                | Block::Repeater
                | Block::PoweredRepeater
                | Block::Comparator
                | Block::PoweredComparator
                | Block::Piston
                | Block::StickyPiston
                | Block::PistonHead
                | Block::MovingPiston
                | Block::Hopper
                | Block::Lever
        )
    }

    /// `Block.getMaterial()`.
    pub fn material(self) -> Material {
        match self {
            Block::Air => Material::AIR,
            Block::Plant => Material::PLANT,
            Block::Liquid => Material::LIQUID,
            Block::RedstoneWire
            | Block::RedstoneTorch
            | Block::UnlitRedstoneTorch
            | Block::Repeater
            | Block::PoweredRepeater
            | Block::Comparator
            | Block::PoweredComparator => Material::DECORATION,
            Block::Piston | Block::StickyPiston | Block::PistonHead | Block::MovingPiston => {
                Material::BLOCKS_PISTON
            }
            Block::Hopper => Material::NORMAL,
            Block::Lever => Material::DECORATION,
            Block::RedstoneLamp | Block::LitRedstoneLamp => Material::REDSTONE_LAMP,
            // `GlowstoneBlock` is `Material.GLASS`: no solid top, no conduction.
            Block::Glowstone => Material::TRANSLUCENT,
            Block::Obsidian | Block::Bedrock => Material::BLOCKS_PISTON,
            Block::Other(_) => Material::NORMAL,
            Block::Solid | Block::Dispenser | Block::Dropper | Block::RedstoneBlock => {
                Material::NORMAL
            }
        }
    }

    /// `Block.getMiningTime(...) == -1.0F`: unbreakable blocks block pistons.
    pub fn is_unbreakable(self) -> bool {
        matches!(self, Block::Bedrock | Block::MovingPiston)
    }

    /// `Block.getPistonMoveBehavior()` (`Material.getPistonMoveBehavior`).
    pub fn piston_move_behavior(self) -> i32 {
        self.material().piston_move_behavior
    }

    pub fn id(self) -> u16 {
        match self {
            Block::Air => 0,
            Block::Solid => 1,
            Block::Bedrock => 7,
            Block::Obsidian => 49,
            Block::RedstoneWire => 55,
            Block::RedstoneTorch => 76,
            Block::UnlitRedstoneTorch => 75,
            Block::Repeater => 93,
            Block::PoweredRepeater => 94,
            Block::Comparator => 149,
            Block::PoweredComparator => 150,
            Block::Piston => 33,
            Block::StickyPiston => 29,
            Block::PistonHead => 34,
            Block::MovingPiston => 36,
            Block::Hopper => 154,
            Block::Dispenser => 23,
            Block::Dropper => 158,
            Block::Lever => 69,
            Block::RedstoneBlock => 152,
            Block::RedstoneLamp => 123,
            Block::LitRedstoneLamp => 124,
            Block::Glowstone => 89,
            Block::Plant => 31,
            Block::Liquid => 9,
            Block::Other(id) => id,
        }
    }

    /// The comparator input a lever reads through `isSameDiode` checks.
    pub fn is_diode(self) -> bool {
        is_repeater(self) || is_comparator(self)
    }
}

/// `DiodeBlock.isDiode`: either repeater state or either comparator state.
pub fn is_repeater(block: Block) -> bool {
    matches!(block, Block::Repeater | Block::PoweredRepeater)
}

pub fn is_comparator(block: Block) -> bool {
    matches!(block, Block::Comparator | Block::PoweredComparator)
}
