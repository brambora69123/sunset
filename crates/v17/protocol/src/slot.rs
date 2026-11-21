//! 1.7.10 item stack on the wire (`PacketByteBuf.writeItem`/`readItem`).

/// A 1.7 item stack: id, count, metadata and the raw NBT compound bytes.
///
/// The NBT payload is kept as raw (uncompressed) bytes rather than a parsed
/// compound: the parse belongs to the NBT crate, the wire format lives here.
/// Vanilla only serializes the NBT when the item is damageable or syncs it;
/// that decision is the caller's (item facts are not in this crate).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Slot {
    pub id: i16,
    pub count: i8,
    pub meta: i16,
    pub nbt: Option<Vec<u8>>,
}

impl Slot {
    pub fn new(id: i16, count: i8, meta: i16) -> Self {
        Self {
            id,
            count,
            meta,
            nbt: None,
        }
    }

    pub fn with_nbt(id: i16, count: i8, meta: i16, nbt: Vec<u8>) -> Self {
        Self {
            id,
            count,
            meta,
            nbt: Some(nbt),
        }
    }
}
