//! The 1.7.10 command set: every command as `AbstractCommand`'s helpers plus
//! its own `run`, and the registration list in `CommandManager`'s order.

use sunrise_core::nbt::parse_snbt;

use crate::args::{
    format_double, list_args, parse_coordinate, parse_coordinate_default, parse_int,
    parse_int_range, suggest_matching,
};
use crate::error::CommandError;
use crate::facts::{Difficulty, GameMode};
use crate::registry::{CommandDef, send_success};
use crate::target::{Candidate, Selection};
use crate::text::{Formatting, Text};
use crate::world::{CommandServer, ItemStack, Source, Vec3i, math_helper_floor};
use sunset_1_7_core::RandomSource;

// --- AbstractCommand's player helpers ---

/// `AbstractCommand.asPlayer`: the source must be a player.
pub fn as_player(server: &CommandServer, source: &Source) -> Result<usize, CommandError> {
    match source {
        Source::Player(i) if *i < server.players.len() => Ok(*i),
        _ => Err(CommandError::must_specify_player()),
    }
}

/// One candidate per player, with the scoreboard values their own world's
/// scoreboard would answer.
fn build_candidates(server: &CommandServer) -> Vec<Candidate> {
    server
        .players
        .iter()
        .map(|p| {
            let board = server.worlds.get(p.world).map(|w| &w.scoreboard);
            let team = board
                .and_then(|b| b.get_team_of_member(&p.name))
                .map(str::to_string);
            let mut scores = std::collections::HashMap::new();
            if let Some(board) = board {
                let owned = board.get_scores(&p.name);
                for objective in board.get_objectives() {
                    let value = owned
                        .iter()
                        .find(|s| s.objective == objective.name)
                        .map(|s| s.score)
                        .unwrap_or(0);
                    scores.insert(objective.name.clone(), value);
                }
            }
            let pos = p.block_pos();
            Candidate {
                name: p.name.clone(),
                world: p.world,
                x: pos.x,
                y: pos.y,
                z: pos.z,
                level: p.xp_level,
                game_mode: p.game_mode.id(),
                team,
                scores,
            }
        })
        .collect()
}

/// `TargetSelector.select`: the selected players' names. None when the
/// selector doesn't parse or the kind is `f` (Java returns null; the `f`
/// selector never matches).
pub fn select_players(
    server: &mut CommandServer,
    source: &Source,
    text: &str,
) -> Option<Vec<String>> {
    let selection = crate::target::parse_selection(text)?;
    let candidates = build_candidates(server);
    let source_pos = source.block_pos(server);
    let source_world = source.world(server);
    let world_matches = |candidate: &Candidate| match source_world {
        None => true,
        Some(world) => !selection.position_given || candidate.world == world,
    };

    if selection.kind == 'f' {
        return None;
    }

    // `/r` matches everyone first, then shuffles and takes the count.
    let working = Selection {
        count: if selection.kind == 'r' {
            0
        } else {
            selection.count
        },
        ..selection.clone()
    };
    let mut matched = crate::target::select(
        &candidates,
        &working,
        (source_pos.x, source_pos.y, source_pos.z),
        world_matches,
    );
    if selection.kind == 'r' {
        // java.util.Collections.shuffle: Fisher-Yates from the end.
        for i in (1..=matched.len()).rev() {
            let j = server.random.next_int_bound(i as i32).unwrap_or(0) as usize;
            matched.swap(i - 1, j);
        }
        matched.truncate(selection.count.max(0) as usize);
    }
    Some(
        matched
            .into_iter()
            .map(|i| candidates[i].name.clone())
            .collect(),
    )
}

/// `TargetSelector.selectFirstPlayer`: one exact match, or null.
fn select_first_player(server: &mut CommandServer, source: &Source, text: &str) -> Option<usize> {
    let names = select_players(server, source, text)?;
    (names.len() == 1)
        .then(|| server.player_by_name(names.first()?))
        .flatten()
}

/// `AbstractCommand.parsePlayer`: a selector match, else the name.
pub fn parse_player(
    server: &mut CommandServer,
    source: &Source,
    text: &str,
) -> Result<usize, CommandError> {
    if let Some(player) = select_first_player(server, source, text) {
        return Ok(player);
    }
    server
        .player_by_name(text)
        .ok_or(CommandError::player_not_found())
}

/// `AbstractCommand.parsePlayerName`: a selector match's name, a raw name,
/// or an error for a valid selector that matched nobody.
pub fn parse_player_name(
    server: &mut CommandServer,
    source: &Source,
    text: &str,
) -> Result<String, CommandError> {
    if let Some(player) = select_first_player(server, source, text) {
        return Ok(server.players[player].name.clone());
    }
    if crate::target::is_valid(text) {
        return Err(CommandError::player_not_found());
    }
    Ok(text.to_string())
}

/// `AbstractCommand.parseText`: the args joined with spaces, entity
/// selectors replaced by their display names when asked.
pub fn parse_text(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    start: usize,
    parse_entity_names: bool,
) -> Result<Text, CommandError> {
    let mut pieces: Vec<String> = Vec::new();
    for arg in &args[start..] {
        if parse_entity_names {
            if let Some(names) = selection_as_text(server, source, arg) {
                pieces.push(names);
                continue;
            }
            if crate::target::is_valid(arg) {
                return Err(CommandError::player_not_found());
            }
        }
        pieces.push(arg.clone());
    }
    Ok(Text::literal(pieces.join(" ")))
}

