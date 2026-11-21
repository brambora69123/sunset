//! Per-biome facts the heightmap weights and the surface builder read:
//! `Biome.baseHeight`, `Biome.heightVariation`, the surface/subsurface blocks
//! and the temperature, from `Biome.java` and its subclasses. Biome ids 0..39,
//! what the layer chain emits, and the 128+ mutants `Biome.mutate` registers.

use crate::{DIRT, GRASS, MYCELIUM, SAND, STAINED_HARDENED_CLAY, STONE};

/// `Blocks.SNOW`, the ice plains spikes' surface block (id 80).
const SNOW: u16 = 80;

/// `Biome.baseHeight`, the height weight each node blends.
pub fn base_height(id: i32) -> f32 {
    match id {
        0 | 10 => -1.0,                                // OCEAN_HEIGHT
        1 | 2 | 12 | 35 => 0.125,                      // PLAINS_HEIGHT
        3 | 34 => 1.0,                                 // EXTREME_HILLS_HEIGHT
        5 | 30 | 32 => 0.2,                            // TAIGA_HEIGHT
        6 => -0.2,                                     // SWAMP_HEIGHT
        7 | 11 => -0.5,                                // RIVER_HEIGHT
        13 | 17 | 18 | 19 | 22 | 28 | 31 | 33 => 0.45, // HILLS_HEIGHT
        14 => 0.2,                                     // MUSHROOM_HEIGHT
        15 | 16 | 26 => 0.0,                           // MUSHROOM_SHORE_HEIGHT
        20 => 0.8,                                     // EXTREME_HILLS_HEIGHT.diminish()
        24 => -1.8,                                    // DEEP_OCEAN_HEIGHT
        25 => 0.1,                                     // STONE_BEACH_HEIGHT
        36 | 38 | 39 => 1.5,                           // PLATEAU_HEIGHT
        // Mutated ids. `MutatedBiome` copies the original's height and adds
        // 0.1F to the base and 0.2F to the variation; the classes that
        // override `mutate` build their own, marked below.
        129 => 0.125,             // Sunflower Plains: PlainsBiome, PLAINS_HEIGHT
        130 => 0.125 + 0.1,       // Desert M
        131 | 162 => 1.0,         // Extreme Hills M/+ M: setHeight(original)
        132 => 0.1,               // Flower Forest: DEFAULT_HEIGHT + 0.2 variation
        133 => 0.2 + 0.1,         // Taiga M
        134 => -0.2 + 0.1,        // Swampland M
        140 => 0.125 + 0.3,       // Ice Plains Spikes: PLAINS_HEIGHT + 0.3
        149 | 151 => 0.1 + 0.1,   // Jungle M, Jungle Edge M
        155 | 157 => 0.1 + 0.1,   // Birch Forest M, Roofed Forest M
        156 => 0.45 + 0.1,        // Birch Forest Hills M
        158 => 0.2 + 0.1,         // Cold Taiga M
        160 | 161 => 0.2,         // Mega Spruce Taiga: setHeight(Mega Taiga)
        163 => 0.125 * 0.5 + 0.3, // Shattered Savanna
        164 => 1.5 * 0.5 + 0.3,   // Shattered Savanna Plateau
        165 => 0.1,               // Mesa (Bryce): DEFAULT_HEIGHT
        166 | 167 => 0.45,        // Mesa Plateau F M/Plateau M: HILLS_HEIGHT
        // DEFAULT_HEIGHT: FOREST, HELL, THE_END, JUNGLE, JUNGLE_EDGE,
        // BIRCH_FOREST, ROOFED_FOREST, MESA
        _ => 0.1,
    }
}

