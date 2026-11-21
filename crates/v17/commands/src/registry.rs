//! The command dispatcher: `CommandRegistry`'s registration with aliases,
//! the run loop with its target-selector fan-out, permission checks and
//! error rendering, and per-command suggestions.

use sunrise_core::java_hash_map::JavaHashMap;

use crate::error::CommandError;
use crate::text::{Formatting, Text};
use crate::world::{CommandServer, Source};

/// One command's run: the args come with any target selector already
/// substituted by the dispatcher.
pub type RunFn = fn(&mut CommandServer, &Source, &[String]) -> Result<(), CommandError>;

/// `hasTargetSelectorAt(args, index)`: whether the arg at the index is the
/// command's player slot the dispatcher may fan out over.
pub type SelectorAtFn = fn(&[String], usize) -> bool;

/// `getSuggestions(source, args)`: the args after the command name; None for
/// no suggestions (the client shows nothing).
pub type SuggestFn = fn(&mut CommandServer, &Source, &[&str]) -> Option<Vec<String>>;

/// One registered command (`AbstractCommand` + its `Command` interface bits).
pub struct CommandDef {
    pub name: &'static str,
    /// `getUsage(source)`: the usage translation key.
    pub usage_key: &'static str,
    /// `getRequiredPermissionLevel`.
    pub level: i32,
    /// `getAliases`.
    pub aliases: &'static [&'static str],
    pub selector_at: SelectorAtFn,
    pub run: RunFn,
    pub suggest: Option<SuggestFn>,
    /// `canUse` overrides, as `/seed`'s singleplayer check takes.
    pub can_use_override: Option<fn(&mut CommandServer, &Source) -> bool>,
}

/// `AbstractCommand.hasTargetSelectorAt`'s default: no slot.
pub fn no_selector(_args: &[String], _index: usize) -> bool {
    false
}

#[derive(Default)]
pub struct CommandRegistry {
    /// `commandsByName`: name and alias -> def index, in Java HashMap order.
    by_name: JavaHashMap<String, usize>,
    /// `commands`: the defs in registration order.
    defs: Vec<CommandDef>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// `register`: the name and every alias point at the def; an alias never
    /// takes a real command's name slot.
    pub fn register(&mut self, def: CommandDef) {
        let index = self.defs.len();
        self.by_name.put(def.name.to_string(), index);
        for alias in def.aliases {
            let takes = match self.by_name.get(*alias) {
                None => true,
                Some(&i) => self.defs[i].name != *alias,
            };
            if takes {
                self.by_name.put((*alias).to_string(), index);
            }
        }
        self.defs.push(def);
    }

    /// `get(name)`.
    pub fn get(&self, name: &str) -> Option<&CommandDef> {
        self.by_name.get(name).map(|&i| &self.defs[i])
    }

    /// `getCommands()`: every name and alias, in Java HashMap order.
    pub fn names(&self) -> Vec<String> {
        self.by_name.entries().map(|(k, _)| k.clone()).collect()
    }

    /// `getAvailableCommands(source)`: the defs the source can use, in
    /// registration order (`/help` sorts them by name after).
    pub fn available(&self, server: &mut CommandServer, source: &Source) -> Vec<&CommandDef> {
        self.defs
            .iter()
            .filter(|def| can_use(def, server, source))
            .collect()
    }

    /// `CommandRegistry.run(source, command)`: trim, strip the leading `/`,
    /// split, look up, permission-check, fan out over a multi-match selector
    /// or run once, and render errors as Java does. Returns the success
    /// count.
    pub fn run(&mut self, server: &mut CommandServer, source: &Source, line: &str) -> i32 {
        let trimmed = line.trim();
        let trimmed = trimmed.strip_prefix('/').unwrap_or(trimmed);
        let parts = split_java(trimmed);
        let name = parts[0].clone();
        let args: Vec<String> = parts[1..].to_vec();

        let Some(def_index) = self.by_name.get(name.as_str()).copied() else {
            return self.fail(server, source, CommandError::not_found());
        };

        let def = &self.defs[def_index];
        if !can_use(def, server, source) {
            server.send(
                source,
                Text::translated("commands.generic.permission").with_color(Formatting::Red),
            );
            return 0;
        }
        let run_fn = def.run;
        let selector_at = def.selector_at;

        // The first arg sitting in the command's player slot that matches
        // more than one player (`getIndexOfTargetSelector`).
        let mut selector_index: Option<usize> = None;
        for (i, arg) in args.iter().enumerate() {
            if selector_at(&args, i) && crate::target::matches_multiple(arg) {
                selector_index = Some(i);
                break;
            }
        }

        let mut successes = 0;
        match selector_index {
            None => match run_fn(server, source, &args) {
                Ok(()) => successes += 1,
                Err(error) => {
                    self.fail(server, source, error);
                }
            },
            Some(index) => {
                let selector = args[index].clone();
                match crate::commands::select_players(server, source, &selector) {
                    // Java's select returns null for an unparsable selector:
                    // the for-each NPEs into the generic catch.
                    None => {
                        return self.fail(
                            server,
                            source,
                            CommandError::exception("commands.generic.exception"),
                        );
                    }
                    Some(players) => {
                        for player in players {
                            let mut args = args.clone();
                            args[index] = player;
                            match run_fn(server, source, &args) {
                                Ok(()) => successes += 1,
                                Err(error) => {
                                    self.fail(server, source, error);
                                }
                            }
                        }
                    }
                }
            }
        }
        successes
    }

