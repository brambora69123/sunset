//! Animal breeding and baby growth, from `entity/living/mob/passive/PassiveEntity.java`
//! and `passive/animal/AnimalEntity.java`, plus `AnimalBreedGoal`.

use crate::blocks::block_by_id;
use crate::goal::Goal;
use crate::math;
use crate::mob::{Mob, MobKind};
use crate::world::{MobRef, MobWorld, TargetRef};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `PassiveEntity.mobTick` — the breeding age counter, through `setBreedingAge`
/// so the box resizes the tick the baby grows up.
pub fn passive_mob_tick(mob: &mut Mob) {
    let age = mob.breeding_age;
    if age < 0 {
        set_breeding_age(mob, age + 1);
    } else if age > 0 {
        set_breeding_age(mob, age - 1);
    }
}

/// `PassiveEntity.growUp(age)` — `age` is in seconds.
pub fn grow_up(mob: &mut Mob, age: i32) {
    let mut new_age = mob.breeding_age + age * 20;
    if new_age > 0 {
        new_age = 0;
    }

    mob.breeding_age = new_age;
}

/// `PassiveEntity.setBreedingAge` — clamps upward growth at zero and resizes the box.
/// The hitbox halves for babies, like `PassiveEntity.setAgeSize(0.5F)`.
pub fn set_breeding_age(mob: &mut Mob, age: i32) {
    mob.breeding_age = age;
    let (base_width, base_height) = mob.kind.size();
    let scale = if mob.is_baby() { 0.5 } else { 1.0 };
    mob.width = base_width * scale;
    mob.height = base_height * scale;
}

/// `AnimalEntity.mobTick` — the love timer and its heart particles.
/// `rng` is the world RNG, as 1.7.10 mobs share it.
pub fn animal_mob_tick(mob: &mut Mob, rng: &mut LegacyRandom, world: &mut dyn MobWorld) {
    if mob.breeding_age != 0 {
        mob.in_love_timer = 0;
    }

    if mob.in_love_timer > 0 {
        mob.in_love_timer -= 1;
        if mob.in_love_timer % 10 == 0 {
            let _var2 = rng.next_gaussian() * 0.02;
            let _var4 = rng.next_gaussian() * 0.02;
            let _var6 = rng.next_gaussian() * 0.02;
            world.entity_event(mob.id, 18);
        }
    } else {
        mob.breeding_ticks = 0;
    }

    passive_mob_tick(mob);
}

/// `AnimalEntity.lovePlayer(player)` — wheat interaction.
pub fn love_player(mob: &mut Mob, player_id: u32) {
    mob.in_love_timer = 600;
    mob.love_causing_player = Some(player_id);
    mob.walk_target = None;
}

/// `AnimalEntity.canBreedWith(other)`.
pub fn can_breed_with(mob: &MobRef, other: &MobRef) -> bool {
    if other.id == mob.id {
        return false;
    }
    if other.kind != mob.kind {
        return false;
    }
    mob.in_love && other.in_love
}

/// `AnimalEntity.findTarget()` — the legacy-AI target scan.
pub fn animal_find_target(mob: &Mob, world: &dyn MobWorld) -> Option<TargetRef> {
    if mob.fast_movement_cooldown > 0 {
        return None;
    }

    let range = 8.0f64;
    let pos = (mob.x, mob.y, mob.z);
    if mob.in_love_timer > 0 {
        for other in world.mobs_of_kind(mob.kind) {
            if other.id != mob.id
                && other.in_love
                && pos_in_grown_box(pos, range, (other.x, other.y, other.z))
            {
                return Some(TargetRef::Mob(other.id));
            }
        }
    } else if mob.breeding_age == 0 {
        for player in world.players() {
            if pos_in_grown_box(pos, range, (player.x, player.y, player.z))
                && world.player_holds_breeding_item(player.id, mob.kind)
            {
                return Some(TargetRef::Player(player.id));
            }
        }
    } else if mob.breeding_age > 0 {
        for other in world.mobs_of_kind(mob.kind) {
            if other.id != mob.id
                && other.breeding_age < 0
                && pos_in_grown_box(pos, range, (other.x, other.y, other.z))
            {
                return Some(TargetRef::Mob(other.id));
            }
        }
    }

    None
}