/// `Biome.heightVariation`, the variation weight each node blends.
pub fn height_variation(id: i32) -> f32 {
    match id {
        0 | 10 | 24 => 0.1,                           // OCEAN_HEIGHT, DEEP_OCEAN_HEIGHT
        1 | 2 | 12 | 35 => 0.05,                      // PLAINS_HEIGHT
        3 | 34 => 0.5,                                // EXTREME_HILLS_HEIGHT
        5 | 30 | 32 => 0.2,                           // TAIGA_HEIGHT
        6 => 0.1,                                     // SWAMP_HEIGHT
        7 | 11 => 0.0,                                // RIVER_HEIGHT
        13 | 17 | 18 | 19 | 22 | 28 | 31 | 33 => 0.3, // HILLS_HEIGHT
        14 => 0.3,                                    // MUSHROOM_HEIGHT
        15 | 16 | 26 => 0.025,                        // MUSHROOM_SHORE_HEIGHT
        20 => 0.3,                                    // diminished EXTREME_HILLS (0.5 * 0.6)
        25 => 0.8,                                    // STONE_BEACH_HEIGHT
        36 | 38 | 39 => 0.025,                        // PLATEAU_HEIGHT
        // Mutated ids, as `base_height` above.
        129 => 0.05,              // Sunflower Plains: PLAINS_HEIGHT
        130 => 0.05 + 0.2,        // Desert M
        131 | 162 => 0.5,         // Extreme Hills M/+ M
        132 => 0.2 + 0.2,         // Flower Forest: `mutate` adds 0.2
        133 => 0.2 + 0.2,         // Taiga M
        134 => 0.1 + 0.2,         // Swampland M
        140 => 0.05 + 0.4,        // Ice Plains Spikes: PLAINS_HEIGHT + 0.4
        149 | 151 => 0.2 + 0.2,   // Jungle M, Jungle Edge M
        155 | 157 => 0.2 + 0.2,   // Birch Forest M, Roofed Forest M
        156 => 0.3 + 0.2,         // Birch Forest Hills M
        158 => 0.2 + 0.2,         // Cold Taiga M
        160 | 161 => 0.2,         // Mega Spruce Taiga
        163 => 0.05 * 0.5 + 1.2,  // Shattered Savanna
        164 => 0.025 * 0.5 + 1.2, // Shattered Savanna Plateau
        165 => 0.2,               // Mesa (Bryce)
        166 | 167 => 0.3,         // Mesa Plateau F M, Mesa Plateau M
        _ => 0.2,                 // DEFAULT_HEIGHT
    }
}

/// `(surfaceBlock, surfaceBlockMetadata, subsurfaceBlock)`, the fields the
/// surface builder starts from. The depth-dependent overrides live in the
/// surface pass: `ExtremeHillsBiome` (3, 131, 162), `TaigaBiome` variants 1
/// and 2 (32, 33, 160, 161) and the shattered savanna (163, 164); the pass
/// implements the extreme hills one.
pub fn surface_blocks(id: i32) -> (u16, u8, u16) {
    match id {
        2 | 17 | 16 | 26 | 130 => (SAND, 0, SAND), // DesertBiome, BeachBiome
        14 | 15 => (MYCELIUM, 0, DIRT),            // MushroomBiome
        25 => (STONE, 0, STONE),                   // StoneBeachBiome
        37..=39 | 165..=167 => (SAND, 1, STAINED_HARDENED_CLAY), // MesaBiome
        140 => (SNOW, 0, DIRT),                    // Ice Plains Spikes: surfaceBlock
        _ => (GRASS, 0, DIRT),                     // Biome field defaults
    }
}

