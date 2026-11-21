//! The 1.8-style packet compression layer: a varint uncompressed size in front
//! of the payload, zlib-compressed once the payload reaches the threshold.
//!
//! Vanilla 1.7.10 has no packet compression — this layer is here for the
//! connection-layer contract and sits disabled by default: nothing in the
//! 1.7.10 handshake or login turns it on, so the wire format stays vanilla.

use std::io::{Read, Write};

use crate::buf::{BufError, PacketBuf};

/// Compress a packet payload for the wire: `[varint uncompressed size][data]`,
/// where a zero size marks raw data below the threshold and the data is zlib
/// once the payload reaches it.
pub fn compress_frame(payload: &[u8], threshold: u32) -> Vec<u8> {
    let mut out = PacketBuf::new();
    if (payload.len() as u64) < threshold as u64 {
        out.write_var_int(0);
        let mut out = out.into_written();
        out.extend_from_slice(payload);
        return out;
    }
    out.write_var_int(payload.len() as i32);
    let mut out = out.into_written();
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(payload).expect("zlib write");
    out.extend_from_slice(&encoder.finish().expect("zlib finish"));
    out
}

/// Undo [`compress_frame`]: read the uncompressed size, inflate or pass through.
pub fn decompress_frame(frame: &[u8]) -> Result<Vec<u8>, BufError> {
    let mut buf = PacketBuf::wrap(frame);
    let uncompressed = buf.read_var_int()?;
    let rest = buf.rest();
    if uncompressed < 0 {
        return Err(BufError::Zlib);
    }
    if uncompressed == 0 {
        return Ok(rest.to_vec());
    }
    let mut decoder = flate2::read::ZlibDecoder::new(rest);
    let mut out = Vec::with_capacity(uncompressed as usize);
    decoder.read_to_end(&mut out).map_err(|_| BufError::Zlib)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_threshold_stays_raw() {
        let payload = b"hello world";
        let framed = compress_frame(payload, 100);
        // A zero size marks raw data, exactly like 1.8's compressor.
        assert_eq!(framed[0], 0);
        assert_eq!(&framed[1..], payload);
        assert_eq!(decompress_frame(&framed).unwrap(), payload);
    }

    #[test]
    fn above_threshold_is_zlib() {
        let payload: Vec<u8> = b"abcdefgh".iter().copied().cycle().take(4096).collect();
        let framed = compress_frame(&payload, 256);
        // A compressed frame is smaller than the raw payload for this data.
        assert!(framed.len() < payload.len() + 2);
        assert_eq!(decompress_frame(&framed).unwrap(), payload);
    }

    #[test]
    fn threshold_zero_always_compresses() {
        let framed = compress_frame(b"hello", 0);
        assert_ne!(&framed[1..], b"hello");
        assert_eq!(decompress_frame(&framed).unwrap(), b"hello");
    }

    #[test]
    fn rejects_bad_frames() {
        assert!(decompress_frame(&[0xff, 0xff]).is_err());
        assert!(decompress_frame(&[0x01, 0x00]).is_err());
    }
}
