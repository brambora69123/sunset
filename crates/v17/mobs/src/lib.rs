//! Minecraft 1.7.10 mob AI, ported for sunset.
//!
//! Ported from the 1.7.10 decompiled sources in
//! `.local/minecraft/1.7.10/sources/server/net/minecraft/`:
//!
//! - [`goal`]: `Goal`/`GoalSelector` goal selection (`entity/ai/goal/`)
//! - [`path`], [`pathfinder`], [`navigation`]: pathfinding and navigation (`entity/ai/pathing/`)
//! - [`spawn`]: hostile and passive spawn cycle (`world/NaturalSpawner.java`)
//! - [`breeding`]: animal breeding and baby growth (`entity/living/mob/passive/`)
//! - [`mob`]: the mob entity and its tick chains (`entity/living/mob/MobEntity.java`)
//! - [`controls`]: movement/look/jump controls (`entity/ai/control/`)
//! - [`fixture`]: a fixture [`world::MobWorld`] for probes and tests

// The port keeps vanilla's `if (!(x < y))` float comparisons and clamp ladders as written.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::manual_clamp)]

pub mod blocks;
pub mod breeding;
pub mod controls;
pub mod fixture;
pub mod goal;
pub mod math;
pub mod mob;
pub mod navigation;
pub mod path;
pub mod pathfinder;
pub mod spawn;
pub mod world;
