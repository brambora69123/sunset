//! Packets, grouped by protocol state the way 1.7.10's
//! `net.minecraft.network.packet` is.

pub mod handshake;
pub mod login;
pub mod play;
pub mod status;

/// A packet, tagged with its kind. Keep-alive and ping are identical on the
/// wire in both directions, so they carry one variant each.
#[derive(Debug, Clone, PartialEq)]
pub enum Packet {
    Handshake(handshake::Handshake),
    StatusRequest(status::StatusRequest),
    StatusResponse(status::StatusResponse),
    Ping(status::Ping),
    LoginStart(login::LoginStart),
    EncryptionRequest(login::EncryptionRequest),
    EncryptionResponse(login::EncryptionResponse),
    LoginSuccess(login::LoginSuccess),
    /// Login-state disconnect, s2c 0x00 (`LoginFailS2CPacket`).
    LoginDisconnect(Disconnect),
    /// Play-state disconnect, s2c 0x40 (`DisconnectS2CPacket`).
    Disconnect(Disconnect),
    KeepAlive(play::KeepAlive),
    Flying(play::Flying),
    PlayerPosition(play::PlayerPosition),
    PlayerLook(play::PlayerLook),
    PlayerPositionLook(play::PlayerPositionLook),
    ClientSettings(play::ClientSettings),
    ClientStatus(play::ClientStatus),
    JoinGame(play::JoinGame),
    ChatMessage(play::ChatMessage),
    WorldTime(play::WorldTime),
    EntityEquipment(play::EntityEquipment),
    SpawnPosition(play::SpawnPosition),
    UpdateHealth(play::UpdateHealth),
    Respawn(play::Respawn),
    PositionAndLook(play::PositionAndLook),
    HeldItemChange(play::HeldItemChange),
    SetSlot(play::SetSlot),
    PlayerAbilities(play::PlayerAbilities),
}

/// The disconnect reason, kept as raw chat JSON; the text parse belongs to
/// the text crate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Disconnect {
    pub reason_json: String,
}

impl Disconnect {
    pub fn read(buf: &mut crate::buf::Buf) -> crate::buf::Result<Self> {
        Ok(Self {
            reason_json: buf.read_string(32767)?,
        })
    }

    pub fn write(&self, buf: &mut crate::buf::Buf) -> crate::buf::Result<()> {
        buf.write_string(&self.reason_json)
    }
}
