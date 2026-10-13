use std::net::{Ipv4Addr, SocketAddrV4};

use crate::codec::{CodecError, Reader, WireRead, WireWrite, Writer};

/// Packet 0x4b from the client to the masterserver (client_recv_packet): a ticket to join `server`, asked for with
/// the client's game and master auth numbers. The address goes as a little-endian number, so its octets reversed.
#[derive(Debug, Clone, PartialEq)]
pub struct TicketRequest {
    pub game_auth: u32,
    pub master_auth: u32,
    pub server: SocketAddrV4,
}

impl WireWrite for TicketRequest {
    fn write(&self, w: &mut Writer) {
        w.byte(0x4b);
        w.u32(self.game_auth);
        w.u32(self.master_auth);
        w.u32(u32::from(*self.server.ip()));
        w.u16(self.server.port());
    }
}

impl WireRead for TicketRequest {
    /// After the packet's type byte.
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let (game_auth, master_auth) = (r.u32()?, r.u32()?);
        let ip = Ipv4Addr::from(r.u32()?);
        Ok(Self { game_auth, master_auth, server: SocketAddrV4::new(ip, r.u16()?) })
    }
}

/// Packet 0x4b from the masterserver to the client: the ticket the server it asked to join will take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket {
    pub ticket: u32,
}

impl WireWrite for Ticket {
    fn write(&self, w: &mut Writer) {
        w.byte(0x4b);
        w.u32(self.ticket);
    }
}

impl WireRead for Ticket {
    /// After the packet's type byte.
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        Ok(Self { ticket: r.u32()? })
    }
}
