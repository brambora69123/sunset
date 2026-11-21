//! The mobs parity probe: deterministic output over a fixture world, for later
//! bit-exact comparisons against the 1.7.10 jars. Run with `cargo +1.99.1 run
//! -p sunset-1-7-mobs --bin probe`.

use sunset_1_7_core::LegacyRandom;
use sunset_1_7_mobs::breeding::{AnimalBreedGoal, passive_mob_tick, set_breeding_age};
use sunset_1_7_mobs::fixture::FixtureWorld;
use sunset_1_7_mobs::goal::Goal;
use sunset_1_7_mobs::goal::GoalSelector;
use sunset_1_7_mobs::mob::{Mob, MobKind};
use sunset_1_7_mobs::pathfinder::PathFinder;
use sunset_1_7_mobs::spawn::{NaturalSpawner, SpawnEntry, populate_chunk};
use sunset_1_7_mobs::world::MobWorld;

fn make_cow(seed: i64) -> Mob {
    Mob::seeded(MobKind::Cow, seed, 0.0, 0.0, 0.0)
}

fn make_zombie(seed: i64) -> Mob {
    Mob::seeded(MobKind::Zombie, seed, 0.0, 0.0, 0.0)
}

fn main() {
    println!("=== goal selection ===");
    goal_scenario();

    println!("=== pathfinding ===");
    path_scenario();

    println!("=== hostile spawn cycle ===");
    spawn_scenario();

    println!("=== passive population ===");
    populate_scenario();

    println!("=== breeding ===");
    breeding_scenario();

    println!("=== baby growth ===");
    growth_scenario();
}

