//! `PacketByteBuf`, ported from 1.7.10's `net.minecraft.network.PacketByteBuf`.
//!
//! `read_var_int`/`write_var_int`/`var_int_size` are exact ports. `var_long`
//! and the packed position long are not part of the 1.7.10 wire format (they
//! arrived in later versions); they are kept here as extra codecs.

use crate::slot::Slot;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Read ran past the end of the buffer (netty's `IndexOutOfBoundsException`).
    UnexpectedEof,
    /// More than five bytes carried a VarInt continuation.
    VarIntTooBig,
    /// More than ten bytes carried a VarLong continuation.
    VarLongTooBig,
    /// Encoded string buffer length above `max * 4` or below zero
    /// (`readString`).
    StringBufferTooLong { len: i32, max: usize },
    /// Encoded string above 32767 bytes (`writeString`).
    StringTooBig { len: usize },
    /// Decoded string longer than `max` chars.
    StringTooLong { len: usize, max: usize },
    /// Byte-array or NBT length below zero.
    NegativeLength { len: i32 },
    /// Frame payload does not fit a three-byte VarInt length prefix
    /// (`SizePrepender`).
    FrameTooLarge { len: usize },
    /// Length prefix wider than three bytes (`SplitterHandler`).
    CorruptedFrame,
    /// Handshake next-state id outside -1..2 (`NetworkProtocol.byId` null).
    UnknownProtocolState { id: i32 },
    /// Packet id outside the state's table (`PacketDecoder`: "Bad packet id").
    UnknownPacketId { state: i32, id: i32 },
    /// Body bytes left over after a full packet read
    /// (`PacketDecoder`: "Packet was larger than I expected").
    PacketTooLarge { extra: usize },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of buffer"),
            Self::VarIntTooBig => write!(f, "VarInt too big"),
            Self::VarLongTooBig => write!(f, "VarLong too big"),
            Self::StringBufferTooLong { len, max } => write!(
                f,
                "The received encoded string buffer length is longer than maximum allowed ({len} > {})",
                max * 4
            ),
            Self::StringTooBig { len } => {
                write!(f, "String too big (was {len} bytes encoded, max 32767)")
            }
            Self::StringTooLong { len, max } => write!(
                f,
                "The received string length is longer than maximum allowed ({len} > {max})"
            ),
            Self::NegativeLength { len } => write!(f, "length below zero: {len}"),
            Self::FrameTooLarge { len } => write!(f, "unable to fit {len} into 3"),
            Self::CorruptedFrame => write!(f, "length wider than 21-bit"),
            Self::UnknownProtocolState { id } => write!(f, "unknown protocol state {id}"),
            Self::UnknownPacketId { state, id } => write!(f, "Bad packet id {id} (state {state})"),
            Self::PacketTooLarge { extra } => write!(
                f,
                "Packet was larger than I expected, found {extra} bytes extra"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Byte buffer with a read cursor, wrapping a `Vec<u8>`.
#[derive(Debug, Default, Clone)]
pub struct Buf {
    data: Vec<u8>,
    pos: usize,
}

impl From<Vec<u8>> for Buf {
    fn from(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }
}

impl Buf {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            pos: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn get(&self, index: usize) -> Option<u8> {
        self.data.get(index).copied()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&end| end <= self.data.len())
            .ok_or(Error::UnexpectedEof)?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Reads a byte, as netty does (signed in Java).
    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.take(1)?[0] as i8)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.read_i32()? as u32))
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.read_i64()? as u64))
    }

    /// Exact port of `PacketByteBuf.readVarInt`: five bytes at most, the
    /// sixth (or an exhausted buffer) is an error.
    pub fn read_var_int(&mut self) -> Result<i32> {
        let mut value: u32 = 0;
        for shift in 0u32.. {
            let byte = self.read_u8()?;
            if shift == 5 {
                return Err(Error::VarIntTooBig);
            }
            value |= ((byte & 0x7f) as u32) << (shift * 7);
            if byte & 0x80 == 0 {
                return Ok(value as i32);
            }
        }
        unreachable!()
    }

    /// VarLong, as later versions read it (not part of the 1.7.10 wire).
    pub fn read_var_long(&mut self) -> Result<i64> {
        let mut value: u64 = 0;
        for shift in 0u32.. {
            let byte = self.read_u8()?;
            if shift == 10 {
                return Err(Error::VarLongTooBig);
            }
            value |= ((byte & 0x7f) as u64) << (shift * 7);
            if byte & 0x80 == 0 {
                return Ok(value as i64);
            }
        }
        unreachable!()
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        Ok(self.take(n)?.to_vec())
    }

    /// Exact port of `PacketByteBuf.readString(maxLength)`: the length is a
    /// VarInt bounded by `max * 4` encoded bytes; the decoded string is
    /// bounded by `max` chars. Malformed UTF-8 decodes with U+FFFD, as Java's
    /// `new String(bytes, UTF_8)` does.
    pub fn read_string(&mut self, max: usize) -> Result<String> {
        let len = self.read_var_int()?;
        if len > (max as i32).saturating_mul(4) {
            return Err(Error::StringBufferTooLong { len, max });
        }
        if len < 0 {
            return Err(Error::NegativeLength { len });
        }
        let bytes = self.read_bytes(len as usize)?;
        let string = String::from_utf8_lossy(&bytes).into_owned();
        if string.encode_utf16().count() > max {
            return Err(Error::StringTooLong {
                len: string.encode_utf16().count(),
                max,
            });
        }
        Ok(string)
    }

    /// Exact port of `Packet.writeByteArray`/`readByteArray`: short-prefixed.
    pub fn write_byte_array(&mut self, bytes: &[u8]) {
        self.write_i16(bytes.len() as i16);
        self.data.extend_from_slice(bytes);
    }

    pub fn read_byte_array(&mut self) -> Result<Vec<u8>> {
        let len = self.read_i16()?;
        if len < 0 {
            return Err(Error::NegativeLength { len: len as i32 });
        }
        self.read_bytes(len as usize)
    }

    /// Exact port of `PacketByteBuf.writeNbtCompound`/`readNbtCompound`: the
    /// raw compound bytes, short-prefixed, or -1 for null. The parse belongs
    /// to the NBT crate.
    pub fn write_nbt(&mut self, nbt: Option<&[u8]>) {
        match nbt {
            None => self.write_i16(-1),
            Some(bytes) => {
                self.write_i16(bytes.len() as i16);
                self.data.extend_from_slice(bytes);
            }
        }
    }

    pub fn read_nbt(&mut self) -> Result<Option<Vec<u8>>> {
        let len = self.read_i16()?;
        if len < 0 {
            return Ok(None);
        }
        Ok(Some(self.read_bytes(len as usize)?))
    }

    /// Exact port of `PacketByteBuf.writeItem`/`readItem`.
    pub fn write_slot(&mut self, slot: Option<&Slot>) {
        match slot {
            None => self.write_i16(-1),
            Some(slot) => {
                self.write_i16(slot.id);
                self.write_i8(slot.count);
                self.write_i16(slot.meta);
                self.write_nbt(slot.nbt.as_deref());
            }
        }
    }

    pub fn read_slot(&mut self) -> Result<Option<Slot>> {
        let id = self.read_i16()?;
        if id < 0 {
            return Ok(None);
        }
        let count = self.read_i8()?;
        let meta = self.read_i16()?;
        let nbt = self.read_nbt()?;
        Ok(Some(Slot {
            id,
            count,
            meta,
            nbt,
        }))
    }

    /// Block position packed into a long (26 + 12 + 26 bits), the packing
    /// later versions use. 1.7.10 packets carry positions as int triples.
    pub fn write_position(&mut self, x: i32, y: i32, z: i32) {
        let packed =
            ((x as i64 & 0x3ff_ffff) << 38) | ((z as i64 & 0x3ff_ffff) << 12) | (y as i64 & 0xfff);
        self.write_i64(packed);
    }

    pub fn read_position(&mut self) -> Result<[i32; 3]> {
        let packed = self.read_i64()?;
        let x = (packed >> 38) as i32;
        let y = ((packed << 52) >> 52) as i32;
        let z = ((packed << 26) >> 38) as i32;
        Ok([x, y, z])
    }

    pub fn write_bool(&mut self, value: bool) {
        self.data.push(value as u8);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.data.push(value as u8);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u16(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.write_i32(value.to_bits() as i32);
    }

    pub fn write_f64(&mut self, value: f64) {
        self.write_i64(value.to_bits() as i64);
    }

    /// Exact port of `PacketByteBuf.writeVarInt`.
    pub fn write_var_int(&mut self, value: i32) {
        let mut value = value as u32;
        while value & 0xffff_ff80 != 0 {
            self.data.push((value & 0x7f) as u8 | 0x80);
            value >>= 7;
        }
        self.data.push(value as u8);
    }

    /// VarLong, as later versions write it (not part of the 1.7.10 wire).
    pub fn write_var_long(&mut self, value: i64) {
        let mut value = value as u64;
        while value & 0xffff_ffff_ffff_ff80 != 0 {
            self.data.push((value & 0x7f) as u8 | 0x80);
            value >>= 7;
        }
        self.data.push(value as u8);
    }

    /// Exact port of `PacketByteBuf.writeString`: UTF-8, VarInt-prefixed,
    /// 32767 bytes at most.
    pub fn write_string(&mut self, value: &str) -> Result<()> {
        let bytes = value.as_bytes();
        if bytes.len() > 32767 {
            return Err(Error::StringTooBig { len: bytes.len() });
        }
        self.write_var_int(bytes.len() as i32);
        self.data.extend_from_slice(bytes);
        Ok(())
    }
}

