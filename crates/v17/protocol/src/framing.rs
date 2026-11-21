//! Frame layer: 1.7.10's `SizePrepender` and `SplitterHandler`. Every packet
//! is VarInt length-prefixed, and the prefix is at most three bytes
//! (21-bit length).

use crate::buf::{Buf, Error, Result};

/// The length prefix is at most three VarInt bytes (`SizePrepender` refuses
/// wider ones).
pub const MAX_LENGTH_PREFIX_BYTES: usize = 3;

/// Prepends the VarInt length of `payload`, exactly as `SizePrepender` does.
pub fn frame(payload: &[u8]) -> Result<Vec<u8>> {
    if crate::buf::var_int_size(payload.len() as i32) > MAX_LENGTH_PREFIX_BYTES {
        return Err(Error::FrameTooLarge { len: payload.len() });
    }
    let mut out = Buf::new();
    out.write_var_int(payload.len() as i32);
    let mut out = out.into_vec();
    out.extend_from_slice(payload);
    Ok(out)
}

/// Splits one framed packet off the front of `buf`, mirroring
/// `SplitterHandler`: `Ok(None)` when more bytes are needed, an error when
/// the length prefix is wider than three bytes.
pub fn unframe(buf: &mut Buf) -> Result<Option<Vec<u8>>> {
    let start = buf.position();
    for prefix in 0..MAX_LENGTH_PREFIX_BYTES {
        let Some(byte) = buf.get(start + prefix) else {
            return Ok(None); // not enough bytes for the length prefix yet
        };
        if byte & 0x80 == 0 {
            let prefix_end = start + prefix + 1;
            let mut len: u32 = 0;
            for (shift, byte) in (0..prefix_end).map(|i| buf.get(i).unwrap()).enumerate() {
                len |= ((byte & 0x7f) as u32) << (shift * 7);
            }
            let payload_start = prefix_end;
            if buf.len() - payload_start < len as usize {
                return Ok(None); // payload not complete yet
            }
            let payload = buf.as_slice()[payload_start..payload_start + len as usize].to_vec();
            buf.seek(payload_start + len as usize);
            return Ok(Some(payload));
        }
    }
    Err(Error::CorruptedFrame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        for payload in [&b""[..], &[0x01u8][..], &[0u8; 300][..]] {
            let framed = frame(payload).unwrap();
            let mut buf = Buf::from(framed);
            assert_eq!(unframe(&mut buf).unwrap().unwrap(), payload);
            assert_eq!(buf.remaining(), 0);
        }
    }

    #[test]
    fn unframe_waits_for_more_data() {
        let payload = [0x01u8, 0x02];
        let framed = frame(&payload).unwrap();
        // One byte at a time: None until the last byte arrives.
        let mut buf = Buf::new();
        for byte in framed.iter().take(framed.len() - 1) {
            buf.write_u8(*byte);
            assert_eq!(unframe(&mut buf).unwrap(), None);
        }
        buf.write_u8(*framed.last().unwrap());
        assert_eq!(unframe(&mut buf).unwrap().unwrap(), payload);
    }

    #[test]
    fn unframe_keeps_trailing_bytes() {
        let mut buf = Buf::from(frame(&[0x0a, 0x0b]).unwrap());
        buf.write_u8(0x99);
        assert_eq!(unframe(&mut buf).unwrap().unwrap(), vec![0x0a, 0x0b]);
        assert_eq!(buf.remaining(), 1);
    }

    #[test]
    fn unframe_rejects_wide_prefix() {
        let mut buf = Buf::from(vec![0x80, 0x80, 0x80]);
        assert!(matches!(unframe(&mut buf), Err(Error::CorruptedFrame)));
    }

    #[test]
    fn frame_rejects_oversize() {
        assert!(matches!(
            frame(&[0u8; 1 << 21]),
            Err(Error::FrameTooLarge { .. })
        ));
    }
}