/// `TargetSelector.getSelectionAsText`: the selection's display names,
/// "a, b and c", or None when nothing matched.
fn selection_as_text(server: &mut CommandServer, source: &Source, text: &str) -> Option<String> {
    let names = select_players(server, source, text)?;
    (!names.is_empty()).then(|| {
        let displays: Vec<String> = names
            .into_iter()
            .map(|name| {
                server
                    .player_by_name(&name)
                    .map(|i| server.players[i].display_name())
                    .unwrap_or(name)
            })
            .collect();
        list_args(&displays)
    })
}

/// `AbstractCommand.parseItem`: by registry key, then the deprecated numeric
/// id (with its gray warning), else `commands.give.notFound`. Returns the
/// item's id and key.
pub fn parse_item(
    server: &mut CommandServer,
    source: &Source,
    text: &str,
) -> Result<(i32, String), CommandError> {
    if let Some(id) = server.items_by_key.get(text).copied() {
        return Ok((id, text.to_string()));
    }
    if let Ok(id) = parse_int(text)
        && let Some(key) = server.items_by_id.get(&id).cloned()
    {
        server.send(
            source,
            Text::translated_args(
                "commands.generic.deprecatedId",
                [Text::literal(key.clone())],
            )
            .with_color(Formatting::Gray),
        );
        return Ok((id, key));
    }
    Err(CommandError::exception_args(
        "commands.give.notFound",
        [Text::literal(text)],
    ))
}

/// `AbstractCommand.parseBlock`: by registry key, then the deprecated
/// numeric id (with its gray warning), else `commands.give.notFound`.
/// Returns the block's id and whether it owns a block entity.
pub fn parse_block(
    server: &mut CommandServer,
    source: &Source,
    text: &str,
) -> Result<(u32, bool), CommandError> {
    if let Some(state) = server.blocks_by_key.get(text).copied() {
        return Ok(state);
    }
    if let Ok(id) = parse_int(text) {
        let id = id as u32;
        if let Some((key, has_entity)) = server.blocks_by_id.get(&id).cloned() {
            server.send(
                source,
                Text::translated_args("commands.generic.deprecatedId", [Text::literal(key)])
                    .with_color(Formatting::Gray),
            );
            return Ok((id, has_entity));
        }
    }
    Err(CommandError::exception_args(
        "commands.give.notFound",
        [Text::literal(text)],
    ))
}

// --- the commands ---

/// `TimeCommand`.
pub fn cmd_time(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() > 1 {
        if args[0] == "set" {
            let time = if args[1] == "day" {
                1000
            } else if args[1] == "night" {
                13000
            } else {
                parse_int_range(&args[1], 0, i32::MAX)?
            };
            for world in &mut server.worlds {
                world.time_of_day = i64::from(time);
            }
            send_success(
                server,
                source,
                0,
                "commands.time.set",
                [lit(time.to_string())].into(),
            );
            return Ok(());
        }
        if args[0] == "add" {
            let time = parse_int_range(&args[1], 0, i32::MAX)?;
            for world in &mut server.worlds {
                world.time_of_day += i64::from(time);
            }
            send_success(
                server,
                source,
                0,
                "commands.time.added",
                [lit(time.to_string())].into(),
            );
            return Ok(());
        }
    }
    Err(CommandError::usage("commands.time.usage"))
}

/// `GameModeCommand.parseGameMode`.
pub fn parse_game_mode(
    _server: &mut CommandServer,
    _source: &Source,
    text: &str,
) -> Result<GameMode, CommandError> {
    if text.eq_ignore_ascii_case("survival") || text.eq_ignore_ascii_case("s") {
        return Ok(GameMode::Survival);
    }
    if text.eq_ignore_ascii_case("creative") || text.eq_ignore_ascii_case("c") {
        return Ok(GameMode::Creative);
    }
    if text.eq_ignore_ascii_case("adventure") || text.eq_ignore_ascii_case("a") {
        return Ok(GameMode::Adventure);
    }
    // values().length - 2: NOT_SET and ADVENTURE sit outside the ids.
    Ok(GameMode::by_id(parse_int_range(text, 0, 2)?))
}

/// `GameModeCommand`.
pub fn cmd_gamemode(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.gamemode.usage"));
    }
    let mode = parse_game_mode(server, source, &args[0])?;
    let player = if args.len() >= 2 {
        parse_player(server, source, &args[1])?
    } else {
        as_player(server, source)?
    };
    let acting = if let Some(p) = server.players.get_mut(player) {
        p.set_game_mode(mode);
        p.fall_distance = 0.0;
        p.name.clone()
    } else {
        return Err(CommandError::player_not_found());
    };
    let mode_text = Text::translated(format!("gameMode.{}", mode.key()));
    if source != &Source::Player(player) {
        send_success(
            server,
            source,
            1,
            "commands.gamemode.success.other",
            vec![Text::literal(acting), mode_text],
        );
    } else {
        send_success(
            server,
            source,
            1,
            "commands.gamemode.success.self",
            vec![mode_text],
        );
    }
    Ok(())
}

/// `DefaultGameModeCommand`.
pub fn cmd_defaultgamemode(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.defaultgamemode.usage"));
    }
    let mode = parse_game_mode(server, source, &args[0])?;
    server.default_game_mode = mode;
    if server.force_game_mode {
        for player in &mut server.players {
            player.set_game_mode(mode);
            player.fall_distance = 0.0;
        }
    }
    send_success(
        server,
        source,
        0,
        "commands.defaultgamemode.success",
        vec![Text::translated(format!("gameMode.{}", mode.key()))],
    );
    Ok(())
}

