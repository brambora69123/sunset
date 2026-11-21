//! Minecraft 1.7.10 assets, ported for sunset: the resource pack pipeline
//! (format 1), the `sounds.json` format, the `.lang` files, the font width
//! tables, and the 1.7.10 block/item id → texture-name mapping.
//!
//! Ported from the 1.7.10 decompiled reference (calamus-modernized names,
//! exact logic): `net.minecraft.client.resource.*`,
//! `net.minecraft.client.sound.*` (resource side),
//! `net.minecraft.client.render.TextRenderer` (widths), and the sprite
//! registration of every block and item. The drawing and OpenAL halves are
//! the render crate's job; this crate owns the resource-side contract.

pub mod font;
pub mod identifier;
pub mod json;
pub mod lang;
pub mod resource;
pub mod sound;
pub mod sprites;

pub use identifier::Identifier;
