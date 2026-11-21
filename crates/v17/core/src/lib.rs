//! Shared primitives for the 1.7.10 port.

/// `java.util.Random`, exact. Re-exported so every 1.7.10 crate draws identically.
pub use sunrise_core::random::{LegacyRandom, RandomSource};
