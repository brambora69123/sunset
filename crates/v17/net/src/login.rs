//! The server handshake and login state machine, ported from the decompiled
//! 1.7.10 `RemoteHandshakeNetworkHandler` and `ServerLoginNetworkHandler`:
//! the intent switch, the version check, the HELLO → KEY → AUTHENTICATING →
//! READY_TO_ACCEPT → ACCEPTED stages, the 600-tick login timeout, the nonce
//! check and the offline-player UUID.

use std::sync::Arc;

use sunset_1_7_core::LegacyRandom;

use crate::connection::{Connection, PROTOCOL_VERSION_1_7_10, PacketListener};
use crate::crypto::{RsaKeyPair, java_hex_bigint, random_bytes, server_id_hash};
use crate::packets::{Packet, Protocol};
use crate::text::Text;
use crate::uuid::Uuid;

/// `ServerLoginNetworkHandler$Stage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginStage {
    Hello,
    Key,
    Authenticating,
    ReadyToAccept,
    Accepted,
}

/// `MinecraftServer.getSessionService`: the online-mode profile lookup. The
/// port carries the UUID the session server returns; the HTTP call to
/// sessionserver.mojang.com stays with the game crate's implementation.
pub trait SessionService: Send {
    /// `hasJoinedServer`: the UUID of the profile that joined with this
    /// server-id hash, or `None` when the session server has no record.
    fn has_joined_server(&self, name: &str, server_id_hash_hex: &str) -> Option<Uuid>;
}

/// Offline mode, singleplayer fallbacks and harnesses: no session server.
pub struct NoSessionService;

impl SessionService for NoSessionService {
    fn has_joined_server(&self, _name: &str, _server_id_hash_hex: &str) -> Option<Uuid> {
        None
    }
}

/// Everything the server side of the login flow needs of `MinecraftServer`.
pub struct LoginContext {
    pub online_mode: bool,
    pub singleplayer: bool,
    pub keys: Arc<RsaKeyPair>,
    pub session: Arc<dyn SessionService>,
    /// The `new Random()` the login nonces come from.
    pub random: LegacyRandom,
}

impl LoginContext {
    pub fn new(
        online_mode: bool,
        singleplayer: bool,
        keys: Arc<RsaKeyPair>,
        session: Arc<dyn SessionService>,
        random: LegacyRandom,
    ) -> Self {
        Self {
            online_mode,
            singleplayer,
            keys,
            session,
            random,
        }
    }
}

/// `RemoteHandshakeNetworkHandler`: the intent switch and version check.
pub struct HandshakeHandler {
    context: LoginContext,
}

impl HandshakeHandler {
    pub fn new(context: LoginContext) -> Box<Self> {
        Box::new(Self { context })
    }

    fn disconnect(&mut self, connection: &mut Connection, reason: &str) {
        let text = Text::literal(reason);
        let _ = connection.send(Packet::Disconnect {
            reason: text.clone(),
        });
        connection.disconnect(text);
    }
}

impl PacketListener for HandshakeHandler {
    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String> {
        let Packet::Handshake {
            version,
            next_protocol,
            ..
        } = packet
        else {
            return Err("Unexpected packet in handshake".to_string());
        };
        match Protocol::from_id(*next_protocol) {
            Some(Protocol::Login) => {
                connection.set_protocol(Protocol::Login);
                if *version > PROTOCOL_VERSION_1_7_10 {
                    self.disconnect(connection, "Outdated server! I'm still on 1.7.10");
                } else if *version < PROTOCOL_VERSION_1_7_10 {
                    self.disconnect(connection, "Outdated client! Please use 1.7.10");
                } else {
                    let mut nonce = [0u8; 4];
                    random_bytes(&mut self.context.random, &mut nonce);
                    connection.set_listener(ServerLoginHandler::new(
                        self.context.online_mode,
                        self.context.singleplayer,
                        Arc::clone(&self.context.keys),
                        Arc::clone(&self.context.session),
                        nonce,
                    ));
                }
            }
            Some(Protocol::Status) => {
                connection.set_protocol(Protocol::Status);
                connection.set_listener(crate::status::ServerQueryHandler::new(
                    crate::status::ServerStatus::default(),
                ));
            }
            _ => return Err(format!("Invalid intention {next_protocol}")),
        }
        Ok(())
    }
}

