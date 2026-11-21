//! The packet registry for the connection flow, ported from the decompiled
//! 1.7.10 `NetworkProtocol` packet tables and the packet classes: handshake
//! (intent), login (start, encryption, success, fail), status (request,
//! response, ping) and the play-state keepalive and disconnect. Every other
//! play packet arrives as [`Packet::Unknown`]; the caller registers its ids
//! with [`Connection::register`].

use crate::buf::{BufError, PacketBuf};
use crate::text::Text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// `NetworkProtocol.HANDSHAKE`, id -1.
    Handshake,
    /// `NetworkProtocol.PLAY`, id 0.
    Play,
    /// `NetworkProtocol.STATUS`, id 1.
    Status,
    /// `NetworkProtocol.LOGIN`, id 2.
    Login,
}

impl Protocol {
    pub fn id(self) -> i32 {
        match self {
            Self::Handshake => -1,
            Self::Play => 0,
            Self::Status => 1,
            Self::Login => 2,
        }
    }

    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            -1 => Some(Self::Handshake),
            0 => Some(Self::Play),
            1 => Some(Self::Status),
            2 => Some(Self::Login),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    /// `HandshakeC2SPacket`; `next_protocol` is the intended state id.
    Handshake {
        version: i32,
        address: String,
        port: u16,
        next_protocol: i32,
    },
    /// `HelloC2SPacket` (login start).
    LoginStart { name: String },
    /// `KeyC2SPacket` (encryption response).
    EncryptionResponse {
        shared_secret: Vec<u8>,
        verify_token: Vec<u8>,
    },
    /// `HelloS2CPacket` (encryption request).
    EncryptionRequest {
        server_id: String,
        public_key: Vec<u8>,
        verify_token: Vec<u8>,
    },
    /// `LoginSuccessS2CPacket`.
    LoginSuccess { uuid: String, name: String },
    /// `LoginFailS2CPacket` and `DisconnectS2CPacket`: the reason as JSON text.
    Disconnect { reason: Text },
    /// `ServerStatusC2SPacket`.
    StatusRequest,
    /// `ServerStatusS2CPacket`.
    StatusResponse { json: String },
    /// `PingC2SPacket` / `PingS2CPacket` (query).
    Ping { time: i64 },
    /// `KeepAliveC2SPacket` / `KeepAliveS2CPacket`.
    KeepAlive { id: i32 },
    /// A packet in a registered id this crate's registry does not decode.
    Unknown { id: i32, payload: Vec<u8> },
}

impl Packet {
    /// `Packet.create` + `Packet.read`: decode a payload with the receivable
    /// packet map (a client receives s2c, a server receives c2s). An id the
    /// registry does not decode comes back as [`Packet::Unknown`]. `buf` sits
    /// after the packet id; whatever it did not read is leftover.
    pub fn decode(
        protocol: Protocol,
        is_client: bool,
        id: i32,
        buf: &mut PacketBuf,
    ) -> Result<Packet, BufError> {
        match (protocol, is_client, id) {
            (Protocol::Handshake, false, 0) => Ok(Packet::Handshake {
                version: buf.read_var_int()?,
                address: buf.read_string(255)?,
                port: buf.read_u16()?,
                next_protocol: buf.read_var_int()?,
            }),
            (Protocol::Login, false, 0) => Ok(Packet::LoginStart {
                name: buf.read_string(16)?,
            }),
            (Protocol::Login, false, 1) => Ok(Packet::EncryptionResponse {
                shared_secret: buf.read_byte_array()?,
                verify_token: buf.read_byte_array()?,
            }),
            (Protocol::Login, true, 1) => Ok(Packet::EncryptionRequest {
                server_id: buf.read_string(20)?,
                public_key: buf.read_byte_array()?,
                verify_token: buf.read_byte_array()?,
            }),
            (Protocol::Login, true, 2) => Ok(Packet::LoginSuccess {
                uuid: buf.read_string(36)?,
                name: buf.read_string(16)?,
            }),
            (Protocol::Status, false, 0) => Ok(Packet::StatusRequest),
            (Protocol::Status, false, 1) | (Protocol::Status, true, 1) => Ok(Packet::Ping {
                time: buf.read_i64()?,
            }),
            (Protocol::Status, true, 0) => Ok(Packet::StatusResponse {
                json: buf.read_string(32767)?,
            }),
            (Protocol::Play, false, 0) | (Protocol::Play, true, 0) => Ok(Packet::KeepAlive {
                id: buf.read_i32()?,
            }),
            (Protocol::Play, true, 64) => Ok(Packet::Disconnect {
                reason: Text::from_json(&buf.read_string(32767)?)
                    .unwrap_or_else(|| Text::literal("")),
            }),
            _ => Ok(Packet::Unknown {
                id,
                payload: buf.rest().to_vec(),
            }),
        }
    }