fn pos_in_grown_box(pos: (f64, f64, f64), grown: f64, other: (f64, f64, f64)) -> bool {
    // shape.grown(8, 8, 8) center test; the exact box intersect needs the other
    // entity's box, which the seam does not carry
    let dx = other.0 - pos.0;
    let dy = other.1 - pos.1;
    let dz = other.2 - pos.2;
    dx.abs() <= grown && dy.abs() <= grown && dz.abs() <= grown
}

/// `AnimalEntity.targetInSight` — the mating walk and the 60-tick breed clock.
pub fn animal_target_in_sight(
    mob: &mut Mob,
    world: &mut dyn MobWorld,
    target: TargetRef,
    distance: f64,
) {
    match target {
        TargetRef::Player(player_id) => {
            if distance < 3.0 {
                // face the player: the legacy AI steers yaw directly
                if let Some(player) = world.players().iter().find(|p| p.id == player_id) {
                    let dx = player.x - mob.x;
                    let dz = player.z - mob.z;
                    mob.yaw = (math::atan2(dz, dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
                    mob.stopped_moving = true;
                }
            }
            if !world.player_holds_breeding_item(player_id, mob.kind) {
                mob.walk_target = None;
            }
        }
        TargetRef::Mob(other_id) => {
            let Some(other) = world.mob_ref(other_id) else {
                mob.breeding_ticks = 0;
                mob.walk_target = None;
                return;
            };
            if mob.breeding_age > 0 && other.breeding_age < 0 {
                if distance < 2.5 {
                    mob.stopped_moving = true;
                }
            } else if mob.in_love_timer > 0 && other.in_love {
                if other.walk_target.is_none() {
                    world.set_walk_target(other.id, Some(TargetRef::Mob(mob.id)));
                }

                let chase = other.walk_target == Some(TargetRef::Mob(mob.id));
                if chase && distance < 3.5 {
                    world.add_love_ticks(other.id, 1);
                    mob.in_love_timer += 1;
                    mob.breeding_ticks += 1;
                    if mob.breeding_ticks % 4 == 0 {
                        world.entity_event(mob.id, 18);
                    }

                    if mob.breeding_ticks == 60 {
                        legacy_breed(mob, world, other_id);
                    }
                } else {
                    mob.breeding_ticks = 0;
                }
            } else {
                mob.breeding_ticks = 0;
                mob.walk_target = None;
            }
        }
    }
}

/// `AnimalEntity.breed(mate)` — the legacy-AI breed: the 7 heart draws come from
/// the animal's own RNG, the child spawns at the parent's yaw and pitch.
fn legacy_breed(mob: &mut Mob, world: &mut dyn MobWorld, mate_id: u32) {
    if make_child(mob.kind).is_none() {
        return;
    }

    mob.breeding_age = 6000;
    set_breeding_age(mob, 6000);
    mob.in_love_timer = 0;
    mob.breeding_ticks = 0;
    mob.walk_target = None;
    world.finish_breeding(mate_id);

    let mut child = make_child(mob.kind).unwrap();
    child.set_position_and_angles(mob.x, mob.y, mob.z, mob.yaw, mob.pitch);
    set_breeding_age(&mut child, -24000);
    draw_hearts(mob, world);
    world.add_mob(child);
}

/// The 7 heart particle draws: three gaussians then three floats per heart,
/// from the animal's own RNG (`AnimalEntity.breed` / `AnimalBreedGoal.breed`).
fn draw_hearts(mob: &mut Mob, world: &mut dyn MobWorld) {
    for _ in 0..7 {
        let vx = mob.rng.next_gaussian() * 0.02;
        let vy = mob.rng.next_gaussian() * 0.02;
        let vz = mob.rng.next_gaussian() * 0.02;
        let px = mob.x + (mob.rng.next_float() * mob.width * 2.0 - mob.width) as f64;
        let py = mob.y + 0.5 + (mob.rng.next_float() * mob.height) as f64;
        let pz = mob.z + (mob.rng.next_float() * mob.width * 2.0 - mob.width) as f64;
        world.add_particle("heart", px, py, pz, vx, vy, vz);
    }
}

/// `AnimalEntity.makeChild` at the type level: the child kind equals the parents'.
pub fn make_child(kind: MobKind) -> Option<Mob> {
    Some(Mob::new(kind, 0.0, 0.0, 0.0))
}

/// `AnimalEntity.canSpawn()` — on grass with light above 8, plus the pathfinder check.
pub fn animal_can_spawn(mob: &Mob, world: &dyn MobWorld) -> bool {
    let var1 = math::floor(mob.x);
    let var2 = math::floor(mob.min_y());
    let var3 = math::floor(mob.z);
    block_by_id(world.get_block(var1, var2 - 1, var3)).is_grass
        && world.actual_light(var1, var2, var3) > 8
        && mob.pathfinder_can_spawn(world)
}

/// `AnimalBreedGoal` — the goal-selector form of the same behavior.
pub struct AnimalBreedGoal {
    /// The animal running the goal, by id into the world's mob list.
    pub animal_id: u32,
    pub speed_modifier: f64,
    mate: Option<u32>,
    breed_timer: i32,
}

impl AnimalBreedGoal {
    pub fn new(animal_id: u32, speed_modifier: f64) -> Self {
        AnimalBreedGoal {
            animal_id,
            speed_modifier,
            mate: None,
            breed_timer: 0,
        }
    }

    /// `findMate()`.
    fn find_mate(&self, mob: &Mob, world: &mut dyn MobWorld) -> Option<u32> {
        let mob_ref = mob.ref_snapshot();
        let mut best: Option<u32> = None;
        let mut best_dist = f64::MAX;
        for other in world.mobs_of_kind(mob.kind) {
            if can_breed_with(&mob_ref, &other) {
                let dist = mob.squared_distance_to(other.x, other.y, other.z);
                if dist < best_dist {
                    best_dist = dist;
                    best = Some(other.id);
                }
            }
        }
        best
    }
}

impl Goal for AnimalBreedGoal {
    fn controls(&self) -> u8 {
        3
    }

    /// `canStart()`.
    fn can_start(&mut self, mob: &mut Mob, world: &mut dyn MobWorld) -> bool {
        if !mob.is_in_love() {
            return false;
        }

        self.mate = self.find_mate(mob, world);
        self.mate.is_some()
    }

    /// `shouldContinue()`.
    fn should_continue(&mut self, _mob: &mut Mob, world: &mut dyn MobWorld) -> bool {
        let Some(mate) = self.mate else {
            return false;
        };
        match world.mob_ref(mate) {
            Some(mate_ref) => mate_ref.in_love && self.breed_timer < 60,
            None => false,
        }
    }

    /// `stop()`.
    fn stop(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {
        self.mate = None;
        self.breed_timer = 0;
    }

    /// `tick()`.
    fn tick(&mut self, mob: &mut Mob, world: &mut dyn MobWorld) {
        let Some(mate) = self.mate else {
            return;
        };
        let Some(mate_ref) = world.mob_ref(mate) else {
            return;
        };
        let (tx, ty, tz) = (mate_ref.x, mate_ref.y, mate_ref.z);
        mob.look_control.set_look_at(
            tx,
            ty + mate_ref.height as f64 * 0.85,
            tz,
            10.0,
            mob.get_look_pitch_speed(),
        );
        // snapshot the shape first: the navigation borrows it and the mob separately
        let shape = mob.shape();
        let speed = self.speed_modifier;
        mob.navigation
            .move_to_entity(&*world, &shape, tx, ty, tz, speed);
        self.breed_timer += 1;
        if self.breed_timer >= 60 {
            let dist_sq = mob.squared_distance_to(tx, ty, tz);
            if dist_sq < 9.0 {
                self.breed(mob, world, mate);
            }
        }
    }
}

impl AnimalBreedGoal {
    /// `breed()` — the goal's own breed path, with the experience orb.
    fn breed(&mut self, mob: &mut Mob, world: &mut dyn MobWorld, mate: u32) {
        if make_child(mob.kind).is_none() {
            return;
        }
        // stats and achievements are player-side; the port keeps the love-player chain
        set_breeding_age(mob, 6000);
        mob.in_love_timer = 0;
        world.finish_breeding(mate);

        let mut child = make_child(mob.kind).unwrap();
        child.set_position_and_angles(mob.x, mob.y, mob.z, 0.0, 0.0);
        set_breeding_age(&mut child, -24000);
        draw_hearts(mob, world);
        world.add_mob(child);

        if world.game_rule("doMobLoot") {
            let xp = mob.rng.next_int_bound(7).expect("bound > 0") + 1;
            world.add_particle("xp", mob.x, mob.y, mob.z, 0.0, 0.0, xp as f64);
        }
    }
}

/// `MobEntity.getLookPitchSpeed()`.
pub fn look_pitch_speed() -> f32 {
    40.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{FixtureWorld, mob_at};

    #[test]
    fn breeding_age_counts_up_from_negative() {
        let mut cow = mob_at(0.0, 0.0, 0.0);
        cow.kind = MobKind::Cow;
        set_breeding_age(&mut cow, -24000);
        assert!(cow.is_baby());
        assert!((cow.width - 0.45).abs() < 1e-5, "baby box halves");
        for _ in 0..24000 {
            passive_mob_tick(&mut cow);
        }
        assert_eq!(cow.breeding_age, 0);
        assert!(!cow.is_baby());
        assert!((cow.width - 0.9).abs() < 1e-5, "adult box restored");
    }

    #[test]
    fn breeding_age_counts_down_to_zero() {
        let mut cow = mob_at(0.0, 0.0, 0.0);
        cow.kind = MobKind::Cow;
        set_breeding_age(&mut cow, 6000);
        for _ in 0..6000 {
            passive_mob_tick(&mut cow);
        }
        assert_eq!(cow.breeding_age, 0);
        passive_mob_tick(&mut cow);
        assert_eq!(
            cow.breeding_age, 0,
            "never positive past the set value... stays clamped at 0"
        );
    }

    #[test]
    fn grow_up_never_positive() {
        let mut cow = mob_at(0.0, 0.0, 0.0);
        cow.kind = MobKind::Cow;
        set_breeding_age(&mut cow, -12000);
        grow_up(&mut cow, 300);
        assert_eq!(cow.breeding_age, -6000);
        grow_up(&mut cow, 1000);
        assert_eq!(cow.breeding_age, 0, "clamped at 0");
    }

    #[test]
    fn love_expires_and_blocks_at_nonzero_age() {
        let mut cow = mob_at(0.0, 0.0, 0.0);
        cow.kind = MobKind::Cow;
        let mut world = FixtureWorld::new();
        let mut rng = LegacyRandom::new(0);
        love_player(&mut cow, 0);
        assert!(cow.is_in_love());
        set_breeding_age(&mut cow, 6000);
        animal_mob_tick(&mut cow, &mut rng, &mut world);
        assert_eq!(cow.in_love_timer, 0, "age != 0 clears love");
    }

    #[test]
    fn love_timer_counts_down() {
        let mut cow = mob_at(0.0, 0.0, 0.0);
        cow.kind = MobKind::Cow;
        let mut world = FixtureWorld::new();
        let mut rng = LegacyRandom::new(0);
        love_player(&mut cow, 0);
        animal_mob_tick(&mut cow, &mut rng, &mut world);
        assert_eq!(cow.in_love_timer, 599);
    }

    #[test]
    fn breed_requires_same_kind_in_love() {
        let a = MobRef {
            id: 1,
            kind: MobKind::Cow,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.9,
            height: 1.3,
            breeding_age: 0,
            in_love: true,
            walk_target: None,
            love_causing_player: None,
        };
        let mut b = a;
        b.id = 2;
        assert!(can_breed_with(&a, &b));
        b.in_love = false;
        assert!(!can_breed_with(&a, &b));
        b.kind = MobKind::Pig;
        b.in_love = true;
        assert!(!can_breed_with(&a, &b));
        let self_ref = a;
        assert!(!can_breed_with(&a, &self_ref), "cannot breed with self");
    }

    #[test]
    fn animal_spawn_needs_grass_and_light() {
        let mut world = FixtureWorld::new();
        world.set_block(0, 0, 0, 2); // grass
        world.set_light(0, 1, 0, 9);
        let cow = mob_at(0.5, 1.0, 0.5);
        assert!(animal_can_spawn(&cow, &world));
        world.set_light(0, 1, 0, 7);
        assert!(!animal_can_spawn(&cow, &world));
        world.set_light(0, 1, 0, 9);
        world.set_block(0, 0, 0, 3); // dirt
        assert!(!animal_can_spawn(&cow, &world));
    }
}
