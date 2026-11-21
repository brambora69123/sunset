//! `ScoreboardCommand`: objectives, players and teams subcommands over the
//! shared `Scoreboard`, with the 1.7.10 limits (16-char names, 32-char
//! display names) and exact value rules.

use crate::args::{list_args, parse_int, parse_int_range, suggest_matching};
use crate::error::CommandError;
use crate::registry::send_success;
use crate::scoreboard::{Criterion, Scoreboard};
use crate::text::{Formatting, Text};
use crate::world::{CommandServer, Source};

fn lit(text: impl Into<String>) -> Text {
    Text::literal(text.into())
}

/// `ScoreboardCommand.getScoreboard`: world 0's board.
fn board(server: &mut CommandServer) -> &mut Scoreboard {
    &mut server.worlds[0].scoreboard
}

/// `getObjective(name, write)`.
fn get_objective(server: &mut CommandServer, name: &str, write: bool) -> Result<(), CommandError> {
    match board(server).get_objective(name) {
        None => Err(CommandError::exception_args(
            "commands.scoreboard.objectiveNotFound",
            [lit(name)],
        )),
        Some(objective) if write && objective.criterion.is_read_only() => Err(
            CommandError::exception_args("commands.scoreboard.objectiveReadOnly", [lit(name)]),
        ),
        Some(_) => Ok(()),
    }
}

/// `getTeam(name)`.
fn team_exists(server: &mut CommandServer, name: &str) -> Result<(), CommandError> {
    if board(server).get_team(name).is_none() {
        return Err(CommandError::exception_args(
            "commands.scoreboard.teamNotFound",
            [lit(name)],
        ));
    }
    Ok(())
}

