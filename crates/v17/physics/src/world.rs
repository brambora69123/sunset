//! The world side of physics: what `World.getCollisions`, `containsLiquid`,
//! `applyLiquidDrag` and friends need from the block tables. The game crates
//! implement [`PhysWorld`]; the scans here are exact ports of the 1.7.10
//! `World` methods.

use crate::aabb::Aabb;
use crate::material::Material;
use crate::math;

/// The block facts entity physics reads. Coordinates are block positions.
pub trait PhysWorld {
    /// The block's collision box in world coords (`Block.getCollisionShape`,
    /// filtered by `Block.addCollisions`); `None` when the block does not
    /// collide (air, liquids, ...).
    fn block_collision(&self, x: i32, y: i32, z: i32) -> Option<Aabb>;

    /// `Block.getMaterial`.
    fn block_material(&self, x: i32, y: i32, z: i32) -> Material;

    /// `Block.slipperiness` (0.6 for most blocks).
    fn block_slipperiness(&self, x: i32, y: i32, z: i32) -> f32;

    /// `LiquidBlock.getHeightLoss` for this liquid block's metadata.
    fn liquid_height_loss(&self, x: i32, y: i32, z: i32) -> f32;

    /// The flow vector `Block.applyMaterialDrag` adds to the drag
    /// accumulator for this liquid block.
    fn liquid_flow(&self, x: i32, y: i32, z: i32) -> [f64; 3] {
        let _ = (x, y, z);
        [0.0, 0.0, 0.0]
    }

    /// A ladder or vine at this block (`LivingEntity.isClimbing`).
    fn block_is_climbable(&self, x: i32, y: i32, z: i32) -> bool {
        let _ = (x, y, z);
        false
    }

    /// A ladder at this block (`Entity.move`'s walk-distance Y component).
    fn block_is_ladder(&self, x: i32, y: i32, z: i32) -> bool {
        let _ = (x, y, z);
        false
    }

    /// Fire, flowing lava or lava at this block (`World.containsFireSource`).
    fn block_is_fire_source(&self, x: i32, y: i32, z: i32) -> bool {
        let _ = (x, y, z);
        false
    }

    /// `World.isRaining` at this column position.
    fn is_raining(&self, x: i32, y: i32, z: i32) -> bool {
        let _ = (x, y, z);
        false
    }
}

/// `World.getBlockCollisions`: every colliding block box the shape touches.
/// Beyond ±30000000 the border is bedrock.
pub fn get_block_collisions(world: &dyn PhysWorld, shape: &Aabb) -> Vec<Aabb> {
    let x0 = math::floor(shape.min_x);
    let x1 = math::floor(shape.max_x + 1.0);
    let y0 = math::floor(shape.min_y);
    let y1 = math::floor(shape.max_y + 1.0);
    let z0 = math::floor(shape.min_z);
    let z1 = math::floor(shape.max_z + 1.0);
    let mut collisions = Vec::new();
    for x in x0..x1 {
        for z in z0..z1 {
            for y in (y0 - 1)..y1 {
                let block = if !(-30_000_000..30_000_000).contains(&x)
                    || !(-30_000_000..30_000_000).contains(&z)
                {
                    Some(Aabb::cube(x, y, z))
                } else {
                    world.block_collision(x, y, z)
                };
                if let Some(box_) = block
                    && shape.intersects(&box_)
                {
                    collisions.push(box_);
                }
            }
        }
    }
    collisions
}

/// `World.getCollisions`: block collisions plus the collision shapes of
/// nearby entities (`nearby`, gathered by the caller) that intersect the shape.
pub fn get_collisions(world: &dyn PhysWorld, nearby: &[Aabb], shape: &Aabb) -> Vec<Aabb> {
    let mut collisions = get_block_collisions(world, shape);
    for box_ in nearby {
        if box_.intersects(shape) {
            collisions.push(*box_);
        }
    }
    collisions
}

