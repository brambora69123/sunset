//! `PacketByteBuf`, ported from the decompiled 1.7.10 source: the varint codec,
//! the primitives, `readString` with its maximum-length errors and the
//! short-prefixed byte arrays `Packet.writeByteArray` shares.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufError {
    UnexpectedEof,
    VarIntTooBig,
    /// `readString`: encoded length over `maxLength * 4`.
    StringBufTooLong {
        got: usize,
        max: usize,
    },
    /// `readString`: encoded length below zero.
    StringBufNegative,
    /// `readString`: decoded string over `maxLength` characters.
    StringTooLong {
        got: usize,
        max: usize,
    },
    /// `writeString`: "String too big (was {n} bytes encoded, max 32767)".
    StringTooBig {
        chars: usize,
    },
    /// `readByteArray`: "Key was smaller than nothing!  Weird key!".
    KeyNegative,
    /// Bad zlib data in a compressed frame.
    Zlib,
    /// `PacketEncoder`: "Can't serialize unregistered packet".
    UnregisteredPacket,
    InvalidUtf8,
}

impl fmt::Display for BufError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of buffer"),
            Self::VarIntTooBig => write!(f, "VarInt too big"),
            Self::StringBufTooLong { got, max } => write!(
                f,
                "The received encoded string buffer length is longer than maximum allowed ({got} > {max})"
            ),
            Self::StringBufNegative => write!(
                f,
                "The received encoded string buffer length is less than zero! Weird string!"
            ),
            Self::StringTooLong { got, max } => write!(
                f,
                "The received string length is longer than maximum allowed ({got} > {max})"
            ),
            Self::StringTooBig { chars } => {
                write!(f, "String too big (was {chars} bytes encoded, max 32767)")
            }
            Self::KeyNegative => write!(f, "Key was smaller than nothing!  Weird key!"),
            Self::Zlib => write!(f, "bad zlib data"),
            Self::UnregisteredPacket => write!(f, "Can't serialize unregistered packet"),
            Self::InvalidUtf8 => write!(f, "invalid utf-8 string"),
        }
    }
}

impl std::error::Error for BufError {}

/// The read and write cursor over one packet payload.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PacketBuf {
    data: Vec<u8>,
    pos: usize,
}

impl PacketBuf {
    /// An empty buffer to write into.
    pub fn new() -> Self {
        Self::default()
    }

