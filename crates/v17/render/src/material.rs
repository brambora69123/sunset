//! Block material flags, ported from `net.minecraft.block.material.Material`.
//!
//! Only the flags the render side reads are carried: opacity (which decides
//! `Block.isTranslucent`), the `translucent` marker (which decides
//! `Material.isSolidBlocking`), movement blocking (ambient-occlusion darkness)
//! and the solidity used by the liquid height sampler.

/// The material constants the renderer compares by identity. Vanilla uses
/// object identity (`==`), which matters for pumpkins, liquids and snow.
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
    /// `Material.isOpaque()`: the default is true; air and plant materials
    /// override it to false. This is what `Block.isTranslucent` reads.
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
    pub const GRASS: Material = Material::opaque(MaterialId::Grass, true);
    pub const DIRT: Material = Material::opaque(MaterialId::Dirt, true);
    pub const STONE: Material = Material::opaque(MaterialId::Stone, true);
    pub const WOOD: Material = Material::opaque(MaterialId::Wood, true);
    pub const SAND: Material = Material::opaque(MaterialId::Sand, true);
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
    pub const PLANT: Material = Material {
        id: MaterialId::Plant,
        air: false,
        liquid: false,
        translucent: false,
        opaque: false,
        blocks_movement: false,
        solid: false,
    };
    pub const GLASS: Material = Material::translucent_solid(MaterialId::Glass);
    /// `Material.ICE`: translucent like glass, but a solid block.
    pub const ICE: Material = Material::translucent_solid(MaterialId::Ice);
    /// `Material.SNOW_LAYER`: a translucent plant material.
    pub const SNOW_LAYER: Material = Material {
        id: MaterialId::SnowLayer,
        air: false,
        liquid: false,
        translucent: true,
        opaque: false,
        blocks_movement: false,
        solid: false,
    };
    /// `Material.PUMPKIN`.
    pub const PUMPKIN: Material = Material::opaque(MaterialId::Pumpkin, true);

    const fn opaque(id: MaterialId, blocks_movement: bool) -> Material {
        Material {
            id,
            air: false,
            liquid: false,
            translucent: false,
            opaque: true,
            blocks_movement,
            solid: true,
        }
    }

    /// `Material.setTranslucent()`: solid and movement-blocking but
    /// see-through for face culling (leaves, glass).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_is_not_solid_or_opaque() {
        assert!(!Material::AIR.is_solid());
        assert!(!Material::AIR.is_opaque());
        assert!(!blocks_movement(Material::AIR));
    }

    fn flags(material: Material) -> (bool, bool, bool, bool) {
        (
            material.liquid,
            material.blocks_movement,
            material.is_solid(),
            material.is_opaque(),
        )
    }

    fn blocks_movement(material: Material) -> bool {
        material.blocks_movement
    }

    #[test]
    fn water_is_liquid_not_solid() {
        let (liquid, blocks_movement, solid, _) = flags(Material::WATER);
        assert!(liquid);
        assert!(!blocks_movement);
        assert!(!solid);
    }

    #[test]
    fn translucent_material_does_not_solid_block() {
        assert!(!Material::GLASS.is_solid_blocking());
        assert!(blocks_movement(Material::GLASS));
        assert!(Material::STONE.is_solid_blocking());
    }

    #[test]
    fn glass_is_opaque_for_block_translucency_but_plants_are_not() {
        // `Block.isTranslucent` is `!material.isOpaque()`, and only air and
        // plant materials override `isOpaque`.
        assert!(Material::GLASS.is_opaque());
        assert!(!Material::PLANT.is_opaque());
        assert!(!Material::SNOW_LAYER.is_opaque());
    }
}
