//! Anvil region files, as 1.7.10's `net.minecraft.world.chunk.storage.RegionFile`
//! reads and writes them: 8 KiB of header (1024 chunk locations, then 1024
//! timestamps, each a big-endian `int`), then 4 KiB sectors. A chunk's payload
//! is its length (`int`), one compression byte (1 gzip, 2 zlib) and the
//! compressed NBT, zero-padded to the sector.
//!
//! Placement and bookkeeping follow the Java class exactly, including its
//! quirks: the sector count is `(size + 5) / 4096 + 1` (one more than the
//! payload needs), a write of 256 or more sectors is silently dropped, a
//! fresh file grows one run at a time at the end, freed sectors are only
//! reused once no existing gap fits, and the header's sectors 0 and 1 are
//! never handed out. In the found-run case the header entry is written before
//! the payload; in the append case, after — vanilla's crash windows, kept.
//!
//! The Java class runs one open file per region in a static cache; this port
//! is the file itself. `RegionIo`'s naming (`r.<x>.<z>.mca`, chunk
//! coordinates `& 31`, region coordinates `>> 5`) lives in [`region_path`]
//! and [`chunk_index`].

use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const SECTOR: usize = 4096;
const HEADER: usize = 2 * SECTOR;

/// The compression byte a chunk payload starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ChunkCompression {
    /// `DeflaterOutputStream` at the default level: a zlib stream. What
    /// vanilla 1.7.10 writes.
    Zlib = 2,
    /// `GZIPInputStream`'s member; 1.7.10 reads it for older saves only.
    Gzip = 1,
}
impl ChunkCompression {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Gzip),
            2 => Some(Self::Zlib),
            _ => None,
        }
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// One stored chunk: the compressed bytes, how they are compressed and the
/// timestamp the header's second table carries for the chunk.
pub struct ChunkData {
    pub compression: ChunkCompression,
    pub data: Vec<u8>,
    pub timestamp: u32,
}
impl ChunkData {
    /// The chunk's NBT bytes, decompressed. `max_output` stands in for the
    /// stream reader's endless patience: Java reads until the stream says so,
    /// this refuses a payload past the cap, which no honest 1.7.10 chunk
    /// reaches.
    pub fn decompress(&self, max_output: usize) -> io::Result<Vec<u8>> {
        match self.compression {
            ChunkCompression::Zlib => crate::java_zip::unzlib(&self.data, max_output),
            ChunkCompression::Gzip => crate::java_zip::gunzip(&self.data),
        }
    }
}

/// `RegionIo.getRegionFile`'s path for the region holding the chunk:
/// `<dir>/region/r.<x >> 5>.<z >> 5>.mca`.
pub fn region_path(world_dir: &Path, chunk_x: i32, chunk_z: i32) -> PathBuf {
    world_dir
        .join("region")
        .join(format!("r.{}.{}.mca", chunk_x >> 5, chunk_z >> 5))
}

/// `RegionFile`'s local slot: `chunkX & 31 + (chunkZ & 31) * 32`, the header
/// row the chunk occupies. Outside 0..31 either way it is `None`
/// (`isOutsideRegion`): the caller treats that as no chunk.
pub fn chunk_index(chunk_x: i32, chunk_z: i32) -> Option<usize> {
    let (x, z) = (chunk_x & 31, chunk_z & 31);
    if (0..32).contains(&x) && (0..32).contains(&z) {
        Some(x as usize + z as usize * 32)
    } else {
        None
    }
}

/// A region file. [`RegionFile::open`] does the constructor's work: a file
/// shorter than 8 KiB is padded to it (a fresh file gets the empty header), a
/// length that is not a multiple of 4 KiB is padded to the next one, and both
/// tables are read into memory.
pub struct RegionFile {
    file: File,
    path: PathBuf,
    /// The location table: each entry `(sector << 8) | sector_count`, 0 for
    /// no chunk (`chunkBlockInfo`).
    offsets: [u32; 1024],
    /// The timestamp table, seconds since the epoch (`chunkSaveTimes`).
    timestamps: [u32; 1024],
    /// One flag per sector of the file: is it free? The header's sectors 0
    /// and 1 start in use, as vanilla's `blockEmptyFlags` makes them. The
    /// vector grows when the file grows.
    block_empty_flags: Vec<bool>,
    /// `bytesWritten`, kept for parity with the Java field (vanilla never
    /// reads it either).
    bytes_written: u64,
}