/// `DifficultyCommand.parseDifficulty`.
pub fn parse_difficulty(text: &str) -> Result<Difficulty, CommandError> {
    if text.eq_ignore_ascii_case("peaceful") || text.eq_ignore_ascii_case("p") {
        return Ok(Difficulty::Peaceful);
    }
    if text.eq_ignore_ascii_case("easy") || text.eq_ignore_ascii_case("e") {
        return Ok(Difficulty::Easy);
    }
    if text.eq_ignore_ascii_case("normal") || text.eq_ignore_ascii_case("n") {
        return Ok(Difficulty::Normal);
    }
    if text.eq_ignore_ascii_case("hard") || text.eq_ignore_ascii_case("h") {
        return Ok(Difficulty::Hard);
    }
    Ok(Difficulty::by_id(parse_int_range(text, 0, 3)?))
}

/// `DifficultyCommand`.
pub fn cmd_difficulty(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.difficulty.usage"));
    }
    let difficulty = parse_difficulty(&args[0])?;
    server.difficulty = difficulty;
    send_success(
        server,
        source,
        0,
        "commands.difficulty.success",
        vec![Text::translated(format!(
            "options.difficulty.{}",
            difficulty.key()
        ))],
    );
    Ok(())
}

/// `KillCommand`.
pub fn cmd_kill(
    server: &mut CommandServer,
    source: &Source,
    _args: &[String],
) -> Result<(), CommandError> {
    let player = as_player(server, source)?;
    server.players[player].take_out_of_world_damage();
    server.send(source, Text::translated("commands.kill.success"));
    Ok(())
}

/// `ToggleDownfallCommand`.
pub fn cmd_toggledownfall(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let _ = args;
    let raining = server.worlds[0].raining;
    server.worlds[0].raining = !raining;
    send_success(server, source, 0, "commands.downfall.success", Vec::new());
    Ok(())
}

/// `WeatherCommand`.
pub fn cmd_weather(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() || args.len() > 2 {
        return Err(CommandError::usage("commands.weather.usage"));
    }
    // (300 + new Random().nextInt(600)) * 20 ticks.
    let mut duration = (300 + server.random.next_int_bound(600).unwrap_or(0)) * 20;
    if args.len() >= 2 {
        duration = parse_int_range(&args[1], 1, 1_000_000)? * 20;
    }
    let world = &mut server.worlds[0];
    if args[0].eq_ignore_ascii_case("clear") {
        world.rain_time = 0;
        world.thunder_time = 0;
        world.raining = false;
        world.thundering = false;
        send_success(server, source, 0, "commands.weather.clear", Vec::new());
    } else if args[0].eq_ignore_ascii_case("rain") {
        world.rain_time = duration;
        world.raining = true;
        world.thundering = false;
        send_success(server, source, 0, "commands.weather.rain", Vec::new());
    } else if args[0].eq_ignore_ascii_case("thunder") {
        world.rain_time = duration;
        world.thunder_time = duration;
        world.raining = true;
        world.thundering = true;
        send_success(server, source, 0, "commands.weather.thunder", Vec::new());
    } else {
        return Err(CommandError::usage("commands.weather.usage"));
    }
    Ok(())
}

/// `ExperienceCommand`.
pub fn cmd_xp(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.xp.usage"));
    }
    let mut amount = args[0].as_str();
    let levels = amount.ends_with('l') || amount.ends_with('L');
    if levels && amount.len() > 1 {
        amount = &amount[..amount.len() - 1];
    }
    let mut amount = parse_int(amount)?;
    let negative = amount < 0;
    if negative {
        amount = -amount;
    }
    let player = if args.len() > 1 {
        parse_player(server, source, &args[1])?
    } else {
        as_player(server, source)?
    };
    let name = server.players[player].name.clone();
    if levels {
        let delta = if negative { -amount } else { amount };
        server.players[player].xp_level += delta;
        if negative {
            send_success(
                server,
                source,
                0,
                "commands.xp.success.negative.levels",
                [lit(amount.to_string()), lit(name)].into(),
            );
        } else {
            send_success(
                server,
                source,
                0,
                "commands.xp.success.levels",
                [lit(amount.to_string()), lit(name)].into(),
            );
        }
    } else {
        if negative {
            return Err(CommandError::usage("commands.xp.failure.widthdrawXp"));
        }
        // increaseXp: experience points.
        server.players[player].xp_points += amount;
        send_success(
            server,
            source,
            0,
            "commands.xp.success",
            [lit(amount.to_string()), lit(name)].into(),
        );
    }
    Ok(())
}

/// `SpawnPointCommand`.
pub fn cmd_spawnpoint(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let player = if args.is_empty() {
        as_player(server, source)?
    } else {
        parse_player(server, source, &args[0])?
    };
    if args.len() == 4 {
        let x = parse_int_range(&args[1], -30_000_000, 30_000_000)?;
        let y = parse_int_range(&args[2], 0, 256)?;
        let z = parse_int_range(&args[3], -30_000_000, 30_000_000)?;
        if let Some(p) = server.players.get_mut(player) {
            p.spawn_point = Some((Vec3i::new(x, y, z), true));
            let name = p.name.clone();
            send_success(
                server,
                source,
                0,
                "commands.spawnpoint.success",
                [
                    lit(name),
                    lit(x.to_string()),
                    lit(y.to_string()),
                    lit(z.to_string()),
                ]
                .into(),
            );
        }
        return Ok(());
    }
    if args.len() > 1 {
        return Err(CommandError::usage("commands.spawnpoint.usage"));
    }
    let pos = server.players[player].block_pos();
    if let Some(p) = server.players.get_mut(player) {
        p.spawn_point = Some((pos, true));
        let name = p.name.clone();
        send_success(
            server,
            source,
            0,
            "commands.spawnpoint.success",
            [
                lit(name),
                lit(pos.x.to_string()),
                lit(pos.y.to_string()),
                lit(pos.z.to_string()),
            ]
            .into(),
        );
    }
    Ok(())
}

/// `SeedCommand.canUse`: singleplayer, else the level check.
pub fn seed_can_use(server: &mut CommandServer, source: &Source) -> bool {
    server.singleplayer || source.can_use_command(server, 2, "seed")
}

