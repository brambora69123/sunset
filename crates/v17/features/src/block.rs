//! Blocks the features touch. Discriminants are the 1.7.10 block ids.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum Block {
    Air = 0,
    Stone = 1,
    Grass = 2,
    Dirt = 3,
    Cobblestone = 4,
    Planks = 5,
    Sapling = 6,
    Bedrock = 7,
    FlowingWater = 8,
    Water = 9,
    FlowingLava = 10,
    Lava = 11,
    Sand = 12,
    Gravel = 13,
    GoldOre = 14,
    IronOre = 15,
    CoalOre = 16,
    Log = 17,
    Leaves = 18,
    Glass = 20,
    LapisOre = 21,
    Sandstone = 24,
    TallGrass = 31,
    DeadBush = 32,
    YellowFlower = 37,
    RedFlower = 38,
    BrownMushroom = 39,
    RedMushroom = 40,
    StoneSlab = 44,
    MossyCobblestone = 48,
    Obsidian = 49,
    Torch = 50,
    MobSpawner = 52,
    Chest = 54,
    DiamondOre = 56,
    Farmland = 60,
    RedstoneOre = 73,
    SnowLayer = 78,
    Ice = 79,
    SnowBlock = 80,
    Cactus = 81,
    Clay = 82,
    Reeds = 83,
    Pumpkin = 86,
    Netherrack = 87,
    MonsterEgg = 97,
    BrownMushroomBlock = 99,
    RedMushroomBlock = 100,
    MelonBlock = 103,
    Vine = 106,
    Mycelium = 110,
    LilyPad = 111,
    Cocoa = 127,
    EmeraldOre = 129,
    StainedHardenedClay = 159,
    Leaves2 = 161,
    Log2 = 162,
    HardenedClay = 172,
    PackedIce = 174,
    DoublePlant = 175,
}

