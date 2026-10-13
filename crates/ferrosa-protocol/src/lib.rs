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

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct CharacterCustomization {
    pub gender: u8,
    pub head: u8,
    pub skin: u8,
    pub hair_color: u8,
    pub hair_style: u8,
    pub eye_color: u8,
    pub model: u8,
    pub necklace: u8,
    pub suit_color: u8,
    pub tie_color: u8,
}

impl Default for CharacterCustomization {
    fn default() -> Self {
        Self { gender: 0, head: 4, skin: 2, hair_color: 4, hair_style: 6, eye_color: 6, model: 1, necklace: 0, suit_color: 0, tie_color: 1 }
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
