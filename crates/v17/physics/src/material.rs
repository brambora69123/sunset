//! 1.7.10 `Material` (net.minecraft.block.material.Material): the block
//! behaviour flags physics branches on. The game crates own the real table;
//! `Solid` stands for every non-air, non-plant material (liquids included —
//! vanilla's liquid materials are solid for `Material.isSolid`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Air,
    Water,
    Lava,
    Plant,
    Solid,
}

impl Material {
    /// `Material.isLiquid`.
    pub fn is_liquid(self) -> bool {
        matches!(self, Material::Water | Material::Lava)
    }

    /// `Material.isSolid`: base true; air and plant materials override false.
    pub fn is_solid(self) -> bool {
        !matches!(self, Material::Air | Material::Plant)
    }
}
