//! Minecraft 1.7.10 commands, ported for sunset: the command set and its
//! parsing, with 1.7.10 argument and NBT semantics.
//!
//! Ported from the calamus-mapped 1.7.10 decompile
//! (`.local/minecraft/1.7.10/sources/*/net/minecraft/server/command/`):
//! every message key, range check and error path follows the Java source.
//!
//! Layout
//! - [`args`]: `AbstractCommand`'s parse helpers (`parseInt`, `parseDouble`,
//!   `parseCoordinate` with `~` semantics, booleans, text joining).
//! - [`target`]: `TargetSelector` — the `@[parf][...]` pattern, its argument
//!   walk, and `PlayerManager.getMatching`'s filter and sort.
//! - [`rules`], [`scoreboard`], [`facts`]: the game rules, the scoreboard,
//!   and the facts commands read (game modes, difficulties, effects,
//!   enchantments, entity ids).
//! - [`world`]: the in-memory server the commands run against (worlds,
//!   players, inventories, message routing).
//! - [`registry`]: `CommandRegistry` — registration with aliases, the run
//!   loop with its selector fan-out, and suggestions.
//! - [`commands`], [`scoreboard_cmd`]: the command set.
//!
//! A probe prints deterministic output: `cargo +1.99.1 run -p
//! sunset-1-7-commands --bin commands`.

pub mod args;
pub mod commands;
pub mod entities;
pub mod error;
pub mod facts;
pub mod registry;
pub mod rules;
pub mod scoreboard;
pub mod scoreboard_cmd;
pub mod target;
pub mod text;
pub mod world;
