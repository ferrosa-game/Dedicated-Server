use crate::codec::{Reader, WireRead, WireWrite, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarInfo {
    pub gender: u8,
    pub head: u8,
    pub skin_color: u8,
    pub hair_color: u8,
    pub hair: u8,
    pub eye_color: u8,
}

impl AvatarInfo {
    pub fn from_avatar_info(info: u32) -> Self {
        Self {
            gender: (info & 0x1) as u8,
            head: (((info >> 1) & 0xF) as u8).min(4),
            skin_color: (((info >> 5) & 0x7) as u8).min(5),
            hair_color: (((info >> 8) & 0xF) as u8).min(11),
            hair: (((info >> 12) & 0xF) as u8).min(8),
            eye_color: (((info >> 16) & 0xF) as u8).min(7),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinRequest {
    pub version: u8,
    pub account_id: u32,
    pub auth_ticket: u32,
    pub player_name: String,
    pub avatar_info: AvatarInfo,
    pub password: String,
    pub phone_number: u32,
    pub protocol_version: u8,
    pub is_ferrosa: bool
}

impl AvatarInfo {
    /// The 32-bit look the client sends: gender, head, skin, hair colour, hair style and eyes.
    pub fn to_avatar_info(&self) -> u32 {
        self.gender as u32
            | (self.head as u32) << 1
            | (self.skin_color as u32) << 5
            | (self.hair_color as u32) << 8
            | (self.hair as u32) << 12
            | (self.eye_color as u32) << 16
    }
}

impl WireWrite for JoinRequest {
    /// client_recv_packet's join request: the type byte (2) and the request.
    fn write(&self, w: &mut Writer) {
        w.byte(2);
        w.bits(self.version as i32, 8);
        w.u32(self.account_id);
        w.u32(self.auth_ticket);
        w.string(&self.player_name, 32);
        w.u32(self.avatar_info.to_avatar_info());
        w.string(&self.password, 32);
        w.bits(self.protocol_version as i32, 8);
        w.u32(self.phone_number);
    }
}

impl WireRead for JoinRequest {
    fn read(reader: &mut Reader) -> Result<Self, crate::codec::CodecError> {
        let version = reader.u8()?;
        let account_id = reader.u32()?;
        let auth_ticket = reader.u32()?;
        let player_name = reader.string(32)?;
        let avatar_info = reader.u32()?;
        let password = reader.string(32)?;
        let protocol_version = reader.u8()?;
        let phone_number = reader.u32()?;

        let is_ferrosa = if let Ok(packet) = reader.bits(1) {
            packet != 0
        } else {
            false
        };

        Ok(JoinRequest {
            version,
            account_id,
            auth_ticket,
            player_name,
            avatar_info: AvatarInfo::from_avatar_info(avatar_info),
            password,
            phone_number,
            protocol_version,
            is_ferrosa
        })
    }
}
