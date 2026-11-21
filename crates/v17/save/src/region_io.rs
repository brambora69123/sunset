//! `RegionIo`: the map of open region files behind the chunk storage, one
//! file per 32×32 chunk region, named `r.<x >> 5>.<z >> 5>.mca` under the
//! world folder's `region/` folder. Opening one more file than 256 closes
//! every file in the map (`RegionIo.flush`); a `flush` is also what a world
//! save calls at the end.

use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

use crate::region::{RegionFile, chunk_index, region_path};

/// `RegionIo`: the region files the world holds open.
#[derive(Default)]
pub struct RegionIo {
    files: HashMap<PathBuf, RegionFile>,
}

impl RegionIo {
    pub fn new() -> Self {
        Self::default()
    }

    /// `RegionIo.getRegionFile`: the region file the chunk belongs to,
    /// opened (and its folder created) on first use. The map holds up to 256
    /// files: opening one more flushes it first, closing every file.
    pub fn region_file(
        &mut self,
        world_dir: &Path,
        chunk_x: i32,
        chunk_z: i32,
    ) -> io::Result<&mut RegionFile> {
        let path = region_path(world_dir, chunk_x, chunk_z);
        if !self.files.contains_key(&path) {
            fs::create_dir_all(path.parent().unwrap_or(Path::new(".")))?;
            if self.files.len() >= 256 {
                self.flush();
            }
            self.files.insert(path.clone(), RegionFile::open(&path)?);
        }
        Ok(self.files.get_mut(&path).unwrap())
    }

    /// `RegionIo.flush`: close every region file in the map.
    pub fn flush(&mut self) {
        self.files.clear();
    }

    /// `RegionIo.getChunkInputStream`: the chunk's stored bytes through the
    /// region file, `None` for a chunk it does not hold.
    pub fn chunk_data(
        &mut self,
        world_dir: &Path,
        chunk_x: i32,
        chunk_z: i32,
    ) -> io::Result<Option<crate::region::ChunkData>> {
        self.region_file(world_dir, chunk_x, chunk_z)?
            .read_chunk(chunk_x & 31, chunk_z & 31)
    }

    /// `NbtIo.write` into `RegionIo.getChunkOutputStream` + `close`: the NBT
    /// bytes compressed and stored for the chunk, stamped `timestamp`.
    pub fn write_chunk(
        &mut self,
        world_dir: &Path,
        chunk_x: i32,
        chunk_z: i32,
        nbt: &[u8],
        timestamp: u32,
    ) -> io::Result<()> {
        debug_assert!(
            chunk_index(chunk_x, chunk_z).is_some(),
            "chunk outside 0..31"
        );
        self.region_file(world_dir, chunk_x, chunk_z)?.write_chunk(
            chunk_x & 31,
            chunk_z & 31,
            nbt,
            timestamp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_opens_on_first_use_and_flushes_at_256() {
        let dir = std::env::temp_dir().join(format!("sunset-region-io-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut io = RegionIo::new();
        let nbt = vec![7u8; 1_000];
        io.write_chunk(&dir, 0, 0, &nbt, 5).unwrap();
        // The region folder and file exist on first use.
        assert!(dir.join("region/r.0.0.mca").is_file());
        let data = io.chunk_data(&dir, 0, 0).unwrap().unwrap();
        assert_eq!(data.decompress(1 << 20).unwrap(), nbt);
        assert_eq!(data.timestamp, 5);
        // Another region file opens on first use; the map holds both.
        io.write_chunk(&dir, 32, 0, &nbt, 6).unwrap();
        assert!(dir.join("region/r.1.0.mca").is_file());
        assert!(io.chunk_data(&dir, 32, 0).unwrap().is_some());
        // A chunk the region does not hold is absent, not an error.
        assert!(io.chunk_data(&dir, 5, 5).unwrap().is_none());
        // 256 files flush the map: the next open starts fresh.
        io.flush();
        let data = io.chunk_data(&dir, 0, 0).unwrap().unwrap();
        assert_eq!(data.decompress(1 << 20).unwrap(), nbt);
        fs::remove_dir_all(&dir).unwrap();
    }
}