/// `Biome.temperature`, the base of the surface ice line.
pub fn temperature(id: i32) -> f32 {
    match id {
        2 | 17 | 8 => 2.0,       // DESERT, HELL
        3 | 20 | 34 | 25 => 0.2, // EXTREME_HILLS family, STONE_BEACH
        5 | 19 => 0.25,          // TAIGA
        6 => 0.8,                // SWAMPLAND
        10..=13 => 0.0,          // FROZEN_OCEAN, FROZEN_RIVER, ICE_PLAINS, ICE_MOUNTAINS
        14 | 15 => 0.9,          // MUSHROOM
        16 => 0.8,               // BEACH
        21..=23 => 0.95,         // JUNGLE, JUNGLE_HILLS, JUNGLE_EDGE
        26 => 0.05,              // COLD_BEACH
        30 | 31 => -0.5,         // COLD_TAIGA
        32 | 33 => 0.3,          // MEGA_TAIGA
        35 => 1.2,               // SAVANNA
        36 => 1.0,               // SAVANNA_PLATEAU
        // Mutated ids: `MutatedBiome` copies the original's temperature; the
        // overrides that set their own are `PlainsBiome` (0.8), `IceBiome`
        // (0.0), `TaigaBiome` (0.25) and `SavannaBiome` ((original + 1) * 0.5).
        129 => 0.8,               // Sunflower Plains
        130 => 2.0,               // Desert M
        131 | 162 => 0.2,         // Extreme Hills M, Extreme Hills+ M
        132 | 157 => 0.7,         // Flower Forest, Roofed Forest M
        133 | 160 | 161 => 0.25,  // Taiga M, Mega Spruce Taiga (and its alias)
        134 => 0.8,               // Swampland M
        140 => 0.0,               // Ice Plains Spikes
        149 | 151 => 0.95,        // Jungle M, Jungle Edge M
        155 | 156 => 0.6,         // Birch Forest M, Birch Forest Hills M
        158 => -0.5,              // Cold Taiga M
        163 => (1.2 + 1.0) * 0.5, // Shattered Savanna
        164 => (1.0 + 1.0) * 0.5, // Shattered Savanna Plateau
        165..=167 => 2.0,         // Mesa mutants
        _ => 0.5,                 // Biome constructor default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_constants() {
        assert_eq!(base_height(0), -1.0);
        assert_eq!(base_height(24), -1.8);
        assert_eq!(base_height(36), 1.5);
        assert_eq!(height_variation(7), 0.0);
        assert_eq!(height_variation(26), 0.025);
        assert_eq!(height_variation(20), 0.3);
    }

    #[test]
    fn surface_constants() {
        assert_eq!(surface_blocks(1), (GRASS, 0, DIRT));
        assert_eq!(surface_blocks(2), (SAND, 0, SAND));
        assert_eq!(surface_blocks(14), (MYCELIUM, 0, DIRT));
        assert_eq!(surface_blocks(25), (STONE, 0, STONE));
        assert_eq!(surface_blocks(37), (SAND, 1, STAINED_HARDENED_CLAY));
    }

    #[test]
    fn temperature_constants() {
        assert_eq!(temperature(1), 0.5);
        assert_eq!(temperature(30), -0.5);
        assert_eq!(temperature(35), 1.2);
    }

    /// `Biome.baseHeight` and `heightVariation` of every registered mutant,
    /// bit for bit as the 1.7.10 jar holds them (`Float.floatToIntBits`).
    /// The rounding is load-bearing: `0.05F + 0.4F` is one ulp above the
    /// literal 0.45 and both kinds of height feed the same f32 weights.
    #[test]
    fn mutated_heights_match_the_vanilla_fields() {
        // (id, baseHeight bits, heightVariation bits).
        let want: [(i32, u32, u32); 21] = [
            (129, 0x3e00_0000, 0x3d4c_cccd), // 0.125 / 0.05
            (130, 0x3e66_6666, 0x3e80_0000), // 0.225 / 0.25
            (131, 0x3f80_0000, 0x3f00_0000), // 1.0 / 0.5
            (132, 0x3dcc_cccd, 0x3ecc_cccd), // 0.1 / 0.4
            (133, 0x3e99_999a, 0x3ecc_cccd), // 0.3 / 0.4
            (134, 0xbdcc_cccd, 0x3e99_999a), // -0.1 / 0.3
            (140, 0x3ed9_999a, 0x3ee6_6667), // 0.425 / 0.45000002
            (149, 0x3e4c_cccd, 0x3ecc_cccd), // 0.2 / 0.4
            (151, 0x3e4c_cccd, 0x3ecc_cccd), // 0.2 / 0.4
            (155, 0x3e4c_cccd, 0x3ecc_cccd), // 0.2 / 0.4
            (156, 0x3f0c_cccd, 0x3f00_0000), // 0.55 / 0.5
            (157, 0x3e4c_cccd, 0x3ecc_cccd), // 0.2 / 0.4
            (158, 0x3e99_999a, 0x3ecc_cccd), // 0.3 / 0.4
            (160, 0x3e4c_cccd, 0x3e4c_cccd), // 0.2 / 0.2
            (161, 0x3e4c_cccd, 0x3e4c_cccd), // Mega Taiga Hills alias of 160
            (162, 0x3f80_0000, 0x3f00_0000), // 1.0 / 0.5
            (163, 0x3eb9_999a, 0x3f9c_cccd), // 0.3625 / 1.225
            (164, 0x3f86_6666, 0x3f9b_3334), // 1.05 / 1.2125001
            (165, 0x3dcc_cccd, 0x3e4c_cccd), // 0.1 / 0.2
            (166, 0x3ee6_6666, 0x3e99_999a), // 0.45 / 0.3
            (167, 0x3ee6_6666, 0x3e99_999a), // 0.45 / 0.3
        ];
        for (id, base, variation) in want {
            assert_eq!(base_height(id).to_bits(), base, "biome {id} base height");
            assert_eq!(
                height_variation(id).to_bits(),
                variation,
                "biome {id} height variation"
            );
        }
    }

    /// The surface and temperature fields `MutatedBiome` copies from its
    /// original, and the ones the overridden `mutate` methods set themselves.
    #[test]
    fn mutated_fields_match_the_vanilla_biomes() {
        // Grass/dirt mutants: Sunflower Plains, Extreme Hills M/+ M, Flower
        // Forest, Taiga M, Swampland M, Jungle M/Edge M, the birch and roofed
        // forests, Cold Taiga M and the mega taigas.
        for id in [
            129, 131, 132, 133, 134, 149, 151, 155, 156, 157, 158, 160, 161, 162,
        ] {
            assert_eq!(surface_blocks(id), (GRASS, 0, DIRT), "biome {id}");
        }
        assert_eq!(surface_blocks(130), (SAND, 0, SAND)); // Desert M
        assert_eq!(surface_blocks(140), (SNOW, 0, DIRT)); // Ice Plains Spikes
        for id in 165..=167 {
            assert_eq!(
                surface_blocks(id),
                (SAND, 1, STAINED_HARDENED_CLAY),
                "biome {id}"
            );
        }

        // (id, temperature bits) from the same jar read.
        let want: [(i32, u32); 21] = [
            (129, 0x3f4c_cccd), // 0.8
            (130, 0x4000_0000), // 2.0
            (131, 0x3e4c_cccd), // 0.2
            (132, 0x3f33_3333), // 0.7
            (133, 0x3e80_0000), // 0.25
            (134, 0x3f4c_cccd), // 0.8
            (140, 0x0000_0000), // 0.0
            (149, 0x3f73_3333), // 0.95
            (151, 0x3f73_3333), // 0.95
            (155, 0x3f19_999a), // 0.6
            (156, 0x3f19_999a), // 0.6
            (157, 0x3f33_3333), // 0.7
            (158, 0xbf00_0000), // -0.5
            (160, 0x3e80_0000), // 0.25
            (161, 0x3e80_0000), // 0.25
            (162, 0x3e4c_cccd), // 0.2
            (163, 0x3f8c_cccd), // 1.1
            (164, 0x3f80_0000), // 1.0
            (165, 0x4000_0000), // 2.0
            (166, 0x4000_0000), // 2.0
            (167, 0x4000_0000), // 2.0
        ];
        for (id, bits) in want {
            assert_eq!(temperature(id).to_bits(), bits, "biome {id} temperature");
        }
    }
}