impl Block {
    pub fn from_id(id: u8) -> Block {
        match id {
            1 => Block::Stone,
            2 => Block::Grass,
            3 => Block::Dirt,
            4 => Block::Cobblestone,
            5 => Block::Planks,
            6 => Block::Sapling,
            7 => Block::Bedrock,
            8 => Block::FlowingWater,
            9 => Block::Water,
            10 => Block::FlowingLava,
            11 => Block::Lava,
            12 => Block::Sand,
            13 => Block::Gravel,
            14 => Block::GoldOre,
            15 => Block::IronOre,
            16 => Block::CoalOre,
            17 => Block::Log,
            18 => Block::Leaves,
            20 => Block::Glass,
            21 => Block::LapisOre,
            24 => Block::Sandstone,
            31 => Block::TallGrass,
            32 => Block::DeadBush,
            37 => Block::YellowFlower,
            38 => Block::RedFlower,
            39 => Block::BrownMushroom,
            40 => Block::RedMushroom,
            44 => Block::StoneSlab,
            48 => Block::MossyCobblestone,
            49 => Block::Obsidian,
            50 => Block::Torch,
            52 => Block::MobSpawner,
            54 => Block::Chest,
            56 => Block::DiamondOre,
            60 => Block::Farmland,
            73 => Block::RedstoneOre,
            78 => Block::SnowLayer,
            79 => Block::Ice,
            80 => Block::SnowBlock,
            81 => Block::Cactus,
            82 => Block::Clay,
            83 => Block::Reeds,
            86 => Block::Pumpkin,
            87 => Block::Netherrack,
            97 => Block::MonsterEgg,
            99 => Block::BrownMushroomBlock,
            100 => Block::RedMushroomBlock,
            103 => Block::MelonBlock,
            106 => Block::Vine,
            110 => Block::Mycelium,
            111 => Block::LilyPad,
            127 => Block::Cocoa,
            129 => Block::EmeraldOre,
            159 => Block::StainedHardenedClay,
            161 => Block::Leaves2,
            162 => Block::Log2,
            172 => Block::HardenedClay,
            174 => Block::PackedIce,
            175 => Block::DoublePlant,
            _ => Block::Air,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }

    /// Material.AIR.
    pub fn is_air_material(self) -> bool {
        self == Block::Air
    }

    /// `Material.LEAVES`: both leaf blocks.
    pub fn is_leaves(self) -> bool {
        matches!(self, Block::Leaves | Block::Leaves2)
    }

    /// Material.isLiquid.
    pub fn is_liquid(self) -> bool {
        matches!(
            self,
            Block::Water | Block::FlowingWater | Block::Lava | Block::FlowingLava
        )
    }

    /// `Material.isSolid`. Leaves, ice, glass and plants are not solid materials.
    pub fn is_solid(self) -> bool {
        matches!(
            self,
            Block::Stone
                | Block::Grass
                | Block::Dirt
                | Block::Cobblestone
                | Block::Planks
                | Block::Bedrock
                | Block::Sand
                | Block::Gravel
                | Block::GoldOre
                | Block::IronOre
                | Block::CoalOre
                | Block::Log
                | Block::Log2
                | Block::LapisOre
                | Block::Sandstone
                | Block::MossyCobblestone
                | Block::Obsidian
                | Block::Clay
                | Block::Mycelium
                | Block::Farmland
                | Block::Netherrack
                | Block::SnowBlock
                | Block::DiamondOre
                | Block::RedstoneOre
                | Block::MobSpawner
                | Block::Chest
                | Block::MonsterEgg
                | Block::MelonBlock
                | Block::StoneSlab
                | Block::EmeraldOre
                | Block::StainedHardenedClay
                | Block::HardenedClay
                | Block::PackedIce
        )
    }

    /// `Material.blocksMovement`. Air, liquids, plants and thin blocks do not.
    pub fn blocks_movement(self) -> bool {
        self.is_solid()
            || matches!(
                self,
                Block::Leaves | Block::Leaves2 | Block::Ice | Block::Glass
            )
    }

    /// `Material.isReplaceable`: what a feature may write straight through.
    pub fn is_replaceable(self) -> bool {
        self.is_air_material()
            || matches!(
                self,
                Block::TallGrass | Block::DeadBush | Block::Vine | Block::SnowLayer
            )
            || self.is_liquid()
    }

    /// `isOpaqueCube`: leaves, ice, glass and thin blocks are not opaque.
    pub fn is_opaque(self) -> bool {
        self.is_solid()
            || matches!(
                self,
                Block::SnowBlock | Block::Sandstone | Block::Netherrack
            )
    }

    /// `AbstractTreeFeature.canReplace`.
    pub fn tree_can_replace(self) -> bool {
        self.is_air_material()
            || matches!(self, Block::Leaves | Block::Leaves2)
            || matches!(
                self,
                Block::Grass
                    | Block::Dirt
                    | Block::Log
                    | Block::Log2
                    | Block::Sapling
                    | Block::Vine
            )
    }

    /// `Block.getOpacity`, the value the chunk height map compares against
    /// zero: an opaque cube is 255, leaves 1, water and ice 3, and anything
    /// light passes through — plants, lava, glass, snow layers — is 0.
    pub fn height_opacity(self) -> u8 {
        match self {
            Block::Stone
            | Block::Grass
            | Block::Dirt
            | Block::Cobblestone
            | Block::Planks
            | Block::Bedrock
            | Block::Sand
            | Block::Gravel
            | Block::GoldOre
            | Block::IronOre
            | Block::CoalOre
            | Block::Log
            | Block::Log2
            | Block::LapisOre
            | Block::Sandstone
            | Block::StoneSlab
            | Block::MossyCobblestone
            | Block::Obsidian
            | Block::DiamondOre
            | Block::Farmland
            | Block::RedstoneOre
            | Block::SnowBlock
            | Block::Clay
            | Block::Pumpkin
            | Block::Netherrack
            | Block::MonsterEgg
            | Block::BrownMushroomBlock
            | Block::RedMushroomBlock
            | Block::MelonBlock
            | Block::Mycelium
            | Block::EmeraldOre
            | Block::StainedHardenedClay
            | Block::HardenedClay
            | Block::PackedIce => 255,
            Block::Water | Block::FlowingWater | Block::Ice => 3,
            Block::Leaves | Block::Leaves2 => 1,
            // Air, lava, glass, and every plant and thin block.
            _ => 0,
        }
    }

    /// `PlantBlock.canBePlacedOn`: plants survive on grass, dirt or farmland.
    pub fn plant_can_survive(self) -> bool {
        matches!(self, Block::Grass | Block::Dirt | Block::Farmland)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for block in [
            Block::Stone,
            Block::Log,
            Block::Log2,
            Block::Leaves2,
            Block::MossyCobblestone,
            Block::SnowLayer,
            Block::RedMushroomBlock,
            Block::StoneSlab,
            Block::MonsterEgg,
            Block::MelonBlock,
            Block::EmeraldOre,
            Block::StainedHardenedClay,
            Block::HardenedClay,
            Block::PackedIce,
            Block::DoublePlant,
        ] {
            assert_eq!(Block::from_id(block.id()), block);
        }
    }

    #[test]
    fn material_facts_match_vanilla() {
        assert!(Block::Water.is_liquid() && !Block::Water.is_solid());
        assert!(Block::Leaves.blocks_movement() && !Block::Leaves.is_solid());
        assert!(Block::Ice.blocks_movement() && !Block::Ice.is_solid());
        assert!(!Block::TallGrass.blocks_movement());
        assert!(!Block::SnowLayer.blocks_movement());
        assert!(Block::Stone.is_opaque() && !Block::Leaves.is_opaque() && !Block::Ice.is_opaque());
        assert!(Block::PackedIce.is_solid() && Block::MelonBlock.is_solid());
        assert!(!Block::DoublePlant.is_solid() && !Block::DoublePlant.blocks_movement());
        // `Material.isReplaceable`: the plants a feature scatters into count.
        assert!(Block::TallGrass.is_replaceable() && Block::SnowLayer.is_replaceable());
        assert!(Block::Water.is_replaceable() && !Block::Grass.is_replaceable());
        assert!(Block::Vine.is_replaceable() && !Block::DoublePlant.is_replaceable());
    }
}