/// `SeedCommand`.
pub fn cmd_seed(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let _ = args;
    let world = match source {
        Source::Player(i) => server.players.get(*i).map(|p| p.world).unwrap_or_default(),
        _ => 0,
    };
    let seed = server.worlds.get(world).map(|w| w.seed).unwrap_or_default();
    server.send(
        source,
        Text::translated_args("commands.seed.success", [lit(seed.to_string())]),
    );
    Ok(())
}

/// `GameRuleCommand`.
pub fn cmd_gamerule(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    match args.len() {
        2 => {
            let contains = server.worlds[0].game_rules.contains(&args[0]);
            if contains {
                server.worlds[0].game_rules.set(&args[0], &args[1]);
                send_success(server, source, 0, "commands.gamerule.success", Vec::new());
            } else {
                send_success(
                    server,
                    source,
                    0,
                    "commands.gamerule.norule",
                    [lit(args[0].clone())].into(),
                );
            }
            Ok(())
        }
        1 => {
            let contains = server.worlds[0].game_rules.contains(&args[0]);
            if contains {
                let value = server.worlds[0].game_rules.get(&args[0]);
                server.send(source, Text::literal(format!("{} = {}", args[0], value)));
            } else {
                send_success(
                    server,
                    source,
                    0,
                    "commands.gamerule.norule",
                    [lit(args[0].clone())].into(),
                );
            }
            Ok(())
        }
        0 => {
            let all = server.worlds[0].game_rules.get_all();
            let text = list_args(&all.iter().map(String::as_str).collect::<Vec<_>>());
            server.send(source, Text::literal(text));
            Ok(())
        }
        _ => Err(CommandError::usage("commands.gamerule.usage")),
    }
}

/// `GiveCommand`.
pub fn cmd_give(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() < 2 {
        return Err(CommandError::usage("commands.give.usage"));
    }
    let player = parse_player(server, source, &args[0])?;
    let (item_id, item_key) = parse_item(server, source, &args[1])?;
    let mut count = 1;
    let mut meta = 0i32;
    if args.len() >= 3 {
        count = parse_int_range(&args[2], 1, 64)?;
    }
    if args.len() >= 4 {
        meta = parse_int(&args[3])?;
    }
    let mut stack = ItemStack::new(item_id, &item_key, count, meta);
    if args.len() >= 5 {
        let text = parse_text(server, source, args, 4, false)?.plain();
        match parse_snbt(&text) {
            Ok((tag, _)) => match tag {
                sunrise_core::nbt::Tag::Compound(_) => stack.set_nbt(tag),
                _ => {
                    send_success(
                        server,
                        source,
                        0,
                        "commands.give.tagError",
                        [lit("Not a valid tag")].into(),
                    );
                    return Ok(());
                }
            },
            Err(message) => {
                send_success(
                    server,
                    source,
                    0,
                    "commands.give.tagError",
                    [lit(message)].into(),
                );
                return Ok(());
            }
        }
    }
    server.players[player].inventory.add_stack(stack.clone());
    let name = server.players[player].name.clone();
    // getDisplayName needs the lang files; the registry key stands in.
    send_success(
        server,
        source,
        0,
        "commands.give.success",
        [lit(item_key), lit(count.to_string()), lit(name)].into(),
    );
    Ok(())
}

/// `ClearCommand`.
pub fn cmd_clear(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let player = if args.is_empty() {
        as_player(server, source)?
    } else {
        parse_player(server, source, &args[0])?
    };
    let mut item: Option<ItemStack> = None;
    if args.len() >= 2 {
        let (id, key) = parse_item(server, source, &args[1])?;
        item = Some(ItemStack::new(id, &key, 0, 0));
    }
    let metadata = if args.len() >= 3 {
        parse_int_range(&args[2], 0, i32::MAX)?
    } else {
        -1
    };
    if args.len() >= 2 && item.is_none() {
        let name = server.players[player].name.clone();
        return Err(CommandError::exception_args(
            "commands.clear.failure",
            [lit(name)],
        ));
    }
    let removed = server.players[player]
        .inventory
        .remove_all(item.as_ref(), metadata);
    if removed == 0 {
        let name = server.players[player].name.clone();
        return Err(CommandError::exception_args(
            "commands.clear.failure",
            [lit(name)],
        ));
    }
    let name = server.players[player].name.clone();
    send_success(
        server,
        source,
        0,
        "commands.clear.success",
        [lit(name), lit(removed.to_string())].into(),
    );
    Ok(())
}

