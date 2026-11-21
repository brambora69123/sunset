//! Login state: `HelloC2SPacket`, `KeyC2SPacket`, `HelloS2CPacket`,
//! `LoginSuccessS2CPacket`, `LoginFailS2CPacket`.
//!
//! The key packets carry the raw encrypted bytes; RSA and AES live in the
//! encryption layer, not here.

use crate::buf::{Buf, Result};

/// Login start, c2s 0x00: the username, at most 16 chars.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginStart {
    pub name: String,
}

impl LoginStart {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            name: buf.read_string(16)?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.name)
    }
}

pub const LOGIN_START_ID: i32 = 0x00;

/// Encryption request, s2c 0x01: server id (at most 20 chars), the public
/// key and the verify token, both short-prefixed byte arrays.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl EncryptionRequest {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            server_id: buf.read_string(20)?,
            public_key: buf.read_byte_array()?,
            verify_token: buf.read_byte_array()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.server_id)?;
        buf.write_byte_array(&self.public_key);
        buf.write_byte_array(&self.verify_token);
        Ok(())
    }
}

pub const ENCRYPTION_REQUEST_ID: i32 = 0x01;

/// Encryption response, c2s 0x01: the encrypted shared secret and verify
/// token, both short-prefixed byte arrays.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl EncryptionResponse {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            shared_secret: buf.read_byte_array()?,
            verify_token: buf.read_byte_array()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_byte_array(&self.shared_secret);
        buf.write_byte_array(&self.verify_token);
        Ok(())
    }
}

pub const ENCRYPTION_RESPONSE_ID: i32 = 0x01;

/// Login success, s2c 0x02: the UUID string (at most 36 chars) and the
/// username (at most 16 chars).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginSuccess {
    pub uuid: String,
    pub username: String,
}

impl LoginSuccess {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            uuid: buf.read_string(36)?,
            username: buf.read_string(16)?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.uuid)?;
        buf.write_string(&self.username)
    }
}

pub const LOGIN_SUCCESS_ID: i32 = 0x02;