/// `ScoreboardCommand.run`.
pub fn cmd_scoreboard(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
) -> Result<(), CommandError> {
    if !args.is_empty() {
        if args[0].eq_ignore_ascii_case("objectives") {
            if args.len() == 1 {
                return Err(CommandError::usage("commands.scoreboard.objectives.usage"));
            }
            if args[1].eq_ignore_ascii_case("list") {
                return list_objectives(server, source);
            }
            if args[1].eq_ignore_ascii_case("add") {
                if args.len() < 4 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.objectives.add.usage",
                    ));
                }
                return add_objective(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("remove") {
                if args.len() != 3 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.objectives.remove.usage",
                    ));
                }
                return remove_objective(server, source, &args[2]);
            }
            if args[1].eq_ignore_ascii_case("setdisplay") {
                if args.len() != 3 && args.len() != 4 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.objectives.setdisplay.usage",
                    ));
                }
                return set_display(server, source, args, 2);
            }
            return Err(CommandError::usage("commands.scoreboard.objectives.usage"));
        }
        if args[0].eq_ignore_ascii_case("players") {
            if args.len() == 1 {
                return Err(CommandError::usage("commands.scoreboard.players.usage"));
            }
            if args[1].eq_ignore_ascii_case("list") {
                if args.len() > 3 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.players.list.usage",
                    ));
                }
                return list_players(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("add") {
                if args.len() != 5 {
                    return Err(CommandError::usage("commands.scoreboard.players.add.usage"));
                }
                return set_score(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("remove") {
                if args.len() != 5 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.players.remove.usage",
                    ));
                }
                return set_score(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("set") {
                if args.len() != 5 {
                    return Err(CommandError::usage("commands.scoreboard.players.set.usage"));
                }
                return set_score(server, source, args, 2);
            }
            if !args[1].eq_ignore_ascii_case("reset") {
                return Err(CommandError::usage("commands.scoreboard.players.usage"));
            }
            if args.len() != 3 {
                return Err(CommandError::usage(
                    "commands.scoreboard.players.reset.usage",
                ));
            }
            return reset_score(server, source, args, 2);
        }
        if args[0].eq_ignore_ascii_case("teams") {
            if args.len() == 1 {
                return Err(CommandError::usage("commands.scoreboard.teams.usage"));
            }
            if args[1].eq_ignore_ascii_case("list") {
                if args.len() > 3 {
                    return Err(CommandError::usage("commands.scoreboard.teams.list.usage"));
                }
                return list_teams(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("add") {
                if args.len() < 3 {
                    return Err(CommandError::usage("commands.scoreboard.teams.add.usage"));
                }
                return add_team(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("remove") {
                if args.len() != 3 {
                    return Err(CommandError::usage(
                        "commands.scoreboard.teams.remove.usage",
                    ));
                }
                return remove_team(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("empty") {
                if args.len() != 3 {
                    return Err(CommandError::usage("commands.scoreboard.teams.empty.usage"));
                }
                return empty_team(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("join") {
                let source_is_player = source.is_player();
                if args.len() < 4 && (args.len() != 3 || !source_is_player) {
                    return Err(CommandError::usage("commands.scoreboard.teams.join.usage"));
                }
                return join_team(server, source, args, 2);
            }
            if args[1].eq_ignore_ascii_case("leave") {
                let source_is_player = source.is_player();
                if args.len() < 3 && !source_is_player {
                    return Err(CommandError::usage("commands.scoreboard.teams.leave.usage"));
                }
                return leave_team(server, source, args, 2);
            }
            if !args[1].eq_ignore_ascii_case("option") {
                return Err(CommandError::usage("commands.scoreboard.teams.usage"));
            }
            if args.len() != 4 && args.len() != 5 {
                return Err(CommandError::usage(
                    "commands.scoreboard.teams.option.usage",
                ));
            }
            return update_team(server, source, args, 2);
        }
    }
    Err(CommandError::usage("commands.scoreboard.usage"))
}

/// `listObjectives`.
fn list_objectives(server: &mut CommandServer, source: &Source) -> Result<(), CommandError> {
    let objectives: Vec<crate::scoreboard::Objective> = board(server)
        .get_objectives()
        .into_iter()
        .cloned()
        .collect();
    if objectives.is_empty() {
        return Err(CommandError::exception(
            "commands.scoreboard.objectives.list.empty",
        ));
    }
    server.send(
        source,
        Text::translated_args(
            "commands.scoreboard.objectives.list.count",
            [lit(objectives.len().to_string())],
        )
        .with_color(Formatting::DarkGreen),
    );
    for objective in &objectives {
        server.send(
            source,
            Text::translated_args(
                "commands.scoreboard.objectives.list.entry",
                [
                    lit(objective.name.clone()),
                    lit(objective.display_name.clone()),
                    lit(objective.criterion.name().to_string()),
                ],
            ),
        );
    }
    Ok(())
}

/// `addObjective`.
fn add_objective(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let name = args[index].clone();
    let criterion_name = &args[index + 1];
    let Some(criterion) = Criterion::by_name(criterion_name) else {
        return Err(CommandError::usage_args(
            "commands.scoreboard.objectives.add.wrongType",
            [lit(criterion_name.clone())],
        ));
    };
    if board(server).get_objective(&name).is_some() {
        return Err(CommandError::exception_args(
            "commands.scoreboard.objectives.add.alreadyExists",
            [lit(name)],
        ));
    }
    if name.len() > 16 {
        return Err(CommandError::exception_args(
            "commands.scoreboard.objectives.add.tooLong",
            [lit(name), lit("16")],
        ));
    }
    if name.is_empty() {
        return Err(CommandError::usage(
            "commands.scoreboard.objectives.add.usage",
        ));
    }
    if args.len() > index + 2 {
        let display = args[index + 2..].join(" ");
        if display.len() > 32 {
            return Err(CommandError::exception_args(
                "commands.scoreboard.objectives.add.displayTooLong",
                [lit(display), lit("32")],
            ));
        }
        board(server).create_objective(name.clone(), criterion);
        if !display.is_empty()
            && let Some(objective) = board(server).objectives.get_mut(&name)
        {
            objective.display_name = display;
        }
    } else {
        board(server).create_objective(name.clone(), criterion);
    }
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.objectives.add.success",
        [lit(name)].into(),
    );
    Ok(())
}

/// `removeObjective`.
fn remove_objective(
    server: &mut CommandServer,
    source: &Source,
    name: &str,
) -> Result<(), CommandError> {
    get_objective(server, name, false)?;
    board(server).remove_objective(name);
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.objectives.remove.success",
        [lit(name)].into(),
    );
    Ok(())
}

/// `setDisplayObjective`.
fn set_display(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let slot_name = args[index].clone();
    let Some(slot) = Scoreboard::display_slot(&slot_name) else {
        // Java parses first, then reports the invalid slot.
        return Err(CommandError::exception_args(
            "commands.scoreboard.objectives.setdisplay.invalidSlot",
            [lit(slot_name)],
        ));
    };
    let objective = if args.len() == 4 {
        let name = &args[index + 1];
        get_objective(server, name, false)?;
        Some(name.clone())
    } else {
        None
    };
    board(server).set_display_objective(slot, objective.clone());
    if let Some(name) = objective {
        send_success(
            server,
            source,
            0,
            "commands.scoreboard.objectives.setdisplay.successSet",
            [
                lit(Scoreboard::display_location(slot).unwrap_or("").to_string()),
                lit(name),
            ]
            .into(),
        );
    } else {
        send_success(
            server,
            source,
            0,
            "commands.scoreboard.objectives.setdisplay.successCleared",
            [lit(Scoreboard::display_location(slot)
                .unwrap_or("")
                .to_string())]
            .into(),
        );
    }
    Ok(())
}

/// `listPlayers`.
fn list_players(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    if args.len() > index {
        let owner = &args[index];
        let scores: Vec<crate::scoreboard::Score> = board(server)
            .get_scores(owner)
            .into_iter()
            .cloned()
            .collect();
        if scores.is_empty() {
            return Err(CommandError::exception_args(
                "commands.scoreboard.players.list.player.empty",
                [lit(owner.clone())],
            ));
        }
        server.send(
            source,
            Text::translated_args(
                "commands.scoreboard.players.list.player.count",
                [lit(scores.len().to_string()), lit(owner.clone())],
            )
            .with_color(Formatting::DarkGreen),
        );
        for score in &scores {
            server.send(
                source,
                Text::translated_args(
                    "commands.scoreboard.players.list.player.entry",
                    [
                        lit(score.score.to_string()),
                        lit(score.objective.clone()),
                        lit(score.objective.clone()),
                    ],
                ),
            );
        }
        return Ok(());
    }
    let owners = board(server).get_score_owners();
    if owners.is_empty() {
        return Err(CommandError::exception(
            "commands.scoreboard.players.list.empty",
        ));
    }
    server.send(
        source,
        Text::translated_args(
            "commands.scoreboard.players.list.count",
            [lit(owners.len().to_string())],
        )
        .with_color(Formatting::DarkGreen),
    );
    let names = list_args(&owners.iter().map(String::as_str).collect::<Vec<_>>());
    server.send(source, Text::literal(names));
    Ok(())
}

/// `setScore`: the set, add and remove forms.
fn set_score(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let action = args[index - 1].clone();
    let owner = super::commands::parse_player_name(server, source, &args[index])?;
    let objective_name = args[index + 1].clone();
    get_objective(server, &objective_name, true)?;
    let value = if action.eq_ignore_ascii_case("set") {
        parse_int(&args[index + 2])?
    } else {
        parse_int_range(&args[index + 2], 1, i32::MAX)?
    };
    // Read-only criteria refuse through IllegalStateException, which surfaces
    // as the generic exception; the write guard above already refuses health.
    let score = board(server).get_or_create_score(&owner, &objective_name);
    let current = score.score;
    let next = if action.eq_ignore_ascii_case("set") {
        value
    } else if action.eq_ignore_ascii_case("add") {
        current.wrapping_add(value)
    } else {
        current.wrapping_sub(value)
    };
    score.score = next;
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.players.set.success",
        [lit(objective_name), lit(owner), lit(next.to_string())].into(),
    );
    Ok(())
}

/// `resetScore`.
fn reset_score(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let owner = super::commands::parse_player_name(server, source, &args[index])?;
    board(server).remove_scores(&owner);
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.players.reset.success",
        [lit(owner)].into(),
    );
    Ok(())
}

/// `addTeam`.
fn add_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let name = args[index].clone();
    if board(server).get_team(&name).is_some() {
        return Err(CommandError::exception_args(
            "commands.scoreboard.teams.add.alreadyExists",
            [lit(name)],
        ));
    }
    if name.len() > 16 {
        return Err(CommandError::exception_args(
            "commands.scoreboard.teams.add.tooLong",
            [lit(name), lit("16")],
        ));
    }
    if name.is_empty() {
        return Err(CommandError::usage("commands.scoreboard.teams.add.usage"));
    }
    board(server).add_team(name.clone());
    if args.len() > index + 1 {
        let display = args[index + 1..].join(" ");
        if display.len() > 32 {
            return Err(CommandError::exception_args(
                "commands.scoreboard.teams.add.displayTooLong",
                [lit(display), lit("32")],
            ));
        }
        if !display.is_empty()
            && let Some(team) = board(server).teams.get_mut(&name)
        {
            team.display_name = display;
        }
    }
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.teams.add.success",
        [lit(name)].into(),
    );
    Ok(())
}

/// `removeTeam`.
fn remove_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let name = &args[index];
    team_exists(server, name)?;
    board(server).remove_team(name);
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.teams.remove.success",
        [lit(name.clone())].into(),
    );
    Ok(())
}