impl RegionFile {
    /// `new RegionFile(file)`: open for reading and writing, creating the
    /// file when absent, and read both tables.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        let mut length = file.metadata()?.len();
        if length < HEADER as u64 {
            // 2048 writeInt(0): the empty header, over a fresh or short file.
            file.seek(SeekFrom::Start(length))?;
            let zeros = [0u8; SECTOR];
            let mut remaining = HEADER as u64 - length;
            while remaining > 0 {
                let take = remaining.min(SECTOR as u64) as usize;
                file.write_all(&zeros[..take])?;
                remaining -= take as u64;
            }
            length = HEADER as u64;
        }
        if length % SECTOR as u64 != 0 {
            // Zero-fill the ragged tail to the next sector.
            file.seek(SeekFrom::End(0))?;
            let zeros = [0u8; SECTOR];
            file.write_all(&zeros[..(SECTOR as u64 - length % SECTOR as u64) as usize])?;
            length += SECTOR as u64 - length % SECTOR as u64;
        }
        let mut header = vec![0; HEADER];
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut header)?;
        let mut offsets = [0u32; 1024];
        let mut timestamps = [0u32; 1024];
        for i in 0..1024 {
            offsets[i] = u32::from_be_bytes(header[i * 4..i * 4 + 4].try_into().unwrap());
            timestamps[i] = u32::from_be_bytes(
                header[SECTOR + i * 4..SECTOR + i * 4 + 4]
                    .try_into()
                    .unwrap(),
            );
        }
        let sector_count = (length / SECTOR as u64) as usize;
        let mut block_empty_flags = vec![true; sector_count];
        block_empty_flags[0] = false;
        block_empty_flags[1] = false;
        // Mark every chunk's run in use. An entry whose run overruns the file
        // (or claims no sectors) marks nothing, as vanilla's bounds check
        // skips it; its chunk reads as absent.
        for &value in offsets.iter().filter(|&&v| v != 0) {
            let (start, count) = ((value >> 8) as usize, (value & 0xff) as usize);
            if start + count <= sector_count {
                for flag in &mut block_empty_flags[start..start + count] {
                    *flag = false;
                }
            }
        }
        Ok(Self {
            file,
            path,
            offsets,
            timestamps,
            block_empty_flags,
            bytes_written: 0,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `getChunkBlockInfo != 0`: the header claims a chunk here.
    pub fn has_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        chunk_index(chunk_x, chunk_z).is_some_and(|i| self.offsets[i] != 0)
    }

    /// `getChunkInputStream`: the chunk's stored bytes, or `None` for a chunk
    /// the file does not hold or cannot deliver. Every failure vanilla
    /// returns `null` for is a `None` here, never an error: a damaged chunk
    /// is a missing chunk, and the game regenerates it.
    pub fn read_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> io::Result<Option<ChunkData>> {
        let Some(i) = chunk_index(chunk_x, chunk_z) else {
            return Ok(None);
        };
        let value = self.offsets[i];
        if value == 0 {
            return Ok(None);
        }
        let (start, count) = ((value >> 8) as usize, (value & 0xff) as usize);
        if start + count > self.block_empty_flags.len() {
            return Ok(None);
        }
        self.file
            .seek(SeekFrom::Start(start as u64 * SECTOR as u64))?;
        let mut head = [0; 5];
        if self.read_full(&mut head)? < head.len() {
            return Ok(None); // "Chunk header is truncated"
        }
        let length = i32::from_be_bytes(head[..4].try_into().unwrap());
        if length > (count * SECTOR) as i32 || length <= 0 {
            return Ok(None);
        }
        let Some(compression) = ChunkCompression::from_byte(head[4]) else {
            return Ok(None); // An unknown byte, as any but 1 and 2 is.
        };
        let mut data = vec![0; length as usize - 1];
        if self.read_full(&mut data)? != data.len() {
            return Ok(None); // A short read: vanilla's DataInputStream would EOF.
        }
        Ok(Some(ChunkData {
            compression,
            data,
            timestamp: self.timestamps[i],
        }))
    }

    /// A `RandomAccessFile.read` on a plain file returns fewer bytes only at
    /// EOF; loop to fill the buffer, counting short reads as `read` does.
    fn read_full(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut filled = 0;
        while filled < buffer.len() {
            match self.file.read(&mut buffer[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        Ok(filled)
    }

    /// `getChunkOutputStream` + `close`: compress the NBT bytes with zlib and
    /// store them for the chunk, the way `writeChunkData` runs after
    /// `DeflaterOutputStream` wrapped them. `timestamp` is what the header's
    /// second table records for the chunk (`getTimeMillis() / 1000` in
    /// vanilla; the caller's clock here).
    pub fn write_chunk(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        nbt: &[u8],
        timestamp: u32,
    ) -> io::Result<()> {
        let compressed = crate::java_zip::zlib(nbt);
        self.write_chunk_data(chunk_x, chunk_z, &compressed, timestamp)
    }

    /// `writeChunkData`, on already-compressed bytes.
    fn write_chunk_data(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        data: &[u8],
        timestamp: u32,
    ) -> io::Result<()> {
        let Some(i) = chunk_index(chunk_x, chunk_z) else {
            return Ok(()); // `isOutsideRegion`: getChunkOutputStream returned null.
        };
        let old = self.offsets[i];
        let (old_start, old_count) = ((old >> 8) as usize, (old & 0xff) as usize);
        // (size + 5) / 4096 + 1: vanilla's count, one more than the payload
        // needs (its last sector is shared with the next chunk's, in effect).
        let required = (data.len() + 5) / SECTOR + 1;
        if required >= 256 {
            // Silently dropped, as vanilla drops it: a chunk whose payload
            // passes 255 sectors never writes; the old data, if any, stays.
            return Ok(());
        }
        if old != 0 && old_count == required {
            // Same-size overwrite, in place, the flags untouched. A stale
            // entry past the file's end writes there, growing the file.
            self.write_payload(old_start, data)?;
        } else {
            // Free the old run in the flags first. An entry whose run
            // overruns the file aborts here, as vanilla's `ArrayList.set`
            // throws and its write is caught and dropped.
            if old != 0 && old_start + old_count > self.block_empty_flags.len() {
                return Err(invalid("chunk entry overruns the file"));
            }
            for flag in &mut self.block_empty_flags[old_start..old_start + old_count] {
                *flag = true;
            }
            match self.find_free_run(required) {
                Some(start) => {
                    // Header entry first, then the flags, then the payload.
                    self.offsets[i] = ((start as u32) << 8) | required as u32;
                    self.write_offset(i, self.offsets[i])?;
                    for flag in &mut self.block_empty_flags[start..start + required] {
                        *flag = false;
                    }
                    self.write_payload(start, data)?;
                }
                None => {
                    // Append: grow the file with zero sectors, payload, then
                    // the header entry.
                    let start = self.block_empty_flags.len();
                    for _ in 0..required {
                        self.file.seek(SeekFrom::End(0))?;
                        self.file.write_all(&[0; SECTOR])?;
                        self.block_empty_flags.push(false);
                    }
                    self.bytes_written += (SECTOR * required) as u64;
                    self.write_payload(start, data)?;
                    self.offsets[i] = ((start as u32) << 8) | required as u32;
                    self.write_offset(i, self.offsets[i])?;
                }
            }
        }
        self.timestamps[i] = timestamp;
        self.write_timestamp(i, timestamp)
    }

    /// The payload: length (data + 1), the zlib byte, the bytes, and the last
    /// sector zero-padded.
    fn write_payload(&mut self, start: usize, data: &[u8]) -> io::Result<()> {
        self.file
            .seek(SeekFrom::Start(start as u64 * SECTOR as u64))?;
        self.file
            .write_all(&((data.len() + 1) as u32).to_be_bytes())?;
        self.file.write_all(&[ChunkCompression::Zlib as u8])?;
        let mut rest = data;
        while !rest.is_empty() {
            let take = rest.len().min(SECTOR);
            let mut chunk = rest[..take].to_vec();
            chunk.resize(SECTOR, 0);
            self.file.write_all(&chunk)?;
            rest = &rest[take..];
        }
        Ok(())
    }

    /// `blockEmptyFlags.indexOf(true)` and the scan after it: the first run
    /// of `count` consecutive free sectors. `None` means the file grows.
    fn find_free_run(&self, count: usize) -> Option<usize> {
        let flags = &self.block_empty_flags;
        let mut run_start = None;
        let mut run_length = 0;
        for (i, &free) in flags.iter().enumerate() {
            if free {
                if run_length == 0 {
                    run_start = Some(i);
                }
                run_length += 1;
                if run_length >= count {
                    return run_start;
                }
            } else {
                run_start = None;
                run_length = 0;
            }
        }
        None
    }

    fn write_offset(&mut self, i: usize, value: u32) -> io::Result<()> {
        self.file.seek(SeekFrom::Start((i * 4) as u64))?;
        self.file.write_all(&value.to_be_bytes())
    }

    fn write_timestamp(&mut self, i: usize, value: u32) -> io::Result<()> {
        self.file.seek(SeekFrom::Start((SECTOR + i * 4) as u64))?;
        self.file.write_all(&value.to_be_bytes())
    }

    /// Which chunks the file holds, as `(chunk_x, chunk_z)` local to the
    /// region (0..31 each), in header order.
    pub fn present_chunks(&self) -> Vec<[i32; 2]> {
        let mut chunks = Vec::new();
        for (i, &value) in self.offsets.iter().enumerate() {
            if value != 0 {
                chunks.push([(i % 32) as i32, (i / 32) as i32]);
            }
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunset_1_7_core::{LegacyRandom, RandomSource};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sunset-1-7-save-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Incompressible bytes, so a payload's compressed size is its own.
    fn incompressible(len: usize) -> Vec<u8> {
        let mut random = LegacyRandom::new(0x5EED_1710);
        (0..len).map(|_| random.next_int() as u8).collect()
    }

    #[test]
    fn a_fresh_file_gets_the_empty_header() {
        let dir = temp_dir("fresh");
        let path = dir.join("r.0.0.mca");
        let region = RegionFile::open(&path).unwrap();
        assert_eq!(path.metadata().unwrap().len(), HEADER as u64);
        assert!(!region.has_chunk(0, 0));
        assert_eq!(region.present_chunks(), Vec::<[i32; 2]>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_then_read_roundtrips() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("r.3.-1.mca");
        let mut region = RegionFile::open(&path).unwrap();
        let nbt = incompressible(10_000);
        region.write_chunk(5, 7, &nbt, 1_700_000_000).unwrap();
        assert!(region.has_chunk(5, 7));
        assert!(!region.has_chunk(5, 8));
        let chunk = region.read_chunk(5, 7).unwrap().unwrap();
        assert_eq!(chunk.compression, ChunkCompression::Zlib);
        assert_eq!(chunk.timestamp, 1_700_000_000);
        assert_eq!(chunk.decompress(1 << 20).unwrap(), nbt);
        // (compressed + 5) / 4096 + 1: vanilla's count for this size.
        let compressed = crate::java_zip::zlib(&nbt);
        let value = region.offsets[chunk_index(5, 7).unwrap()];
        assert_eq!(value & 0xff, ((compressed.len() + 5) / 4096 + 1) as u32);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rewrite_that_fits_stays_in_place() {
        let dir = temp_dir("inplace");
        let path = dir.join("r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        region.write_chunk(1, 1, &[1u8; 100], 10).unwrap();
        let before = region.offsets[chunk_index(1, 1).unwrap()];
        region.write_chunk(1, 1, &[2u8; 100], 20).unwrap();
        let after = region.offsets[chunk_index(1, 1).unwrap()];
        assert_eq!(before >> 8, after >> 8, "same-size rewrite moves the chunk");
        assert_eq!(after & 0xff, before & 0xff);
        let chunk = region.read_chunk(1, 1).unwrap().unwrap();
        assert_eq!(chunk.decompress(1 << 20).unwrap(), [2u8; 100]);
        assert_eq!(chunk.timestamp, 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_growing_rewrite_keeps_the_neighbours() {
        let dir = temp_dir("grow");
        let path = dir.join("r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        region.write_chunk(0, 0, &[1u8; 100], 1).unwrap();
        region.write_chunk(1, 0, &[2u8; 100], 2).unwrap();
        let big = incompressible(100_000); // ~100 KiB compressed: no longer fits
        region.write_chunk(0, 0, &big, 3).unwrap();
        let small = region.read_chunk(1, 0).unwrap().unwrap();
        assert_eq!(small.decompress(1 << 20).unwrap(), [2u8; 100]);
        let grown = region.read_chunk(0, 0).unwrap().unwrap();
        assert_eq!(grown.decompress(1 << 20).unwrap(), big);
        // The old run was marked free again.
        assert!(region.block_empty_flags.iter().filter(|&&f| !f).count() >= 27);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shrinking_rewrite_reuses_the_freed_gap() {
        let dir = temp_dir("gap");
        let path = dir.join("r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        let big = incompressible(100_000);
        region.write_chunk(0, 0, &big, 1).unwrap();
        region.write_chunk(1, 0, &big, 2).unwrap();
        region.write_chunk(0, 0, &[0u8; 100], 3).unwrap();
        let value = region.offsets[chunk_index(0, 0).unwrap()];
        assert_eq!((value >> 8) as usize, 2, "the freed run is the first fit");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_chunk_over_255_sectors_is_dropped() {
        let dir = temp_dir("oversize");
        let path = dir.join("r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        region.write_chunk(0, 0, &[9u8; 100], 1).unwrap();
        let oversized = incompressible(1_048_576); // > 255 sectors, compressed
        region.write_chunk(0, 0, &oversized, 2).unwrap();
        let chunk = region.read_chunk(0, 0).unwrap().unwrap();
        assert_eq!(chunk.decompress(1 << 20).unwrap(), [9u8; 100]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_survive_a_truncated_payload() {
        let dir = temp_dir("damage");
        let path = dir.join("r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        region.write_chunk(0, 0, &[4u8; 5_000], 1).unwrap();
        // Cut the file after the payload's first sector: the length field
        // says more than is there.
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len((2 * SECTOR) as u64).unwrap();
        drop(file);
        let mut region = RegionFile::open(&path).unwrap();
        assert!(region.read_chunk(0, 0).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_short_file_is_padded_to_sectors() {
        let dir = temp_dir("ragged");
        let path = dir.join("r.0.0.mca");
        std::fs::write(&path, [7u8; 100]).unwrap();
        let region = RegionFile::open(&path).unwrap();
        assert_eq!(path.metadata().unwrap().len(), HEADER as u64);
        assert_eq!(region.block_empty_flags.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chunk_index_and_paths_match_region_io() {
        assert_eq!(chunk_index(0, 0), Some(0));
        assert_eq!(chunk_index(31, 31), Some(31 + 31 * 32));
        assert_eq!(chunk_index(32, 0), Some(0), "wraps like chunkX & 31");
        assert_eq!(chunk_index(-1, -1), Some(31 + 31 * 32));
        assert_eq!(
            region_path(Path::new("/w"), 33, -2),
            PathBuf::from("/w/region/r.1.-1.mca")
        );
    }

    #[test]
    fn a_region_file_written_by_another_tool_reads() {
        // tests/r.0.0.mca: a header with one zlib chunk in slot 0, written by
        // a Python driver of the same layout.
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/r.0.0.mca");
        let mut region = RegionFile::open(&path).unwrap();
        assert!(region.has_chunk(0, 0));
        assert!(!region.has_chunk(31, 31));
        let chunk = region.read_chunk(0, 0).unwrap().unwrap();
        assert_eq!(chunk.compression, ChunkCompression::Zlib);
        assert_eq!(chunk.timestamp, 1_700_000_000);
        let nbt = chunk.decompress(1 << 20).unwrap();
        let (named, _) =
            sunrise_core::nbt::read_named(&nbt, sunrise_core::nbt::Limits::saved()).unwrap();
        assert!(matches!(named.tag, sunrise_core::nbt::Tag::Compound(_)));
        // Slot 0's entry: sector 2, one sector.
        assert_eq!(region.offsets[0], (2 << 8) | 1);
    }
}