/// `World.containsLiquid(Box)`: any liquid block the bounds reach.
pub fn contains_liquid(world: &dyn PhysWorld, bounds: &Aabb) -> bool {
    let mut x0 = math::floor(bounds.min_x);
    let x1 = math::floor(bounds.max_x + 1.0);
    let mut y0 = math::floor(bounds.min_y);
    let y1 = math::floor(bounds.max_y + 1.0);
    let mut z0 = math::floor(bounds.min_z);
    let z1 = math::floor(bounds.max_z + 1.0);
    if bounds.min_x < 0.0 {
        x0 -= 1;
    }
    if bounds.min_y < 0.0 {
        y0 -= 1;
    }
    if bounds.min_z < 0.0 {
        z0 -= 1;
    }
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                if world.block_material(x, y, z).is_liquid() {
                    return true;
                }
            }
        }
    }
    false
}

/// `World.containsMaterial(Box, Material)`.
pub fn contains_material(world: &dyn PhysWorld, bounds: &Aabb, material: Material) -> bool {
    let x0 = math::floor(bounds.min_x);
    let x1 = math::floor(bounds.max_x + 1.0);
    let y0 = math::floor(bounds.min_y);
    let y1 = math::floor(bounds.max_y + 1.0);
    let z0 = math::floor(bounds.min_z);
    let z1 = math::floor(bounds.max_z + 1.0);
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                if world.block_material(x, y, z) == material {
                    return true;
                }
            }
        }
    }
    false
}

/// `World.containsFireSource`.
pub fn contains_fire_source(world: &dyn PhysWorld, bounds: &Aabb) -> bool {
    let x0 = math::floor(bounds.min_x);
    let x1 = math::floor(bounds.max_x + 1.0);
    let y0 = math::floor(bounds.min_y);
    let y1 = math::floor(bounds.max_y + 1.0);
    let z0 = math::floor(bounds.min_z);
    let z1 = math::floor(bounds.max_z + 1.0);
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                if world.block_is_fire_source(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

/// What `World.applyLiquidDrag` reports: whether the bounds reach the liquid,
/// and the normalized flow push (× 0.014) it adds to the velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidDrag {
    pub in_liquid: bool,
    pub push: [f64; 3],
}

/// `World.applyLiquidDrag(bounds, material)`: water/lava drag over the bounds.
pub fn apply_liquid_drag(world: &dyn PhysWorld, bounds: &Aabb, material: Material) -> LiquidDrag {
    let x0 = math::floor(bounds.min_x);
    let x1 = math::floor(bounds.max_x + 1.0);
    let y0 = math::floor(bounds.min_y);
    let y1 = math::floor(bounds.max_y + 1.0);
    let z0 = math::floor(bounds.min_z);
    let z1 = math::floor(bounds.max_z + 1.0);
    let mut in_liquid = false;
    let mut flow = [0.0f64; 3];
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                if world.block_material(x, y, z) == material {
                    let surface = y as f64 + 1.0 - world.liquid_height_loss(x, y, z) as f64;
                    if y1 as f64 >= surface {
                        in_liquid = true;
                        let add = world.liquid_flow(x, y, z);
                        flow[0] += add[0];
                        flow[1] += add[1];
                        flow[2] += add[2];
                    }
                }
            }
        }
    }

    let length = (flow[0] * flow[0] + flow[1] * flow[1] + flow[2] * flow[2]).sqrt();
    let push = if length > 0.0 {
        // Vec3d.normalize's 1.0E-4 guard: tiny accumulators normalize to zero.
        if length < 1.0e-4 {
            [0.0, 0.0, 0.0]
        } else {
            [
                flow[0] / length * 0.014,
                flow[1] / length * 0.014,
                flow[2] / length * 0.014,
            ]
        }
    } else {
        [0.0, 0.0, 0.0]
    };
    LiquidDrag { in_liquid, push }
}