/// `emptyTeam`.
fn empty_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let name = &args[index];
    team_exists(server, name)?;
    let members: Vec<String> = board(server)
        .get_team(name)
        .map(|team| team.members.clone())
        .unwrap_or_default();
    if members.is_empty() {
        return Err(CommandError::exception_args(
            "commands.scoreboard.teams.empty.alreadyEmpty",
            [lit(name.clone())],
        ));
    }
    for member in &members {
        board(server).remove_member_from_team(member, name);
    }
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.teams.empty.success",
        [lit(members.len().to_string()), lit(name.clone())].into(),
    );
    Ok(())
}

/// `listTeams`.
fn list_teams(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    if args.len() > index {
        let name = &args[index];
        team_exists(server, name)?;
        let members = board(server)
            .get_team(name)
            .map(|team| team.members.clone())
            .unwrap_or_default();
        if members.is_empty() {
            return Err(CommandError::exception_args(
                "commands.scoreboard.teams.list.player.empty",
                [lit(name.clone())],
            ));
        }
        server.send(
            source,
            Text::translated_args(
                "commands.scoreboard.teams.list.player.count",
                [lit(members.len().to_string()), lit(name.clone())],
            )
            .with_color(Formatting::DarkGreen),
        );
        let names = list_args(&members.iter().map(String::as_str).collect::<Vec<_>>());
        server.send(source, Text::literal(names));
        return Ok(());
    }
    let teams: Vec<crate::scoreboard::Team> =
        board(server).get_teams().into_iter().cloned().collect();
    if teams.is_empty() {
        return Err(CommandError::exception(
            "commands.scoreboard.teams.list.empty",
        ));
    }
    server.send(
        source,
        Text::translated_args(
            "commands.scoreboard.teams.list.count",
            [lit(teams.len().to_string())],
        )
        .with_color(Formatting::DarkGreen),
    );
    for team in &teams {
        server.send(
            source,
            Text::translated_args(
                "commands.scoreboard.teams.list.entry",
                [
                    lit(team.name.clone()),
                    lit(team.display_name.clone()),
                    lit(team.members.len().to_string()),
                ],
            ),
        );
    }
    Ok(())
}

