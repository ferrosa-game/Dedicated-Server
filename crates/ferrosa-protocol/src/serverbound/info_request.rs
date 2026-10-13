use crate::codec::{WireRead, WireWrite, Writer};

#[derive(Debug, Clone, PartialEq)]
pub struct InfoRequest {
    pub version: u8,
    pub timestamp: u32,
}

impl WireRead for InfoRequest {
    fn read(r: &mut crate::codec::Reader) -> Result<Self, crate::codec::CodecError> {
        let version = r.u8()?;
        let timestamp = r.u32()?;

        Ok(InfoRequest { version, timestamp })
    }
}

impl WireWrite for InfoRequest {
    /// The server list's ping: the type byte (0), the client's version and when it asked.
    fn write(&self, w: &mut Writer) {
        w.byte(0);
        w.byte(self.version);
        w.u32(self.timestamp);
    }
}
