//! Debug: exact piston test reproduction.

use sunset_1_7_redstone::block::Block;
use sunset_1_7_redstone::world::World;

fn solid_at(world: &mut World, x: i32, y: i32, z: i32) {
    world.set_block_with_metadata(x, y, z, Block::Solid, 0, 2);
}

fn main() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3); // direct neighbor
    solid_at(&mut world, 1, 1, 0); // the block to push
    world.tick();
    println!(
        "block={:?} meta={} front={:?} pushed2={:?} events={}",
        world.get_block(0, 1, 0),
        world.get_block_metadata(0, 1, 0),
        world.get_block(1, 1, 0),
        world.get_block(2, 1, 0),
        world.events.len()
    );
}