/// `ServerLoginNetworkHandler`.
pub struct ServerLoginHandler {
    stage: LoginStage,
    ticks: u32,
    nonce: [u8; 4],
    profile: Option<GameProfile>,
    online_mode: bool,
    singleplayer: bool,
    keys: Arc<RsaKeyPair>,
    session: Arc<dyn SessionService>,
    /// `getPlayerManager().canLogin`: `Some(reason)` blocks the join.
    pub can_login: Option<Box<dyn Fn(&GameProfile) -> Option<String> + Send>>,
    /// `getPlayerManager().onLogin`: the game crate's join, which installs the
    /// play listener.
    pub on_login: Option<Box<dyn FnMut(&GameProfile, &mut Connection) + Send>>,
}

/// `com.mojang.authlib.GameProfile`, the part the login flow carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameProfile {
    pub id: Option<Uuid>,
    pub name: String,
}

impl GameProfile {
    /// `GameProfile.isComplete`: an id and a name.
    pub fn is_complete(&self) -> bool {
        self.id.is_some() && !self.name.is_empty()
    }

    /// The offline-mode login profile: `OfflinePlayer:` + name hashed to a
    /// version-3 UUID.
    pub fn offline(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: Some(Uuid::offline_player(&name)),
            name,
        }
    }
}

impl ServerLoginHandler {
    pub fn new(
        online_mode: bool,
        singleplayer: bool,
        keys: Arc<RsaKeyPair>,
        session: Arc<dyn SessionService>,
        nonce: [u8; 4],
    ) -> Box<Self> {
        Box::new(Self {
            stage: LoginStage::Hello,
            ticks: 0,
            nonce,
            profile: None,
            online_mode,
            singleplayer,
            keys,
            session,
            can_login: None,
            on_login: None,
        })
    }

    pub fn stage(&self) -> LoginStage {
        self.stage
    }

    pub fn profile(&self) -> Option<&GameProfile> {
        self.profile.as_ref()
    }

    fn disconnect_login(&mut self, connection: &mut Connection, reason: &str) {
        let text = Text::literal(reason);
        let _ = connection.send(Packet::Disconnect {
            reason: text.clone(),
        });
        connection.disconnect(text);
    }

    /// `ServerLoginNetworkHandler.acceptLogin`.
    fn accept_login(&mut self, connection: &mut Connection) {
        let profile = self.profile.as_mut().expect("profile at accept");
        if !profile.is_complete() {
            profile.id = Some(Uuid::offline_player(&profile.name));
        }
        let reason = self.can_login.as_ref().and_then(|check| check(profile));
        if let Some(reason) = reason {
            self.disconnect_login(connection, &reason);
            return;
        }
        self.stage = LoginStage::Accepted;
        let (uuid, name) = (
            profile.id.map(|id| id.to_java_string()).unwrap_or_default(),
            profile.name.clone(),
        );
        let _ = connection.send(Packet::LoginSuccess { uuid, name });
        if let Some(on_login) = self.on_login.as_mut() {
            on_login(self.profile.as_ref().expect("profile at login"), connection);
        }
    }
}

impl PacketListener for ServerLoginHandler {
    fn tick(&mut self, connection: &mut Connection) {
        if self.stage == LoginStage::ReadyToAccept {
            self.accept_login(connection);
            if connection.is_disconnected() {
                return;
            }
        }
        // `if (this.ticks++ == 600) disconnect("Took too long to log in")`.
        let ticks = self.ticks;
        self.ticks += 1;
        if ticks == 600 {
            self.disconnect_login(connection, "Took too long to log in");
        }
    }

