//! The client side of the login flow, ported from the decompiled 1.7.10
//! `ClientLoginNetworkHandler`: the encryption request handling (shared secret
//! from the sanctioned RNG, both fields RSA-encrypted under the server's X.509
//! key, the encryption switch right after the response goes out), the play
//! switch on login success and the disconnect on login fail.

use sunset_1_7_core::LegacyRandom;

use crate::connection::{Connection, PacketListener};
use crate::crypto::{RsaPublicKey, parse_public_key_der, random_bytes};
use crate::packets::{Packet, Protocol};

/// `ClientLoginNetworkHandler`.
pub struct ClientLoginHandler {
    random: LegacyRandom,
    /// `joinServer` against the session service; offline clients skip it.
    pub join_server: Option<Box<dyn FnMut(&str, &str) + Send>>,
}

impl ClientLoginHandler {
    pub fn new(random: LegacyRandom) -> Box<Self> {
        Box::new(Self {
            random,
            join_server: None,
        })
    }
}

impl PacketListener for ClientLoginHandler {
    fn handle(&mut self, packet: &Packet, connection: &mut Connection) -> Result<(), String> {
        match packet {
            Packet::EncryptionRequest {
                server_id,
                public_key,
                verify_token,
            } => {
                let public: RsaPublicKey =
                    parse_public_key_der(public_key).map_err(|error| error.to_string())?;
                let mut secret = [0u8; 16];
                random_bytes(&mut self.random, &mut secret);
                // `new BigInteger(digest(...)).toString(16)`, the join hash.
                let hash = crate::crypto::server_id_hash(server_id, public_key, &secret);
                let hash_hex = crate::crypto::java_hex_bigint(&hash);
                if let Some(join) = self.join_server.as_mut() {
                    join(&hash_hex, "");
                }
                connection
                    .send(Packet::EncryptionResponse {
                        shared_secret: public
                            .encrypt_pkcs1_v15(&secret, &mut self.random)
                            .map_err(|error| error.to_string())?,
                        verify_token: public
                            .encrypt_pkcs1_v15(verify_token, &mut self.random)
                            .map_err(|error| error.to_string())?,
                    })
                    .map_err(|error| error.to_string())?;
                // The vanilla switch runs in the write-complete listener, so
                // the response goes out in the clear and everything after is
                // encrypted.
                connection.set_encryption(&secret);
            }
            Packet::LoginSuccess { .. } => {
                connection.set_protocol(Protocol::Play);
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
    use crate::text::Text;

    #[test]
    fn login_success_switches_to_play() {
        let mut connection = Connection::new(true);
        connection.set_protocol(Protocol::Login);
        let mut listener = ClientLoginHandler::new(LegacyRandom::new(1));
        listener
            .handle(
                &Packet::LoginSuccess {
                    uuid: "b50ad385-829d-3141-a216-7e7d7539ba7f".into(),
                    name: "Notch".into(),
                },
                &mut connection,
            )
            .unwrap();
        assert_eq!(connection.protocol(), Protocol::Play);
    }

    #[test]
    fn disconnect_closes() {
        let mut connection = Connection::new(true);
        connection.set_protocol(Protocol::Login);
        let mut listener = ClientLoginHandler::new(LegacyRandom::new(1));
        listener
            .handle(
                &Packet::Disconnect {
                    reason: Text::literal("nope"),
                },
                &mut connection,
            )
            .unwrap();
        assert!(connection.is_disconnected());
    }
}
