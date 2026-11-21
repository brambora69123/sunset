//! Minecraft 1.7.10 world persistence, ported from
//! `net.minecraft.world.storage` and `net.minecraft.world.chunk.storage`:
//!
//! - [`java_zip`]: the gzip and zlib streams `java.util.zip` writes, from
//!   the zlib the JDK bundles (`sunrise-deflate`), so saved bytes match
//!   vanilla's.
//! - [`region`]: the Anvil region file — the location and timestamp tables,
//!   4 KiB sectors, the placement quirks of 1.7.10's `RegionFile`.
//! - [`region_io`]: `RegionIo`, the map of open region files behind chunk
//!   storage, with its 256-file flush.
//! - [`level_dat`]: `level.dat`'s 1.7.10 field set (`WorldData`), its
//!   rotation through `level.dat_new` and `level.dat_old`, and the version
//!   stamp 19133 — no `DataVersion`; that starts in 1.8.
//! - [`player`]: the player compound (`playerdata/<uuid>.dat`, and `Player`
//!   inside single-player `level.dat`), `PlayerEntity.writeNbtWithoutId`'s
//!   field set and the read chain's coercions.
//!
//! The NBT underneath every file is `sunrise_core::nbt`, whose compound
//! keeps `java.util.HashMap`'s iteration order — what 1.7.10's `NbtCompound`
//! writes its entries in.

pub mod java_zip;
pub mod level_dat;
pub mod player;
pub mod region;
pub mod region_io;