/// Priorities plus mutex bits and the third-tick re-evaluation cadence.
fn goal_scenario() {
    struct CountingGoal {
        controls: u8,
        start: bool,
        ticks: std::rc::Rc<std::cell::Cell<i32>>,
        started: std::rc::Rc<std::cell::Cell<i32>>,
    }

    impl CountingGoal {
        fn new(controls: u8, start: bool) -> Self {
            CountingGoal {
                controls,
                start,
                ticks: Default::default(),
                started: Default::default(),
            }
        }
    }

    impl Goal for CountingGoal {
        fn controls(&self) -> u8 {
            self.controls
        }

        fn can_start(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) -> bool {
            self.start
        }

        fn should_continue(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) -> bool {
            self.start
        }

        fn start(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {
            self.started.set(self.started.get() + 1);
        }

        fn tick(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {
            self.ticks.set(self.ticks.get() + 1);
        }
    }

    let mut world = FixtureWorld::new();
    let mut selector = GoalSelector::new();
    let wander = CountingGoal::new(1, true);
    let wander_ticks = wander.ticks.clone();
    let wander_started = wander.started.clone();
    let swim = CountingGoal::new(2, true);
    let swim_ticks = swim.ticks.clone();
    let id_wander = selector.add_goal(3, Box::new(wander));
    let id_swim = selector.add_goal(6, Box::new(swim));
    let mut mob = make_zombie(0);
    for _ in 0..7 {
        selector.tick(&mut mob, &mut world);
    }
    println!(
        "wander active={} ticks={} started={}",
        selector.is_active(id_wander),
        wander_ticks.get(),
        wander_started.get()
    );
    println!(
        "swim active={} ticks={}",
        selector.is_active(id_swim),
        swim_ticks.get()
    );
}

/// A* over a fixture world with a wall with a gap.
fn path_scenario() {
    let mut world = FixtureWorld::new();
    for x in -4..20 {
        for z in -4..20 {
            world.set_block(x, 0, z, 2);
        }
    }
    // a wall along x=8 with a gap at z=12
    for z in -4..20 {
        if z != 12 {
            world.set_block(8, 1, z, 1);
        }
    }
    let mut finder = PathFinder::new(true, false, false, false);
    let mob = Mob::seeded(MobKind::Zombie, 1, 2.5, 1.0, 2.5);
    let shape = mob.shape();
    let path = finder.find_path_to_block(&world, &shape, 14, 1, 14, 32.0);
    match path {
        Some(path) => {
            println!("path length={}", path.length());
            for i in 0..path.length() {
                let (x, y, z) = path.node(i);
                println!("  node {i}: {x} {y} {z}");
            }
        }
        None => println!("path none"),
    }
}

/// The per-tick spawn attempt over a grass world with a player.
fn spawn_scenario() {
    let mut world = FixtureWorld::new();
    for x in -20..20 {
        for z in -20..20 {
            world.set_block(x, 0, z, 2);
        }
    }
    world.add_player(0.0, 1.0, 0.0);
    world.set_spawn_entries(vec![SpawnEntry {
        kind: MobKind::Zombie,
        weight: 10,
        min_group_size: 2,
        max_group_size: 4,
        make: make_zombie,
    }]);
    let mut spawner = NaturalSpawner::new();
    let mut rng = LegacyRandom::new(42);
    let mut count = 0;
    for _ in 0..200 {
        count += spawner.tick(&mut world, &mut rng, 1000, true, true, true);
    }
    println!("spawned={count}");
    for mob in world.mobs() {
        println!(
            "  {} id={} at {:.3} {:.3} {:.3} yaw={:.3}",
            mob.kind.entity_key(),
            mob.id,
            mob.x,
            mob.y,
            mob.z,
            mob.yaw
        );
    }
}

/// The initial CREATURE population over a grass chunk range.
fn populate_scenario() {
    let mut world = FixtureWorld::new();
    for x in -32..32 {
        for z in -32..32 {
            world.set_block(x, 0, z, 2);
        }
    }
    let mut rng = LegacyRandom::new(7);
    for chunk in 0..40 {
        let x = (chunk % 8 - 4) * 16;
        let z = (chunk / 8 - 2) * 16;
        populate_chunk(
            &mut world,
            &mut rng,
            2000,
            &[SpawnEntry {
                kind: MobKind::Cow,
                weight: 10,
                min_group_size: 2,
                max_group_size: 4,
                make: make_cow,
            }],
            0.5,
            x,
            z,
            16,
            16,
        );
    }
    println!("population={}", world.mobs().len());
    for mob in world.mobs() {
        println!(
            "  {} id={} at {:.3} {:.3} {:.3}",
            mob.kind.entity_key(),
            mob.id,
            mob.x,
            mob.y,
            mob.z
        );
    }
}

/// Two cows in love: the breed goal runs its 60-tick clock and spawns a baby.
fn breeding_scenario() {
    let mut world = FixtureWorld::new();
    for x in -8..8 {
        for z in -8..8 {
            world.set_block(x, 0, z, 2);
        }
    }
    let mut a = make_cow(1);
    a.set_position_and_angles(0.5, 1.0, 0.5, 0.0, 0.0);
    let mut b = make_cow(2);
    b.set_position_and_angles(1.5, 1.0, 0.5, 0.0, 0.0);
    world.add_mob(a);
    world.add_mob(b);

    // the animal running the goal, standing at cow A's position
    let mut mob = make_cow(1);
    mob.set_position_and_angles(0.5, 1.0, 0.5, 0.0, 0.0);
    mob.in_love_timer = 600;

    for other in 0..2 {
        if let Some(m) = world.mob_mut(other) {
            m.in_love_timer = 600;
        }
    }

    mob.on_ground = true;
    let mut goal = AnimalBreedGoal::new(0, 1.0);
    if goal.can_start(&mut mob, &mut world) {
        for _ in 0..61 {
            goal.tick(&mut mob, &mut world);
            if world.mobs().len() > 2 {
                break;
            }
        }
    }
    println!("mobs={}", world.mobs().len());
    for m in world.mobs() {
        println!(
            "  id={} age={} baby={} at {:.3} {:.3} {:.3}",
            m.id,
            m.breeding_age,
            m.is_baby(),
            m.x,
            m.y,
            m.z
        );
    }
}

/// A baby cow at -24000: 24000 age ticks reach zero and restore the box.
fn growth_scenario() {
    let mut cow = make_cow(3);
    set_breeding_age(&mut cow, -24000);
    println!("start baby={} width={:.5}", cow.is_baby(), cow.width);
    for _ in 0..23999 {
        passive_mob_tick(&mut cow);
    }
    println!(
        "tick 23999: age={} baby={} width={:.5}",
        cow.breeding_age,
        cow.is_baby(),
        cow.width
    );
    passive_mob_tick(&mut cow);
    println!(
        "tick 24000: age={} baby={} width={:.5}",
        cow.breeding_age,
        cow.is_baby(),
        cow.width
    );
}
