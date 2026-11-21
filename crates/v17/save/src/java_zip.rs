//! Disk compression as `java.util.zip` writes and reads it on the JDK vanilla
//! 1.7.10 runs on: gzip members for `level.dat`, player files and map data;
//! zlib streams (deflate at the default level) for region chunk payloads.
//!
//! Compression is [`sunrise_deflate`], a port of the zlib the JDK bundles,
//! so the bytes written here match vanilla's saves byte for byte.

use std::io;

thread_local! {
    /// This thread's zlib inflater, reset per stream instead of reallocated.
    static INFLATER: std::cell::RefCell<Option<flate2::Decompress>> =
        const { std::cell::RefCell::new(None) };
    /// zlib's 250 KiB of encoder state, kept per thread between saves.
    static DEFLATER: std::cell::RefCell<Option<sunrise_deflate::Deflater>> =
        const { std::cell::RefCell::new(None) };
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// `NbtIo.writeCompressed`: one gzip member of the bytes, as
/// `GZIPOutputStream` writes it on a JDK bundling zlib 1.3.2.
pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() / 2 + 64);
    DEFLATER.with_borrow_mut(|deflater| {
        deflater
            .get_or_insert_with(sunrise_deflate::Deflater::new)
            .gzip(bytes, &mut out)
    });
    out
}

/// `DeflaterOutputStream` at its defaults: a zlib stream (RFC 1950), what
/// region files' chunk payloads carry.
pub fn zlib(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 64);
    DEFLATER.with_borrow_mut(|deflater| {
        deflater
            .get_or_insert_with(sunrise_deflate::Deflater::new)
            .zlib(bytes, &mut out)
    });
    out
}

/// `NbtIo.readCompressed`: reads the first gzip member, as Java's
/// `GZIPInputStream` does — the header's optional fields recognized, the
/// header CRC verified, the trailer's CRC-32 and length checked, a bad
/// stream refused. A member after the first is left unread, as
/// `readCompressed` leaves it: the stream it wraps never reaches it.
pub fn gunzip(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let body = gzip_body(bytes).ok_or_else(|| invalid("invalid gzip header"))?;
    let mut output = Vec::new();
    INFLATER.with_borrow_mut(|inflater| {
        let decoder = inflater.get_or_insert_with(|| flate2::Decompress::new(false));
        decoder.reset(false);
        let mut consumed = 0usize;
        loop {
            let before_out = output.len();
            output.resize(before_out + 8192, 0);
            let status = decoder
                .decompress(
                    &body[consumed..],
                    &mut output[before_out..],
                    flate2::FlushDecompress::None,
                )
                .map_err(|e| invalid(e.to_string()))?;
            let produced = decoder.total_out() as usize - before_out;
            let after = decoder.total_in() as usize;
            output.truncate(before_out + produced);
            if let flate2::Status::StreamEnd = status {
                // `GZIPInputStream.readTrailer`: CRC-32 and the length mod 2³²,
                // both little-endian, right after the deflate data.
                let trailer = body
                    .get(after..)
                    .ok_or_else(|| invalid("bad gzip trailer"))?;
                let (crc, length) = match trailer {
                    [a, b, c, d, e, f, g, h, ..] => (
                        u32::from_le_bytes([*a, *b, *c, *d]),
                        u32::from_le_bytes([*e, *f, *g, *h]),
                    ),
                    _ => return Err(invalid("bad gzip trailer")),
                };
                if crc != crc32fast::hash(&output) {
                    return Err(invalid("gzip crc mismatch"));
                }
                if length != output.len() as u32 {
                    return Err(invalid("gzip length mismatch"));
                }
                return Ok(output);
            }
            if produced == 0 && after == consumed {
                return Err(invalid("truncated gzip stream"));
            }
            consumed = after;
        }
    })
}