/// `joinTeam`.
fn join_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let team = args[index].clone();
    team_exists(server, &team)?;
    let mut joined: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    if source.is_player() && index + 1 == args.len() {
        let own = super::commands::as_player(server, source)?;
        let name = server.players[own].name.clone();
        if board(server).add_member_to_team(&name, &team) {
            joined.push(name);
        } else {
            failed.push(name);
        }
    } else {
        for arg in &args[index + 1..] {
            let name = super::commands::parse_player_name(server, source, arg)?;
            if board(server).add_member_to_team(&name, &team) {
                joined.push(name);
            } else {
                failed.push(name);
            }
        }
    }
    if !joined.is_empty() {
        let names = list_args(&joined.iter().map(String::as_str).collect::<Vec<_>>());
        send_success(
            server,
            source,
            0,
            "commands.scoreboard.teams.join.success",
            [lit(joined.len().to_string()), lit(team.clone()), lit(names)].into(),
        );
    }
    if !failed.is_empty() {
        let names = list_args(&failed.iter().map(String::as_str).collect::<Vec<_>>());
        return Err(CommandError::exception_args(
            "commands.scoreboard.teams.join.failure",
            [lit(failed.len().to_string()), lit(team), lit(names)],
        ));
    }
    Ok(())
}

/// `leaveTeam`.
fn leave_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let mut left: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    if source.is_player() && index == args.len() {
        let own = super::commands::as_player(server, source)?;
        let name = server.players[own].name.clone();
        if board(server).remove_member_from_team_any(&name) {
            left.push(name);
        } else {
            failed.push(name);
        }
    } else {
        for arg in &args[index..] {
            let name = super::commands::parse_player_name(server, source, arg)?;
            if board(server).remove_member_from_team_any(&name) {
                left.push(name);
            } else {
                failed.push(name);
            }
        }
    }
    if !left.is_empty() {
        let names = list_args(&left.iter().map(String::as_str).collect::<Vec<_>>());
        send_success(
            server,
            source,
            0,
            "commands.scoreboard.teams.leave.success",
            [lit(left.len().to_string()), lit(names)].into(),
        );
    }
    if !failed.is_empty() {
        let names = list_args(&failed.iter().map(String::as_str).collect::<Vec<_>>());
        return Err(CommandError::exception_args(
            "commands.scoreboard.teams.leave.failure",
            [lit(failed.len().to_string()), lit(names)],
        ));
    }
    Ok(())
}

