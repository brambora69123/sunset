//! The query (status ping) handler and the `ServerStatus` JSON, ported from
//! the decompiled 1.7.10 `ServerQueryNetworkHandler` and `ServerStatus` and
//! its Gson serializers: the field order and null omission are vanilla's.

use crate::connection::{Connection, PacketListener};
use crate::login::GameProfile;
use crate::packets::Packet;
use crate::text::Text;

/// `ServerStatus$Version`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

/// `ServerStatus$Players`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Players {
    pub max: i32,
    pub online: i32,
    /// The sample list, `None` when the server sends no sample.
    pub sample: Option<Vec<GameProfile>>,
}

/// `ServerStatus`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerStatus {
    pub description: Option<Text>,
    pub players: Option<Players>,
    pub version: Option<Version>,
    pub favicon: Option<String>,
}

impl ServerStatus {
    /// The 1.7.10 default: version 1.7.10, protocol 5.
    pub fn vanilla_1_7_10(description: Text, max: i32, online: i32) -> Self {
        Self {
            description: Some(description),
            players: Some(Players {
                max,
                online,
                sample: None,
            }),
            version: Some(Version {
                name: "1.7.10".to_string(),
                protocol: crate::connection::PROTOCOL_VERSION_1_7_10,
            }),
            favicon: None,
        }
    }

    /// Gson field order: description, players, version, favicon; nulls out.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        let mut first = true;
        let mut field = |json: &str| {
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(json);
        };
        if let Some(description) = &self.description {
            field(&format!("\"description\":{}", description.to_json()));
        }
        if let Some(players) = &self.players {
            field(&format!("\"players\":{}", players_json(players)));
        }
        if let Some(version) = &self.version {
            field(&format!(
                "\"version\":{{\"name\":\"{}\",\"protocol\":{}}}",
                json_escape(&version.name),
                version.protocol
            ));
        }
        if let Some(favicon) = &self.favicon {
            field(&format!("\"favicon\":\"{}\"", json_escape(favicon)));
        }
        out.push('}');
        out
    }
}

fn players_json(players: &Players) -> String {
    let mut out = format!("{{\"max\":{},\"online\":{}", players.max, players.online);
    if let Some(sample) = &players.sample {
        let profiles: Vec<String> = sample
            .iter()
            .map(|profile| {
                format!(
                    "{{\"id\":\"{}\",\"name\":\"{}\"}}",
                    profile.id.map(|id| id.to_java_string()).unwrap_or_default(),
                    json_escape(&profile.name)
                )
            })
            .collect();
        out.push_str(&format!(",\"sample\":[{}]", profiles.join(",")));
    }
    out.push('}');
    out
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// `ServerQueryNetworkHandler`: the status request answer and the ping echo.
pub struct ServerQueryHandler {
    pub status: ServerStatus,
}

impl ServerQueryHandler {
    pub fn new(status: ServerStatus) -> Box<Self> {
        Box::new(Self { status })
    }
}

impl PacketListener for ServerQueryHandler {
    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String> {
        match packet {
            Packet::StatusRequest => {
                let _ = connection.send(Packet::StatusResponse {
                    json: self.status.to_json(),
                });
            }
            Packet::Ping { time } => {
                let _ = connection.send(Packet::Ping { time: *time });
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_json_matches_vanilla_shape() {
        let status = ServerStatus::vanilla_1_7_10(Text::literal("A Server"), 20, 0);
        assert_eq!(
            status.to_json(),
            "{\"description\":{\"text\":\"A Server\"},\"players\":{\"max\":20,\"online\":0},\
             \"version\":{\"name\":\"1.7.10\",\"protocol\":5}}"
        );
    }

    #[test]
    fn status_json_omits_nulls_and_orders_fields() {
        let status = ServerStatus {
            version: Some(Version {
                name: "1.7.10".into(),
                protocol: 5,
            }),
            ..ServerStatus::default()
        };
        assert_eq!(
            status.to_json(),
            "{\"version\":{\"name\":\"1.7.10\",\"protocol\":5}}"
        );
    }

    #[test]
    fn players_json_carries_sample() {
        let players = Players {
            max: 100,
            online: 1,
            sample: Some(vec![GameProfile::offline("Notch")]),
        };
        assert_eq!(
            players_json(&players),
            "{\"max\":100,\"online\":1,\"sample\":[{\"id\":\"b50ad385-829d-3141-a216-7e7d7539ba7f\",\"name\":\"Notch\"}]}"
        );
    }

    #[test]
    fn query_echoes_ping() {
        let mut connection = Connection::new(false);
        connection.set_protocol(crate::packets::Protocol::Status);
        let mut listener =
            ServerQueryHandler::new(ServerStatus::vanilla_1_7_10(Text::literal("hi"), 20, 0));
        listener
            .handle(&Packet::Ping { time: 987 }, &mut connection)
            .unwrap();
        // The echo went out as a play of id 1 with the i64 payload.
        let (_, payload) = Packet::Ping { time: 987 }
            .encode(crate::packets::Protocol::Status, false)
            .unwrap();
        let expected: Vec<u8> = [0x01].iter().copied().chain(987i64.to_be_bytes()).collect();
        assert_eq!(payload, expected);
    }
}
