//! Minecraft 1.7.10 protocol, ported for sunset. See docs/ for the port contract.
//!
//! Protocol version 5 (1.7.2-1.7.10, 1.7.10 sends 5): the wire layer
//! ([`buf`]), the frame layer ([`framing`]), the packet table ([`registry`])
//! and the handshake, status, login and play packets needed to log in and
//! spawn ([`packets`]). The item stack ([`slot`]) keeps its NBT as raw bytes.
//!
//! Not part of the 1.7.10 wire, kept as extra codecs: VarLong and the packed
//! position long ([`buf`]).

pub mod buf;
pub mod framing;
pub mod packets;
pub mod registry;
pub mod slot;

pub use buf::{Buf, Error, Result};
pub use registry::{
    Direction, PROTOCOL_VERSION, ProtocolState, VERSION_STRING, decode_packet, encode_packet,
    read_framed_packet, write_framed_packet,
};
pub use slot::Slot;
