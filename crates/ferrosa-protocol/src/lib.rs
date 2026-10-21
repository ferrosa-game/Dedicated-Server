use serde::{Deserialize, Serialize};

use crate::{
    codec::{CodecError, Reader, WireRead, WireWrite, Writer},
    masterserver::{AuthPacket, server_addr::ServerAddress},
    serverbound::{game::ClientGamePacket, info_request::InfoRequest, join_request::JoinRequest},
};

pub mod clientbound;
pub mod codec;
pub mod masterserver;
pub mod serverbound;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GameMode {
    Driving = 1,
    Racing = 2,
    Round = 3,
    World = 4,
    Eliminator = 5,
    CoOp = 6,
    Versus = 7,
    None = 8,
}

impl TryFrom<u32> for GameMode {
    type Error = CodecError;

    fn try_from(v: u32) -> Result<Self, CodecError> {
        Ok(match v {
            1 => Self::Driving,
            2 => Self::Racing,
            3 => Self::Round,
            4 => Self::World,
            5 => Self::Eliminator,
            6 => Self::CoOp,
            7 => Self::Versus,
            8 => Self::None,
            _ => return Err(CodecError::BadEnum(v)),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Team {
    Goldmen = 0,
    Monsota = 1,
    OXS = 2,
    Nexaco = 3,
    Pentacom = 4,
    Prodocon = 5,
    /// The crews of round car chases.
    Mission = 7,

    Spectator = 17,
}

impl TryFrom<i32> for Team {
    type Error = CodecError;

    fn try_from(v: i32) -> Result<Self, CodecError> {
        Ok(match v {
            0 => Self::Goldmen,
            1 => Self::Monsota,
            2 => Self::OXS,
            3 => Self::Nexaco,
            4 => Self::Pentacom,
            5 => Self::Prodocon,
            7 => Self::Mission,
            17 => Self::Spectator,
            _ => return Err(CodecError::BadEnum(v as u32)),
        })
    }
}

impl Team {
    pub const CORPORATIONS: [Team; 6] = [Team::Goldmen, Team::Monsota, Team::OXS, Team::Nexaco, Team::Pentacom, Team::Prodocon];
    pub const CORPORATION_NAMES: [&str; 6] = ["Goldmen Inc", "Monsota", "OXS International", "Nexaco", "Pentacom", "Prodocon"];
}

/// A player's gender, which picks their body, head and hair models. Sent in 1 bit.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gender {
    #[default]
    Female = 0,
    Male = 1,
}

impl Gender {
    /// The gender a bit says: 1 is male.
    pub fn from_bit(bit: u32) -> Self {
        if bit & 1 == 1 { Self::Male } else { Self::Female }
    }
}

/// What a player wears: casual clothes, a suit, or the second suit (fsuit2/msuit2) mission bots and guards wear. Sent in
/// 5 bits.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Model {
    #[default]
    Casual = 0,
    Suit = 1,
    Suit2 = 2,
}

impl TryFrom<u8> for Model {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        Ok(match v {
            0 => Self::Casual,
            1 => Self::Suit,
            2 => Self::Suit2,
            _ => return Err(v),
        })
    }
}

/// The colours a suit comes in: the columns of the character palette's suit row. Sent in 4 bits.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SuitColor {
    #[default]
    Brown = 0,
    Black = 1,
    Grey = 2,
    Navy = 3,
    Teal = 4,
    Purple = 5,
    Tan = 6,
    White = 7,
    Red = 8,
    Indigo = 9,
    DarkNavy = 10,
    DarkGrey = 11,
}

impl SuitColor {
    pub const COUNT: usize = 12;
}

impl TryFrom<u8> for SuitColor {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        if (v as usize) < Self::COUNT { Ok(unsafe { std::mem::transmute::<u8, SuitColor>(v) }) } else { Err(v) }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct CharacterCustomization {
    pub gender: Gender,
    pub head: u8,
    pub skin: u8,
    pub hair_color: u8,
    pub hair_style: u8,
    pub eye_color: u8,
    pub model: Model,
    pub necklace: u8,
    pub suit_color: SuitColor,
    pub tie_color: u8,
}

impl Default for CharacterCustomization {
    fn default() -> Self {
        Self { gender: Gender::Female, head: 4, skin: 2, hair_color: 4, hair_style: 6, eye_color: 6, model: Model::Suit, necklace: 0, suit_color: SuitColor::Brown, tie_color: 1 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerboundPacket {
    InfoRequest(InfoRequest),
    JoinRequest(JoinRequest),
    AuthPacket(AuthPacket),
    LeaveGame,
    MasterServerPing(ServerAddress),
    GamePacket(Box<ClientGamePacket>),
}

/// Packet 7: the client leaving the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveGame;

impl WireWrite for LeaveGame {
    fn write(&self, w: &mut Writer) {
        w.byte(7);
    }
}

pub fn parse_frame(data: &[u8]) -> Option<(u8, &[u8])> {
    if data.len() < 5 || &data[..4] != b"7DFP" {
        return None;
    }

    Some((data[4], &data[5..]))
}

pub fn decode_packet(type_byte: u8, body: &[u8]) -> Result<ServerboundPacket, CodecError> {
    let mut r = Reader::new(body);

    Ok(match type_byte {
        0 => ServerboundPacket::InfoRequest(InfoRequest::read(&mut r)?),
        2 => ServerboundPacket::JoinRequest(JoinRequest::read(&mut r)?),
        4 => ServerboundPacket::GamePacket(Box::new(ClientGamePacket::read(&mut r)?)),
        7 => ServerboundPacket::LeaveGame,
        64 => ServerboundPacket::MasterServerPing(ServerAddress::read(&mut r)?),
        66 => ServerboundPacket::AuthPacket(AuthPacket::read(&mut r)?),
        other => return Err(CodecError::BadEnum(other as u32)),
    })
}

pub fn frame_packet(packet: impl WireWrite) -> Vec<u8> {
    let mut w = Writer::new();
    w.bytes(b"7DFP");
    packet.write(&mut w);

    w.into_vec()
}
