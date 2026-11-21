//! Deterministic physics probe for later parity checks: runs a scripted
//! entity through fall, walk, step-up, wall clip, jump, sprint jump, water,
//! lava and knockback against the [`fixture::TestWorld`] tables and prints
//! the state after each phase.
//!
//! Run: `cargo +1.99.1 run -p sunset-1-7-physics --bin physics_probe`

use sunset_1_7_core::LegacyRandom;
use sunset_1_7_physics::EntityPhysics;
use sunset_1_7_physics::fixture::{Liquid, TestWorld};

const EYE: f64 = (1.8f32 * 0.85) as f64;

fn fall_entity(world: &TestWorld, from: f64) -> EntityPhysics {
    let mut entity = EntityPhysics::living(0.6, 1.8);
    entity.set_position(0.5, from, 0.5);
    let mut ticks = 0;
    // move_relative applies gravity after each move, vanilla tick order.
    while !entity.on_ground && ticks < 400 {
        entity.move_relative(world, 0.0, 0.0);
        ticks += 1;
    }
    entity
}

fn main() {
    let world = TestWorld::flat();

    // 1. Fall from eye level 20 and land.
    let entity = fall_entity(&world, 20.0);
    println!(
        "fall y={:.6} ground={} fall_distance={:.6} damage={:?}",
        entity.y, entity.on_ground, entity.fall_distance, entity.pending_damage
    );

    // 2. Walk 20 ticks forward.
    let mut walker = fall_entity(&world, EYE);
    walker.velocity_x = 0.0;
    for _ in 0..20 {
        walker.tick_start();
        walker.move_relative(&world, 0.0, 0.05);
    }
    println!(
        "walk x={:.6} z={:.6} walk_distance={:.6} ground={}",
        walker.x, walker.z, walker.walk_distance, walker.on_ground
    );

    // 3. Step up onto a half block.
    let mut stepped = TestWorld::flat();
    stepped.half.push(((3, 0, 0), 0.5));
    let mut stepper = fall_entity(&stepped, EYE);
    stepper.set_position(2.5, EYE, 0.5);
    stepper.on_ground = true;
    stepper.move_entity(&stepped, 0.3, -0.0784, 0.0, &[]);
    println!(
        "step x={:.6} y={:.6} ground={}",
        stepper.x, stepper.y, stepper.on_ground
    );

    // 4. Walk into a wall.
    let mut walled = TestWorld::flat();
    for y in 0..2 {
        walled.solid.push((10, y, 0));
    }
    let mut pusher = fall_entity(&walled, EYE);
    pusher.set_position(9.5, EYE, 0.5);
    pusher.on_ground = true;
    pusher.move_entity(&walled, 0.5, -0.0784, 0.0, &[]);
    println!(
        "wall x={:.6} collide={} velocity_x={:.6}",
        pusher.x, pusher.colliding_horizontally, pusher.velocity_x
    );

    // 5. Jump, then sprint jump facing +Z.
    let mut jumper = fall_entity(&world, EYE);
    jumper.on_ground = true;
    jumper.jump();
    println!("jump vy={:.6}", jumper.velocity_y);
    jumper.velocity_y = 0.0;
    jumper.velocity_x = 0.0;
    jumper.velocity_z = 0.0;
    jumper.sprinting = true;
    jumper.jump();
    println!(
        "sprint_jump vy={:.6} vx={:.6} vz={:.6}",
        jumper.velocity_y, jumper.velocity_x, jumper.velocity_z
    );

    // 6. Water: enter, fall distance reset, travel down, swim up at a wall.
    let mut water = TestWorld::tank(Liquid::Water, 20, 0, 20, 4, 4, 4);
    for y in 0..4 {
        water.solid.push((24, y, 20));
    }
    let mut swimmer = EntityPhysics::living(0.6, 1.8);
    swimmer.set_position(22.5, 3.5, 22.5);
    swimmer.fall_distance = 12.0;
    let entered = swimmer.check_water_collisions(&water);
    println!(
        "water in_water={} fall_distance={:.6}",
        entered, swimmer.fall_distance
    );
    swimmer.tick_start();
    swimmer.move_relative(&water, 0.0, 0.0);
    println!("water_travel vy={:.6}", swimmer.velocity_y);
    let mut surfer = EntityPhysics::living(0.6, 1.8);
    surfer.set_position(23.6, 5.0, 20.5);
    surfer.check_water_collisions(&water);
    surfer.velocity_x = 0.1;
    surfer.tick_start();
    surfer.move_relative(&water, 0.0, 0.0);
    println!(
        "swim_up collide={} vy={:.6}",
        surfer.colliding_horizontally, surfer.velocity_y
    );

    // 7. Lava travel.
    let lava = TestWorld::tank(Liquid::Lava, 20, 0, 20, 4, 4, 4);
    let mut bather = EntityPhysics::living(0.6, 1.8);
    bather.set_position(22.5, 2.0, 22.5);
    let in_lava = bather.is_in_lava(&lava);
    bather.tick_start();
    bather.move_relative(&lava, 0.0, 0.0);
    println!("lava in_lava={} vy={:.6}", in_lava, bather.velocity_y);

    // 8. Knockback from +Z, seeded random.
    let mut hit = EntityPhysics::living(0.6, 1.8);
    hit.random = LegacyRandom::new(42);
    hit.apply_knockback(1.0, 0.0, 1.0);
    println!(
        "knockback vx={:.6} vy={:.6} vz={:.6}",
        hit.velocity_x, hit.velocity_y, hit.velocity_z
    );

    // 9. Void damage input below y - 64.
    let mut doomed = EntityPhysics::living(0.6, 1.8);
    doomed.set_position(0.5, -65.0, 0.5);
    doomed.void_tick();
    println!("void y={:.6} damage={:?}", doomed.y, doomed.pending_damage);
}