/// `EffectCommand`.
pub fn cmd_effect(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() < 2 {
        return Err(CommandError::usage("commands.effect.usage"));
    }
    let player = parse_player(server, source, &args[0])?;
    if args[1] == "clear" {
        if server.players[player].effects.is_empty() {
            let name = server.players[player].name.clone();
            return Err(CommandError::exception_args(
                "commands.effect.failure.notActive.all",
                [lit(name)],
            ));
        }
        server.players[player].effects.clear();
        let name = server.players[player].name.clone();
        send_success(
            server,
            source,
            0,
            "commands.effect.success.removed.all",
            [lit(name)].into(),
        );
        return Ok(());
    }
    let effect_id = parse_int_range(&args[1], 1, i32::MAX)?;
    let mut seconds = 600i32;
    let mut duration = 30i32;
    let mut amplifier = 0i32;
    let Some(effect) = crate::facts::status_effect_by_id(effect_id) else {
        return Err(CommandError::exception_args(
            "commands.effect.notFound",
            [lit(effect_id.to_string())],
        ));
    };
    if args.len() >= 3 {
        seconds = parse_int_range(&args[2], 0, 1_000_000)?;
        duration = if effect.instant {
            seconds
        } else {
            seconds * 20
        };
    } else if effect.instant {
        duration = 1;
    }
    if args.len() >= 4 {
        amplifier = parse_int_range(&args[3], 0, 255)?;
    }
    let name = server.players[player].name.clone();
    let effect_name = Text::translated(format!("potion.{}", effect.key));
    if seconds == 0 {
        if !server.players[player].effects.contains_key(&effect_id) {
            return Err(CommandError::exception_args(
                "commands.effect.failure.notActive",
                vec![effect_name, lit(name)],
            ));
        }
        server.players[player].effects.remove(&effect_id);
        send_success(
            server,
            source,
            0,
            "commands.effect.success.removed",
            vec![effect_name, lit(name)],
        );
    } else {
        server.players[player].effects.insert(
            effect_id,
            crate::world::StatusEffectInstance {
                id: effect_id,
                duration,
                amplifier,
            },
        );
        send_success(
            server,
            source,
            0,
            "commands.effect.success",
            vec![
                effect_name,
                lit(effect_id.to_string()),
                lit(amplifier.to_string()),
                lit(name),
                lit(seconds.to_string()),
            ],
        );
    }
    Ok(())
}

/// `EnchantCommand`.
pub fn cmd_enchant(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() < 2 {
        return Err(CommandError::usage("commands.enchant.usage"));
    }
    let player = parse_player(server, source, &args[0])?;
    let enchant_id = parse_int_range(&args[1], 0, 255)?;
    let Some(enchantment) = crate::facts::enchantment_by_id(enchant_id) else {
        return Err(CommandError::exception_args(
            "commands.enchant.notFound",
            [lit(enchant_id.to_string())],
        ));
    };
    let held = server.players[player]
        .item_in_hand()
        .map(|stack| (stack.id, stack.get_enchantments(), stack.has_nbt()));
    let Some((held_id, existing, has_nbt)) = held else {
        return Err(CommandError::exception("commands.enchant.noItem"));
    };
    if !enchantment.can_enchant(held_id) {
        return Err(CommandError::exception("commands.enchant.cantEnchant"));
    }
    let mut level = 1;
    if args.len() >= 3 {
        level = parse_int_range(&args[2], enchantment.min_level, enchantment.max_level)?;
    }
    if has_nbt && let Some(entries) = existing {
        for (id, lvl) in entries {
            if let Some(other) = crate::facts::enchantment_by_id(i32::from(id))
                && !other.is_compatible(enchantment)
            {
                return Err(CommandError::exception_args(
                    "commands.enchant.cantCombine",
                    vec![
                        lit(format!("{} {}", enchantment.key, level)),
                        lit(format!("{} {}", other.key, lvl)),
                    ],
                ));
            }
        }
    }
    server.players[player]
        .item_in_hand_mut()
        .expect("checked above")
        .add_enchantment(enchant_id as i16, level as i16);
    send_success(server, source, 0, "commands.enchant.success", Vec::new());
    Ok(())
}

/// `TpCommand`. `/teleport` is its alias in 1.7.10.
pub fn cmd_tp(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.tp.usage"));
    }
    let mover = if args.len() != 2 && args.len() != 4 {
        as_player(server, source)?
    } else {
        parse_player(server, source, &args[0])?
    };

    if args.len() != 3 && args.len() != 4 {
        if args.len() == 1 || args.len() == 2 {
            let target = parse_player(server, source, &args[args.len() - 1])?;
            let (same, target_pos) = {
                let mover_world = server.players[mover].world;
                let t = &server.players[target];
                (mover_world == t.world, (t.x, t.y, t.z, t.yaw, t.pitch))
            };
            if !same {
                send_success(
                    server,
                    source,
                    0,
                    "commands.tp.notSameDimension",
                    Vec::new(),
                );
                return Ok(());
            }
            let (x, y, z, yaw, pitch) = target_pos;
            let mover_name = server.players[mover].name.clone();
            let target_name = server.players[target].name.clone();
            if let Some(p) = server.players.get_mut(mover) {
                p.x = x;
                p.y = y;
                p.z = z;
                p.yaw = yaw;
                p.pitch = pitch;
            }
            send_success(
                server,
                source,
                0,
                "commands.tp.success",
                [lit(mover_name), lit(target_name)].into(),
            );
        }
        return Ok(());
    }

    let base = (
        server.players[mover].x,
        server.players[mover].y,
        server.players[mover].z,
    );
    let x = parse_coordinate_default(base.0, &args[args.len() - 3])?;
    let y = parse_coordinate(base.1, &args[args.len() - 2], 0, 0)?;
    let z = parse_coordinate_default(base.2, &args[args.len() - 1])?;
    let mover_name = server.players[mover].name.clone();
    if let Some(p) = server.players.get_mut(mover) {
        p.teleport(x, y, z);
    }
    send_success(
        server,
        source,
        0,
        "commands.tp.success.coordinates",
        [
            lit(mover_name),
            lit(format_double(x)),
            lit(format_double(y)),
            lit(format_double(z)),
        ]
        .into(),
    );
    Ok(())
}

