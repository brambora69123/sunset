//! Status state: `ServerStatusC2SPacket`, `ServerStatusS2CPacket`,
//! `PingC2SPacket`, `PingS2CPacket`.

use crate::buf::{Buf, Result};

/// Status request, c2s 0x00: empty body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusRequest;

impl StatusRequest {
    pub fn read(_buf: &mut Buf) -> Result<Self> {
        Ok(Self)
    }

    pub fn write(&self, _buf: &mut Buf) -> Result<()> {
        Ok(())
    }
}

pub const STATUS_REQUEST_ID: i32 = 0x00;

/// Status response, s2c 0x00: the server list JSON string. Kept raw; the
/// JSON parse belongs to the caller.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusResponse {
    pub json: String,
}

impl StatusResponse {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            json: buf.read_string(32767)?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_string(&self.json)
    }
}

pub const STATUS_RESPONSE_ID: i32 = 0x00;

/// Ping, c2s 0x01 and s2c 0x01, identical on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ping {
    pub time: i64,
}

impl Ping {
    pub fn read(buf: &mut Buf) -> Result<Self> {
        Ok(Self {
            time: buf.read_i64()?,
        })
    }

    pub fn write(&self, buf: &mut Buf) -> Result<()> {
        buf.write_i64(self.time);
        Ok(())
    }
}

pub const PING_ID: i32 = 0x01;
