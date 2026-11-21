//! Deterministic combat probe for later parity checks: fixed seeds in, fixed
//! numbers out. Run with `cargo +1.99.1 run -p sunset-1-7-combat --bin combat_probe`.

use sunset_1_7_combat::MobType;
use sunset_1_7_combat::armor::{ArmorMaterial, ArmorPiece};
use sunset_1_7_combat::attack;
use sunset_1_7_combat::damage::{DamageSource, Difficulty, sources};
use sunset_1_7_combat::enchant::ids;
use sunset_1_7_combat::hunger::HungerManager;
use sunset_1_7_combat::living::{DamageContext, INVULNERABLE_TICKS, Living};
use sunset_1_7_combat::xp::{
    ExperienceOrb, PlayerXp, next_level_experience, player_death_orbs, round_orb_size,
    split_into_orbs,
};
use sunset_1_7_core::LegacyRandom;

fn main() {
    melee();
    armor_pipeline();
    hunger();
    xp();
    orbs();
}

fn melee() {
    println!("== melee ==");
    let bare = attack::AttackInput {
        base_damage: 1.0,
        weapon_enchants: &[],
        target_mob_type: MobType::Undefined,
        target_is_living: true,
        target_on_fire: false,
        sprinting: false,
        crit_conditions: false,
    };
    let punch = attack::melee_attack(&bare);
    println!("punch damage={} kb={}", punch.damage, punch.knockback_level);

    let iron_sword: [(i16, i16); 0] = [];
    let crit_in = attack::AttackInput {
        base_damage: 4.0,
        weapon_enchants: &iron_sword,
        crit_conditions: true,
        ..bare
    };
    let crit = attack::melee_attack(&crit_in);
    println!("crit sword damage={} crit={}", crit.damage, crit.crit);

    let sharp: Vec<(i16, i16)> = vec![(ids::SHARPNESS, 4), (ids::KNOCKBACK, 2)];
    let smite_in = attack::AttackInput {
        base_damage: 7.0,
        weapon_enchants: &sharp,
        target_mob_type: MobType::Undead,
        sprinting: true,
        crit_conditions: true,
        ..bare
    };
    let smite = attack::melee_attack(&smite_in);
    println!(
        "sharp4+kb2 sprint crit vs undead damage={} kb={} crit={}",
        smite.damage, smite.knockback_level, smite.crit
    );

    let fire: Vec<(i16, i16)> = vec![(ids::FIRE_ASPECT, 2)];
    let fire_in = attack::AttackInput {
        base_damage: 6.0,
        weapon_enchants: &fire,
        target_mob_type: MobType::Undefined,
        ..bare
    };
    let fire_out = attack::melee_attack(&fire_in);
    println!(
        "fire aspect 2 damage={} fire_armed={} fire_ticks={:?}",
        fire_out.damage, fire_out.fire_armed, fire_out.fire_ticks
    );

    println!(
        "kb velocity yaw=-90 level2 = {:?}",
        attack::attack_knockback_velocity(-90.0, 2)
    );
}

fn armor_pipeline() {
    println!("== armor pipeline ==");
    let mut rng = LegacyRandom::new(2026);
    let mut victim = Living::new(20.0);
    victim.is_player = true;
    victim.armor.slots = [
        Some(ArmorPiece::new(ArmorMaterial::Diamond, 0)),
        Some(ArmorPiece::new(ArmorMaterial::Diamond, 1)),
        Some(ArmorPiece::new(ArmorMaterial::Diamond, 2)),
        {
            let mut helmet = ArmorPiece::new(ArmorMaterial::Diamond, 3);
            helmet.enchants = vec![(ids::PROTECTION, 4)];
            Some(helmet)
        },
    ];
    println!("protection points={}", victim.armor.total_protection());

    let hit = sources::CACTUS.set_attacker(1);
    let result = victim.take_damage(
        &hit,
        8.0,
        &mut DamageContext {
            rng: &mut rng,
            difficulty: Difficulty::Normal,
            attacker_delta: Some((0.0, 4.0)),
            blocking: false,
            creative: false,
        },
    );
    println!(
        "diamond+prot4 cactus 8.0: health={:.4} damage={:.4} exhaustion={:.2} timer={} swing={:.4}",
        victim.health,
        result.damage,
        result.exhaustion,
        victim.invulnerable_timer,
        victim.damaged_swing_dir
    );

    // The second cactus hit inside the window is refused.
    let again = victim.take_damage(
        &hit,
        8.0,
        &mut DamageContext {
            rng: &mut rng,
            difficulty: Difficulty::Normal,
            attacker_delta: Some((0.0, 4.0)),
            blocking: false,
            creative: false,
        },
    );
    println!(
        "same hit again: applied={} health={:.4}",
        again.applied, victim.health
    );

    // A bigger hit lands its difference.
    let more = victim.take_damage(
        &hit,
        11.0,
        &mut DamageContext {
            rng: &mut rng,
            difficulty: Difficulty::Normal,
            attacker_delta: Some((0.0, 4.0)),
            blocking: false,
            creative: false,
        },
    );
    println!(
        "11.0 in window: applied={} health={:.4}",
        more.applied, victim.health
    );

    // Falling anvil with a helmet.
    let mut anvil_victim = Living::new(20.0);
    anvil_victim.is_player = true;
    anvil_victim.armor.slots[3] = Some(ArmorPiece::new(ArmorMaterial::Iron, 3));
    let result = anvil_victim.take_damage(
        &sources::ANVIL,
        10.0,
        &mut DamageContext {
            rng: &mut rng,
            difficulty: Difficulty::Normal,
            attacker_delta: None,
            blocking: false,
            creative: false,
        },
    );
    println!(
        "anvil 10.0 iron helmet: health={:.4} damage={:.4} helmet={}",
        anvil_victim.health,
        result.damage,
        anvil_victim.armor.slots[3].is_some()
    );

    // Knockback against resistance.
    let mut pushed = Living::new(20.0);
    pushed.velocity = [0.1, 0.3, -0.1];
    pushed.apply_knockback(2.0, 0.0, &mut rng);
    println!(
        "knockback dx=2 velocity=({:.6}, {:.6}, {:.6})",
        pushed.velocity[0], pushed.velocity[1], pushed.velocity[2]
    );

    println!("melee exhaustion={}", attack::HIT_EXHAUSTION);
    println!("invulnerable_ticks={}", INVULNERABLE_TICKS);
}

