//! The commands probe: deterministic output for later parity checks.

use sunset_1_7_commands::registry::builtin_registry;
use sunset_1_7_commands::world::{CommandServer, Player, Source, Vec3i};

fn main() {
    let mut server = CommandServer::single(-4172829371415774343);
    let mut alice = Player::new("Alice");
    alice.op = true;
    alice.x = 10.0;
    alice.y = 64.0;
    alice.z = 20.0;
    let mut bob = Player::new("Bob");
    bob.x = 30.0;
    bob.y = 64.0;
    bob.z = 40.0;
    server.players.push(alice);
    server.players.push(bob);
    server.worlds[0].loaded_chunks.push((0, 0));
    server.worlds[0].loaded_chunks.push((10, 20));
    server.worlds[0].loaded_chunks.push((10, 21));
    server.worlds[0].loaded_chunks.push((10, 22));
    server.worlds[0].loaded_chunks.push((30, 40));

    let mut registry = builtin_registry();
    let console = Source::Console;
    let alice_src = Source::Player(0);

    let lines: Vec<(&Source, String)> = vec![
        (&console, "/seed".into()),
        (&alice_src, "/time set day".into()),
        (&alice_src, "/time add 500".into()),
        (&alice_src, "/time set night".into()),
        (&alice_src, "/time add -5".into()),
        (&alice_src, "/gamemode creative".into()),
        (&alice_src, "/gamemode 1 Bob".into()),
        (&alice_src, "/gamemode s Bob".into()),
        (&alice_src, "/gamemode 3".into()),
        (&alice_src, "/difficulty hard".into()),
        (&alice_src, "/difficulty 2".into()),
        (&alice_src, "/difficulty 9".into()),
        (&console, "/give Alice diamond_sword".into()),
        (&console, "/give Alice minecraft:diamond_sword".into()),
        (&console, "/give Bob minecraft:diamond 5".into()),
        (&console, "/give Alice minecraft:diamond 5".into()),
        (&console, "/give Alice 264 3 0".into()),
        (&console, "/give Alice diamond 65".into()),
        (&console, "/give Alice nope".into()),
        (&console, "/enchant Alice 16 5".into()),
        (&console, "/enchant Alice 19 3".into()),
        (&console, "/enchant Alice 99 1".into()),
        (&alice_src, "/clear Bob".into()),
        (&alice_src, "/clear Bob minecraft:diamond".into()),
        (&alice_src, "/clear Nobody".into()),
        (&alice_src, "/tp Bob 1 64 2".into()),
        (&alice_src, "/tp Bob ~1 ~ ~-1".into()),
        (&alice_src, "/tp Bob Alice".into()),
        (&alice_src, "/tp Bob".into()),
        (&alice_src, "/tp".into()),
        (&alice_src, "/spawnpoint Bob".into()),
        (&alice_src, "/spawnpoint Bob 1 65 2".into()),
        (&console, "/weather clear".into()),
        (&console, "/weather thunder 100".into()),
        (&console, "/weather rain".into()),
        (&console, "/toggledownfall".into()),
        (&alice_src, "/kill Bob".into()),
        (&alice_src, "/effect Bob 1 30 2".into()),
        (&alice_src, "/effect Bob 6 5".into()),
        (&alice_src, "/effect Bob clear".into()),
        (&alice_src, "/effect Bob 99 5".into()),
        (&console, "/xp 100 Alice".into()),
        (&console, "/xp 5l Alice".into()),
        (&console, "/xp -3l Alice".into()),
        (&console, "/xp -5 Alice".into()),
        (&alice_src, "/gamerule".into()),
        (&alice_src, "/gamerule doDaylightCycle".into()),
        (&alice_src, "/gamerule doDaylightCycle false".into()),
        (&alice_src, "/gamerule doDaylightCycle".into()),
        (&alice_src, "/gamerule custom true".into()),
        (
            &alice_src,
            "/scoreboard objectives add coins dummy Money".into(),
        ),
        (&alice_src, "/scoreboard objectives add coins dummy".into()),
        (
            &alice_src,
            "/scoreboard objectives add coins2 health".into(),
        ),
        (&alice_src, "/scoreboard players set Bob coins 10".into()),
        (&alice_src, "/scoreboard players add Bob coins 5".into()),
        (&alice_src, "/scoreboard players remove Bob coins 2".into()),
        (&alice_src, "/scoreboard players list Bob".into()),
        (&alice_src, "/scoreboard players list".into()),
        (&alice_src, "/scoreboard teams add red".into()),
        (&alice_src, "/scoreboard teams join red Bob".into()),
        (&alice_src, "/scoreboard teams list red".into()),
        (&alice_src, "/scoreboard teams option red color red".into()),
        (
            &alice_src,
            "/scoreboard objectives setdisplay sidebar coins".into(),
        ),
        (&alice_src, "/scoreboard objectives list".into()),
        (&alice_src, "/setblock 10 64 20 minecraft:stone".into()),
        (
            &alice_src,
            "/setblock 10 64 20 minecraft:stone 0 keep".into(),
        ),
        (
            &alice_src,
            "/setblock 10 64 20 minecraft:stone 0 destroy".into(),
        ),
        (
            &alice_src,
            "/setblock 10 64 20 minecraft:chest 0 replace".into(),
        ),
        (&alice_src, "/summon Zombie 10 64 20".into()),
        (&alice_src, "/summon Creeper 10 64 21".into()),
        (&alice_src, "/summon Nope".into()),
        (
            &alice_src,
            "/summon MinecartRideable 10 64 22 {Type:1}".into(),
        ),
        (&alice_src, "/help".into()),
        (&alice_src, "/help 1".into()),
        (&alice_src, "/help 99".into()),
        (&alice_src, "/help nope".into()),
        (&alice_src, "/nope".into()),
    ];

    for (source, line) in lines {
        let count = registry.run(&mut server, source, &line);
        println!("> {line} -> {count} success(es)");
    }

    println!("--- player messages ---");
    for player in &server.players {
        println!("{} ({}):", player.name, player.display_name());
        for message in &player.messages {
            println!(
                "  [{}] {}",
                message.plain(),
                if message.italic { "(italic)" } else { "" }
            );
        }
    }
    println!("--- console ---");
    for message in &server.console {
        println!("  {}", message.plain());
    }
    println!("--- world state ---");
    let world = &server.worlds[0];
    println!("time {} seed {}", world.time_of_day, world.seed);
    println!("raining {} thundering {}", world.raining, world.thundering);
    println!(
        "difficulty {} default_gamemode {}",
        server.difficulty.key(),
        server.default_game_mode.key()
    );
    let mut block_list: Vec<String> = world
        .blocks
        .iter()
        .map(|((x, y, z), state)| format!("{x},{y},{z} -> {} meta {}", state.id, state.meta))
        .collect();
    block_list.sort();
    for line in block_list {
        println!("block {line}");
    }
    for entity in &world.entities {
        println!(
            "entity {} at {} {} {}",
            entity.id, entity.x, entity.y, entity.z
        );
    }
    for player in &server.players {
        println!(
            "spawn {} {:?} health {} xp {} levels {} points {} mode {}",
            player.name,
            player.spawn_point,
            player.health,
            player.xp_points,
            player.xp_level,
            player.game_mode.id(),
            player.game_mode.key()
        );
    }
    let _ = Vec3i::default();
}
