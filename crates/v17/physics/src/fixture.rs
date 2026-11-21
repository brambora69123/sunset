//! Block tables for physics tests and the probe: flat solid ground below
//! y = 0, plus walls, a half step and liquid tanks. Not part of the port.

use crate::aabb::Aabb;
use crate::material::Material;
use crate::world::PhysWorld;

pub struct TestWorld {
    /// Solid ground for every block below y = 0.
    pub ground: bool,
    pub solid: Vec<(i32, i32, i32)>,
    pub water: Vec<(i32, i32, i32)>,
    pub lava: Vec<(i32, i32, i32)>,
    /// Partial collision boxes: (block, top height), like a slab.
    pub half: Vec<((i32, i32, i32), f64)>,
}

impl TestWorld {
    pub fn flat() -> Self {
        Self {
            ground: true,
            solid: Vec::new(),
            water: Vec::new(),
            lava: Vec::new(),
            half: Vec::new(),
        }
    }

    /// A still liquid tank: `w`×`h`×`d` blocks from (x, y, z).
    pub fn tank(kind: Liquid, x: i32, y: i32, z: i32, w: i32, h: i32, d: i32) -> Self {
        let mut world = Self::flat();
        for dx in 0..w {
            for dy in 0..h {
                for dz in 0..d {
                    let block = (x + dx, y + dy, z + dz);
                    match kind {
                        Liquid::Water => world.water.push(block),
                        Liquid::Lava => world.lava.push(block),
                    }
                }
            }
        }
        world
    }

    fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        (self.ground && y < 0) || self.solid.contains(&(x, y, z))
    }
}

pub enum Liquid {
    Water,
    Lava,
}

impl PhysWorld for TestWorld {
    fn block_collision(&self, x: i32, y: i32, z: i32) -> Option<Aabb> {
        if self.is_solid(x, y, z) {
            return Some(Aabb::cube(x, y, z));
        }
        for (block, top) in &self.half {
            if *block == (x, y, z) {
                return Some(Aabb::new(
                    x as f64,
                    y as f64,
                    z as f64,
                    x as f64 + 1.0,
                    *top,
                    z as f64 + 1.0,
                ));
            }
        }
        None
    }

    fn block_material(&self, x: i32, y: i32, z: i32) -> Material {
        if self.is_solid(x, y, z) {
            Material::Solid
        } else if self.water.contains(&(x, y, z)) {
            Material::Water
        } else if self.lava.contains(&(x, y, z)) {
            Material::Lava
        } else {
            Material::Air
        }
    }

    fn block_slipperiness(&self, _x: i32, _y: i32, _z: i32) -> f32 {
        0.6
    }

    fn liquid_height_loss(&self, _x: i32, _y: i32, _z: i32) -> f32 {
        1.0 / 9.0
    }
}
