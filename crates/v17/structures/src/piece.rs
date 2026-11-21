//! One structure piece. Start placement only needs the bounds; piece assembly
//! (remaining work) grows this with the piece bodies, facing and NBT.

use crate::bounds::StructureBox;

/// Which structure a piece belongs to, tagged by its 1.7.10 class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    /// `VillagePieces$Start` — the village well, the first village piece.
    VillageStart,
    /// `TemplePieces$DesertPyramid`.
    DesertPyramid,
    /// `TemplePieces$JungleTemple`.
    JungleTemple,
    /// `TemplePieces$WitchHut`.
    WitchHut,
    /// `MineshaftPieces$Room`.
    MineshaftRoom,
    /// `StrongholdPieces$Start` — the spiral staircase, the first stronghold piece.
    StrongholdStart,
}

/// A piece: its kind, the biome the village Start sits in (vanilla's
/// `isDesertVillage`), and its bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructurePiece {
    pub kind: PieceKind,
    /// `VillagePieces$Start.isDesertVillage`.
    pub is_desert: bool,
    pub bounds: StructureBox,
}

impl StructurePiece {
    pub fn new(kind: PieceKind, bounds: StructureBox) -> Self {
        Self {
            kind,
            is_desert: false,
            bounds,
        }
    }
}
