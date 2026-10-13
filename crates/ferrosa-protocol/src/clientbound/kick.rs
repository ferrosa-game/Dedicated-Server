use crate::codec::{CodecError, Reader, WireRead, WireWrite, Writer};

#[derive(Debug, Clone, PartialEq)]
pub struct KickClient {
    pub reason: String,
}

impl WireWrite for KickClient {
    fn write(&self, writer: &mut Writer) {
        writer.byte(3);
        writer.byte(self.reason.len() as u8);
        writer.bytes(self.reason.as_bytes());
    }
}

impl WireRead for KickClient {
    /// After the packet's type byte: the client takes the length from six bits.
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let len = r.bits(6)? as usize;
        Ok(Self { reason: String::from_utf8_lossy(r.bytes(len)?).into_owned() })
    }
}
