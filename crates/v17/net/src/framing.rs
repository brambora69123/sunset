//! Packet length framing, ported from the decompiled 1.7.10 `SplitterHandler`
//! (inbound) and `SizePrepender` (outbound). The length is a varint prefix of
//! at most three bytes, so a frame carries at most 21 bits of length.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// `CorruptedFrameException`: "length wider than 21-bit".
    LengthTooWide,
    /// `SizePrepender`: "unable to fit {n} into 3".
    FrameTooBig(usize),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LengthTooWide => write!(f, "length wider than 21-bit"),
            Self::FrameTooBig(len) => write!(f, "unable to fit {len} into 3"),
        }
    }
}

impl std::error::Error for FrameError {}

/// `SplitterHandler.decode`: pull one frame off the front of the plaintext
/// stream. `Ok(None)` needs more bytes and leaves the buffer alone.
pub fn split_frame(stream: &mut Vec<u8>) -> Result<Option<Vec<u8>>, FrameError> {
    // Read up to three bytes for the length varint; a partial varint waits.
    let mut length_bytes = [0u8; 3];
    for index in 0..3 {
        if index >= stream.len() {
            return Ok(None);
        }
        let byte = stream[index];
        length_bytes[index] = byte;
        if byte >= 0x80 {
            continue;
        }
        let length = decode_var_int(&length_bytes[..=index]);
        if stream.len() < index + 1 + length {
            return Ok(None);
        }
        stream.drain(..index + 1);
        return Ok(Some(stream.drain(..length).collect()));
    }
    Err(FrameError::LengthTooWide)
}

/// `SizePrepender.encode`: the length varint in front of the payload.
pub fn frame(payload: &[u8]) -> Result<Vec<u8>, FrameError> {
    let length = payload.len();
    if crate::buf::get_var_int_size_bytes(length as i32) > 3 {
        return Err(FrameError::FrameTooBig(length));
    }
    let mut buf = crate::buf::PacketBuf::new();
    buf.write_var_int(length as i32);
    let mut out = buf.into_written();
    out.extend_from_slice(payload);
    Ok(out)
}

fn decode_var_int(bytes: &[u8]) -> usize {
    let mut result: usize = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        result |= ((byte & 0x7f) as usize) << (index * 7);
        if byte & 0x80 == 0 {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_waits_for_a_complete_frame() {
        let mut stream = vec![0x03, 0x01, 0x02];
        assert_eq!(split_frame(&mut stream).unwrap(), None);
        stream.push(0x03);
        assert_eq!(split_frame(&mut stream).unwrap(), Some(vec![1, 2, 3]));
        assert!(stream.is_empty());
    }

    #[test]
    fn split_two_byte_length() {
        // 0x80 0x01 = 128, one continuation byte.
        let mut stream: Vec<u8> = [0x80, 0x01]
            .iter()
            .copied()
            .chain((0..128).map(|i| i as u8))
            .collect();
        let frame = split_frame(&mut stream).unwrap().unwrap();
        assert_eq!(frame.len(), 128);
        assert!(stream.is_empty());
    }

    #[test]
    fn split_two_byte_length_needs_the_full_payload() {
        let mut stream = vec![0x80, 0x01, 0x00];
        assert_eq!(split_frame(&mut stream).unwrap(), None);
    }

    #[test]
    fn split_rejects_a_wide_length() {
        let mut stream = vec![0x80, 0x80, 0x80, 0x01];
        assert_eq!(split_frame(&mut stream), Err(FrameError::LengthTooWide));
    }

    #[test]
    fn split_zero_length_frame() {
        let mut stream = vec![0x00];
        assert_eq!(split_frame(&mut stream).unwrap(), Some(vec![]));
    }

    #[test]
    fn frame_round_trips() {
        for length in [0usize, 1, 127, 128, 16383, 16384, 2097151] {
            let payload: Vec<u8> = (0..length).map(|i| (i % 251) as u8).collect();
            let framed = frame(&payload).unwrap();
            let mut stream = framed;
            assert_eq!(split_frame(&mut stream).unwrap(), Some(payload));
        }
    }

    #[test]
    fn frame_rejects_over_21_bits() {
        let payload = vec![0u8; 2097152];
        assert_eq!(frame(&payload), Err(FrameError::FrameTooBig(2097152)));
    }
}