    /// `PacketEncoder`: the packet id for this state and direction (a client
    /// sends c2s, a server sends s2c), then the payload.
    pub fn encode(&self, protocol: Protocol, is_client: bool) -> Result<(i32, Vec<u8>), BufError> {
        let kind = self.kind().ok_or(BufError::UnregisteredPacket)?;
        let id = match outgoing_id(kind, protocol, is_client) {
            Some(id) => id,
            None => return Err(BufError::UnregisteredPacket),
        };
        let mut buf = PacketBuf::new();
        buf.write_var_int(id);
        match self {
            Packet::Handshake {
                version,
                address,
                port,
                next_protocol,
            } => {
                buf.write_var_int(*version);
                buf.write_string(address)?;
                buf.write_u16(*port);
                buf.write_var_int(*next_protocol);
            }
            Packet::LoginStart { name } => {
                buf.write_string(name)?;
            }
            Packet::EncryptionResponse {
                shared_secret,
                verify_token,
            } => {
                buf.write_byte_array(shared_secret);
                buf.write_byte_array(verify_token);
            }
            Packet::EncryptionRequest {
                server_id,
                public_key,
                verify_token,
            } => {
                buf.write_string(server_id)?;
                buf.write_byte_array(public_key);
                buf.write_byte_array(verify_token);
            }
            Packet::LoginSuccess { uuid, name } => {
                buf.write_string(uuid)?;
                buf.write_string(name)?;
            }
            Packet::Disconnect { reason } => {
                buf.write_string(&reason.to_json())?;
            }
            Packet::StatusRequest => {}
            Packet::StatusResponse { json } => {
                buf.write_string(json)?;
            }
            Packet::Ping { time } => {
                buf.write_i64(*time);
            }
            Packet::KeepAlive { id } => {
                buf.write_i32(*id);
            }
            Packet::Unknown { payload, .. } => {
                buf.write_bytes(payload);
            }
        }
        Ok((id, buf.into_written()))
    }

    /// The id this packet is read under in the state (a client receives s2c, a
    /// server receives c2s).
    pub fn incoming_id(&self, protocol: Protocol, is_client: bool) -> Option<i32> {
        let kind = self.kind()?;
        if is_client {
            s2c_id(kind, protocol)
        } else {
            c2s_id(kind, protocol)
        }
    }

