//! The 1.7.10 biome ids the structure checks read (`Biome.java`).

/// `Biome.PLAINS`.
pub const PLAINS: i32 = 1;
/// `Biome.DESERT`.
pub const DESERT: i32 = 2;
/// `Biome.SWAMPLAND`.
pub const SWAMPLAND: i32 = 6;
/// `Biome.DESERT_HILLS`.
pub const DESERT_HILLS: i32 = 17;
/// `Biome.JUNGLE`.
pub const JUNGLE: i32 = 21;
/// `Biome.JUNGLE_HILLS`.
pub const JUNGLE_HILLS: i32 = 22;
/// `Biome.SAVANNA`.
pub const SAVANNA: i32 = 35;

/// `StrongholdStructure.validBiomes`: every 1.7.10 biome whose `baseHeight` is
/// above 0 — all but the oceans (0, 24), the rivers (7, 11), the swampland
/// (-0.2), the mushroom-island shore (0.0) and the beaches (16, 26), which
/// `Biome.java`'s `setHeight` calls leave at or below sea level.
pub const STRONGHOLD_BIOMES: [i32; 31] = [
    PLAINS,
    DESERT,
    3,
    4,
    5,
    8,
    9,
    12,
    13,
    14,
    DESERT_HILLS,
    18,
    19,
    20,
    JUNGLE,
    JUNGLE_HILLS,
    23,
    25,
    27,
    28,
    29,
    30,
    31,
    32,
    33,
    34,
    SAVANNA,
    36,
    37,
    38,
    39,
];
