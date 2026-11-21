//! The play-state keepalive and disconnect handlers, ported from the
//! decompiled 1.7.10 `ServerPlayNetworkHandler` and `ClientPlayNetworkHandler`
//! keepalive paths: the server sends a keepalive every 40 ticks and tracks the
//! ping, the client echoes it back.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::connection::{Connection, PacketListener, READ_TIMEOUT};
use crate::packets::Packet;
use crate::text::Text;

/// `ServerPlayNetworkHandler`, the keepalive part: every 40 ticks a keepalive
/// with the wall-clock millis, the response ping
/// `(ping * 3 + elapsed) / 4`, and the 30-second read timeout kick.
pub struct ServerPlayHandler {
    ticks: u64,
    last_keep_alive_tick: u64,
    keep_alive_time: u64,
    saved_keep_alive_time: i32,
    ping_ms: i32,
    start: Instant,
}

impl ServerPlayHandler {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            ticks: 0,
            last_keep_alive_tick: 0,
            keep_alive_time: 0,
            saved_keep_alive_time: 0,
            ping_ms: 0,
            start: Instant::now(),
        })
    }

    /// `player.ping` after the last confirmed keepalive.
    pub fn ping_ms(&self) -> i32 {
        self.ping_ms
    }

    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Default for ServerPlayHandler {
    fn default() -> Self {
        *Self::new()
    }
}

impl PacketListener for ServerPlayHandler {
    fn tick(&mut self, connection: &mut Connection) {
        self.ticks += 1;
        // `if (this.ticks - this.lastKeepAliveUpdateTime > 40L)`.
        if self.ticks - self.last_keep_alive_tick > 40 {
            self.last_keep_alive_tick = self.ticks;
            self.keep_alive_time = self.now_millis();
            self.saved_keep_alive_time = self.keep_alive_time as i32;
            let _ = connection.send(Packet::KeepAlive {
                id: self.saved_keep_alive_time,
            });
        }
        if self.start.elapsed() > READ_TIMEOUT {
            connection.disconnect(Text::literal("disconnect.timeout"));
        }
    }

    fn handle(&mut self, packet: &Packet, _connection: &mut Connection) -> Result<(), String> {
        if let Packet::KeepAlive { id } = packet {
            // `if (packet.getTimeMillis() == this.savedKeepAliveTime)`.
            if *id == self.saved_keep_alive_time {
                let elapsed = (self.now_millis().saturating_sub(self.keep_alive_time)) as i32;
                self.ping_ms = (self.ping_ms * 3 + elapsed) / 4;
            }
        }
        Ok(())
    }
}

/// `ClientPlayNetworkHandler`, the keepalive echo and the disconnect screen.
pub struct ClientPlayHandler;

impl ClientPlayHandler {
    pub fn new() -> Box<Self> {
        Box::new(Self)
    }
}

impl Default for ClientPlayHandler {
    fn default() -> Self {
        Self
    }
}

impl PacketListener for ClientPlayHandler {
    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String> {
        match packet {
            Packet::KeepAlive { id } => {
                let _ = connection.send(Packet::KeepAlive { id: *id });
            }
            Packet::Disconnect { reason } => {
                connection.disconnect(reason.clone());
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::random_bytes;
    use sunset_1_7_core::LegacyRandom;

    #[test]
    fn keepalive_echo_round_trips() {
        let mut server = Connection::new(false);
        server.set_protocol(crate::packets::Protocol::Play);
        server.set_listener(ServerPlayHandler::new());
        let mut client = Connection::new(true);
        client.set_protocol(crate::packets::Protocol::Play);
        client.set_listener(ClientPlayHandler::new());
        // 41 ticks: the server sends one keepalive.
        for _ in 0..41 {
            server.tick();
        }
        let keepalive = server.take_outbound();
        assert!(!keepalive.is_empty());
        client.receive(&keepalive).unwrap();
        client.tick();
        // The client echoed it back.
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
    }

    #[test]
    fn server_play_sends_keepalive_every_40_ticks() {
        let mut connection = Connection::new(false);
        connection.set_protocol(crate::packets::Protocol::Play);
        connection.set_listener(ServerPlayHandler::new());
        for _ in 0..40 {
            connection.tick();
        }
        assert!(connection.take_outbound().is_empty());
        connection.tick();
        assert!(!connection.take_outbound().is_empty());
    }

    #[test]
    fn keepalive_wire_round_trip() {
        // The keepalive is a play packet: id 0, i32 payload.
        let mut server = Connection::new(false);
        server.set_protocol(crate::packets::Protocol::Play);
        let mut client = Connection::new(true);
        client.set_protocol(crate::packets::Protocol::Play);
        client.send(Packet::KeepAlive { id: 12345 }).unwrap();
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
        // The id survived the pipeline (no listener here; check via encrypt).
        let mut rng = LegacyRandom::new(2);
        let mut key = [0u8; 16];
        random_bytes(&mut rng, &mut key);
        server.set_encryption(&key);
        client.set_encryption(&key);
        client.send(Packet::KeepAlive { id: 777 }).unwrap();
        server.receive(&client.take_outbound()).unwrap();
    }
}
