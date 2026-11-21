//! Minecraft 1.7.10 net: the connection layer, ported for sunset.
//!
//! Ported from the decompiled 1.7.10 `net.minecraft.network` package:
//! [`buf`] is `PacketByteBuf`, [`framing`] is `SplitterHandler`/`SizePrepender`,
//! [`compression`] is the 1.8-style zlib layer with a threshold (vanilla 1.7.10
//! never enables it; it is here for the connection-layer contract), [`crypto`]
//! is `EncryptionUtils` (AES/CFB8 stream, SHA-1 server-id digest, RSA-1024),
//! [`packets`] is the packet registry for the connection-flow packets,
//! [`connection`] is `Connection` (pipeline, protocol state, tick, disconnect),
//! [`login`] is the server handshake + login state machine, [`client`] the
//! client-side login, [`play`] the keepalive handlers, [`status`] the query
//! handler, and [`uuid`] the Java `UUID` semantics (offline-player UUIDs).
//!
//! The pipeline mirrors netty's handler order:
//!
//! - inbound: decrypt → frame split → decompress → packet decode
//! - outbound: packet encode → compress → frame (length prefix) → encrypt
//!
//! so with encryption on, the length prefix is part of the encrypted stream and
//! one CFB8 keystream runs across all frames, exactly like vanilla.

pub mod buf;
pub mod client;
pub mod compression;
pub mod connection;
pub mod crypto;
pub mod framing;
pub mod login;
pub mod packets;
pub mod play;
pub mod status;
pub mod text;
pub mod uuid;

pub use buf::{BufError, PacketBuf};
pub use client::ClientLoginHandler;
pub use compression::{compress_frame, decompress_frame};
pub use connection::{Connection, ConnectionError, PROTOCOL_VERSION_1_7_10, PacketListener};
pub use crypto::{
    AesCfb8, CryptoError, RsaKeyPair, RsaPublicKey, java_hex_bigint, parse_public_key_der,
    random_bytes, server_id_hash,
};
pub use framing::{FrameError, frame, split_frame};
pub use login::{
    HandshakeHandler, LoginStage, NoSessionService, ServerLoginHandler, SessionService,
};
pub use packets::{Packet, Protocol};
pub use play::{ClientPlayHandler, ServerPlayHandler};
pub use status::{Players, ServerQueryHandler, ServerStatus, Version};
pub use text::Text;
pub use uuid::Uuid;
