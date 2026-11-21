//! NBT (Named Binary Tag) in the form `net.minecraft.nbt` writes it: big-endian
//! primitives, modified UTF-8 strings, and a root tag that is a named compound.
//!
//! This is the 1.7.10 save form: every chunk, `level.dat`, player file and
//! structure file in the world folder carries one of these compounds, gzip- or
//! zlib-wrapped by the storage layer above this crate.

use std::fmt;

/// Tag type ids, as the binary format numbers them.
pub mod tag {
    pub const END: u8 = 0;
    pub const BYTE: u8 = 1;
    pub const SHORT: u8 = 2;
    pub const INT: u8 = 3;
    pub const LONG: u8 = 4;
    pub const FLOAT: u8 = 5;
    pub const DOUBLE: u8 = 6;
    pub const BYTE_ARRAY: u8 = 7;
    pub const STRING: u8 = 8;
    pub const LIST: u8 = 9;
    pub const COMPOUND: u8 = 10;
    pub const INT_ARRAY: u8 = 11;
}

/// An NBT value. Names are kept only where the format carries them: compounds
/// hold named entries, lists hold unnamed elements, the root holds one unnamed
/// compound (the name is read and discarded, as `NbtIo` does).
#[derive(Debug, Clone, PartialEq)]
pub enum Nbt {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    /// `element` is the tag type the list declared on read (or that the first
    /// element fixed); an empty list written by `NbtList` carries type 0.
    List {
        element: u8,
        items: Vec<Nbt>,
    },
    /// Insertion-ordered named entries, like a compound's write order.
    Compound(Vec<(String, Nbt)>),
    IntArray(Vec<i32>),
}

impl Nbt {
    /// The tag's type id, as it is written into the stream.
    pub fn type_id(&self) -> u8 {
        match self {
            Nbt::Byte(_) => tag::BYTE,
            Nbt::Short(_) => tag::SHORT,
            Nbt::Int(_) => tag::INT,
            Nbt::Long(_) => tag::LONG,
            Nbt::Float(_) => tag::FLOAT,
            Nbt::Double(_) => tag::DOUBLE,
            Nbt::ByteArray(_) => tag::BYTE_ARRAY,
            Nbt::String(_) => tag::STRING,
            Nbt::List { .. } => tag::LIST,
            Nbt::Compound(_) => tag::COMPOUND,
            Nbt::IntArray(_) => tag::INT_ARRAY,
        }
    }

