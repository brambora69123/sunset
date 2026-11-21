//! The packet table: 1.7.10's `NetworkProtocol` — one state, one direction,
//! one id per packet.

use crate::buf::{Buf, Result};
use crate::packets::{Disconnect, Packet, handshake, login, play, status};

/// `NetworkProtocol`: handshake (-1), play (0), status (1), login (2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolState {
    Handshake = -1,
    #[default]
    Play = 0,
    Status = 1,
    Login = 2,
}

impl ProtocolState {
    pub fn id(self) -> i32 {
        self as i32
    }

    pub fn by_id(id: i32) -> Option<Self> {
        match id {
            -1 => Some(Self::Handshake),
            0 => Some(Self::Play),
            1 => Some(Self::Status),
            2 => Some(Self::Login),
            _ => None,
        }
    }
}

/// Which way a packet travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Clientbound,
    Serverbound,
}

/// The protocol version 1.7.10 sends in the handshake and the server list.
pub const PROTOCOL_VERSION: i32 = 5;
/// 1.7.10's version string, as `ServerStatus$Version` carries it.
pub const VERSION_STRING: &str = "1.7.10";

/// Reads one packet body off `buf`, after the packet id: `PacketDecoder`
/// without the logging. Unknown ids are `None`, as 1.7.10's
/// `Packet.create` null is.
pub fn decode_packet(
    state: ProtocolState,
    direction: Direction,
    id: i32,
    buf: &mut Buf,
) -> Result<Option<Packet>> {
    let packet = match (state, direction, id) {
        (ProtocolState::Handshake, Direction::Serverbound, handshake::ID) => {
            Packet::Handshake(handshake::Handshake::read(buf)?)
        }

        (ProtocolState::Status, Direction::Serverbound, status::STATUS_REQUEST_ID) => {
            Packet::StatusRequest(status::StatusRequest::read(buf)?)
        }
        (ProtocolState::Status, Direction::Serverbound, status::PING_ID) => {
            Packet::Ping(status::Ping::read(buf)?)
        }
        (ProtocolState::Status, Direction::Clientbound, status::STATUS_RESPONSE_ID) => {
            Packet::StatusResponse(status::StatusResponse::read(buf)?)
        }
        (ProtocolState::Status, Direction::Clientbound, status::PING_ID) => {
            Packet::Ping(status::Ping::read(buf)?)
        }

        (ProtocolState::Login, Direction::Serverbound, login::LOGIN_START_ID) => {
            Packet::LoginStart(login::LoginStart::read(buf)?)
        }
        (ProtocolState::Login, Direction::Serverbound, login::ENCRYPTION_RESPONSE_ID) => {
            Packet::EncryptionResponse(login::EncryptionResponse::read(buf)?)
        }
        (ProtocolState::Login, Direction::Clientbound, 0x00) => {
            Packet::LoginDisconnect(Disconnect::read(buf)?)
        }
        (ProtocolState::Login, Direction::Clientbound, login::ENCRYPTION_REQUEST_ID) => {
            Packet::EncryptionRequest(login::EncryptionRequest::read(buf)?)
        }
        (ProtocolState::Login, Direction::Clientbound, login::LOGIN_SUCCESS_ID) => {
            Packet::LoginSuccess(login::LoginSuccess::read(buf)?)
        }

        (ProtocolState::Play, Direction::Serverbound, play::KEEP_ALIVE_ID) => {
            Packet::KeepAlive(play::KeepAlive::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::FLYING_ID) => {
            Packet::Flying(play::Flying::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::PLAYER_POSITION_ID) => {
            Packet::PlayerPosition(play::PlayerPosition::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::PLAYER_LOOK_ID) => {
            Packet::PlayerLook(play::PlayerLook::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::PLAYER_POSITION_LOOK_ID) => {
            Packet::PlayerPositionLook(play::PlayerPositionLook::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::CLIENT_SETTINGS_ID) => {
            Packet::ClientSettings(play::ClientSettings::read(buf)?)
        }
        (ProtocolState::Play, Direction::Serverbound, play::CLIENT_STATUS_ID) => {
            Packet::ClientStatus(play::ClientStatus::read(buf)?)
        }

        (ProtocolState::Play, Direction::Clientbound, play::KEEP_ALIVE_ID) => {
            Packet::KeepAlive(play::KeepAlive::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::JOIN_GAME_ID) => {
            Packet::JoinGame(play::JoinGame::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::CHAT_MESSAGE_ID) => {
            Packet::ChatMessage(play::ChatMessage::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::WORLD_TIME_ID) => {
            Packet::WorldTime(play::WorldTime::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::ENTITY_EQUIPMENT_ID) => {
            Packet::EntityEquipment(play::EntityEquipment::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::SPAWN_POSITION_ID) => {
            Packet::SpawnPosition(play::SpawnPosition::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::UPDATE_HEALTH_ID) => {
            Packet::UpdateHealth(play::UpdateHealth::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::RESPAWN_ID) => {
            Packet::Respawn(play::Respawn::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::POSITION_AND_LOOK_ID) => {
            Packet::PositionAndLook(play::PositionAndLook::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::HELD_ITEM_CHANGE_ID) => {
            Packet::HeldItemChange(play::HeldItemChange::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::SET_SLOT_ID) => {
            Packet::SetSlot(play::SetSlot::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::PLAYER_ABILITIES_ID) => {
            Packet::PlayerAbilities(play::PlayerAbilities::read(buf)?)
        }
        (ProtocolState::Play, Direction::Clientbound, play::DISCONNECT_ID) => {
            Packet::Disconnect(Disconnect::read(buf)?)
        }

        _ => return Ok(None),
    };
    Ok(Some(packet))
}

/// Writes one packet: the id, then the body, exactly as `PacketEncoder` does.
/// Keep-alive and ping carry one id per direction pair (both 0x00 / 0x01), so
/// one write covers both ways.
pub fn encode_packet(packet: &Packet, buf: &mut Buf) -> Result<()> {
    match packet {
        Packet::Handshake(packet) => {
            buf.write_var_int(handshake::ID);
            packet.write(buf)
        }
        Packet::StatusRequest(packet) => {
            buf.write_var_int(status::STATUS_REQUEST_ID);
            packet.write(buf)
        }
        Packet::StatusResponse(packet) => {
            buf.write_var_int(status::STATUS_RESPONSE_ID);
            packet.write(buf)
        }
        Packet::Ping(packet) => {
            buf.write_var_int(status::PING_ID);
            packet.write(buf)
        }
        Packet::LoginStart(packet) => {
            buf.write_var_int(login::LOGIN_START_ID);
            packet.write(buf)
        }
        Packet::EncryptionRequest(packet) => {
            buf.write_var_int(login::ENCRYPTION_REQUEST_ID);
            packet.write(buf)
        }
        Packet::EncryptionResponse(packet) => {
            buf.write_var_int(login::ENCRYPTION_RESPONSE_ID);
            packet.write(buf)
        }
        Packet::LoginSuccess(packet) => {
            buf.write_var_int(login::LOGIN_SUCCESS_ID);
            packet.write(buf)
        }
        Packet::LoginDisconnect(packet) => {
            buf.write_var_int(0x00);
            packet.write(buf)
        }
        Packet::Disconnect(packet) => {
            buf.write_var_int(play::DISCONNECT_ID);
            packet.write(buf)
        }
        Packet::KeepAlive(packet) => {
            buf.write_var_int(play::KEEP_ALIVE_ID);
            packet.write(buf)
        }
        Packet::Flying(packet) => {
            buf.write_var_int(play::FLYING_ID);
            packet.write(buf)
        }
        Packet::PlayerPosition(packet) => {
            buf.write_var_int(play::PLAYER_POSITION_ID);
            packet.write(buf)
        }
        Packet::PlayerLook(packet) => {
            buf.write_var_int(play::PLAYER_LOOK_ID);
            packet.write(buf)
        }
        Packet::PlayerPositionLook(packet) => {
            buf.write_var_int(play::PLAYER_POSITION_LOOK_ID);
            packet.write(buf)
        }
        Packet::ClientSettings(packet) => {
            buf.write_var_int(play::CLIENT_SETTINGS_ID);
            packet.write(buf)
        }
        Packet::ClientStatus(packet) => {
            buf.write_var_int(play::CLIENT_STATUS_ID);
            packet.write(buf)
        }
        Packet::JoinGame(packet) => {
            buf.write_var_int(play::JOIN_GAME_ID);
            packet.write(buf)
        }
        Packet::ChatMessage(packet) => {
            buf.write_var_int(play::CHAT_MESSAGE_ID);
            packet.write(buf)
        }
        Packet::WorldTime(packet) => {
            buf.write_var_int(play::WORLD_TIME_ID);
            packet.write(buf)
        }
        Packet::EntityEquipment(packet) => {
            buf.write_var_int(play::ENTITY_EQUIPMENT_ID);
            packet.write(buf)
        }
        Packet::SpawnPosition(packet) => {
            buf.write_var_int(play::SPAWN_POSITION_ID);
            packet.write(buf)
        }
        Packet::UpdateHealth(packet) => {
            buf.write_var_int(play::UPDATE_HEALTH_ID);
            packet.write(buf)
        }
        Packet::Respawn(packet) => {
            buf.write_var_int(play::RESPAWN_ID);
            packet.write(buf)
        }
        Packet::PositionAndLook(packet) => {
            buf.write_var_int(play::POSITION_AND_LOOK_ID);
            packet.write(buf)
        }
        Packet::HeldItemChange(packet) => {
            buf.write_var_int(play::HELD_ITEM_CHANGE_ID);
            packet.write(buf)
        }
        Packet::SetSlot(packet) => {
            buf.write_var_int(play::SET_SLOT_ID);
            packet.write(buf)
        }
        Packet::PlayerAbilities(packet) => {
            buf.write_var_int(play::PLAYER_ABILITIES_ID);
            packet.write(buf)
        }
    }
}

/// The full receive pipeline, `SplitterHandler` + `PacketDecoder`: splits one
/// framed packet, reads the id, decodes the body and refuses bytes left over.
/// `Ok(None)` when more bytes are needed for a full frame.
pub fn read_framed_packet(
    state: ProtocolState,
    direction: Direction,
    buf: &mut Buf,
) -> Result<Option<Packet>> {
    let Some(frame) = crate::framing::unframe(buf)? else {
        return Ok(None);
    };
    let mut body = Buf::from(frame);
    let id = body.read_var_int()?;
    let Some(packet) = decode_packet(state, direction, id, &mut body)? else {
        return Err(crate::buf::Error::UnknownPacketId {
            state: state.id(),
            id,
        });
    };
    if body.remaining() > 0 {
        return Err(crate::buf::Error::PacketTooLarge {
            extra: body.remaining(),
        });
    }
    Ok(Some(packet))
}

/// Frames and writes one packet, `PacketEncoder` + `SizePrepender`.
pub fn write_framed_packet(packet: &Packet, buf: &mut Buf) -> Result<()> {
    let mut body = Buf::new();
    encode_packet(packet, &mut body)?;
    buf.as_mut_vec()
        .extend_from_slice(&crate::framing::frame(body.as_slice())?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::Error;
    use crate::framing::frame;
    use crate::slot::Slot;

    #[test]
    fn state_ids_match_vanilla() {
        assert_eq!(ProtocolState::Handshake.id(), -1);
        assert_eq!(ProtocolState::Play.id(), 0);
        assert_eq!(ProtocolState::Status.id(), 1);
        assert_eq!(ProtocolState::Login.id(), 2);
        assert_eq!(ProtocolState::by_id(3), None);
    }

    #[test]
    fn handshake_known_bytes() {
        // A vanilla 1.7.10 client joining a LAN address: version 5,
        // "127.0.0.1", port 25565, login next.
        let handshake = handshake::Handshake {
            protocol_version: 5,
            server_address: "127.0.0.1".into(),
            server_port: 25565,
            next_state: ProtocolState::Login,
        };
        let mut buf = Buf::new();
        encode_packet(&Packet::Handshake(handshake.clone()), &mut buf).unwrap();
        assert_eq!(
            buf.as_slice(),
            &[
                0x00, // packet id 0
                0x05, // protocol version 5
                0x09, b'1', b'2', b'7', b'.', b'0', b'.', b'0', b'.', b'1', 0x63,
                0xdd, // port 25565
                0x02, // login next
            ][..]
        );
        let decoded = decode_packet(
            ProtocolState::Handshake,
            Direction::Serverbound,
            0x00,
            &mut Buf::from(buf.as_slice()[1..].to_vec()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(decoded, Packet::Handshake(handshake));
    }

    #[test]
    fn handshake_unknown_next_state_is_error() {
        // Version 5, address "x", port 1, next state 3: outside -1..2.
        let mut buf = Buf::from(vec![0x05, 0x01, b'x', 0x00, 0x01, 0x03]);
        assert!(matches!(
            decode_packet(
                ProtocolState::Handshake,
                Direction::Serverbound,
                0x00,
                &mut buf
            ),
            Err(Error::UnknownProtocolState { id: 3 })
        ));
    }

    #[test]
    fn login_sequence_round_trip() {
        // The offline-mode login: start, success. Encryption packets round
        // trip the raw bytes.
        for (packet, state, direction) in [
            (
                Packet::LoginStart(login::LoginStart {
                    name: "Notch".into(),
                }),
                ProtocolState::Login,
                Direction::Serverbound,
            ),
            (
                Packet::EncryptionRequest(login::EncryptionRequest {
                    server_id: String::new(),
                    public_key: vec![0x30; 162],
                    verify_token: vec![1, 2, 3, 4],
                }),
                ProtocolState::Login,
                Direction::Clientbound,
            ),
            (
                Packet::EncryptionResponse(login::EncryptionResponse {
                    shared_secret: vec![0xaa; 128],
                    verify_token: vec![1, 2, 3, 4],
                }),
                ProtocolState::Login,
                Direction::Serverbound,
            ),
            (
                Packet::LoginSuccess(login::LoginSuccess {
                    uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
                    username: "Notch".into(),
                }),
                ProtocolState::Login,
                Direction::Clientbound,
            ),
        ] {
            let mut buf = Buf::new();
            write_framed_packet(&packet, &mut buf).unwrap();
            let decoded = read_framed_packet(state, direction, &mut buf)
                .unwrap()
                .unwrap();
            assert_eq!(decoded, packet);
            assert_eq!(buf.remaining(), 0);
        }
    }

    #[test]
    fn login_disconnect_uses_id_0() {
        let packet = Packet::LoginDisconnect(Disconnect {
            reason_json: "{\"text\":\"You are banned\"}".into(),
        });
        let mut buf = Buf::new();
        write_framed_packet(&packet, &mut buf).unwrap();
        // Frame prefix, then packet id 0.
        assert_eq!(buf.as_slice()[1], 0x00);
        let decoded = read_framed_packet(ProtocolState::Login, Direction::Clientbound, &mut buf)
            .unwrap()
            .unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn spawn_sequence_round_trip() {
        // Join Game, spawn position, position-and-look, abilities, time,
        // health, held item: what a client needs to spawn.
        for packet in [
            Packet::JoinGame(play::JoinGame {
                entity_id: 1,
                hardcore: false,
                game_mode: play::GameMode::Survival,
                dimension: 0,
                difficulty: Some(play::Difficulty::Normal),
                max_player_count: 8,
                generator_type: play::GeneratorType::Default,
            }),
            Packet::SpawnPosition(play::SpawnPosition { x: 8, y: 64, z: -8 }),
            Packet::PositionAndLook(play::PositionAndLook {
                x: 8.5,
                y: 65.62,
                z: -8.5,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: false,
            }),
            Packet::PlayerAbilities(play::PlayerAbilities {
                invulnerable: false,
                flying: false,
                allow_flying: false,
                creative_mode: false,
                fly_speed: 0.05,
                walk_speed: 0.1,
            }),
            Packet::WorldTime(play::WorldTime {
                time: 1234,
                time_of_day: 6000,
            }),
            Packet::UpdateHealth(play::UpdateHealth {
                health: 20.0,
                hunger: 20,
                saturation: 5.0,
            }),
            Packet::HeldItemChange(play::HeldItemChange { slot: 0 }),
            Packet::SetSlot(play::SetSlot {
                window_id: 0,
                slot: 36,
                item: Some(Slot::new(278, 1, 0)),
            }),
            Packet::EntityEquipment(play::EntityEquipment {
                entity_id: 1,
                slot: 0,
                item: None,
            }),
            Packet::ChatMessage(play::ChatMessage {
                json: "{\"text\":\"hi\"}".into(),
            }),
            Packet::KeepAlive(play::KeepAlive { time_millis: 4242 }),
            Packet::Flying(play::Flying { on_ground: true }),
            Packet::PlayerPosition(play::PlayerPosition {
                x: 8.5,
                min_y: 66.24,
                y: 65.62,
                z: -8.5,
                on_ground: true,
            }),
            Packet::PlayerLook(play::PlayerLook {
                yaw: 90.0,
                pitch: 45.0,
                on_ground: false,
            }),
            Packet::PlayerPositionLook(play::PlayerPositionLook {
                x: 8.5,
                min_y: 66.24,
                y: 65.62,
                z: -8.5,
                yaw: 90.0,
                pitch: 45.0,
                on_ground: true,
            }),
            Packet::ClientSettings(play::ClientSettings {
                language: "en_US".into(),
                view_distance: 8,
                chat_visibility: Some(play::ChatVisibility::Full),
                chat_colors: true,
                difficulty: Some(play::Difficulty::Normal),
                show_cape: true,
            }),
            Packet::ClientStatus(play::ClientStatus { action: 0 }),
            Packet::Disconnect(Disconnect {
                reason_json: "{\"text\":\"bye\"}".into(),
            }),
        ] {
            let state = ProtocolState::Play;
            let direction = match packet {
                Packet::JoinGame(_)
                | Packet::SpawnPosition(_)
                | Packet::PositionAndLook(_)
                | Packet::PlayerAbilities(_)
                | Packet::WorldTime(_)
                | Packet::UpdateHealth(_)
                | Packet::HeldItemChange(_)
                | Packet::SetSlot(_)
                | Packet::EntityEquipment(_)
                | Packet::ChatMessage(_)
                | Packet::Disconnect(_) => Direction::Clientbound,
                _ => Direction::Serverbound,
            };
            let mut buf = Buf::new();
            write_framed_packet(&packet, &mut buf).unwrap();
            let decoded = read_framed_packet(state, direction, &mut buf)
                .unwrap()
                .unwrap();
            assert_eq!(decoded, packet);
        }
    }

    #[test]
    fn keep_alive_and_ping_decode_both_ways() {
        for (state, direction) in [
            (ProtocolState::Play, Direction::Serverbound),
            (ProtocolState::Play, Direction::Clientbound),
            (ProtocolState::Status, Direction::Serverbound),
            (ProtocolState::Status, Direction::Clientbound),
        ] {
            let packet = match state {
                ProtocolState::Play => Packet::KeepAlive(play::KeepAlive { time_millis: 7 }),
                _ => Packet::Ping(status::Ping { time: 7 }),
            };
            let mut buf = Buf::new();
            write_framed_packet(&packet, &mut buf).unwrap();
            assert_eq!(
                read_framed_packet(state, direction, &mut buf)
                    .unwrap()
                    .unwrap(),
                packet
            );
        }
    }

    #[test]
    fn unknown_id_and_leftover_bytes_are_errors() {
        let mut buf = Buf::from(frame(&[0x7f]).unwrap());
        assert!(matches!(
            read_framed_packet(ProtocolState::Play, Direction::Clientbound, &mut buf),
            Err(Error::UnknownPacketId { id: 127, .. })
        ));

        // A keep-alive body with one byte too many.
        let mut buf = Buf::from(frame(&[0x00, 0, 0, 0, 1, 0xff]).unwrap());
        assert!(matches!(
            read_framed_packet(ProtocolState::Play, Direction::Clientbound, &mut buf),
            Err(Error::PacketTooLarge { extra: 1 })
        ));
    }

    #[test]
    fn protocol_version_is_5() {
        assert_eq!(PROTOCOL_VERSION, 5);
        assert_eq!(VERSION_STRING, "1.7.10");
    }
}