    fn kind(&self) -> Option<Kind> {
        Some(match self {
            Packet::Handshake { .. } => Kind::Handshake,
            Packet::LoginStart { .. } => Kind::LoginStart,
            Packet::EncryptionResponse { .. } => Kind::EncryptionResponse,
            Packet::EncryptionRequest { .. } => Kind::EncryptionRequest,
            Packet::LoginSuccess { .. } => Kind::LoginSuccess,
            Packet::Disconnect { .. } => Kind::Disconnect,
            Packet::StatusRequest => Kind::StatusRequest,
            Packet::StatusResponse { .. } => Kind::StatusResponse,
            Packet::Ping { .. } => Kind::Ping,
            Packet::KeepAlive { .. } => Kind::KeepAlive,
            Packet::Unknown { .. } => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Handshake,
    LoginStart,
    EncryptionResponse,
    EncryptionRequest,
    LoginSuccess,
    Disconnect,
    StatusRequest,
    StatusResponse,
    Ping,
    KeepAlive,
}

/// The packet id registered for this packet in this state and direction: a
/// client sends c2s, a server sends s2c.
fn outgoing_id(kind: Kind, protocol: Protocol, is_client: bool) -> Option<i32> {
    if is_client {
        c2s_id(kind, protocol)
    } else {
        s2c_id(kind, protocol)
    }
}

/// The 1.7.10 packet id table (`NetworkProtocol.registerC2S`/`registerS2C`).
fn c2s_id(kind: Kind, protocol: Protocol) -> Option<i32> {
    match (protocol, kind) {
        (Protocol::Handshake, Kind::Handshake) => Some(0),
        (Protocol::Login, Kind::LoginStart) => Some(0),
        (Protocol::Login, Kind::EncryptionResponse) => Some(1),
        (Protocol::Status, Kind::StatusRequest) => Some(0),
        (Protocol::Status, Kind::Ping) => Some(1),
        (Protocol::Play, Kind::KeepAlive) => Some(0),
        _ => None,
    }
}

fn s2c_id(kind: Kind, protocol: Protocol) -> Option<i32> {
    match (protocol, kind) {
        (Protocol::Login, Kind::EncryptionRequest) => Some(1),
        (Protocol::Login, Kind::LoginSuccess) => Some(2),
        (Protocol::Login, Kind::Disconnect) => Some(0),
        (Protocol::Status, Kind::StatusResponse) => Some(0),
        (Protocol::Status, Kind::Ping) => Some(1),
        (Protocol::Play, Kind::KeepAlive) => Some(0),
        (Protocol::Play, Kind::Disconnect) => Some(64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_ids_match_vanilla() {
        // registerC2S(0, HelloC2SPacket), registerC2S(1, KeyC2SPacket)
        assert_eq!(
            Packet::LoginStart { name: "x".into() }.encode(Protocol::Login, true),
            Ok((0, {
                let mut buf = PacketBuf::new();
                buf.write_var_int(0);
                buf.write_string("x").unwrap();
                buf.into_written()
            }))
        );
        assert_eq!(
            Packet::EncryptionResponse {
                shared_secret: vec![1],
                verify_token: vec![2, 3],
            }
            .encode(Protocol::Login, true)
            .unwrap()
            .0,
            1
        );
        // registerS2C(0, LoginFailS2CPacket), registerS2C(1, HelloS2CPacket),
        // registerS2C(2, LoginSuccessS2CPacket)
        assert_eq!(
            Packet::Disconnect {
                reason: Text::literal("nope")
            }
            .incoming_id(Protocol::Login, true),
            Some(0)
        );
        assert_eq!(
            Packet::EncryptionRequest {
                server_id: String::new(),
                public_key: vec![],
                verify_token: vec![],
            }
            .incoming_id(Protocol::Login, true),
            Some(1)
        );
        assert_eq!(
            Packet::LoginSuccess {
                uuid: "u".into(),
                name: "n".into(),
            }
            .incoming_id(Protocol::Login, true),
            Some(2)
        );
    }

    #[test]
    fn play_and_status_ids_match_vanilla() {
        // Play: s2c 0 KeepAlive, 64 Disconnect; c2s 0 KeepAlive.
        assert_eq!(
            Packet::KeepAlive { id: 5 }.incoming_id(Protocol::Play, true),
            Some(0)
        );
        assert_eq!(
            Packet::KeepAlive { id: 5 }.incoming_id(Protocol::Play, false),
            Some(0)
        );
        assert_eq!(
            Packet::Disconnect {
                reason: Text::literal("x")
            }
            .incoming_id(Protocol::Play, true),
            Some(64)
        );
        // Status: c2s 0 request, 1 ping; s2c 0 response, 1 ping.
        assert_eq!(
            Packet::StatusRequest.incoming_id(Protocol::Status, false),
            Some(0)
        );
        assert_eq!(
            Packet::Ping { time: 1 }.incoming_id(Protocol::Status, true),
            Some(1)
        );
        // Handshake: c2s 0.
        assert_eq!(
            Packet::Handshake {
                version: 5,
                address: "a".into(),
                port: 25565,
                next_protocol: 2
            }
            .incoming_id(Protocol::Handshake, false),
            Some(0)
        );
    }

    #[test]
    fn decode_round_trips() {
        let (id, payload) = Packet::Handshake {
            version: 5,
            address: "127.0.0.1".into(),
            port: 25565,
            next_protocol: 2,
        }
        .encode(Protocol::Handshake, true)
        .unwrap();
        // The payload starts with the packet id; decode sits after it.
        let mut buf = PacketBuf::wrap(&payload);
        assert_eq!(buf.read_var_int().unwrap(), id);
        assert_eq!(
            Packet::decode(Protocol::Handshake, false, id, &mut buf).unwrap(),
            Packet::Handshake {
                version: 5,
                address: "127.0.0.1".into(),
                port: 25565,
                next_protocol: 2,
            }
        );
    }

    #[test]
    fn unregistered_ids_come_back_unknown() {
        // Play id 1 (chat in 1.7.10) is not in this crate's registry.
        let payload = [0x00];
        assert_eq!(
            Packet::decode(Protocol::Play, false, 1, &mut PacketBuf::wrap(&payload)).unwrap(),
            Packet::Unknown {
                id: 1,
                payload: vec![0]
            }
        );
    }

    #[test]
    fn encryption_arrays_are_short_prefixed() {
        let (_, payload) = Packet::EncryptionResponse {
            shared_secret: vec![0xaa; 16],
            verify_token: vec![0x55; 4],
        }
        .encode(Protocol::Login, true)
        .unwrap();
        let expected: Vec<u8> = [0x01]
            .iter()
            .copied()
            .chain([0u8, 16])
            .chain([0xaa; 16])
            .chain([0u8, 4])
            .chain([0x55; 4])
            .collect();
        assert_eq!(payload, expected);
    }
}