    /// `NbtCompound.get`: the entry named `key`, of any type.
    pub fn get(&self, key: &str) -> Option<&Nbt> {
        match self {
            Nbt::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// `NbtCompound.getType`: the named entry's type id, 0 when absent.
    pub fn get_type(&self, key: &str) -> u8 {
        self.get(key).map_or(0, Nbt::type_id)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// `NbtCompound.contains(key, type)`: type 99 matches any numeric tag.
    pub fn contains_typed(&self, key: &str, ty: u8) -> bool {
        let found = self.get_type(key);
        found == ty || (ty == 99 && matches!(found, 1..=6))
    }

    /// `NbtElement$Number` coercions, as Java's `as` casts narrow them.
    pub fn as_i8(&self) -> i8 {
        match self {
            Nbt::Byte(v) => *v,
            Nbt::Short(v) => *v as i8,
            Nbt::Int(v) => *v as i8,
            Nbt::Long(v) => *v as i8,
            Nbt::Float(v) => *v as i8,
            Nbt::Double(v) => *v as i8,
            _ => 0,
        }
    }

    pub fn as_i16(&self) -> i16 {
        match self {
            Nbt::Byte(v) => *v as i16,
            Nbt::Short(v) => *v,
            Nbt::Int(v) => *v as i16,
            Nbt::Long(v) => *v as i16,
            Nbt::Float(v) => *v as i16,
            Nbt::Double(v) => *v as i16,
            _ => 0,
        }
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            Nbt::Byte(v) => *v as i32,
            Nbt::Short(v) => *v as i32,
            Nbt::Int(v) => *v,
            Nbt::Long(v) => *v as i32,
            Nbt::Float(v) => *v as i32,
            Nbt::Double(v) => *v as i32,
            _ => 0,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Nbt::Byte(v) => *v as i64,
            Nbt::Short(v) => *v as i64,
            Nbt::Int(v) => *v as i64,
            Nbt::Long(v) => *v,
            Nbt::Float(v) => *v as i64,
            Nbt::Double(v) => *v as i64,
            _ => 0,
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            Nbt::Byte(v) => *v as f32,
            Nbt::Short(v) => *v as f32,
            Nbt::Int(v) => *v as f32,
            Nbt::Long(v) => *v as f32,
            Nbt::Float(v) => *v,
            Nbt::Double(v) => *v as f32,
            _ => 0.0,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            Nbt::Byte(v) => *v as f64,
            Nbt::Short(v) => *v as f64,
            Nbt::Int(v) => *v as f64,
            Nbt::Long(v) => *v as f64,
            Nbt::Float(v) => *v as f64,
            Nbt::Double(v) => *v,
            _ => 0.0,
        }
    }

    /// `NbtCompound.getByte` and friends: 0 for a missing key.
    pub fn get_byte(&self, key: &str) -> i8 {
        self.get(key).map_or(0, Nbt::as_i8)
    }

    pub fn get_short(&self, key: &str) -> i16 {
        self.get(key).map_or(0, Nbt::as_i16)
    }

    pub fn get_int(&self, key: &str) -> i32 {
        self.get(key).map_or(0, Nbt::as_i32)
    }

    pub fn get_long(&self, key: &str) -> i64 {
        self.get(key).map_or(0, Nbt::as_i64)
    }

    pub fn get_float(&self, key: &str) -> f32 {
        self.get(key).map_or(0.0, Nbt::as_f32)
    }

    pub fn get_double(&self, key: &str) -> f64 {
        self.get(key).map_or(0.0, Nbt::as_f64)
    }

    /// `NbtCompound.getBoolean`: any numeric tag, non-zero is true.
    pub fn get_boolean(&self, key: &str) -> bool {
        self.get_byte(key) != 0
    }

    /// `NbtCompound.getString`: the entry's SNBT-ish text, "" when absent.
    pub fn get_string(&self, key: &str) -> String {
        match self.get(key) {
            Some(Nbt::String(s)) => s.clone(),
            Some(other) => other.as_text(),
            None => String::new(),
        }
    }

    /// `NbtElement.asString`/`toString`: numbers as Java prints them, arrays
    /// and lists in square brackets, compounds in braces.
    pub fn as_text(&self) -> String {
        match self {
            Nbt::Byte(v) => v.to_string(),
            Nbt::Short(v) => v.to_string(),
            Nbt::Int(v) => v.to_string(),
            Nbt::Long(v) => v.to_string(),
            Nbt::Float(v) => v.to_string(),
            Nbt::Double(v) => v.to_string(),
            Nbt::ByteArray(v) => {
                let mut s = String::from("[");
                for (i, b) in v.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&b.to_string());
                }
                s.push(']');
                s
            }
            Nbt::String(s) => s.clone(),
            Nbt::List { items, .. } => {
                let mut s = String::from("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&item.as_text());
                }
                s.push(']');
                s
            }
            Nbt::Compound(entries) => {
                let mut s = String::from("{");
                for (key, value) in entries {
                    s.push_str(key);
                    s.push(':');
                    s.push_str(&value.as_text());
                    s.push(',');
                }
                s.push('}');
                s
            }
            Nbt::IntArray(v) => {
                let mut s = String::from("[");
                for (i, item) in v.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&item.to_string());
                }
                s.push(']');
                s
            }
        }
    }

    /// `NbtCompound.getByteArray`: the bytes, empty for a missing key.
    pub fn get_byte_array(&self, key: &str) -> &[u8] {
        match self.get(key) {
            Some(Nbt::ByteArray(v)) => v,
            _ => &[],
        }
    }

    /// `NbtCompound.getIntArray`: the ints, empty for a missing key.
    pub fn get_int_array(&self, key: &str) -> &[i32] {
        match self.get(key) {
            Some(Nbt::IntArray(v)) => v,
            _ => &[],
        }
    }

    /// `NbtCompound.getCompound`: the named compound, empty when absent.
    pub fn get_compound(&self, key: &str) -> &Nbt {
        match self.get(key) {
            Some(Nbt::Compound(_)) => self.get(key).unwrap(),
            _ => &EMPTY_COMPOUND,
        }
    }

    /// `NbtCompound.getList(key, type)`: an empty list unless the named tag is
    /// a list of that element type (a non-empty list of another type reads as empty).
    pub fn get_list(&self, key: &str, element: u8) -> &[Nbt] {
        match self.get(key) {
            Some(Nbt::List { element: e, items }) if *e == element || items.is_empty() => items,
            _ => &[],
        }
    }

    /// `NbtCompound.put`: replace the named entry's value, keeping its position.
    pub fn put(&mut self, key: &str, value: Nbt) {
        if let Nbt::Compound(entries) = self {
            if let Some(slot) = entries.iter_mut().find(|(k, _)| k == key) {
                slot.1 = value;
                return;
            }
            entries.push((key.to_string(), value));
        }
    }

    /// `NbtCompound.putByte`, `putInt`, ... folded into one builder helper.
    pub fn put_int(&mut self, key: &str, value: i32) {
        self.put(key, Nbt::Int(value));
    }

    pub fn put_long(&mut self, key: &str, value: i64) {
        self.put(key, Nbt::Long(value));
    }

    pub fn put_byte(&mut self, key: &str, value: i8) {
        self.put(key, Nbt::Byte(value));
    }

    pub fn put_boolean(&mut self, key: &str, value: bool) {
        self.put(key, Nbt::Byte(value as i8));
    }

    pub fn put_int_array(&mut self, key: &str, value: Vec<i32>) {
        self.put(key, Nbt::IntArray(value));
    }

    pub fn put_byte_array(&mut self, key: &str, value: Vec<u8>) {
        self.put(key, Nbt::ByteArray(value));
    }
}

