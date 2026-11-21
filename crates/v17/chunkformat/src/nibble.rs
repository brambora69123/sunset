//! `net.minecraft.world.chunk.ChunkNibbleStorage`: 4 bits per block in a 16³
//! section, packed two to a byte with the **low** nibble holding the even
//! (x + z + y even) index.
//!
//! The coordinate packing is x/z-swapped versus `AlphaChunkDataArray`'s, which
//! the Region format converter compensates for.

/// One nibble array covering a 4096-block section (2048 bytes), or any even
/// byte-paired storage the chunk format needs.
#[derive(Clone, PartialEq, Eq)]
pub struct NibbleStorage {
    pub data: Vec<u8>,
    /// Bits to shift `z` by (`zBitOffset` in vanilla; 4 for a section array).
    pub z_bit_offset: u32,
    /// Bits to shift `y` by (`yBitOffset`, always the z offset + 4).
    pub y_bit_offset: u32,
}

impl NibbleStorage {
    /// A fresh all-zero array for one section, z shifted by 4: `new ChunkNibbleStorage(size, 4)`.
    pub fn new_section() -> Self {
        Self {
            data: vec![0; 2048],
            z_bit_offset: 4,
            y_bit_offset: 8,
        }
    }

    /// Wraps existing bytes: `new ChunkNibbleStorage(data, zBitOffset)`.
    pub fn from_data(data: Vec<u8>, z_bit_offset: u32) -> Self {
        let y_bit_offset = z_bit_offset + 4;
        Self {
            data,
            z_bit_offset,
            y_bit_offset,
        }
    }

    /// Vanilla's flat index: `y << yBitOffset | z << zBitOffset | x`.
    fn index(&self, x: u8, y: u8, z: u8) -> usize {
        ((y as usize) << self.y_bit_offset) | ((z as usize) << self.z_bit_offset) | x as usize
    }

    /// `ChunkNibbleStorage.get`: even indices from the low nibble, odd from the high.
    pub fn get(&self, x: u8, y: u8, z: u8) -> u8 {
        let i = self.index(x, y, z);
        if i & 1 == 0 {
            self.data[i >> 1] & 15
        } else {
            self.data[i >> 1] >> 4 & 15
        }
    }

    /// `ChunkNibbleStorage.set`, preserving the sibling nibble's bits.
    pub fn set(&mut self, x: u8, y: u8, z: u8, value: u8) {
        let i = self.index(x, y, z);
        let value = value & 15;
        let slot = &mut self.data[i >> 1];
        *slot = if i & 1 == 0 {
            *slot & 0xf0 | value
        } else {
            *slot & 0x0f | value << 4
        };
    }
}

impl std::fmt::Debug for NibbleStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NibbleStorage")
            .field("len", &self.data.len())
            .field("z_bit_offset", &self.z_bit_offset)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nibble_round_trip_matches_vanilla_layout() {
        let mut s = NibbleStorage::new_section();
        // Every value and coordinate pair, so packing order is pinned exactly.
        for y in 0..16u8 {
            for z in 0..16u8 {
                for x in 0..16u8 {
                    s.set(x, y, z, (x + y * 3 + z * 7) & 15);
                }
            }
        }
        for y in 0..16u8 {
            for z in 0..16u8 {
                for x in 0..16u8 {
                    assert_eq!(s.get(x, y, z), (x + y * 3 + z * 7) & 15);
                }
            }
        }
        // Even index holds the low nibble: chunk (0,0,0) = 9 lands in byte 0's
        // low bits, leaving (1,0,0)'s high nibble alone.
        s.set(0, 0, 0, 9);
        assert_eq!(s.data[0] & 0x0f, 9);
        // Odd index, (1,0,0), holds the high nibble.
        s.set(1, 0, 0, 12);
        assert_eq!(s.data[0], 9 | 12 << 4);
    }

    #[test]
    fn set_preserves_sibling_nibble() {
        let mut s = NibbleStorage::new_section();
        s.set(1, 0, 0, 0xf);
        s.set(0, 0, 0, 0x7);
        assert_eq!(s.data[0], 0xf7);
        s.set(1, 0, 0, 0x1);
        assert_eq!(s.data[0], 0x17);
    }

    #[test]
    fn from_data_uses_given_offsets() {
        // As `AlphaChunkDataArray(data, 7)` builds: z shifted 7, y shifted 11.
        let s = NibbleStorage::from_data(vec![0; 2048], 7);
        assert_eq!(s.y_bit_offset, 11);
    }
}