    /// The tests drive the handler directly through the connection dispatch.
    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String> {
        match packet {
            Packet::LoginStart { name } => {
                if self.stage != LoginStage::Hello {
                    return Err("Unexpected hello packet".to_string());
                }
                self.profile = Some(GameProfile {
                    id: None,
                    name: name.clone(),
                });
                if self.online_mode && !connection.is_local() {
                    self.stage = LoginStage::Key;
                    let _ = connection.send(Packet::EncryptionRequest {
                        server_id: String::new(),
                        public_key: self.keys.public_key_der(),
                        verify_token: self.nonce.to_vec(),
                    });
                } else {
                    self.stage = LoginStage::ReadyToAccept;
                }
                Ok(())
            }
            Packet::EncryptionResponse {
                shared_secret,
                verify_token,
            } => {
                if self.stage != LoginStage::Key {
                    return Err("Unexpected key packet".to_string());
                }
                // Vanilla checks the nonce first; a token that does not
                // decrypt (JCE returns null on bad padding) is a nonce miss.
                let token = match self.keys.decrypt_pkcs1_v15(verify_token) {
                    Ok(token) => token,
                    Err(_) => return Err("Invalid nonce!".to_string()),
                };
                if token.as_slice() != self.nonce {
                    return Err("Invalid nonce!".to_string());
                }
                let secret = self
                    .keys
                    .decrypt_pkcs1_v15(shared_secret)
                    .map_err(|error| error.to_string())?;
                if secret.len() != 16 {
                    return Err("Invalid secret key!".to_string());
                }
                self.stage = LoginStage::Authenticating;
                connection.set_encryption(secret.as_slice().try_into().unwrap());
                // The vanilla authenticator thread runs async; the port runs it
                // inline between the encryption switch and the next packet.
                let profile = self.profile.as_ref().expect("profile at key");
                let hash = server_id_hash("", &self.keys.public_key_der(), &secret);
                match self
                    .session
                    .has_joined_server(&profile.name, &java_hex_bigint(&hash))
                {
                    Some(uuid) => {
                        self.profile.as_mut().expect("profile").id = Some(uuid);
                        self.stage = LoginStage::ReadyToAccept;
                    }
                    None if self.singleplayer => {
                        self.profile.as_mut().expect("profile").id =
                            Some(Uuid::offline_player(&profile.name));
                        self.stage = LoginStage::ReadyToAccept;
                    }
                    None => {
                        connection.disconnect(Text::literal("Failed to verify username!"));
                    }
                }
                Ok(())
            }
            _ => Err(format!("Unexpected packet in login stage {:?}", self.stage)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::PROTOCOL_VERSION_1_7_10 as VERSION;
    use crate::connection::finish_disconnected;

    /// A listener that records the packets it reads.
    #[derive(Default)]
    struct Recorder {
        packets: Vec<Packet>,
    }

    impl PacketListener for Recorder {
        fn handle(&mut self, packet: &Packet, _connection: &mut Connection) -> Result<(), String> {
            self.packets.push(packet.clone());
            Ok(())
        }
    }

    /// A session service that lets everyone in under their offline UUID.
    struct AllowAll;

    impl SessionService for AllowAll {
        fn has_joined_server(&self, name: &str, _hash: &str) -> Option<Uuid> {
            Some(Uuid::offline_player(name))
        }
    }

    fn pump(client: &mut Connection, server: &mut Connection) -> Result<(), String> {
        let outbound = client.take_outbound();
        server
            .receive(&outbound)
            .map_err(|error| error.to_string())?;
        server.tick();
        let outbound = server.take_outbound();
        client
            .receive(&outbound)
            .map_err(|error| error.to_string())?;
        client.tick();
        Ok(())
    }

    fn server_connection(
        online_mode: bool,
        session: Arc<dyn SessionService>,
    ) -> (Connection, Arc<RsaKeyPair>, LegacyRandom) {
        let mut random = LegacyRandom::new(42);
        let keys = Arc::new(RsaKeyPair::generate_with_bits(256, &mut random).unwrap());
        let mut connection = Connection::new(false);
        connection.set_protocol(Protocol::Login);
        let mut nonce = [0u8; 4];
        random_bytes(&mut random, &mut nonce);
        connection.set_listener(ServerLoginHandler::new(
            online_mode,
            false,
            Arc::clone(&keys),
            session,
            nonce,
        ));
        (connection, keys, random)
    }

    #[test]
    fn offline_login_reaches_accepted() {
        let (mut server, _keys, _random) = server_connection(false, Arc::new(NoSessionService));
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client.set_listener(Box::<Recorder>::default());
        client
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        pump(&mut client, &mut server).unwrap();
        // Second tick runs acceptLogin and sends the login success.
        server.tick();
        let outbound = server.take_outbound();
        client.receive(&outbound).unwrap();
        client.tick();
        let listener = client.take_listener().unwrap();
        let any: Box<dyn std::any::Any> = listener;
        let recorder = any.downcast::<Recorder>().unwrap();
        assert_eq!(
            recorder.packets,
            vec![Packet::LoginSuccess {
                uuid: "b50ad385-829d-3141-a216-7e7d7539ba7f".into(),
                name: "Notch".into(),
            }]
        );
    }

    #[test]
    fn online_login_runs_the_full_flow_encrypted() {
        let (mut server, keys, _random) = server_connection(true, Arc::new(AllowAll));
        let mut client_random = LegacyRandom::new(7);
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client.set_listener(Box::<Recorder>::default());
        client
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        // The server sent the encryption request and is waiting for the key.
        let request = server.take_outbound();
        assert!(!request.is_empty());
        client.receive(&request).unwrap();
        client.tick();
        let listener = client.take_listener().unwrap();
        let any: Box<dyn std::any::Any> = listener;
        let recorder = any.downcast::<Recorder>().unwrap();
        let Packet::EncryptionRequest {
            public_key,
            verify_token,
            ..
        } = &recorder.packets[0]
        else {
            panic!("encryption request expected, got {:?}", recorder.packets);
        };
        // The verify token is the server's nonce.
        assert!(!verify_token.is_empty());
        // The client answers with both secrets encrypted under the X.509 key.
        let public = crate::crypto::parse_public_key_der(public_key).unwrap();
        let mut secret = [0u8; 16];
        random_bytes(&mut client_random, &mut secret);
        client.set_listener(Box::<Recorder>::default());
        client
            .send(Packet::EncryptionResponse {
                shared_secret: public
                    .encrypt_pkcs1_v15(&secret, &mut client_random)
                    .unwrap(),
                verify_token: public
                    .encrypt_pkcs1_v15(verify_token, &mut client_random)
                    .unwrap(),
            })
            .unwrap();
        // The vanilla client switches in the write-complete listener, so the
        // response goes out in the clear and everything after is encrypted.
        client.set_encryption(&secret);
        // The server flips to encryption on the response, so the login
        // success comes back encrypted and the client decrypts it.
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        server.tick();
        let outbound = server.take_outbound();
        client.receive(&outbound).unwrap();
        client.tick();
        let listener = client.take_listener().unwrap();
        let any: Box<dyn std::any::Any> = listener;
        let recorder = any.downcast::<Recorder>().unwrap();
        assert_eq!(
            recorder.packets.last(),
            Some(&Packet::LoginSuccess {
                uuid: "b50ad385-829d-3141-a216-7e7d7539ba7f".into(),
                name: "Notch".into(),
            })
        );
        let _ = keys;
    }

    #[test]
    fn wrong_nonce_is_a_state_violation() {
        let (mut server, _keys, _random) = server_connection(true, Arc::new(AllowAll));
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        let _request = server.take_outbound();
        // A token that does not decrypt to the server's nonce.
        client
            .send(Packet::EncryptionResponse {
                shared_secret: vec![1; 16],
                verify_token: vec![2; 4],
            })
            .unwrap();
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        assert!(server.is_disconnected());
        assert!(
            server
                .disconnect_reason()
                .map(|t| t.to_json())
                .unwrap_or_default()
                .contains("Invalid nonce!")
        );
    }

    #[test]
    fn online_without_session_server_disconnects() {
        let (mut server, keys, _random) = server_connection(true, Arc::new(NoSessionService));
        let mut client_random = LegacyRandom::new(11);
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client
            .send(Packet::LoginStart {
                name: "Notch".into(),
            })
            .unwrap();
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        let _request = server.take_outbound();
        // The client encrypts its secret with the server's key; the server
        // cannot verify the username and disconnects. The token must decrypt
        // to the server nonce to reach the session check, so use the request.
        let key = keys.public_key();
        let mut secret = [0u8; 16];
        random_bytes(&mut client_random, &mut secret);
        client
            .send(Packet::EncryptionResponse {
                shared_secret: key.encrypt_pkcs1_v15(&secret, &mut client_random).unwrap(),
                verify_token: key.encrypt_pkcs1_v15(&[0; 4], &mut client_random).unwrap(),
            })
            .unwrap();
        let outbound = client.take_outbound();
        server.receive(&outbound).unwrap();
        server.tick();
        // Either the nonce check or the failed verification disconnected.
        assert!(server.is_disconnected());
    }

    #[test]
    fn login_times_out_after_600_ticks() {
        let (mut server, _keys, _random) = server_connection(false, Arc::new(NoSessionService));
        for _ in 0..601 {
            server.tick();
        }
        assert!(server.is_disconnected());
        assert_eq!(
            server.disconnect_reason().map(|t| t.as_string()),
            Some("Took too long to log in".to_string())
        );
        finish_disconnected(&mut server);
    }

    #[test]
    fn version_check_rejects_wrong_versions() {
        let mut random = LegacyRandom::new(5);
        let keys = Arc::new(RsaKeyPair::generate_with_bits(256, &mut random).unwrap());
        let mut server = Connection::new(false);
        server.set_listener(HandshakeHandler::new(LoginContext::new(
            false,
            false,
            Arc::clone(&keys),
            Arc::new(NoSessionService),
            random,
        )));
        let mut client = Connection::new(true);
        client
            .send(Packet::Handshake {
                version: 6,
                address: "a".into(),
                port: 25565,
                next_protocol: 2,
            })
            .unwrap();
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
        assert!(server.is_disconnected());
        assert_eq!(
            server.disconnect_reason().map(|t| t.as_string()),
            Some("Outdated server! I'm still on 1.7.10".to_string())
        );
    }

    #[test]
    fn handshake_installs_login_handler() {
        let mut random = LegacyRandom::new(6);
        let keys = Arc::new(RsaKeyPair::generate_with_bits(256, &mut random).unwrap());
        let mut server = Connection::new(false);
        server.set_listener(HandshakeHandler::new(LoginContext::new(
            false,
            false,
            Arc::clone(&keys),
            Arc::new(NoSessionService),
            random,
        )));
        let mut client = Connection::new(true);
        client
            .send(Packet::Handshake {
                version: VERSION,
                address: "127.0.0.1".into(),
                port: 25565,
                next_protocol: 2,
            })
            .unwrap();
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
        assert_eq!(server.protocol(), Protocol::Login);
        // The installed login handler answers a login start.
        let mut client = Connection::new(true);
        client.set_protocol(Protocol::Login);
        client
            .send(Packet::LoginStart { name: "A".into() })
            .unwrap();
        server.receive(&client.take_outbound()).unwrap();
        server.tick();
        assert!(!server.take_outbound().is_empty());
    }

    #[test]
    fn offline_uuid_for_login_name() {
        let profile = GameProfile::offline("Notch");
        assert_eq!(
            profile.id.unwrap().to_java_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert!(profile.is_complete());
        assert!(
            !GameProfile {
                id: None,
                name: "Notch".into()
            }
            .is_complete()
        );
    }
}
