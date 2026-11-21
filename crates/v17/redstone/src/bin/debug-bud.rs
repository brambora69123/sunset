//! Debug: quasi-connectivity trace.

use sunset_1_7_redstone::block::Block;
use sunset_1_7_redstone::piston;
use sunset_1_7_redstone::world::World;

fn main() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::StickyPiston, 1, 2);
    world.set_block_with_metadata(0, 3, 0, Block::RedstoneBlock, 0, 3);
    let facing = piston::get_facing(world.get_block_metadata(0, 1, 0));
    println!("facing: {}", facing);
    println!(
        "should_extend: {}",
        piston::should_extend(&world, 0, 1, 0, facing)
    );
    println!("has_signal y+2: {}", world.has_signal(0, 3, 0, 1));
    println!("block_events queued: {}", world.block_event_count());
    world.tick();
    println!("after tick: {:?}", world.get_block(0, 1, 0));
}