/// Exact port of `PacketByteBuf.getVarIntSizeBytes`.
pub fn var_int_size(value: i32) -> usize {
    if value & -128 == 0 {
        1
    } else if value & -16384 == 0 {
        2
    } else if value & -2097152 == 0 {
        3
    } else if value & -268435456 == 0 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_int_known_encodings() {
        let mut buf = Buf::new();
        buf.write_var_int(0);
        buf.write_var_int(1);
        buf.write_var_int(127);
        buf.write_var_int(128);
        buf.write_var_int(2097151);
        buf.write_var_int(2097152);
        buf.write_var_int(-1);
        assert_eq!(
            buf.into_vec(),
            vec![
                0x00, 0x01, 0x7f, 0x80, 0x01, 0xff, 0xff, 0x7f, 0x80, 0x80, 0x80, 0x01, 0xff, 0xff,
                0xff, 0xff, 0x0f,
            ]
        );
    }

    #[test]
    fn var_int_size_table() {
        assert_eq!(var_int_size(0), 1);
        assert_eq!(var_int_size(127), 1);
        assert_eq!(var_int_size(128), 2);
        assert_eq!(var_int_size(16383), 2);
        assert_eq!(var_int_size(16384), 3);
        assert_eq!(var_int_size(2097151), 3);
        assert_eq!(var_int_size(2097152), 4);
        assert_eq!(var_int_size(268435455), 4);
        assert_eq!(var_int_size(268435456), 5);
        assert_eq!(var_int_size(-1), 5);
    }

    #[test]
    fn var_int_round_trip() {
        for value in [
            0i32,
            1,
            127,
            128,
            16383,
            16384,
            2097151,
            2097152,
            i32::MAX,
            i32::MIN,
            -1,
        ] {
            let mut buf = Buf::new();
            buf.write_var_int(value);
            assert_eq!(buf.read_var_int().unwrap(), value);
        }
    }

    #[test]
    fn var_int_too_big() {
        let mut buf = Buf::from(vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
        assert_eq!(buf.read_var_int(), Err(Error::VarIntTooBig));
        let mut buf = Buf::from(vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
        assert_eq!(buf.read_var_int(), Err(Error::VarIntTooBig));
        // Five bytes without a terminator is fine.
        let mut buf = Buf::from(vec![0x80, 0x80, 0x80, 0x80, 0x00]);
        assert_eq!(buf.read_var_int().unwrap(), 0);
    }

    #[test]
    fn var_long_round_trip() {
        for value in [0i64, 1, i64::MAX, i64::MIN, -1] {
            let mut buf = Buf::new();
            buf.write_var_long(value);
            assert_eq!(buf.read_var_long().unwrap(), value);
        }
        let mut buf = Buf::from(vec![0x80; 11]);
        assert_eq!(buf.read_var_long(), Err(Error::VarLongTooBig));
    }

    #[test]
    fn var_int_missing_bytes_is_eof() {
        let mut buf = Buf::from(vec![0x80]);
        assert_eq!(buf.read_var_int(), Err(Error::UnexpectedEof));
    }

    #[test]
    fn string_limits() {
        let mut buf = Buf::new();
        buf.write_string("héllo").unwrap();
        assert_eq!(buf.read_string(16).unwrap(), "héllo");

        // 17 chars, 21 UTF-8 bytes: over the 16-char limit even though the
        // byte length fits 64.
        let mut buf = Buf::new();
        buf.write_string("ééééééééééééééééé").unwrap();
        assert_eq!(
            buf.read_string(16),
            Err(Error::StringTooLong { len: 17, max: 16 })
        );

        // Encoded buffer length over max * 4 (65 > 64) is refused before reading.
        let mut buf = Buf::from(vec![0xc1, 0x00]);
        assert!(matches!(
            buf.read_string(16),
            Err(Error::StringBufferTooLong { len: 65, max: 16 })
        ));

        // A negative length (VarInt -1) passes the max*4 check and reaches
        // the negative check, as in vanilla.
        let mut buf = Buf::from(vec![0xff, 0xff, 0xff, 0xff, 0x0f]);
        assert!(matches!(
            buf.read_string(16),
            Err(Error::NegativeLength { len: -1 })
        ));

        // Write side: 32768 bytes is too big.
        let mut buf = Buf::new();
        assert_eq!(
            buf.write_string(&"a".repeat(32768)),
            Err(Error::StringTooBig { len: 32768 })
        );
        assert!(buf.write_string(&"a".repeat(32767)).is_ok());
    }

    #[test]
    fn string_replaces_bad_utf8() {
        // Java's `new String(bytes, UTF_8)` decodes 0x80 as U+FFFD.
        let mut buf = Buf::from(vec![0x01, 0x80]);
        assert_eq!(buf.read_string(16).unwrap(), "\u{fffd}");
    }

    #[test]
    fn byte_array_round_trip() {
        let mut buf = Buf::new();
        buf.write_byte_array(&[1, 2, 3, 4]);
        assert_eq!(buf.read_byte_array().unwrap(), vec![1, 2, 3, 4]);
        let mut buf = Buf::from(vec![0xff, 0xff]);
        assert_eq!(
            buf.read_byte_array(),
            Err(Error::NegativeLength { len: -1 })
        );
    }

    #[test]
    fn position_packed_long_round_trip() {
        // The packing carries 26-bit signed x/z and 12-bit signed y; the top
        // six x bits are lost outside ±2^25, as in vanilla's packing.
        let mut buf = Buf::new();
        buf.write_position(-1_000_000, 255, 1_000_000);
        assert_eq!(buf.read_position().unwrap(), [-1_000_000, 255, 1_000_000]);
        buf.write_position(33_554_431, 2047, -33_554_431);
        assert_eq!(
            buf.read_position().unwrap(),
            [33_554_431, 2047, -33_554_431]
        );
        buf.write_position(-1, -1, -1);
        assert_eq!(buf.read_position().unwrap(), [-1, -1, -1]);
    }
}
