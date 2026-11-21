//! Block material flags, ported from `net.minecraft.block.material.Material`
//! (1.7.10).
//!
//! Only the flags the render side reads are carried: `isOpaque` (which decides
//! `Block.isTranslucent`), the `translucent` marker (which decides
//! `Material.isSolidBlocking`), movement blocking (ambient-occlusion darkness),
//! fluidity and the solidity the liquid height sampler reads.

/// The material constants the renderer compares by identity: 1.7.10 uses
/// object identity, and the liquid face culling compares the two materials.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialId {
    Air,
    Grass,
    Dirt,
    Stone,
    Wood,
    Sand,
    Water,
    Lava,
    Leaves,
    Plant,
    Glass,
    Ice,
    SnowLayer,
    Snow,
    Cactus,
    Tnt,
    Web,
    Portal,
    Carpet,
    Pumpkin,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Material {
    pub id: MaterialId,
    pub air: bool,
    pub liquid: bool,
    /// `Material.setTranslucent()`; a material's `isSolidBlocking` is false
    /// while this is set.
    pub translucent: bool,
    /// `Material.isOpaque()`: true unless the material is air, a plant, a
    /// carpet, the snow layer, fire or the portal. This is what
    /// `Block.isTranslucent` reads.
    pub opaque: bool,
    pub blocks_movement: bool,
    /// `Material.isSolid()`: false for air, liquids and plant materials.
    pub solid: bool,
}

impl Material {
    pub const AIR: Material = Material {
        id: MaterialId::Air,
        air: true,
        liquid: false,
        translucent: false,
        opaque: false,
        blocks_movement: false,
        solid: false,
    };
    pub const GRASS: Material = Material::opaque(MaterialId::Grass);
    pub const DIRT: Material = Material::opaque(MaterialId::Dirt);
    pub const STONE: Material = Material::opaque(MaterialId::Stone);
    pub const WOOD: Material = Material::opaque(MaterialId::Wood);
    pub const SAND: Material = Material::opaque(MaterialId::Sand);
    pub const SNOW: Material = Material::opaque(MaterialId::Snow);
    pub const PUMPKIN: Material = Material::opaque(MaterialId::Pumpkin);
    pub const WATER: Material = Material {
        id: MaterialId::Water,
        air: false,
        liquid: true,
        translucent: false,
        opaque: true,
        blocks_movement: false,
        solid: false,
    };
    pub const LAVA: Material = Material {
        id: MaterialId::Lava,
        ..Material::WATER
    };
    pub const LEAVES: Material = Material::translucent_solid(MaterialId::Leaves);
    pub const GLASS: Material = Material::translucent_solid(MaterialId::Glass);
    pub const ICE: Material = Material::translucent_solid(MaterialId::Ice);
    pub const CACTUS: Material = Material::translucent_solid(MaterialId::Cactus);
    pub const TNT: Material = Material::translucent_solid(MaterialId::Tnt);
    pub const PLANT: Material = Material {
        id: MaterialId::Plant,
        air: false,
        liquid: false,
        translucent: false,
        opaque: false,
        blocks_movement: false,
        solid: false,
    };
    /// `Material.WEB` (`Material$04645074`): movement passes, nothing else does.
    pub const WEB: Material = Material {
        id: MaterialId::Web,
        air: false,
        liquid: false,
        translucent: false,
        opaque: true,
        blocks_movement: false,
        solid: true,
    };
    /// `Material.PORTAL`: not solid, but opaque for the render side.
    pub const PORTAL: Material = Material {
        id: MaterialId::Portal,
        air: false,
        liquid: false,
        translucent: false,
        opaque: true,
        blocks_movement: true,
        solid: false,
    };
    /// `Material.CARPET`: a plant material that draws like a plate.
    pub const CARPET: Material = Material {
        id: MaterialId::Carpet,
        ..Material::PLANT
    };
    /// `Material.SNOW_LAYER`: a translucent plant material.
    pub const SNOW_LAYER: Material = Material {
        id: MaterialId::SnowLayer,
        translucent: true,
        ..Material::PLANT
    };

    const fn opaque(id: MaterialId) -> Material {
        Material {
            id,
            air: false,
            liquid: false,
            translucent: false,
            opaque: true,
            blocks_movement: true,
            solid: true,
        }
    }

    /// `Material.setTranslucent()`: solid and movement-blocking, but
    /// see-through for `isSolidBlocking` (leaves, glass, ice, cactus, TNT).
    const fn translucent_solid(id: MaterialId) -> Material {
        Material {
            id,
            air: false,
            liquid: false,
            translucent: true,
            opaque: true,
            blocks_movement: true,
            solid: true,
        }
    }

    /// `Material.isOpaque()`.
    pub fn is_opaque(&self) -> bool {
        self.opaque
    }

    /// `Material.isSolidBlocking()`.
    pub fn is_solid_blocking(&self) -> bool {
        if self.translucent {
            false
        } else {
            self.blocks_movement
        }
    }

    /// `Material.isSolid()`.
    pub fn is_solid(&self) -> bool {
        self.solid
    }
}