    /// A read cursor over a complete packet payload.
    pub fn wrap(payload: &[u8]) -> Self {
        Self {
            data: payload.to_vec(),
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

    /// The bytes not yet read.
    pub fn rest(&self) -> &[u8] {
        &self.data[self.pos.min(self.data.len())..]
    }

    /// The written bytes, for `PacketEncoder`.
    pub fn written(&self) -> &[u8] {
        &self.data
    }

    pub fn into_written(self) -> Vec<u8> {
        self.data
    }

    fn take(&mut self, count: usize) -> Result<&[u8], BufError> {
        if self.pos + count > self.data.len() {
            return Err(BufError::UnexpectedEof);
        }
        let start = self.pos;
        self.pos += count;
        Ok(&self.data[start..self.pos])
    }

    pub fn read_boolean(&mut self) -> Result<bool, BufError> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u8(&mut self) -> Result<u8, BufError> {
        Ok(self.take(1)?[0])
    }

    pub fn read_i8(&mut self) -> Result<i8, BufError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16(&mut self) -> Result<u16, BufError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn read_i16(&mut self) -> Result<i16, BufError> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub fn read_i32(&mut self) -> Result<i32, BufError> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn read_i64(&mut self) -> Result<i64, BufError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// `PacketByteBuf.readVarInt`: five bytes at most, "VarInt too big" past that.
    pub fn read_var_int(&mut self) -> Result<i32, BufError> {
        let mut result: i32 = 0;
        let mut count = 0;
        loop {
            let byte = self.read_u8()?;
            if count < 5 {
                result |= ((byte & 0x7f) as i32) << (count * 7);
            }
            count += 1;
            if count > 5 {
                return Err(BufError::VarIntTooBig);
            }
            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }
    }

    /// `readString(maxLength)`: the encoded length caps at `maxLength * 4`, the
    /// decoded string at `maxLength` characters.
    pub fn read_string(&mut self, max_length: usize) -> Result<String, BufError> {
        let encoded = self.read_var_int()?;
        if encoded < 0 {
            return Err(BufError::StringBufNegative);
        }
        let encoded = encoded as usize;
        if encoded > max_length * 4 {
            return Err(BufError::StringBufTooLong {
                got: encoded,
                max: max_length * 4,
            });
        }
        let bytes = self.take(encoded)?;
        let text = String::from_utf8_lossy(bytes);
        if text.chars().count() > max_length {
            return Err(BufError::StringTooLong {
                got: text.chars().count(),
                max: max_length,
            });
        }
        Ok(text.into_owned())
    }

    /// `Packet.readByteArray`: a short-prefixed array.
    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, BufError> {
        let len = self.read_i16()?;
        if len < 0 {
            return Err(BufError::KeyNegative);
        }
        Ok(self.take(len as usize)?.to_vec())
    }

    pub fn write_boolean(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.write_u8(value as u8);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i16(&mut self, value: i16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    /// `PacketByteBuf.writeVarInt`.
    pub fn write_var_int(&mut self, value: i32) {
        let mut value = value;
        while value & -128 != 0 {
            self.write_u8((value & 0x7f) as u8 | 0x80);
            value = ((value as u32) >> 7) as i32;
        }
        self.write_u8(value as u8);
    }

    /// `writeString`: 32767 bytes at most.
    pub fn write_string(&mut self, value: &str) -> Result<(), BufError> {
        let bytes = value.as_bytes();
        if bytes.len() > 32767 {
            return Err(BufError::StringTooBig {
                chars: value.chars().count(),
            });
        }
        self.write_var_int(bytes.len() as i32);
        self.data.extend_from_slice(bytes);
        Ok(())
    }

    /// `Packet.writeByteArray`: a short-prefixed array.
    pub fn write_byte_array(&mut self, value: &[u8]) {
        self.write_i16(value.len() as i16);
        self.data.extend_from_slice(value);
    }

    /// Write raw bytes (`writeBytes`).
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
    }
}

/// `PacketByteBuf.getVarIntSizeBytes`.
pub fn get_var_int_size_bytes(size: i32) -> usize {
    if size & -128 == 0 {
        1
    } else if size & -16384 == 0 {
        2
    } else if size & -2097152 == 0 {
        3
    } else if size & -268435456 == 0 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_int_round_trips() {
        for value in [
            0,
            1,
            127,
            128,
            255,
            16383,
            16384,
            2097151,
            2097152,
            i32::MAX,
            -1,
        ] {
            let mut buf = PacketBuf::new();
            buf.write_var_int(value);
            assert_eq!(buf.read_var_int().unwrap(), value, "value {value}");
        }
    }

    #[test]
    fn var_int_size_table() {
        // (size & -128) == 0 → 1 byte, and so on, exactly like vanilla.
        assert_eq!(get_var_int_size_bytes(0), 1);
        assert_eq!(get_var_int_size_bytes(127), 1);
        assert_eq!(get_var_int_size_bytes(128), 2);
        assert_eq!(get_var_int_size_bytes(16383), 2);
        assert_eq!(get_var_int_size_bytes(16384), 3);
        assert_eq!(get_var_int_size_bytes(2097151), 3);
        assert_eq!(get_var_int_size_bytes(2097152), 4);
        assert_eq!(get_var_int_size_bytes(268435455), 4);
        assert_eq!(get_var_int_size_bytes(268435456), 5);
        assert_eq!(get_var_int_size_bytes(-1), 5);
    }

    #[test]
    fn var_int_too_big() {
        let mut buf = PacketBuf::wrap(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
        assert_eq!(buf.read_var_int(), Err(BufError::VarIntTooBig));
    }

    #[test]
    fn var_int_wire_form() {
        let mut buf = PacketBuf::new();
        buf.write_var_int(2097152);
        assert_eq!(buf.written(), &[0x80, 0x80, 0x80, 0x01]);
    }

    #[test]
    fn string_length_errors() {
        let mut buf = PacketBuf::new();
        buf.write_string("hello").unwrap();
        assert_eq!(buf.read_string(5).unwrap(), "hello");
        // A fresh cursor: the encoded length over maxLength * 4.
        let mut buf = PacketBuf::wrap(&[0x15, b'h']);
        assert!(matches!(
            buf.read_string(4),
            Err(BufError::StringBufTooLong { got: 21, max: 16 })
        ));
        // A decoded string over maxLength characters.
        let mut buf = PacketBuf::new();
        buf.write_string("hello").unwrap();
        assert!(matches!(
            buf.read_string(2),
            Err(BufError::StringTooLong { got: 5, max: 2 })
        ));
        let mut buf = PacketBuf::wrap(&[0x81, 0x00]);
        // 0x81 0x00 encodes 1: over maxLength * 4 = 0.
        assert!(matches!(
            buf.read_string(0),
            Err(BufError::StringBufTooLong { got: 1, max: 0 })
        ));
    }

    #[test]
    fn string_write_limit() {
        let mut buf = PacketBuf::new();
        let long = "x".repeat(32768);
        assert!(buf.write_string(&long).is_err());
        let exact = "x".repeat(32767);
        buf.write_string(&exact).unwrap();
    }

    #[test]
    fn byte_arrays_are_short_prefixed() {
        let mut buf = PacketBuf::new();
        buf.write_byte_array(&[1, 2, 3]);
        assert_eq!(buf.written(), &[0, 3, 1, 2, 3]);
        assert_eq!(buf.read_byte_array().unwrap(), vec![1, 2, 3]);
        let mut buf = PacketBuf::wrap(&[0xff, 0xff]);
        assert_eq!(buf.read_byte_array(), Err(BufError::KeyNegative));
    }
}