/// A shared empty compound, for `get_compound`'s default.
pub static EMPTY_COMPOUND: Nbt = Nbt::Compound(Vec::new());

/// Reads one root compound from an uncompressed NBT stream, as `NbtIo.read` does.
pub fn read(bytes: &[u8]) -> Result<Nbt, NbtError> {
    let mut r = Reader {
        data: bytes,
        pos: 0,
    };
    match r.element(0)? {
        nbt @ Nbt::Compound(_) => Ok(nbt),
        _ => Err(NbtError::RootNotCompound),
    }
}

/// Writes one root compound, unnamed with an empty name, as `NbtIo.write` does.
pub fn write(nbt: &Nbt) -> Result<Vec<u8>, NbtError> {
    let mut out = Vec::new();
    out.push(nbt.type_id());
    write_utf(&mut out, "")?;
    write_payload(&mut out, nbt)?;
    Ok(out)
}

fn write_payload(out: &mut Vec<u8>, nbt: &Nbt) -> Result<(), NbtError> {
    match nbt {
        Nbt::Byte(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::Short(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::Int(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::Long(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::Float(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::Double(v) => out.extend_from_slice(&v.to_be_bytes()),
        Nbt::ByteArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            out.extend_from_slice(v);
        }
        Nbt::String(v) => write_utf(out, v)?,
        Nbt::List { element, items } => {
            // `NbtList.write`: the type of the first element, 0 when empty.
            let ty = items.first().map_or(*element, Nbt::type_id);
            out.push(ty);
            out.extend_from_slice(&(items.len() as i32).to_be_bytes());
            for item in items {
                write_payload(out, item)?;
            }
        }
        Nbt::Compound(entries) => {
            for (key, value) in entries {
                out.push(value.type_id());
                if value.type_id() != tag::END {
                    write_utf(out, key)?;
                    write_payload(out, value)?;
                }
            }
            out.push(tag::END);
        }
        Nbt::IntArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for item in v {
                out.extend_from_slice(&item.to_be_bytes());
            }
        }
    }
    Ok(())
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'_ [u8], NbtError> {
        if self.remaining() < n {
            return Err(NbtError::Eof);
        }
        let start = self.pos;
        self.pos += n;
        Ok(&self.data[start..self.pos])
    }

    fn u8(&mut self) -> Result<u8, NbtError> {
        Ok(self.take(1)?[0])
    }

    fn i16(&mut self) -> Result<i16, NbtError> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, NbtError> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, NbtError> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }

    /// Reads one element (unnamed), depth-guarded like `NbtElement.read`.
    fn element(&mut self, depth: u32) -> Result<Nbt, NbtError> {
        if depth > 512 {
            return Err(NbtError::TooDeep);
        }
        let ty = self.u8()?;
        if ty == tag::END {
            // `NbtIo.read` on an END root: "root tag must be a named compound tag".
            return Err(NbtError::RootNotCompound);
        }
        self.utf()?; // the name, read and discarded
        self.payload(ty, depth)
    }

    fn payload(&mut self, ty: u8, depth: u32) -> Result<Nbt, NbtError> {
        // `NbtList.read` and `NbtCompound.read` both guard their own depth.
        if depth > 512 {
            return Err(NbtError::TooDeep);
        }
        Ok(match ty {
            tag::BYTE => Nbt::Byte(self.u8()? as i8),
            tag::SHORT => Nbt::Short(self.i16()?),
            tag::INT => Nbt::Int(self.i32()?),
            tag::LONG => Nbt::Long(self.i64()?),
            tag::FLOAT => Nbt::Float(f32::from_be_bytes(self.take(4)?.try_into().unwrap())),
            tag::DOUBLE => Nbt::Double(f64::from_be_bytes(self.take(8)?.try_into().unwrap())),
            tag::BYTE_ARRAY => {
                let len = self.i32()?;
                if len < 0 || len as usize > self.remaining() {
                    return Err(NbtError::Length(len));
                }
                Nbt::ByteArray(self.take(len as usize)?.to_vec())
            }
            tag::STRING => Nbt::String(self.utf()?),
            tag::LIST => {
                let element = self.u8()?;
                let len = self.i32()?;
                if len < 0 || len as usize > self.remaining() {
                    return Err(NbtError::Length(len));
                }
                let mut items = Vec::with_capacity(len as usize);
                if element == tag::END && len > 0 {
                    return Err(NbtError::BadListElement);
                }
                for _ in 0..len {
                    items.push(self.payload(element, depth + 1)?);
                }
                Nbt::List { element, items }
            }
            tag::COMPOUND => {
                let mut entries = Vec::new();
                loop {
                    let child = self.u8()?;
                    if child == tag::END {
                        break;
                    }
                    let name = self.utf()?;
                    let value = self.payload(child, depth + 1)?;
                    entries.push((name, value));
                }
                Nbt::Compound(entries)
            }
            tag::INT_ARRAY => {
                let len = self.i32()?;
                if len < 0 || len as usize * 4 > self.remaining() {
                    return Err(NbtError::Length(len));
                }
                let mut items = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    items.push(self.i32()?);
                }
                Nbt::IntArray(items)
            }
            _ => return Err(NbtError::UnknownType(ty)),
        })
    }

    /// Java's `DataInput.readUTF`: an unsigned-short byte count, then modified UTF-8.
    fn utf(&mut self) -> Result<String, NbtError> {
        let len = self.i16()? as u16 as usize;
        let bytes = self.take(len)?;
        modified_utf8(bytes).ok_or(NbtError::Utf)
    }
}