/// `updateTeam`: the color, friendlyfire and seeFriendlyInvisibles options.
fn update_team(
    server: &mut CommandServer,
    source: &Source,
    args: &[String],
    index: usize,
) -> Result<(), CommandError> {
    let team = args[index].clone();
    team_exists(server, &team)?;
    let option = args[index + 1].to_lowercase();
    if !matches!(
        option.as_str(),
        "color" | "friendlyfire" | "seefriendlyinvisibles"
    ) {
        return Err(CommandError::usage(
            "commands.scoreboard.teams.option.usage",
        ));
    }
    if args.len() == 4 {
        if option == "color" {
            return Err(CommandError::usage_args(
                "commands.scoreboard.teams.option.noValue",
                [
                    lit(option.clone()),
                    lit(list_args(&Formatting::get_names(true, false))),
                ],
            ));
        }
        return Err(CommandError::usage_args(
            "commands.scoreboard.teams.option.noValue",
            [lit(option.clone()), lit("true, false")],
        ));
    }
    let value = args[index + 2].clone();
    let value_lower = value.to_lowercase();
    if option == "color" {
        let Some(formatting) = Formatting::by_name(&value_lower) else {
            return Err(CommandError::usage_args(
                "commands.scoreboard.teams.option.noValue",
                [
                    lit(option.clone()),
                    lit(list_args(&Formatting::get_names(true, false))),
                ],
            ));
        };
        if formatting.is_modifier() {
            return Err(CommandError::usage_args(
                "commands.scoreboard.teams.option.noValue",
                [
                    lit(option.clone()),
                    lit(list_args(&Formatting::get_names(true, false))),
                ],
            ));
        }
        let prefix = formatting.prefix();
        let suffix = Formatting::Reset.prefix();
        if let Some(team) = board(server).teams.get_mut(&team) {
            team.prefix = prefix;
            team.suffix = suffix;
        }
    } else if option == "friendlyfire" {
        if !matches!(value_lower.as_str(), "true" | "false") {
            return Err(CommandError::usage_args(
                "commands.scoreboard.teams.option.noValue",
                [lit(option.clone()), lit("true, false")],
            ));
        }
        if let Some(team) = board(server).teams.get_mut(&team) {
            team.allow_friendly_fire = value_lower == "true";
        }
    } else if let Some(team) = board(server).teams.get_mut(&team) {
        team.show_friendly_invisibles = value_lower == "true";
    }
    send_success(
        server,
        source,
        0,
        "commands.scoreboard.teams.option.success",
        [lit(option), lit(team), lit(value)].into(),
    );
    Ok(())
}