    /// One failed run: the error rendered red to the source, usage errors
    /// wrapped in the usage line.
    fn fail(&mut self, server: &mut CommandServer, source: &Source, error: CommandError) -> i32 {
        let message = if error.usage {
            Text::translated_args(
                "commands.generic.usage",
                [Text::translated_args(error.key, error.args)],
            )
            .with_color(Formatting::Red)
        } else {
            Text::translated_args(error.key, error.args).with_color(Formatting::Red)
        };
        server.send(source, message);
        0
    }

    /// `getSuggestions(source, command)`: matching command names for the
    /// first word, the command's own suggestions after.
    pub fn suggestions(
        &mut self,
        server: &mut CommandServer,
        source: &Source,
        line: &str,
    ) -> Option<Vec<String>> {
        // Java splits with `split(" ", -1)`: trailing empties stay, so an
        // arg being typed counts.
        let parts: Vec<&str> = line.split(' ').collect();
        if parts.len() == 1 {
            let mut out = Vec::new();
            for (name, &index) in self.by_name.entries() {
                if crate::args::does_string_start_with(parts[0], name)
                    && can_use(&self.defs[index], server, source)
                {
                    out.push(name.clone());
                }
            }
            return Some(out);
        }
        let index = self.by_name.get(parts[0]).copied()?;
        let def = &self.defs[index];
        def.suggest?(server, source, &parts[1..])
    }
}

/// `AbstractCommand.canUse`: the override, else `canUseCommand(level, name)`.
fn can_use(def: &CommandDef, server: &mut CommandServer, source: &Source) -> bool {
    match def.can_use_override {
        Some(check) => check(server, source),
        None => source.can_use_command(server, def.level, def.name),
    }
}

/// Java's `command.split(" ")`: interior empties stay, trailing empties go,
/// and an empty line splits to one empty string.
fn split_java(line: &str) -> Vec<String> {
    let mut parts: Vec<String> = line.split(' ').map(String::from).collect();
    while parts.last().map(|p| p.is_empty()).unwrap_or(false) {
        parts.pop();
    }
    if parts.is_empty() {
        parts.push(String::new());
    }
    parts
}

/// The full registry with every 1.7.10 command registered
/// (`CommandManager`'s constructor), for probes and per-command suggestion
/// lookups.
pub fn builtin_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    crate::commands::register_all(&mut registry);
    registry
}

/// `AbstractCommand.sendSuccess(source, command, flags, key, args)` as
/// `CommandManager` receives it: the gray admin line goes to the op players
/// (unless the source is a command block with `commandBlockOutput` off) and
/// to the console, and the message reaches the source unless flag bit 1
/// suppresses it.
pub fn send_success(
    server: &mut CommandServer,
    source: &Source,
    flags: i32,
    key: &str,
    args: Vec<Text>,
) {
    let command_block_output = server
        .worlds
        .first()
        .map(|w| w.game_rules.get_boolean("commandBlockOutput"))
        .unwrap_or(true);
    let notify_ops = !(source.is_executor() && !command_block_output);
    let admin = Text::translated_args(
        "chat.type.admin",
        [
            Text::literal(source.name(server)),
            Text::translated_args(key, args.clone()),
        ],
    )
    .with_color(Formatting::Gray)
    .with_italic();
    if notify_ops {
        for i in 0..server.players.len() {
            if Source::Player(i) != *source && server.players[i].op {
                server.send_to_player(i, admin.clone());
            }
        }
    }
    // The console gets the line whenever the source isn't the server itself,
    // which in this port is always.
    server.console.push(admin);
    if (flags & 1) != 1 {
        server.send(source, Text::translated_args(key, args));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_s: &mut CommandServer, _src: &Source, _args: &[String]) -> Result<(), CommandError> {
        Ok(())
    }

    #[test]
    fn aliases_never_take_a_command_name() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandDef {
            name: "help",
            usage_key: "commands.help.usage",
            level: 0,
            aliases: &["?"],
            selector_at: no_selector,
            run: noop,
            suggest: None,
            can_use_override: None,
        });
        registry.register(CommandDef {
            name: "?",
            usage_key: "commands.something.usage",
            level: 0,
            aliases: &[],
            selector_at: no_selector,
            run: noop,
            suggest: None,
            can_use_override: None,
        });
        // The real "?" command keeps its own slot.
        assert_eq!(
            registry.get("?").unwrap().usage_key,
            "commands.something.usage"
        );
        assert_eq!(registry.get("help").unwrap().name, "help");
        assert!(registry.get("?\u{0}never").is_none());
    }

    #[test]
    fn split_java_drops_trailing_empties() {
        assert_eq!(split_java("say"), ["say"]);
        assert_eq!(split_java("say hi"), ["say", "hi"]);
        assert_eq!(
            split_java("say  hi"),
            ["say", "", "hi"],
            "interior empties stay"
        );
        assert_eq!(split_java("say  "), ["say"]);
        assert_eq!(
            split_java(""),
            [""],
            "an empty line splits to one empty string"
        );
    }
}
