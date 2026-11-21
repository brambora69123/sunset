//! Deterministic probe for later parity checks: encodes a full login and
//! spawn packet sequence, prints the framed bytes as hex, then decodes them
//! back and prints the round-tripped fields. No RNG, no I/O beyond stdout.

use sunset_1_7_protocol::packets::Packet;
use sunset_1_7_protocol::{
    Buf, Direction, ProtocolState, Slot, read_framed_packet, write_framed_packet,
};

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let login_sequence: Vec<(ProtocolState, Direction, Packet)> = vec![
        (
            ProtocolState::Handshake,
            Direction::Serverbound,
            Packet::Handshake(sunset_1_7_protocol::packets::handshake::Handshake {
                protocol_version: 5,
                server_address: "127.0.0.1".into(),
                server_port: 25565,
                next_state: ProtocolState::Login,
            }),
        ),
        (
            ProtocolState::Status,
            Direction::Serverbound,
            Packet::StatusRequest(sunset_1_7_protocol::packets::status::StatusRequest),
        ),
        (
            ProtocolState::Status,
            Direction::Clientbound,
            Packet::StatusResponse(sunset_1_7_protocol::packets::status::StatusResponse {
                json: "{\"version\":{\"name\":\"1.7.10\",\"protocol\":5}}".into(),
            }),
        ),
        (
            ProtocolState::Status,
            Direction::Clientbound,
            Packet::Ping(sunset_1_7_protocol::packets::status::Ping {
                time: 1_234_567_890,
            }),
        ),
        (
            ProtocolState::Login,
            Direction::Serverbound,
            Packet::LoginStart(sunset_1_7_protocol::packets::login::LoginStart {
                name: "Notch".into(),
            }),
        ),
        (
            ProtocolState::Login,
            Direction::Clientbound,
            Packet::LoginSuccess(sunset_1_7_protocol::packets::login::LoginSuccess {
                uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".into(),
                username: "Notch".into(),
            }),
        ),
    ];

    let spawn_sequence: Vec<(ProtocolState, Direction, Packet)> = vec![
        (
            ProtocolState::Play,
            Direction::Clientbound,
            Packet::JoinGame(sunset_1_7_protocol::packets::play::JoinGame {
                entity_id: 1,
                hardcore: false,
                game_mode: sunset_1_7_protocol::packets::play::GameMode::Survival,
                dimension: 0,
                difficulty: Some(sunset_1_7_protocol::packets::play::Difficulty::Normal),
                max_player_count: 8,
                generator_type: sunset_1_7_protocol::packets::play::GeneratorType::Default,
            }),
        ),
        (
            ProtocolState::Play,
            Direction::Clientbound,
            Packet::SpawnPosition(sunset_1_7_protocol::packets::play::SpawnPosition {
                x: 8,
                y: 64,
                z: -8,
            }),
        ),
        (
            ProtocolState::Play,
            Direction::Clientbound,
            Packet::PlayerAbilities(sunset_1_7_protocol::packets::play::PlayerAbilities {
                invulnerable: false,
                flying: false,
                allow_flying: false,
                creative_mode: false,
                fly_speed: 0.05,
                walk_speed: 0.1,
            }),
        ),
        (
            ProtocolState::Play,
            Direction::Clientbound,
            Packet::PositionAndLook(sunset_1_7_protocol::packets::play::PositionAndLook {
                x: 8.5,
                y: 65.62,
                z: -8.5,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: false,
            }),
        ),
        (
            ProtocolState::Play,
            Direction::Clientbound,
            Packet::SetSlot(sunset_1_7_protocol::packets::play::SetSlot {
                window_id: 0,
                slot: 36,
                item: Some(Slot::new(278, 1, 0)),
            }),
        ),
        (
            ProtocolState::Play,
            Direction::Serverbound,
            Packet::PlayerPositionLook(sunset_1_7_protocol::packets::play::PlayerPositionLook {
                x: 8.5,
                min_y: 66.24,
                y: 65.62,
                z: -8.5,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: true,
            }),
        ),
    ];

    for (state, direction, packet) in login_sequence.iter().chain(&spawn_sequence) {
        let mut buf = Buf::new();
        write_framed_packet(packet, &mut buf).unwrap();
        println!("{state:?} {direction:?}: {}", hex(buf.as_slice()));
        let decoded = read_framed_packet(*state, *direction, &mut Buf::from(buf.into_vec()))
            .unwrap()
            .unwrap();
        assert_eq!(&decoded, packet);
        println!("  round trip ok: {decoded:?}");
    }
}
