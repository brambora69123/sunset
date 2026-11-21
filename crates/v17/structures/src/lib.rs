//! Minecraft 1.7.10 structures: structure START placement, ported from
//! `world/world/structure/*.java` (MapGenStructure base, Village, Stronghold,
//! Mineshaft, Temple). Piece assembly is a later step.

pub mod biomes;
pub mod bounds;
pub mod feature;
pub mod mineshaft;
pub mod piece;
pub mod start;
pub mod stronghold;
pub mod temple;
pub mod village;

pub use bounds::StructureBox;
pub use feature::{
    StructureFeature, StructureGen, chunk_pos_to_long, next_int, next_int_between, parse_double,
    parse_double_unclamped, parse_int, parse_int_unclamped,
};
pub use mineshaft::MineshaftStructure;
pub use piece::{PieceKind, StructurePiece};
pub use start::StructureStart;
pub use stronghold::StrongholdStructure;
pub use temple::TempleStructure;
pub use village::VillageStructure;
