//! Minecraft 1.7.10 render, ported for sunset.
//!
//! The client block rendering model: `RenderBlocks`-style standard cube
//! rendering with per-face textures and tint, the non-cube shapes, the 1.7.10
//! light and smooth-lighting model, and the old HUD layout.
//!
//! - [`material`]: block materials, the flags the renderer reads.
//! - [`sprite`]: atlas sprites and their UV mapping.
//! - [`tesselator`]: the vertex sink, in vanilla's 32-byte-vertex shape.
//! - [`world`]: the `WorldView` trait and the `getLightColor` packing.
//! - [`block`]: the render-side block descriptor (shape, culling, sprites).
//! - [`stairs`]: the stair shape helpers.
//! - [`render_blocks`]: `RenderBlocks` itself.
//! - [`hud`]: the old HUD layout (hearts, hunger, armor, air, xp, hotbar).
//! - [`fixture`]: sample blocks and a test world.

pub mod block;
pub mod fixture;
pub mod hud;
pub mod material;
pub mod render_blocks;
pub mod sprite;
pub mod stairs;
pub mod tesselator;
pub mod world;
