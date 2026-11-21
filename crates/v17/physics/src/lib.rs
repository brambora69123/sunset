//! Minecraft 1.7.10 entity physics.
//!
//! Ported from the 1.7.10 decompiled sources
//! (`net.minecraft.entity.Entity`, `LivingEntity`): AABB move and clip,
//! strafe acceleration, ground/water/lava travel, gravity, step height,
//! sprint jump, fall distance and fall damage inputs.
//!
//! - [`Aabb`] — `net.minecraft.util.math.Box`
//! - [`math`] — the `MathHelper` subset physics reads
//! - [`world::PhysWorld`] — the block tables the game crates implement
//! - [`entity::EntityPhysics`] — one entity's physics state and movement
//! - [`living`] — `LivingEntity.moveRelative`, jump, knockback, mob tick
//! - [`fixture`] — block tables for tests and the parity probe

pub mod aabb;
pub mod entity;
pub mod fixture;
pub mod living;
pub mod material;
pub mod math;
pub mod world;

pub use aabb::Aabb;
pub use entity::{DamageSource, EntityPhysics};
pub use material::Material;
pub use world::PhysWorld;
