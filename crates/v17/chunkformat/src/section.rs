//! `net.minecraft.world.chunk.WorldChunkSection`: one 16³ slab of the chunk,
//! with the low block ids in `Blocks` and the high nibble of ids above 255 in
//! the `Add` array, plus the metadata and both light arrays.

use crate::nibble::NibbleStorage;

/// Blocks in one section, `4096`, indexed `y << 8 | z << 4 | x`.
pub const SECTION_BLOCKS: usize = 4096;

/// One chunk section: `WorldChunkSection`. `offset_y` is the section's lowest
/// world Y (`y << 4`); `sky_light` is absent only in dimensions with no sky.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub offset_y: i32,
    /// Blocks whose id is not air (id 0), as `nonAirBlockCount` counts them.
    pub non_air_block_count: i32,
    pub blocks: [u8; SECTION_BLOCKS],
    /// The high nibble of block ids above 255, created on demand.
    pub add: Option<NibbleStorage>,
    pub metadata: NibbleStorage,
    pub block_light: NibbleStorage,
    pub sky_light: Option<NibbleStorage>,
}

impl Section {
    /// `new WorldChunkSection(offsetY, hasSkyLight)`: all-zero storage.
    pub fn new(offset_y: i32, has_sky_light: bool) -> Self {
        Self {
            offset_y,
            non_air_block_count: 0,
            blocks: [0; SECTION_BLOCKS],
            add: None,
            metadata: NibbleStorage::new_section(),
            block_light: NibbleStorage::new_section(),
            sky_light: has_sky_light.then(NibbleStorage::new_section),
        }
    }

    /// The flat index of a section-local position: `y << 8 | z << 4 | x`.
    fn index(x: u8, y: u8, z: u8) -> usize {
        (y as usize) << 8 | (z as usize) << 4 | x as usize
    }

    /// `getBlock`'s id math: the low byte plus the `Add` nibble, shifted up 8.
    pub fn block_id(&self, x: u8, y: u8, z: u8) -> u16 {
        let mut id = self.blocks[Self::index(x, y, z)] as u16;
        if let Some(add) = &self.add {
            id |= (add.get(x, y, z) as u16) << 8;
        }
        id
    }

    /// `setBlock`'s id math: low byte always written, `Add` nibble written for
    /// ids above 255 (and zeroed below), air counting kept.
    pub fn set_block_id(&mut self, x: u8, y: u8, z: u8, id: u16) {
        let i = Self::index(x, y, z);
        // Vanilla counts a non-null block that isn't air; here ids are air (0) or set.
        let was_air = self.blocks[i] == 0 && self.add.as_ref().is_none_or(|a| a.get(x, y, z) == 0);
        if !was_air {
            self.non_air_block_count -= 1;
        }
        if id != 0 {
            self.non_air_block_count += 1;
        }

        self.blocks[i] = (id & 0xFF) as u8;
        if id > 255 {
            self.add.get_or_insert_with(NibbleStorage::new_section).set(
                x,
                y,
                z,
                ((id & 3840) >> 8) as u8,
            );
        } else if let Some(add) = &mut self.add {
            add.set(x, y, z, 0);
        }
    }

    /// `getBlockMetadata` / `getBlockLight` / `getSkyLight`, by array.
    pub fn metadata_at(&self, x: u8, y: u8, z: u8) -> u8 {
        self.metadata.get(x, y, z)
    }

    pub fn block_light_at(&self, x: u8, y: u8, z: u8) -> u8 {
        self.block_light.get(x, y, z)
    }

    pub fn sky_light_at(&self, x: u8, y: u8, z: u8) -> u8 {
        self.sky_light.as_ref().map_or(0, |s| s.get(x, y, z))
    }

    pub fn set_metadata(&mut self, x: u8, y: u8, z: u8, value: u8) {
        self.metadata.set(x, y, z, value);
    }

    pub fn set_block_light(&mut self, x: u8, y: u8, z: u8, value: u8) {
        self.block_light.set(x, y, z, value);
    }

    pub fn set_sky_light(&mut self, x: u8, y: u8, z: u8, value: u8) {
        if let Some(sky) = &mut self.sky_light {
            sky.set(x, y, z, value);
        }
    }

    /// `isEmpty`: no non-air blocks.
    pub fn is_empty(&self) -> bool {
        self.non_air_block_count == 0
    }

    /// `validateBlockCounters` for the non-air count: recomputed from the full
    /// ids (a low byte of 0 with an `Add` nibble still marks a block).
    pub fn validate_block_counters(&mut self) {
        let mut count = 0;
        let add = &self.add;
        for (i, &low) in self.blocks.iter().enumerate() {
            let x = (i & 15) as u8;
            let z = ((i >> 4) & 15) as u8;
            let y = (i >> 8) as u8;
            let high = add.as_ref().map_or(0, |a| a.get(x, y, z)) as u16;
            if (low as u16 | high << 8) != 0 {
                count += 1;
            }
        }
        self.non_air_block_count = count;
    }
}

/// A flat array of 65536 block ids, indexed `y << 8 | z << 4 | x` — the
/// section-internal order extended to the full 256-block height.
pub type FlatBlocks = [u16; 65536];

