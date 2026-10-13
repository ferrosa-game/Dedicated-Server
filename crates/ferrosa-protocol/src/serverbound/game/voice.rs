use crate::codec::{CodecError, Reader, WireWrite, Writer};

#[derive(Debug, Clone, PartialEq)]
pub struct VoiceData {
    pub frames: [VoiceFrame; 6],
    pub is_silenced: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoiceFrame {
    pub index: u8,
    pub size: u16,
    pub volume: u8,
    pub data: Vec<u8>,
}

impl WireWrite for VoiceData {
    /// client_send's voice: six frames, then whether the client is silent.
    fn write(&self, w: &mut Writer) {
        for f in &self.frames {
            w.bits(f.index as i32, 6);
            w.bits(f.size as i32, 11);
            w.bits(f.volume as i32, 2);
            w.bytes(&f.data[..(f.size as usize).min(f.data.len())]);
        }
        w.bits(self.is_silenced as i32, 1);
    }
}

pub fn decode_voice_data(reader: &mut Reader) -> Result<VoiceData, CodecError> {
    let mut frames = core::array::from_fn(|_| VoiceFrame { index: 0, size: 0, volume: 0, data: Vec::new() });

    for frame in &mut frames {
        frame.index = reader.bits(6)? as u8;
        frame.size = reader.bits(11)? as u16;
        frame.volume = reader.bits(2)? as u8;

        frame.data = reader.bytes(frame.size as usize)?.to_vec();
    }

    let is_silenced = reader.bits(1)? != 0;

    Ok(VoiceData { frames, is_silenced })
}