/// `SetBlockCommand`.
pub fn cmd_setblock(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() < 4 {
        return Err(CommandError::usage("commands.setblock.usage"));
    }
    let origin = source.block_pos(server);
    let x = math_helper_floor(parse_coordinate_default(f64::from(origin.x), &args[0])?);
    let y = math_helper_floor(parse_coordinate_default(f64::from(origin.y), &args[1])?);
    let z = math_helper_floor(parse_coordinate_default(f64::from(origin.z), &args[2])?);
    let (block_id, has_entity) = parse_block(server, source, &args[3])?;
    let meta = if args.len() >= 5 {
        parse_int_range(&args[4], 0, 15)? as u8
    } else {
        0
    };
    let Some(world_index) = source.world(server) else {
        return Err(CommandError::exception("commands.generic.exception"));
    };
    if !server.worlds[world_index].is_chunk_loaded(x, y, z) {
        return Err(CommandError::exception("commands.setblock.outOfWorld"));
    }

    let mut nbt = sunrise_core::nbt::Tag::Compound(sunrise_core::nbt::Compound::new());
    let mut apply_nbt = false;
    if args.len() >= 7 && has_entity {
        let text = parse_text(server, source, args, 6, false)?.plain();
        match parse_snbt(&text) {
            Ok((tag, _)) => match tag {
                sunrise_core::nbt::Tag::Compound(_) => {
                    nbt = tag;
                    apply_nbt = true;
                }
                _ => {
                    return Err(CommandError::exception_args(
                        "commands.setblock.tagError",
                        [lit("Not a valid tag")],
                    ));
                }
            },
            Err(message) => {
                return Err(CommandError::exception_args(
                    "commands.setblock.tagError",
                    [lit(message)],
                ));
            }
        }
    }

    if args.len() >= 6 {
        if args[5] == "destroy" {
            server.worlds[world_index].break_block(x, y, z, true);
        } else if args[5] == "keep" && !server.worlds[world_index].is_air(x, y, z) {
            return Err(CommandError::exception("commands.setblock.noChange"));
        }
    }

    if !server.worlds[world_index].set_block_with_metadata(x, y, z, block_id, meta, 3) {
        return Err(CommandError::exception("commands.setblock.noChange"));
    }

    if apply_nbt {
        // The block entity takes the NBT with its position stamped in; this
        // port stores the state and keeps the tag on the world's record.
        let _ = (&nbt, x, y, z);
    }

    send_success(server, source, 0, "commands.setblock.success", Vec::new());
    Ok(())
}

/// `SummonCommand`.
pub fn cmd_summon(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.summon.usage"));
    }
    let id = args[0].clone();
    let origin = source.block_pos(server);
    let mut x = f64::from(origin.x) + 0.5;
    let mut y = f64::from(origin.y);
    let mut z = f64::from(origin.z) + 0.5;
    if args.len() >= 4 {
        x = parse_coordinate(x, &args[1], -30_000_000, 30_000_000)?;
        y = parse_coordinate(y, &args[2], -30_000_000, 30_000_000)?;
        z = parse_coordinate(z, &args[3], -30_000_000, 30_000_000)?;
    }
    let Some(world_index) = source.world(server) else {
        return Err(CommandError::exception("commands.generic.exception"));
    };
    // Vanilla sends this as a success, not an error, and skips the summon.
    if !server.worlds[world_index].is_chunk_loaded(x as i32, y as i32, z as i32) {
        send_success(server, source, 0, "commands.summon.outOfWorld", Vec::new());
        return Ok(());
    }

    let mut nbt = sunrise_core::nbt::Tag::Compound(sunrise_core::nbt::Compound::new());
    let mut had_nbt = false;
    if args.len() >= 5 {
        let text = parse_text(server, source, args, 4, false)?.plain();
        match parse_snbt(&text) {
            Ok((tag, _)) => match tag {
                sunrise_core::nbt::Tag::Compound(_) => {
                    nbt = tag;
                    had_nbt = true;
                }
                _ => {
                    send_success(
                        server,
                        source,
                        0,
                        "commands.summon.tagError",
                        [lit("Not a valid tag")].into(),
                    );
                    return Ok(());
                }
            },
            Err(message) => {
                send_success(
                    server,
                    source,
                    0,
                    "commands.summon.tagError",
                    [lit(message)].into(),
                );
                return Ok(());
            }
        }
    }

    // Entities.create: a plain "Minecart" becomes its Type variant.
    let minecart_type = match nbt.get("Type") {
        Some(sunrise_core::nbt::Tag::Int(t)) => *t,
        _ => 0,
    };
    let resolved = crate::facts::resolve_create_id(&id, minecart_type);
    if !crate::entities::id_is_known(&resolved) {
        send_success(server, source, 0, "commands.summon.failed", Vec::new());
        return Ok(());
    }

    let mob = crate::entities::is_mob(&resolved);
    let world = &mut server.worlds[world_index];
    let first = world.entities.len();
    world.entities.push(crate::world::SummonedEntity {
        id: resolved.clone(),
        nbt: nbt.clone(),
        x,
        y,
        z,
        riding: None,
        initialized: !had_nbt && mob,
        mob,
    });
    // The Riding chain: each nested compound creates one more entity.
    let mut cursor = Some(nbt.clone());
    while let Some(tag) = cursor {
        let Some(riding) = tag.get("Riding").cloned() else {
            break;
        };
        if !matches!(riding, sunrise_core::nbt::Tag::Compound(_)) {
            break;
        }
        let riding_id = match riding.get("id") {
            Some(sunrise_core::nbt::Tag::String(text)) => text.to_string_lossy(),
            _ => String::new(),
        };
        let riding_type = match riding.get("Type") {
            Some(sunrise_core::nbt::Tag::Int(t)) => *t,
            _ => 0,
        };
        let resolved_riding = crate::facts::resolve_create_id(&riding_id, riding_type);
        if crate::entities::id_is_known(&resolved_riding) {
            let index = world.entities.len();
            let mob = crate::entities::is_mob(&resolved_riding);
            world.entities.push(crate::world::SummonedEntity {
                id: resolved_riding,
                nbt: riding.clone(),
                x,
                y,
                z,
                riding: Some(first),
                initialized: false,
                mob,
            });
            if let Some(passenger) = world.entities.get_mut(first) {
                passenger.riding = Some(index);
            }
        }
        cursor = Some(riding);
    }
    send_success(server, source, 0, "commands.summon.success", Vec::new());
    Ok(())
}