/// Packs a flat 65536-id array into 16 sections, creating a section only for
/// heights that hold a non-air block (`WorldChunk`'s `Block[]` constructor).
pub fn sections_from_flat(blocks: &FlatBlocks, has_sky_light: bool) -> [Option<Section>; 16] {
    let mut sections: [Option<Section>; 16] = Default::default();
    for (i, &id) in blocks.iter().enumerate() {
        if id == 0 {
            continue;
        }
        let x = (i & 15) as u8;
        let z = ((i >> 4) & 15) as u8;
        let y = (i >> 8) as u8;
        let section = &mut sections[i >> 12];
        let s = section.get_or_insert_with(|| Section::new(((i >> 12) as i32) << 4, has_sky_light));
        s.set_block_id(x, y & 15, z, id);
    }
    sections
}

/// Unpacks sections back into a flat 65536-id array; absent sections are air.
pub fn flat_from_sections(sections: &[Option<Section>; 16]) -> FlatBlocks {
    let mut out = [0u16; 65536];
    for (sy, section) in sections.iter().enumerate() {
        if let Some(s) = section {
            for (i, slot) in out[sy * SECTION_BLOCKS..(sy + 1) * SECTION_BLOCKS]
                .iter_mut()
                .enumerate()
            {
                let x = (i & 15) as u8;
                let z = ((i >> 4) & 15) as u8;
                let y = (i >> 8) as u8;
                *slot = s.block_id(x, y, z);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(i: usize) -> (u8, u8, u8) {
        ((i & 15) as u8, ((i >> 8) & 15) as u8, ((i >> 4) & 15) as u8)
    }

    #[test]
    fn block_ids_round_trip_including_add() {
        let mut s = Section::new(0, true);
        // Low ids and high ids above 255, through the whole id space at 4096 spots.
        for i in 0..SECTION_BLOCKS {
            let id = ((i * 31) % 700) as u16;
            let (x, y, z) = pos(i);
            s.set_block_id(x, y, z, id);
        }
        for i in 0..SECTION_BLOCKS {
            let (x, y, z) = pos(i);
            assert_eq!(s.block_id(x, y, z), ((i * 31) % 700) as u16);
        }
        assert!(s.add.is_some());
    }

    #[test]
    fn add_nibble_zeroed_when_id_drops_below_256() {
        let mut s = Section::new(0, true);
        s.set_block_id(1, 2, 3, 600);
        assert_eq!(s.block_id(1, 2, 3), 600);
        s.set_block_id(1, 2, 3, 4);
        assert_eq!(s.block_id(1, 2, 3), 4);
        // The Add array stays allocated once created, as in vanilla.
        assert!(s.add.is_some());
    }

    #[test]
    fn air_count_tracks_sets() {
        let mut s = Section::new(0, true);
        s.set_block_id(0, 0, 0, 1);
        s.set_block_id(0, 0, 1, 300);
        assert_eq!(s.non_air_block_count, 2);
        // Overwriting a block in place does not change the count.
        s.set_block_id(0, 0, 1, 300);
        assert_eq!(s.non_air_block_count, 2);
        s.set_block_id(0, 0, 1, 0);
        assert_eq!(s.non_air_block_count, 1);
        assert!(!s.is_empty());
        s.set_block_id(0, 0, 0, 0);
        assert!(s.is_empty());
        // Vanilla keeps the Add array allocated once created.
        assert!(s.add.is_some());
    }

    #[test]
    fn light_and_metadata_store_independently() {
        let mut s = Section::new(0, true);
        s.set_metadata(5, 6, 7, 13);
        s.set_block_light(5, 6, 7, 9);
        s.set_sky_light(5, 6, 7, 15);
        assert_eq!(s.metadata_at(5, 6, 7), 13);
        assert_eq!(s.block_light_at(5, 6, 7), 9);
        assert_eq!(s.sky_light_at(5, 6, 7), 15);
        // No sky: writes are dropped, reads return 0.
        let mut dark = Section::new(0, false);
        dark.set_sky_light(5, 6, 7, 15);
        assert_eq!(dark.sky_light_at(5, 6, 7), 0);
    }

    #[test]
    fn flat_packing_round_trips_through_full_height() {
        let mut flat = [0u16; 65536];
        // Deterministic spread across every section and both id ranges.
        for (i, slot) in flat.iter_mut().enumerate() {
            let id = ((i * 2654435761 % 1000003) % 1000) as u16;
            *slot = if id < 500 { 0 } else { id };
        }
        let sections = sections_from_flat(&flat, true);
        // Sections hold non-air only where the flat array does.
        for (sy, section) in sections.iter().enumerate() {
            let any = flat[sy * SECTION_BLOCKS..(sy + 1) * SECTION_BLOCKS]
                .iter()
                .any(|&id| id != 0);
            assert_eq!(section.is_some(), any, "section {sy}");
        }
        assert_eq!(flat_from_sections(&sections), flat);
    }

    #[test]
    fn empty_flat_yields_no_sections() {
        let sections = sections_from_flat(&[0u16; 65536], true);
        assert!(sections.iter().all(Option::is_none));
    }
}
