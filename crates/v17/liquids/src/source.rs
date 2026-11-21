//! `net.minecraft.block.LiquidSourceBlock`: the still block.
//!
//! Water's still block does nothing on its own; it moves only after a neighbor
//! update turns it into the flowing block ([`crate::neighbor_changed`]). Lava's
//! still block also spreads fire.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::block::{Block, FIRE, Material};
use crate::liquid::set_block;
use crate::world::LiquidWorld;

/// `LiquidSourceBlock.tick`: lava's random fire spreading. Water has no
/// still-block tick.
pub(crate) fn tick(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    this: Block,
    random: &mut LegacyRandom,
) {
    if this.material != Material::Lava {
        return;
    }

    let mut x = x;
    let mut y = y;
    let mut z = z;
    let rolls = random.next_int_bound(3).unwrap_or(0);

    for _ in 0..rolls {
        x += random.next_int_bound(3).unwrap_or(0) - 1;
        y += 1;
        z += random.next_int_bound(3).unwrap_or(0) - 1;
        let block = world.get_block(x, y, z);
        if block.material == Material::Air {
            if is_flammable(world, x - 1, y, z)
                || is_flammable(world, x + 1, y, z)
                || is_flammable(world, x, y, z - 1)
                || is_flammable(world, x, y, z + 1)
                || is_flammable(world, x, y - 1, z)
                || is_flammable(world, x, y + 1, z)
            {
                set_block(world, x, y, z, FIRE, random);
                return;
            }
        } else if block.blocks_movement {
            return;
        }
    }

    if rolls == 0 {
        // Vanilla keeps the position the first loop walked to.
        let base_x = x;
        let base_z = z;
        for _ in 0..3 {
            x = base_x + random.next_int_bound(3).unwrap_or(0) - 1;
            z = base_z + random.next_int_bound(3).unwrap_or(0) - 1;
            if world.get_block(x, y + 1, z).material == Material::Air
                && is_flammable(world, x, y, z)
            {
                set_block(world, x, y + 1, z, FIRE, random);
            }
        }
    }
}

/// `LiquidSourceBlock.isFlammable`.
fn is_flammable(world: &dyn LiquidWorld, x: i32, y: i32, z: i32) -> bool {
    world.get_block(x, y, z).flammable
}
