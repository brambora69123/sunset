//! Debug: piston extension trace.

use sunset_1_7_redstone::block::Block;
use sunset_1_7_redstone::piston;
use sunset_1_7_redstone::world::World;

fn main() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(1, 1, 0, Block::Solid, 0, 2);
    println!(
        "should_extend: {}",
        piston::should_extend(
            &world,
            0,
            1,
            0,
            piston::get_facing(world.get_block_metadata(0, 1, 0))
        )
    );
    println!("block_events queued: {}", world.block_event_count());
    world.tick();
    println!(
        "after tick: block={:?} meta={} this_tick={}",
        world.get_block(0, 1, 0),
        world.get_block_metadata(0, 1, 0),
        world.this_tick.len()
    );
    println!(
        "front: {:?} pushed: {:?}",
        world.get_block(1, 1, 0),
        world.get_block(2, 1, 0)
    );
}
