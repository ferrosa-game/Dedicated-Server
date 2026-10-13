use std::net::{Ipv4Addr, SocketAddrV4};

use crate::codec::{CodecError, Reader, WireRead, WireWrite, Writer};

/// Packet 0x48 from the client to the masterserver (masterserver_auth_update): its name and Steam's encrypted app
/// ticket.
#[derive(Debug, Clone, PartialEq)]
pub struct SteamAuth {
    pub name: String,
    pub ticket: Vec<u8>,
}

impl WireWrite for SteamAuth {
    fn write(&self, w: &mut Writer) {
        w.byte(0x48);
        w.string(&self.name, 32);
        w.u32(self.ticket.len() as u32);
        w.bytes(&self.ticket);
    }
}

impl WireRead for SteamAuth {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let name = String::from_utf8_lossy(&r.text(32)?).into_owned();
        let len = r.u32()? as usize;
        Ok(Self { name, ticket: r.bytes(len)?.to_vec() })
    }
}

/// Packet 0x48 from the masterserver: the client's game and master auth numbers, and (when the master auth is not 0)
/// its phone number. A master auth of 0 means the sign-in failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteamAuthReply {
    pub game_auth: u32,
    pub master_auth: u32,
    pub phone_number: Option<u32>,
}

impl WireWrite for SteamAuthReply {
    fn write(&self, w: &mut Writer) {
        w.byte(0x48);
        w.u32(self.game_auth);
        w.u32(self.master_auth);
        if let Some(phone) = self.phone_number {
            w.u32(phone);
        }
    }
}

impl WireRead for SteamAuthReply {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let (game_auth, master_auth) = (r.u32()?, r.u32()?);
        let phone_number = if master_auth != 0 { Some(r.u32()?) } else { None };
        Ok(Self { game_auth, master_auth, phone_number })
    }
}

/// Packet 0x49 from the client to the masterserver: its game and master auth numbers, to be confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthConfirm {
    pub game_auth: u32,
    pub master_auth: u32,
}

impl WireWrite for AuthConfirm {
    fn write(&self, w: &mut Writer) {
        w.byte(0x49);
        w.u32(self.game_auth);
        w.u32(self.master_auth);
    }
}

impl WireRead for AuthConfirm {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        Ok(Self { game_auth: r.u32()?, master_auth: r.u32()? })
    }
}

/// Packet 0x49 from the masterserver: 1 if the auth numbers are good, then the phone number if one follows (the
/// client keeps the one it had if not).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthConfirmReply {
    pub accepted: bool,
    pub phone_number: Option<u32>,
}

impl WireWrite for AuthConfirmReply {
    fn write(&self, w: &mut Writer) {
        w.byte(0x49);
        w.u32(self.accepted as u32);
        if let Some(phone) = self.phone_number {
            w.u32(phone);
        }
    }
}

impl WireRead for AuthConfirmReply {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let accepted = r.u32()? == 1;

        Ok(Self { accepted, phone_number: if accepted { r.u32().ok() } else { None } })
    }
}

/// Packet 0x4a from the client to the masterserver: asking for the server list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerListRequest;

impl WireWrite for ServerListRequest {
    fn write(&self, w: &mut Writer) {
        w.byte(0x4a);
    }
}

/// How many addresses a server list packet carries at most.
pub const SERVER_LIST_PAGE: u32 = 0x80;

/// Packet 0x4a from the masterserver: how many servers there are, where this page starts, and its addresses (each a
/// little-endian address, so its octets reversed, then the port).
#[derive(Debug, Clone, PartialEq)]
pub struct ServerList {
    pub total: u32,
    pub offset: u32,
    pub servers: Vec<SocketAddrV4>,
}

impl WireWrite for ServerList {
    fn write(&self, w: &mut Writer) {
        w.byte(0x4a);
        w.u32(self.total);
        w.u32(self.offset);

        for s in &self.servers {
            w.u32(u32::from(*s.ip()));
            w.u16(s.port());
        }
    }
}

impl WireRead for ServerList {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let (total, offset) = (r.u32()?, r.u32()?);
        let mut servers = Vec::new();

        for _ in 0..total.saturating_sub(offset).min(SERVER_LIST_PAGE) {
            let (Ok(ip), Ok(port)) = (r.u32(), r.u16()) else { break };

            servers.push(SocketAddrV4::new(Ipv4Addr::from(ip), port));
        }

        Ok(Self { total, offset, servers })
    }
}
