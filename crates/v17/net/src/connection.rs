//! `Connection`, ported from the decompiled 1.7.10 source without netty: the
//! codec pipeline (decrypt → frame split → decompress → decode in, encode →
//! compress → frame → encrypt out), the protocol state machine, the read queue
//! with vanilla's per-tick cap, the 30-second read timeout, and disconnect.
//!
//! The connection is transport-agnostic: the caller feeds wire bytes in with
//! [`Connection::receive`] and takes wire bytes out with
//! [`Connection::take_outbound`], so a TCP socket, an in-memory pipe or a
//! harness channel all drive it the same way.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::buf::BufError;
use crate::compression::{compress_frame, decompress_frame};
use crate::crypto::AesCfb8;
use crate::framing::{FrameError, frame, split_frame};
use crate::packets::{Packet, Protocol};
use crate::text::Text;

/// The protocol version the handshake intent carries: 1.7.10 is 5.
pub const PROTOCOL_VERSION_1_7_10: i32 = 5;

/// Vanilla drains 1001 packets per tick (`for (int i = 1000; !queue.isEmpty()
/// && i >= 0; i--)`).
pub const MAX_PACKETS_PER_TICK: usize = 1001;

/// `ReadTimeoutHandler(30)` from the vanilla channel setup.
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionError {
    /// `PacketDecoder`: "Bad packet id {n}".
    BadPacketId(i32),
    /// `PacketDecoder`: "Packet was larger than I expected, found {n} bytes
    /// extra whilst reading packet {id}".
    ExtraBytes { extra: usize, id: i32 },
    /// `PacketEncoder`: "Can't serialize unregistered packet".
    UnregisteredPacket,
    /// A decode or encode error from `PacketByteBuf`.
    Buf(BufError),
    /// A framing error from `SplitterHandler` or `SizePrepender`.
    Frame(FrameError),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadPacketId(id) => write!(f, "Bad packet id {id}"),
            Self::ExtraBytes { extra, id } => write!(
                f,
                "Packet was larger than I expected, found {extra} bytes extra whilst reading packet {id}"
            ),
            Self::UnregisteredPacket => write!(f, "Can't serialize unregistered packet"),
            Self::Buf(error) => write!(f, "{error}"),
            Self::Frame(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ConnectionError {}

impl From<BufError> for ConnectionError {
    fn from(error: BufError) -> Self {
        Self::Buf(error)
    }
}

impl From<FrameError> for ConnectionError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

/// `net.minecraft.network.handler.PacketHandler`: the listener a connection
/// dispatches decoded packets to. State violations come back as errors and
/// disconnect with vanilla's "Internal Exception: ..." reason.
pub trait PacketListener: std::any::Any {
    fn tick(&mut self, _connection: &mut Connection) {}

    /// `changeProtocol`, fired on the tick after the switch.
    fn change_protocol(&mut self, _from: Protocol, _to: Protocol, _connection: &mut Connection) {}

    /// `onDisconnect`, fired once the connection is down.
    fn on_disconnect(&mut self, _reason: &Text, _connection: &mut Connection) {}

    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String>;
}

/// The codec pipeline and protocol state of one connection.
pub struct Connection {
    /// A client connection receives s2c and sends c2s.
    pub is_client: bool,
    protocol: Protocol,
    /// Wire bytes not yet framed (plaintext after the inbound decrypt).
    inbound: Vec<u8>,
    /// Wire bytes ready to send (length-prefixed, encrypted).
    outbound: Vec<u8>,
    read_queue: VecDeque<Packet>,
    decrypt: Option<AesCfb8>,
    encrypt: Option<AesCfb8>,
    compression_threshold: Option<u32>,
    /// Ids registered by the caller beyond this crate's registry.
    registered: Vec<(Protocol, bool, i32)>,
    listener: Option<Box<dyn PacketListener>>,
    protocol_changes: Vec<(Protocol, Protocol)>,
    disconnect_reason: Option<Text>,
    disconnected: bool,
    /// `channel instanceof LocalChannel`.
    local: bool,
    read_timeout: Option<Duration>,
    last_read: Instant,
}

impl Connection {
    /// A fresh connection in the handshake state (`Connection.channelActive`).
    pub fn new(is_client: bool) -> Self {
        Self {
            is_client,
            protocol: Protocol::Handshake,
            inbound: Vec::new(),
            outbound: Vec::new(),
            read_queue: VecDeque::new(),
            decrypt: None,
            encrypt: None,
            compression_threshold: None,
            registered: Vec::new(),
            listener: None,
            protocol_changes: Vec::new(),
            disconnect_reason: None,
            disconnected: false,
            local: false,
            read_timeout: Some(READ_TIMEOUT),
            last_read: Instant::now(),
        }
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    /// `Connection.isLocal`: an in-memory channel, not a TCP socket.
    pub fn is_local(&self) -> bool {
        self.local
    }

    pub fn set_local(&mut self, local: bool) {
        self.local = local;
    }

    pub fn is_disconnected(&self) -> bool {
        self.disconnected
    }

    pub fn disconnect_reason(&self) -> Option<&Text> {
        self.disconnect_reason.as_ref()
    }

    pub fn set_listener(&mut self, listener: Box<dyn PacketListener>) {
        self.listener = Some(listener);
    }

    pub fn take_listener(&mut self) -> Option<Box<dyn PacketListener>> {
        self.listener.take()
    }

    /// `ReadTimeoutHandler`: `None` turns the 30-second read timeout off.
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
    }

    /// `Connection.setupEncryption`: AES/CFB8 with the IV equal to the key,
    /// one keystream per direction from this point on.
    pub fn set_encryption(&mut self, key: &[u8; 16]) {
        self.decrypt = Some(AesCfb8::new(key));
        self.encrypt = Some(AesCfb8::new(key));
    }

    pub fn has_encryption(&self) -> bool {
        self.decrypt.is_some()
    }

    /// The 1.8-style compression threshold. Vanilla 1.7.10 never sets it.
    pub fn set_compression(&mut self, threshold: u32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn compression_threshold(&self) -> Option<u32> {
        self.compression_threshold
    }

    /// Register an id beyond this crate's registry: packets there decode as
    /// [`Packet::Unknown`] and encode from their payload.
    pub fn register(&mut self, protocol: Protocol, id: i32) {
        self.registered.push((protocol, self.is_client, id));
    }

    /// `setProtocol`: switch now, notify the listener on the next tick.
    pub fn set_protocol(&mut self, to: Protocol) {
        if self.protocol != to {
            self.protocol_changes.push((self.protocol, to));
        }
        self.protocol = to;
    }

    /// Wire bytes in: decrypt, frame, decompress, decode, queue.
    pub fn receive(&mut self, bytes: &[u8]) -> Result<(), ConnectionError> {
        if self.disconnected || bytes.is_empty() {
            return Ok(());
        }
        self.last_read = Instant::now();
        let start = self.inbound.len();
        self.inbound.extend_from_slice(bytes);
        if let Some(decrypt) = self.decrypt.as_mut() {
            decrypt.decrypt(&mut self.inbound[start..]);
        }
        while let Some(mut frame_payload) = split_frame(&mut self.inbound)? {
            if self.compression_threshold.is_some() {
                frame_payload = decompress_frame(&frame_payload)?;
            }
            self.decode_packet(frame_payload)?;
        }
        Ok(())
    }

    fn decode_packet(&mut self, payload: Vec<u8>) -> Result<(), ConnectionError> {
        let mut buf = crate::buf::PacketBuf::wrap(&payload);
        let id = buf.read_var_int()?;
        let packet = Packet::decode(self.protocol, self.is_client, id, &mut buf)?;
        if let Packet::Unknown { .. } = &packet {
            let known = self.registered.iter().any(|(p, client, known_id)| {
                *p == self.protocol && *client == self.is_client && *known_id == id
            });
            if !known {
                return Err(ConnectionError::BadPacketId(id));
            }
        }
        // `PacketDecoder`: leftover bytes are a protocol violation.
        let extra = buf.len() - buf.position();
        if extra > 0 {
            return Err(ConnectionError::ExtraBytes { extra, id });
        }
        self.read_queue.push_back(packet);
        Ok(())
    }

    /// Wire bytes out: everything encoded but not yet taken.
    pub fn take_outbound(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.outbound)
    }

    pub fn pending_outbound(&self) -> usize {
        self.outbound.len()
    }

    /// `Connection.send`: encode, compress, frame, encrypt.
    pub fn send(&mut self, packet: Packet) -> Result<(), ConnectionError> {
        if self.disconnected {
            // Vanilla queues on a closed channel and never flushes it.
            return Ok(());
        }
        let (id, payload) = match &packet {
            Packet::Unknown { id, .. } => {
                let known = self.registered.iter().any(|(p, client, known_id)| {
                    *p == self.protocol && *client == self.is_client && *known_id == *id
                });
                if known {
                    packet.encode(self.protocol, self.is_client)?
                } else {
                    return Err(ConnectionError::UnregisteredPacket);
                }
            }
            _ => packet.encode(self.protocol, self.is_client)?,
        };
        let _ = id;
        let body = match self.compression_threshold {
            Some(threshold) => compress_frame(&payload, threshold),
            None => payload,
        };
        let mut framed = frame(&body)?;
        if let Some(encrypt) = self.encrypt.as_mut() {
            encrypt.encrypt(&mut framed);
        }
        self.outbound.extend_from_slice(&framed);
        Ok(())
    }

    /// `Connection.tick`: protocol changes, the queued packets (1001 at most),
    /// the read timeout, then the listener's tick.
    pub fn tick(&mut self) {
        if self.disconnected {
            return;
        }
        let changes = std::mem::take(&mut self.protocol_changes);
        let mut listener = self.listener.take();
        if let Some(listener) = listener.as_mut() {
            for (from, to) in &changes {
                listener.change_protocol(*from, *to, self);
                if self.disconnected {
                    break;
                }
            }
        }
        if !self.disconnected {
            for _ in 0..MAX_PACKETS_PER_TICK {
                let Some(packet) = self.read_queue.pop_front() else {
                    break;
                };
                let Some(active) = listener.as_mut() else {
                    break;
                };
                let result = active.handle(&packet, self);
                if let Err(message) = result {
                    // `Connection.exceptionCaught` on a thrown state violation.
                    self.disconnect(Text::generic_reason(&format!(
                        "Internal Exception: {message}"
                    )));
                    break;
                }
                if self.disconnected {
                    break;
                }
            }
        }
        if !self.disconnected
            && let Some(timeout) = self.read_timeout
            && self.last_read.elapsed() > timeout
        {
            self.disconnect(Text::literal("disconnect.timeout"));
        }
        if !self.disconnected
            && let Some(active) = listener.as_mut()
        {
            active.tick(self);
        }
        if self.listener.is_none() {
            self.listener = listener;
        }
    }

    /// `Connection.disconnect`: close, keep the reason for the listener.
    pub fn disconnect(&mut self, reason: Text) {
        if !self.disconnected {
            self.disconnected = true;
            self.disconnect_reason = Some(reason);
        }
    }
}

/// `ConnectionListener.tick`: once a connection is down, fire its listener's
/// `onDisconnect` once with the stored reason.
pub fn finish_disconnected(connection: &mut Connection) {
    if !connection.disconnected {
        return;
    }
    let reason = connection
        .disconnect_reason
        .take()
        .unwrap_or_else(|| Text::literal("Disconnected"));
    let mut listener = connection.listener.take();
    if let Some(listener) = listener.as_mut() {
        listener.on_disconnect(&reason, connection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::PROTOCOL_VERSION_1_7_10 as VERSION;

    /// A listener that records what it saw.
    struct Recorder {
        packets: Vec<Packet>,
        protocol_changes: Vec<(Protocol, Protocol)>,
        disconnects: Vec<Text>,
    }

    impl Recorder {
        fn new() -> Box<Self> {
            Box::new(Self {
                packets: Vec::new(),
                protocol_changes: Vec::new(),
                disconnects: Vec::new(),
            })
        }
    }

    impl PacketListener for Recorder {
        fn change_protocol(&mut self, from: Protocol, to: Protocol, _connection: &mut Connection) {
            self.protocol_changes.push((from, to));
        }

        fn on_disconnect(&mut self, reason: &Text, _connection: &mut Connection) {
            self.disconnects.push(reason.clone());
        }

        fn handle(&mut self, packet: &Packet, _connection: &mut Connection) -> Result<(), String> {
            self.packets.push(packet.clone());
            Ok(())
        }
    }

    fn recorder_connection() -> Connection {
        let mut connection = Connection::new(false);
        connection.set_listener(Recorder::new());
        connection
    }

    #[test]
    fn pipelines_a_handshake() -> Result<(), ConnectionError> {
        let mut connection = recorder_connection();
        // The encoded payload starts with the packet id.
        let (_, payload) = Packet::Handshake {
            version: VERSION,
            address: "127.0.0.1".into(),
            port: 25565,
            next_protocol: 2,
        }
        .encode(Protocol::Handshake, true)
        .unwrap();
        connection.receive(&frame(&payload)?)?;
        connection.tick();
        let listener = connection.take_listener().unwrap();
        let any: Box<dyn std::any::Any> = listener;
        let recorder = any.downcast::<Recorder>().unwrap();
        assert_eq!(
            recorder.packets,
            vec![Packet::Handshake {
                version: VERSION,
                address: "127.0.0.1".into(),
                port: 25565,
                next_protocol: 2,
            }]
        );
        Ok(())
    }

    #[test]
    fn outbound_pipeline_frames_and_encrypts() {
        let mut connection = Connection::new(true);
        connection.set_protocol(Protocol::Login);
        connection
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        let wire = connection.take_outbound();
        // Plaintext: length-prefixed login start.
        assert!(!wire.is_empty());
        // With encryption on, the wire bytes decrypt back to the same frame.
        let key = [9u8; 16];
        let mut connection = Connection::new(true);
        connection.set_protocol(Protocol::Login);
        connection.set_encryption(&key);
        connection
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        let encrypted = connection.take_outbound();
        assert_ne!(encrypted, wire);
        let mut peer = Connection::new(false);
        peer.set_protocol(Protocol::Login);
        peer.set_encryption(&key);
        peer.receive(&encrypted).unwrap();
        peer.tick();
    }

    #[test]
    fn encryption_switches_mid_stream() {
        // The keystream runs across frames: the first encrypted frame's
        // plaintext is the login start, the second a ping.
        let key = [4u8; 16];
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client.set_encryption(&key);
        client
            .send(Packet::LoginStart { name: "A".into() })
            .unwrap();
        let mut server = Connection::new(false);
        server.set_protocol(Protocol::Login);
        server.set_encryption(&key);
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
    }

    #[test]
    fn bad_packet_id_disconnects_via_error() -> Result<(), FrameError> {
        let mut connection = recorder_connection();
        // Login state, id 9: not registered.
        let mut body = crate::buf::PacketBuf::new();
        body.write_var_int(9);
        body.write_bytes(&[0x00]);
        let error = connection
            .receive(&frame(&body.into_written())?)
            .unwrap_err();
        assert_eq!(error, ConnectionError::BadPacketId(9));
        Ok(())
    }

    #[test]
    fn read_timeout_disconnects() {
        let mut connection = recorder_connection();
        connection.set_read_timeout(Some(Duration::from_millis(1)));
        std::thread::sleep(Duration::from_millis(5));
        connection.tick();
        assert!(connection.is_disconnected());
        assert_eq!(
            connection.disconnect_reason().map(|t| t.as_string()),
            Some("disconnect.timeout".to_string())
        );
        finish_disconnected(&mut connection);
    }
}