/// Decodes Java modified UTF-8 (U+0000 as `c0 80`, astral chars as surrogate
/// pairs each written in three bytes) to a Rust string.
fn modified_utf8(bytes: &[u8]) -> Option<String> {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let a = bytes[i];
        if a & 0x80 == 0 {
            if a == 0 {
                return None;
            }
            out.push(a as char);
            i += 1;
        } else if a & 0xe0 == 0xc0 {
            let b = *bytes.get(i + 1)?;
            if b & 0xc0 != 0x80 {
                return None;
            }
            out.push(char::from_u32(
                ((a & 0x1f) as u32) << 6 | (b & 0x3f) as u32,
            )?);
            i += 2;
        } else if a & 0xf0 == 0xe0 {
            let b = *bytes.get(i + 1)?;
            let c = *bytes.get(i + 2)?;
            if b & 0xc0 != 0x80 || c & 0xc0 != 0x80 {
                return None;
            }
            let cp = ((a & 0x0f) as u32) << 12 | ((b & 0x3f) as u32) << 6 | (c & 0x3f) as u32;
            match char::from_u32(cp) {
                Some(ch) => out.push(ch),
                // A high surrogate: its pair follows as another three-byte unit.
                None if (0xd800..0xdc00).contains(&cp) => {
                    let d = *bytes.get(i + 3)?;
                    let e = *bytes.get(i + 4)?;
                    let f = *bytes.get(i + 5)?;
                    if d & 0xf0 != 0xe0 || e & 0xc0 != 0x80 || f & 0xc0 != 0x80 {
                        return None;
                    }
                    let low =
                        ((d & 0x0f) as u32) << 12 | ((e & 0x3f) as u32) << 6 | (f & 0x3f) as u32;
                    if !(0xdc00..0xe000).contains(&low) {
                        return None;
                    }
                    let combined = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    out.push(char::from_u32(combined)?);
                    i += 6;
                    continue;
                }
                None => return None,
            }
            i += 3;
        } else {
            return None;
        }
    }
    Some(out)
}

