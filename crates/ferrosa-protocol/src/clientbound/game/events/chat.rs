use crate::codec::WireWrite;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChatType {
    Announce = 0,
    Chat = 1,
    ItemSpeak = 2,
    EliminatorAnnouncement = 3,
    AdminChat = 4,
    /// Copied into the game's notice line (game_timer +0xc) rather than shown as a message.
    Notice = 5,
    PrivateMessage = 6,
}

impl TryFrom<u32> for ChatType {
    type Error = crate::codec::CodecError;

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        Ok(match v {
            0 => Self::Announce,
            1 => Self::Chat,
            2 => Self::ItemSpeak,
            3 => Self::EliminatorAnnouncement,
            4 => Self::AdminChat,
            5 => Self::Notice,
            6 => Self::PrivateMessage,
            _ => return Err(crate::codec::CodecError::BadEnum(v)),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventChat {
    pub chat_type: ChatType,
    pub message: String,
    pub speaker_id: i32,
    pub volume: i32,
}

impl WireWrite for EventChat {
    fn write(&self, w: &mut crate::codec::Writer) {
        let bytes = self.message.as_bytes();
        let len = bytes.len().min(63);

        w.bits(len as i32, 6);
        w.bits(self.chat_type as i32, 4);
        w.bits(self.speaker_id, 10);
        w.bits(self.volume, 4);

        for &b in &bytes[..len] {
            w.bits(b as i32, 7);
        }
    }
}
