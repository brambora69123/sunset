//! Debug: hopper chain trace.

use sunset_1_7_redstone::block::Block;
use sunset_1_7_redstone::item::ItemStack;
use sunset_1_7_redstone::world::{BlockEntity, Pos, World};

fn main() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 2, 0, Block::Hopper, 0, 2);
    world.set_block_with_metadata(0, 1, 0, Block::Hopper, 0, 2);
    println!(
        "be at 0,2,0: {:?}",
        world.block_entity(Pos::new(0, 2, 0)).is_some()
    );
    println!(
        "be at 0,1,0: {:?}",
        world.block_entity(Pos::new(0, 1, 0)).is_some()
    );
    if let Some(inv) = world.inventory_at(Pos::new(0, 2, 0)) {
        inv.set_item(0, Some(ItemStack::new(1, 3)));
    }
    for t in 0..10 {
        world.tick();
        let above = count(&world, 0, 2, 0);
        let below = count(&world, 0, 1, 0);
        println!("tick {t}: above={above} below={below}");
    }
}

fn count(world: &World, x: i32, y: i32, z: i32) -> i32 {
    match world.block_entity(Pos::new(x, y, z)) {
        Some(BlockEntity::Inventory(inv)) => inv.get_item(0).map(|s| s.size).unwrap_or(0),
        _ => -1,
    }
}