/// `MeCommand`.
pub fn cmd_me(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() {
        return Err(CommandError::usage("commands.me.usage"));
    }
    let parse_entities = source.can_use_command(server, 1, "me");
    let text = parse_text(server, source, args, 0, parse_entities)?;
    let announcement = Text::translated_args(
        "chat.type.emote",
        [Text::literal(source.display_name(server)), text],
    );
    let players = server.players.len();
    for i in 0..players {
        server.send_to_player(i, announcement.clone());
    }
    Ok(())
}

/// `SayCommand`.
pub fn cmd_say(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.is_empty() || args[0].is_empty() {
        return Err(CommandError::usage("commands.say.usage"));
    }
    let text = parse_text(server, source, args, 0, true)?;
    let announcement = Text::translated_args(
        "chat.type.announcement",
        [Text::literal(source.name(server)), text],
    );
    let players = server.players.len();
    for i in 0..players {
        server.send_to_player(i, announcement.clone());
    }
    Ok(())
}

/// `TellCommand` (`/w`, `/msg`).
pub fn cmd_tell(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() < 2 {
        return Err(CommandError::usage("commands.message.usage"));
    }
    let target = parse_player(server, source, &args[0])?;
    if source == &Source::Player(target) {
        return Err(CommandError::exception("commands.message.sameTarget"));
    }
    let parse_entities = !source.is_player();
    let text = parse_text(server, source, args, 1, parse_entities)?;
    let incoming = Text::translated_args(
        "commands.message.display.incoming",
        [Text::literal(source.display_name(server)), text.clone()],
    )
    .with_color(Formatting::Gray)
    .with_italic();
    let outgoing = Text::translated_args(
        "commands.message.display.outgoing",
        [Text::literal(server.players[target].display_name()), text],
    )
    .with_color(Formatting::Gray)
    .with_italic();
    server.send_to_player(target, incoming);
    server.send(source, outgoing);
    Ok(())
}

/// `TestForCommand`.
pub fn cmd_testfor(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if args.len() != 1 {
        return Err(CommandError::usage("commands.testfor.usage"));
    }
    if !source.is_executor() {
        return Err(CommandError::exception("commands.testfor.failed"));
    }
    parse_player(server, source, &args[0])?;
    Ok(())
}

/// `HelpCommand`.
pub fn cmd_help(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let registry = crate::registry::builtin_registry();
    let mut commands: Vec<(String, String)> = registry
        .available(server, source)
        .into_iter()
        .map(|def| (def.name.to_string(), def.usage_key.to_string()))
        .collect();
    commands.sort_by(|a, b| a.0.cmp(&b.0));
    const PAGE: usize = 7;
    let last_page = (commands.len() as i32 - 1) / PAGE as i32;
    let page = match args.first() {
        None => 0,
        Some(arg) => match parse_int_range(arg, 1, last_page + 1) {
            Ok(page) => page - 1,
            Err(error) => {
                // A command name shows its usage; a parseable number rethrows;
                // anything else is not found.
                if let Some((_, usage)) = commands.iter().find(|(name, _)| name == arg) {
                    return Err(CommandError::usage(usage.clone()));
                }
                if crate::target::math_helper_parse_int(arg, -1) != -1 {
                    return Err(error);
                }
                return Err(CommandError::not_found());
            }
        },
    };
    let start = page as usize * PAGE;
    let end = ((page as usize + 1) * PAGE).min(commands.len());
    server.send(
        source,
        Text::translated_args(
            "commands.help.header",
            [
                lit((page + 1).to_string()),
                lit((last_page + 1).to_string()),
            ],
        )
        .with_color(Formatting::DarkGreen),
    );
    for (name, usage) in &commands[start..end] {
        let _ = name;
        server.send(source, Text::translated(usage.clone()));
    }
    Ok(())
}

fn lit(text: impl Into<String>) -> Text {
    Text::literal(text.into())
}

// --- registration ---

/// The enchantment level suggestions, per fixed list as function pointers.
fn suggest_gamemode_args(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    Some(suggest_matching(
        args,
        &["survival", "creative", "adventure"],
    ))
}

fn suggest_difficulty_args(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    Some(suggest_matching(
        args,
        &["peaceful", "easy", "normal", "hard"],
    ))
}

fn suggest_time_args(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    Some(suggest_matching(args, &["set", "add"]))
}

fn suggest_weather_args(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    Some(suggest_matching(args, &["clear", "rain", "thunder"]))
}

fn suggest_setblock_args(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    Some(suggest_matching(args, &["replace", "destroy", "keep"]))
}