/// Encodes a Rust string as Java modified UTF-8 with its two-byte length.
fn write_utf(out: &mut Vec<u8>, s: &str) -> Result<(), NbtError> {
    let mut bytes = Vec::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        match cp {
            0x01..=0x7f => bytes.push(cp as u8),
            0 | 0x80..=0x7ff => {
                bytes.push(0xc0 | (cp >> 6) as u8);
                bytes.push(0x80 | (cp & 0x3f) as u8);
            }
            0x800..=0xffff => {
                bytes.push(0xe0 | (cp >> 12) as u8);
                bytes.push(0x80 | ((cp >> 6) & 0x3f) as u8);
                bytes.push(0x80 | (cp & 0x3f) as u8);
            }
            _ => {
                // Astral: a surrogate pair, each half written in three bytes.
                let v = cp - 0x10000;
                let high = 0xd800 + (v >> 10);
                let low = 0xdc00 + (v & 0x3ff);
                for half in [high, low] {
                    bytes.push(0xe0 | (half >> 12) as u8);
                    bytes.push(0x80 | ((half >> 6) & 0x3f) as u8);
                    bytes.push(0x80 | (half & 0x3f) as u8);
                }
            }
        }
    }
    if bytes.len() > u16::MAX as usize {
        return Err(NbtError::StringTooLong);
    }
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum NbtError {
    /// The stream ended before the tag did.
    Eof,
    /// A tag type the format does not number.
    UnknownType(u8),
    /// A negative or overlong array or list length.
    Length(i32),
    /// A list declaring element type 0 with items.
    BadListElement,
    /// Nesting deeper than `NbtElement.read` allows (512).
    TooDeep,
    /// A string that is not modified UTF-8.
    Utf,
    /// A string longer than `DataOutput.writeUTF` allows.
    StringTooLong,
    /// `NbtIo.read` on a stream whose root is not a compound.
    RootNotCompound,
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbtError::Eof => write!(f, "unexpected end of NBT data"),
            NbtError::UnknownType(t) => write!(f, "unknown NBT tag type {t}"),
            NbtError::Length(n) => write!(f, "bad NBT length {n}"),
            NbtError::BadListElement => write!(f, "NBT list of END elements"),
            NbtError::TooDeep => write!(f, "NBT nesting deeper than 512"),
            NbtError::Utf => write!(f, "bad modified UTF-8 string"),
            NbtError::StringTooLong => write!(f, "string longer than 65535 bytes"),
            NbtError::RootNotCompound => write!(f, "root tag must be a named compound tag"),
        }
    }
}

impl std::error::Error for NbtError {}