fn hunger() {
    println!("== hunger ==");
    let mut hunger = HungerManager::new();
    hunger.add(8, 0.8); // steak
    println!(
        "after steak: food={} sat={}",
        hunger.food_level, hunger.saturation_level
    );

    hunger.saturation_level = 0.0;
    hunger.food_level = 19;
    let mut player = Living::new(20.0);
    player.set_health(16.0);
    for tick in 1..=250 {
        if player.invulnerable_timer > 0 {
            player.invulnerable_timer -= 1;
        }

        let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
        if outcome.healed {
            println!(
                "tick {tick}: healed to {:.1} exhaust={}",
                player.health, hunger.exhaustion
            );
        }
    }

    // Starvation on hard.
    let mut hunger = HungerManager::new();
    hunger.food_level = 0;
    hunger.saturation_level = 0.0;
    let mut player = Living::new(20.0);
    let mut rng = LegacyRandom::new(5);
    let mut starve_ticks = 0;
    for _ in 0..80 * 30 {
        if player.invulnerable_timer > 0 {
            player.invulnerable_timer -= 1;
        }

        if hunger.tick(&mut player, Difficulty::Hard, false).starve_due {
            starve_ticks += 1;
            let mut ctx = DamageContext {
                rng: &mut rng,
                difficulty: Difficulty::Normal,
                attacker_delta: None,
                blocking: false,
                creative: false,
            };
            player.take_damage(&sources::STARVE, 1.0, &mut ctx);
        }
    }

    println!(
        "hard starve: hits={starve_ticks} health={:.1}",
        player.health
    );
}

fn xp() {
    println!("== xp ==");
    println!(
        "curve: l0={} l15={} l29={} l30={} l40={}",
        next_level_experience(0),
        next_level_experience(15),
        next_level_experience(29),
        next_level_experience(30),
        next_level_experience(40)
    );

    let mut xp = PlayerXp {
        ticks: 1000,
        ..Default::default()
    };
    xp.increase_xp(313);
    println!(
        "313 points: level={} total={} progress={:.6}",
        xp.level, xp.total, xp.progress
    );
    println!("level 3 death orbs={:?}", player_death_orbs(3, false));
    println!("level 40 death orbs={:?}", player_death_orbs(40, false));
    println!(
        "round sizes: 2={} 7={} 20={} 200={} 3000={}",
        round_orb_size(2),
        round_orb_size(7),
        round_orb_size(20),
        round_orb_size(200),
        round_orb_size(3000)
    );

    let mut xp = PlayerXp {
        ticks: 1000,
        ..Default::default()
    };
    let mut rng = LegacyRandom::new(77);
    let mut taken = 0;
    let values = [5, 9, 3, 7, 12];
    for value in values {
        xp.cooldown = 0;
        if let Some(v) = sunset_1_7_combat::xp::orb_pickup(&mut xp, value, &mut rng) {
            taken += v;
        }
    }

    println!(
        "orb pickups: level={} total={} sum={}",
        xp.level, xp.total, taken
    );
}

fn orbs() {
    println!("== orbs ==");
    let mut rng = LegacyRandom::new(99);
    let mut orb = ExperienceOrb::new([0.5, 64.0, 0.5], 17, &mut rng);
    println!(
        "spawn velocity=({:.6}, {:.6}, {:.6})",
        orb.velocity[0], orb.velocity[1], orb.velocity[2]
    );

    orb.pickup_delay = 0;
    for _ in 1..=10 {
        orb.pre_move(false, &mut rng);
        orb.position[0] += orb.velocity[0];
        orb.position[1] += orb.velocity[1];
        orb.position[2] += orb.velocity[2];
        let pull = orb.attraction([0.5, 64.0, 8.0], 1.62);
        orb.velocity[0] += pull[0];
        orb.velocity[1] += pull[1];
        orb.velocity[2] += pull[2];
        orb.post_move(false, 0.6);
        if !orb.age_tick() {
            break;
        }
    }

    println!(
        "after 10 ticks: pos=({:.6}, {:.6}, {:.6}) velocity=({:.6}, {:.6}, {:.6})",
        orb.position[0],
        orb.position[1],
        orb.position[2],
        orb.velocity[0],
        orb.velocity[1],
        orb.velocity[2]
    );

    let orbs = split_into_orbs(313);
    println!("313 split={orbs:?} sum={}", orbs.iter().sum::<i32>());
    let _ = next_level_experience(0);
    let _ = DamageSource::attacker_source("player", 0);
}
