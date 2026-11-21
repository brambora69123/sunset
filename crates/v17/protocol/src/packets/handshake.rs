//! Handshake state, serverbound: `HandshakeC2SPacket`.

use crate::buf::Buf;
use crate::registry::{Direction, ProtocolState};

/// Exact port of `HandshakeC2SPacket`: the version, the address, the port
/// and the state the client wants next.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: ProtocolState,
}

impl Handshake {
    pub fn read(buf: &mut Buf) -> crate::buf::Result<Self> {
        let protocol_version = buf.read_var_int()?;
        let server_address = buf.read_string(255)?;
        let server_port = buf.read_u16()?;
        let next_state_id = buf.read_var_int()?;
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state: ProtocolState::by_id(next_state_id)
                .ok_or(crate::buf::Error::UnknownProtocolState { id: next_state_id })?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> crate::buf::Result<()> {
        buf.write_var_int(self.protocol_version);
        buf.write_string(&self.server_address)?;
        buf.write_u16(self.server_port);
        buf.write_var_int(self.next_state.id());
        Ok(())
    }
}

pub const ID: i32 = 0x00;
pub const DIRECTION: Direction = Direction::Serverbound;