/// `ScoreboardCommand.getSuggestions`.
pub fn suggest_scoreboard(
    server: &mut CommandServer,
    _source: &Source,
    args: &[&str],
) -> Option<Vec<String>> {
    if args.len() == 1 {
        return Some(suggest_matching(args, &["objectives", "players", "teams"]));
    }
    if args[0].eq_ignore_ascii_case("objectives") {
        if args.len() == 2 {
            return Some(suggest_matching(
                args,
                &["list", "add", "remove", "setdisplay"],
            ));
        }
        if args[1].eq_ignore_ascii_case("add") && args.len() == 4 {
            return Some(suggest_matching(
                args,
                &[
                    "dummy",
                    "deathCount",
                    "playerKillCount",
                    "totalKillCount",
                    "health",
                ],
            ));
        }
        if args[1].eq_ignore_ascii_case("remove") && args.len() == 3 {
            return Some(objective_names(server, false));
        }
        if args[1].eq_ignore_ascii_case("setdisplay") {
            if args.len() == 3 {
                return Some(suggest_matching(args, &["list", "sidebar", "belowName"]));
            }
            if args.len() == 4 {
                return Some(objective_names(server, false));
            }
        }
        return None;
    }
    if args[0].eq_ignore_ascii_case("players") {
        if args.len() == 2 {
            return Some(suggest_matching(
                args,
                &["set", "add", "remove", "reset", "list"],
            ));
        }
        if args[1].eq_ignore_ascii_case("set")
            || args[1].eq_ignore_ascii_case("add")
            || args[1].eq_ignore_ascii_case("remove")
        {
            if args.len() == 3 {
                return Some(suggest_matching(
                    args,
                    &server
                        .player_names()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                ));
            }
            if args.len() == 4 {
                return Some(objective_names(server, true));
            }
        } else if (args[1].eq_ignore_ascii_case("reset") || args[1].eq_ignore_ascii_case("list"))
            && args.len() == 3
        {
            return Some(suggest_matching(
                args,
                &board(server)
                    .get_score_owners()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            ));
        }
        return None;
    }
    if args[0].eq_ignore_ascii_case("teams") {
        if args.len() == 2 {
            return Some(suggest_matching(
                args,
                &["add", "remove", "join", "leave", "empty", "list", "option"],
            ));
        }
        if args[1].eq_ignore_ascii_case("join") {
            if args.len() == 3 {
                return Some(team_names(server));
            }
            if args.len() >= 4 {
                return Some(suggest_matching(
                    args,
                    &server
                        .player_names()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                ));
            }
            return None;
        }
        if args[1].eq_ignore_ascii_case("leave") {
            return Some(suggest_matching(
                args,
                &server
                    .player_names()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            ));
        }
        if !args[1].eq_ignore_ascii_case("empty")
            && !args[1].eq_ignore_ascii_case("list")
            && !args[1].eq_ignore_ascii_case("remove")
        {
            if args[1].eq_ignore_ascii_case("option") {
                if args.len() == 3 {
                    return Some(team_names(server));
                }
                if args.len() == 4 {
                    return Some(suggest_matching(
                        args,
                        &["color", "friendlyfire", "seeFriendlyInvisibles"],
                    ));
                }
                if args.len() == 5 {
                    if args[3].eq_ignore_ascii_case("color") {
                        return Some(suggest_matching(args, &Formatting::get_names(true, false)));
                    }
                    if args[3].eq_ignore_ascii_case("friendlyfire")
                        || args[3].eq_ignore_ascii_case("seeFriendlyInvisibles")
                    {
                        return Some(suggest_matching(args, &["true", "false"]));
                    }
                }
            }
            return None;
        }
        if args.len() == 3 {
            return Some(team_names(server));
        }
        return None;
    }
    None
}

/// `getObjectives(write)`: the names, writable ones only when asked.
fn objective_names(server: &mut CommandServer, write: bool) -> Vec<String> {
    board(server)
        .get_objectives()
        .into_iter()
        .filter(|objective| !write || !objective.criterion.is_read_only())
        .map(|objective| objective.name.clone())
        .collect()
}

fn team_names(server: &mut CommandServer) -> Vec<String> {
    board(server).get_team_names()
}