/// `GZIPInputStream`'s header, over the bytes before the raw deflate stream:
/// the fixed magic and method, then any optional fields the flags ask for
/// (extra, name, comment, header CRC). Reserved bits are accepted, as Java
/// accepts them. Returns the raw deflate input.
fn gzip_body(bytes: &[u8]) -> Option<&[u8]> {
    let (header, mut body) = bytes.split_at_checked(10)?;
    if header[..3] != [0x1f, 0x8b, 8] {
        return None;
    }
    let flags = header[3];
    if flags & 4 != 0 {
        let (len, rest) = body.split_first_chunk::<2>()?;
        body = rest.get(usize::from(u16::from_le_bytes(*len))..)?;
    }
    for flag in [8, 16] {
        if flags & flag != 0 {
            let end = body.iter().position(|&b| b == 0)?;
            body = &body[end + 1..];
        }
    }
    if flags & 2 != 0 {
        let (expected, rest) = body.split_first_chunk::<2>()?;
        let actual = crc32fast::hash(&bytes[..bytes.len() - body.len()]);
        if actual as u16 != u16::from_le_bytes(*expected) {
            return None;
        }
        body = rest;
    }
    Some(body)
}

/// `InflaterInputStream` over a zlib stream: the decompressed bytes, with a
/// cap that stands in for the reader's endless patience (Java reads until
/// the stream says so; this refuses a payload past `max_output`, which no
/// honest 1.7.10 chunk or `level.dat` reaches).
pub fn unzlib(bytes: &[u8], max_output: usize) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    INFLATER.with_borrow_mut(|inflater| {
        let decoder = inflater.get_or_insert_with(|| flate2::Decompress::new(true));
        decoder.reset(true);
        let mut consumed = 0usize;
        loop {
            let before_out = output.len();
            if before_out > max_output {
                return Err(invalid("decompression limit exceeded"));
            }
            output.resize((before_out + 8192).min(max_output + 1), 0);
            let status = decoder
                .decompress(
                    &bytes[consumed..],
                    &mut output[before_out..],
                    flate2::FlushDecompress::None,
                )
                .map_err(|e| invalid(e.to_string()))?;
            let produced = decoder.total_out() as usize - before_out;
            let after = decoder.total_in() as usize;
            output.truncate(before_out + produced);
            if output.len() > max_output {
                return Err(invalid("decompression limit exceeded"));
            }
            if let flate2::Status::StreamEnd = status {
                return Ok(output);
            }
            if produced == 0 && after == consumed {
                return Err(invalid("truncated zlib stream"));
            }
            consumed = after;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_roundtrip_matches_java_header() {
        let bytes = b"hello save world".repeat(100);
        let compressed = gzip(&bytes);
        // Java's GZIPOutputStream header: deflate, no flags, no mtime, no
        // extra flags, os 255 ("unknown").
        assert_eq!(&compressed[..10], &[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff]);
        let round = gunzip(&compressed).unwrap();
        assert_eq!(round, bytes);
    }

    #[test]
    fn zlib_roundtrip() {
        let bytes = vec![7u8; 100_000];
        let compressed = zlib(&bytes);
        // RFC 1950: 78 9c at the default level.
        assert_eq!(&compressed[..2], &[0x78, 0x9c]);
        let round = unzlib(&compressed, 1 << 20).unwrap();
        assert_eq!(round, bytes);
    }

    #[test]
    fn gunzip_rejects_bad_trailer() {
        let mut compressed = gzip(b"data");
        let last = compressed.len();
        compressed[last - 5] ^= 0xff; // corrupt the crc
        assert!(gunzip(&compressed).is_err());
    }

    #[test]
    fn another_zlib_gzip_is_read() {
        // A gzip member a different zlib writes (level 9, here): any honest
        // deflate stream must read.
        let bytes = b"The quick brown fox".repeat(50);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(9));
        std::io::Write::write_all(&mut encoder, &bytes).unwrap();
        let member = encoder.finish().unwrap();
        assert_eq!(gunzip(&member).unwrap(), bytes);
    }

    #[test]
    fn unzlib_refuses_a_payload_past_the_cap() {
        let bytes = vec![7u8; 100_000];
        let compressed = zlib(&bytes);
        assert!(unzlib(&compressed, 1024).is_err());
    }
}