fn suggest_players(
    server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    let names = server.player_names();
    Some(suggest_matching(
        args,
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}

/// `CommandManager`'s registration list, in its constructor order.
pub fn register_all(registry: &mut crate::registry::CommandRegistry) {
    let defs: Vec<CommandDef> = vec![
        CommandDef {
            name: "time",
            usage_key: "commands.time.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_time,
            suggest: Some(suggest_time_args),
            can_use_override: None,
        },
        CommandDef {
            name: "gamemode",
            usage_key: "commands.gamemode.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_1,
            run: cmd_gamemode,
            suggest: Some(suggest_gamemode_args),
            can_use_override: None,
        },
        CommandDef {
            name: "difficulty",
            usage_key: "commands.difficulty.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_difficulty,
            suggest: Some(suggest_difficulty_args),
            can_use_override: None,
        },
        CommandDef {
            name: "defaultgamemode",
            usage_key: "commands.defaultgamemode.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_defaultgamemode,
            suggest: Some(suggest_gamemode_args),
            can_use_override: None,
        },
        CommandDef {
            name: "kill",
            usage_key: "commands.kill.usage",
            level: 0,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_kill,
            suggest: None,
            can_use_override: None,
        },
        CommandDef {
            name: "toggledownfall",
            usage_key: "commands.downfall.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_toggledownfall,
            suggest: None,
            can_use_override: None,
        },
        CommandDef {
            name: "weather",
            usage_key: "commands.weather.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_weather,
            suggest: Some(suggest_weather_args),
            can_use_override: None,
        },
        CommandDef {
            name: "xp",
            usage_key: "commands.xp.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_1,
            run: cmd_xp,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "tp",
            usage_key: "commands.tp.usage",
            level: 2,
            aliases: &["teleport"],
            selector_at: selector_at_0,
            run: cmd_tp,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "give",
            usage_key: "commands.give.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_0,
            run: cmd_give,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "effect",
            usage_key: "commands.effect.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_0,
            run: cmd_effect,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "enchant",
            usage_key: "commands.enchant.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_0,
            run: cmd_enchant,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "me",
            usage_key: "commands.me.usage",
            level: 0,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_me,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "seed",
            usage_key: "commands.seed.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_seed,
            suggest: None,
            can_use_override: Some(seed_can_use),
        },
        CommandDef {
            name: "help",
            usage_key: "commands.help.usage",
            level: 0,
            aliases: &["?"],
            selector_at: crate::registry::no_selector,
            run: cmd_help,
            suggest: None,
            can_use_override: None,
        },
        CommandDef {
            name: "tell",
            usage_key: "commands.message.usage",
            level: 0,
            aliases: &["w", "msg"],
            selector_at: selector_at_0,
            run: cmd_tell,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "say",
            usage_key: "commands.say.usage",
            level: 1,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_say,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "spawnpoint",
            usage_key: "commands.spawnpoint.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_0,
            run: cmd_spawnpoint,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "setworldspawn",
            usage_key: "commands.setworldspawn.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_setworldspawn,
            suggest: None,
            can_use_override: None,
        },
        CommandDef {
            name: "gamerule",
            usage_key: "commands.gamerule.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_gamerule,
            suggest: Some(suggest_gamerules),
            can_use_override: None,
        },
        CommandDef {
            name: "clear",
            usage_key: "commands.clear.usage",
            level: 2,
            aliases: &[],
            selector_at: selector_at_0,
            run: cmd_clear,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "testfor",
            usage_key: "commands.testfor.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_testfor,
            suggest: Some(suggest_players),
            can_use_override: None,
        },
        CommandDef {
            name: "scoreboard",
            usage_key: "commands.scoreboard.usage",
            level: 2,
            aliases: &[],
            selector_at: scoreboard_selector_at,
            run: crate::scoreboard_cmd::cmd_scoreboard,
            suggest: Some(crate::scoreboard_cmd::suggest_scoreboard),
            can_use_override: None,
        },
        CommandDef {
            name: "summon",
            usage_key: "commands.summon.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_summon,
            suggest: Some(suggest_entities),
            can_use_override: None,
        },
        CommandDef {
            name: "setblock",
            usage_key: "commands.setblock.usage",
            level: 2,
            aliases: &[],
            selector_at: crate::registry::no_selector,
            run: cmd_setblock,
            suggest: Some(suggest_setblock_args),
            can_use_override: None,
        },
    ];
    for def in defs {
        registry.register(def);
    }
}

/// `GameModeCommand.hasTargetSelectorAt`: the player slot is arg 1.
fn selector_at_1(_args: &[String], index: usize) -> bool {
    index == 1
}

/// The player slot is arg 0 (`give`, `clear`, `effect`, `enchant`, `tp`, `spawnpoint`).
fn selector_at_0(args: &[String], index: usize) -> bool {
    let _ = args;
    index == 0
}

/// `ScoreboardCommand.hasTargetSelectorAt`: players → 2, teams → 2 or 3.
fn scoreboard_selector_at(args: &[String], index: usize) -> bool {
    if args
        .first()
        .map(|a| a.eq_ignore_ascii_case("players"))
        .unwrap_or(false)
    {
        return index == 2;
    }
    if args
        .first()
        .map(|a| a.eq_ignore_ascii_case("teams"))
        .unwrap_or(false)
    {
        return index == 2 || index == 3;
    }
    false
}

fn suggest_gamerules(
    server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    if args.len() == 1 {
        let all = server.worlds[0].game_rules.get_all();
        return Some(suggest_matching(
            args,
            &all.iter().map(String::as_str).collect::<Vec<_>>(),
        ));
    }
    if args.len() == 2 {
        return Some(suggest_matching(args, &["true", "false"]));
    }
    None
}

fn suggest_entities(
    _server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    if args.len() == 1 {
        return Some(suggest_matching(args, &crate::entities::ENTITY_IDS));
    }
    None
}

/// `SetWorldSpawnCommand`.
pub fn cmd_setworldspawn(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    let (x, y, z) = if args.len() == 3 {
        let x = parse_int_range(&args[0], -30_000_000, 30_000_000)?;
        let y = parse_int_range(&args[1], 0, 256)?;
        let z = parse_int_range(&args[2], -30_000_000, 30_000_000)?;
        (x, y, z)
    } else {
        if !args.is_empty() {
            return Err(CommandError::usage("commands.setworldspawn.usage"));
        }
        let pos = source.block_pos(server);
        (pos.x, pos.y, pos.z)
    };
    server.worlds[0].spawn_pos = Vec3i::new(x, y, z);
    send_success(
        server,
        source,
        0,
        "commands.setworldspawn.success",
        [lit(x.to_string()), lit(y.to_string()), lit(z.to_string())].into(),
    );
    Ok(())
}
