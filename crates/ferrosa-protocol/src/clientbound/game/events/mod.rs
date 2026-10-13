use crate::{
    CharacterCustomization, Team,
    clientbound::game::events::{
        bullet::EventBullet, bullet_hit::EventBulletHit, bullet_hole::EventBulletHole, chat::EventChat, explosion::EventExplosion,
        item_set_cell::EventItemSetCell, mission::EventMission, mission_text::EventMissionText, phone_sound::EventPhoneSound, sound::EventSound,
        team_door::EventTeamDoor, update_corporation::EventUpdateCorporation, update_phone::EventUpdatePhone, update_player::EventUpdatePlayer,
        update_player_round::EventUpdatePlayerRound, update_stock::EventUpdateStock, update_vehicle::EventUpdateVehicle,
        update_vehicle_type_color::EventUpdateVehicleTypeColor,
    },
    clientbound::game::{
        VehicleKind,
        events::{chat::ChatType, sound::Sound},
    },
    codec::{CodecError, Reader, WireRead, WireWrite, Writer},
};
use client_state::{EventSetBlockCell, EventUnused, EventUpdateHuman, EventUpdateItem};

pub mod bullet;
pub mod bullet_hit;
pub mod bullet_hole;
pub mod chat;
pub mod client_state;
pub mod explosion;
pub mod item_set_cell;
pub mod mission;
pub mod mission_text;
pub mod phone_sound;
pub mod sound;
pub mod team_door;
pub mod update_corporation;
pub mod update_phone;
pub mod update_player;
pub mod update_player_round;
pub mod update_stock;
pub mod update_vehicle;
pub mod update_vehicle_type_color;

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub tick_created: u32,
    pub kind: ServerEvent,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ServerEvent {
    Bullet(EventBullet),
    BulletHit(EventBulletHit),
    BulletHole(EventBulletHole),
    Chat(EventChat),
    Explosion(EventExplosion),
    ItemSetCell(EventItemSetCell),
    Mission(EventMission),
    MissionText(EventMissionText),
    PhoneSound(EventPhoneSound),
    Sound(EventSound),
    TeamDoor(EventTeamDoor),
    UpdateCorporation(EventUpdateCorporation),
    UpdatePhone(EventUpdatePhone),
    UpdatePlayerRound(EventUpdatePlayerRound),
    UpdatePlayer(EventUpdatePlayer),
    UpdateStock(EventUpdateStock),
    UpdateVehicleTypeColor(EventUpdateVehicleTypeColor),
    UpdateVehicle(EventUpdateVehicle),
    UpdateItem(EventUpdateItem),
    UpdateHuman(EventUpdateHuman),
    SetBlockCell(EventSetBlockCell),
    Unused11(EventUnused),
    Unused14(EventUnused),
    Empty,
}

pub trait EventWrite {
    fn write(&self, w: &mut Writer);
    fn type_id(&self) -> i32;
}

impl WireWrite for Event {
    fn write(&self, w: &mut Writer) {
        let (type_id, body): (i32, &dyn WireWrite) = match &self.kind {
            ServerEvent::Bullet(e) => (0, e),
            ServerEvent::BulletHit(e) => (1, e),
            ServerEvent::Chat(e) => (2, e),
            ServerEvent::UpdateVehicleTypeColor(e) => (3, e),
            ServerEvent::UpdateVehicle(e) => (4, e),
            ServerEvent::UpdatePhone(e) => (6, e),
            ServerEvent::UpdatePlayer(e) => (7, e),
            ServerEvent::UpdatePlayerRound(e) => (8, e),
            ServerEvent::Sound(e) => (9, e),
            ServerEvent::TeamDoor(e) => (10, e),
            ServerEvent::UpdateCorporation(e) => (12, e),
            ServerEvent::UpdateStock(e) => (13, e),
            ServerEvent::ItemSetCell(e) => (15, e),
            ServerEvent::BulletHole(e) => (16, e),
            ServerEvent::PhoneSound(e) => (19, e),
            ServerEvent::Explosion(e) => (20, e),
            ServerEvent::Mission(e) => (21, e),
            ServerEvent::MissionText(e) => (22, e),
            ServerEvent::UpdateItem(e) => (5, e),
            ServerEvent::UpdateHuman(e) => (17, e),
            ServerEvent::SetBlockCell(e) => (18, e),
            ServerEvent::Unused11(e) => {
                w.bits(11, 6);
                w.bits(self.tick_created as i32, 28);
                for v in e.values {
                    w.i32(v);
                }
                return;
            }
            ServerEvent::Unused14(e) => {
                w.bits(14, 6);
                w.bits(self.tick_created as i32, 28);
                for (v, n) in e.values.into_iter().zip([8, 8, 8, 20]) {
                    w.bits(v, n);
                }
                return;
            }
            ServerEvent::Empty => {
                w.bits(23, 6);

                return;
            }
        };

        w.bits(type_id, 6);
        w.bits(self.tick_created as i32, 28);

        body.write(w)
    }
}

/// The packed area and block of the cell events: area plus x << 16, then y plus z << 16.
fn area_block(r: &mut Reader) -> Result<(i32, i32, i32, i32), CodecError> {
    let (a, b) = (r.i32()?, r.i32()?);
    Ok((a & 0xffff, a >> 16, b & 0xffff, b >> 16))
}

/// Seven-bit characters, up to the first zero.
fn chars(r: &mut Reader, n: usize) -> Result<String, CodecError> {
    let b: Vec<u8> = (0..n).map(|_| r.bits(7).map(|c| c as u8)).collect::<Result<_, _>>()?;
    Ok(String::from_utf8_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).into_owned())
}

impl WireRead for Event {
    /// client_unpack_events: the type, then (unless it is the empty event) the tick and the type's fields.
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let kind = r.bits(6)?;
        if kind == 23 {
            return Ok(Self { tick_created: 0, kind: ServerEvent::Empty });
        }
        let tick_created = r.bits(28)?;
        let kind = match kind {
            0 => ServerEvent::Bullet(EventBullet { bullet_type: r.bits(4)? as i32, item_id: r.bits(10)? as i32, pos: r.vec3()?, vel: r.vec3()? }),
            1 => {
                let (unk, hit_type) = (r.bits(4)? as i32, r.bits(6)? as i32);
                ServerEvent::BulletHit(EventBulletHit { unk, hit_type, pos: r.vec3()?, normal: r.vec3()? })
            }
            2 => {
                let len = r.bits(6)? as usize;
                let chat_type = ChatType::try_from(r.bits(4)?)?;
                let (speaker_id, volume) = (r.nullable(10)?, r.bits(4)? as i32);
                ServerEvent::Chat(EventChat { chat_type, message: chars(r, len)?, speaker_id, volume })
            }
            3 => {
                let vehicle_id = r.bits(10)? as i32;
                let vehicle_type = VehicleKind::try_from(r.bits(8)? as u8).map_err(|v| CodecError::BadEnum(v as u32))?;
                ServerEvent::UpdateVehicleTypeColor(EventUpdateVehicleTypeColor { vehicle_id, vehicle_type, vehicle_color: r.bits(4)? as u8 })
            }
            4 => {
                let (vehicle_id, kind, part) = (r.bits(10)? as i32, r.bits(4)? as i32, r.bits(10)? as i32);
                ServerEvent::UpdateVehicle(EventUpdateVehicle { vehicle_id, kind, part, pos: r.vec3()?, velocity: r.vec3()? })
            }
            5 => {
                let a = r.bits(20)? as i32;
                ServerEvent::UpdateItem(EventUpdateItem {
                    item_id: a & 0x3ff,
                    active: (a >> 10) & 1 != 0,
                    item_type: ((a >> 11) & 0x3f) as u8,
                    parent_human: r.nullable(10)?,
                    parent_item: r.nullable(10)?,
                    parent_slot: r.bits(4)? as i32,
                })
            }
            6 => ServerEvent::UpdatePhone(EventUpdatePhone {
                item_id: r.bits(10)? as i32,
                phone_status: r.bits(3)? as i32,
                display_phone_number: r.bits(16)? as i32,
                phone_texture: r.bits(2)? as i32,
            }),
            7 => {
                let a = r.bits(16)? as i32;
                let human_id = r.nullable(10)?;
                let c = r.i32()? as u32;
                let d = r.bits(24)?;
                let active = (a >> 9) & 1 != 0;
                let team = match Team::try_from((a >> 10) - 1) {
                    Ok(t) => t,
                    Err(_) if !active => Team::Spectator,
                    Err(e) => return Err(e),
                };
                let customization = CharacterCustomization {
                    gender: (c & 1) as u8,
                    head: ((c >> 1) & 0xf) as u8,
                    skin: ((c >> 6) & 7) as u8,
                    hair_style: ((c >> 9) & 0xf) as u8,
                    eye_color: (d & 7) as u8,
                    hair_color: ((d >> 3) & 0xf) as u8,
                    model: ((d >> 7) & 0x1f) as u8,
                    suit_color: ((d >> 12) & 0xf) as u8,
                    tie_color: ((d >> 16) & 0xf) as u8,
                    necklace: ((d >> 20) & 0xf) as u8,
                };
                let name = chars(r, 31)?;
                ServerEvent::UpdatePlayer(EventUpdatePlayer {
                    client_id: (a & 0xff) as u32,
                    team,
                    customization,
                    active,
                    is_bot: (a >> 8) & 1 != 0,
                    human_id,
                    name,
                })
            }
            8 => ServerEvent::UpdatePlayerRound(EventUpdatePlayerRound { client_id: r.bits(8)?, money: r.i32()?, stocks: r.i32()?, phone_number: r.u32()? }),
            9 => {
                let sound_type = Sound::try_from(r.bits(8)?)?;
                ServerEvent::Sound(EventSound { sound_type, pos: r.vec3()?, volume: r.f32()?, pitch: r.f32()? })
            }
            10 => ServerEvent::TeamDoor(EventTeamDoor { team_id: r.bits(8)? as i32, door_open: r.bits(1)? != 0 }),
            11 => ServerEvent::Unused11(EventUnused { values: [r.i32()?, r.i32()?, r.i32()?, r.i32()?] }),
            12 => ServerEvent::UpdateCorporation(EventUpdateCorporation { a: r.i32()?, b: r.i32()?, c: r.i32()?, d: r.i32()? }),
            13 => ServerEvent::UpdateStock(EventUpdateStock { prices: [r.i32()?, r.i32()?, r.i32()?] }),
            14 => ServerEvent::Unused14(EventUnused { values: [r.bits(8)? as i32, r.bits(8)? as i32, r.bits(8)? as i32, r.bits(20)? as i32] }),
            15 => {
                let (area, block_x, block_y, block_z) = area_block(r)?;
                let taken = r.u32()?;
                r.bits(2)?;
                ServerEvent::ItemSetCell(EventItemSetCell { area, block_x, block_y, block_z, taken })
            }
            16 => {
                let (area, block_x, block_y, block_z) = area_block(r)?;
                let (cell, face) = (r.u32()?, r.nullable(10)?);
                ServerEvent::BulletHole(EventBulletHole { area, block_x, block_y, block_z, cell, face, pos: r.vec3()?, vel: r.vec3()? })
            }
            17 => ServerEvent::UpdateHuman(EventUpdateHuman { human_id: r.bits(10)? as i32, active: r.bits(1)? != 0, player_id: r.nullable(8)? }),
            18 => {
                let (area, block_x, block_y, block_z) = area_block(r)?;
                ServerEvent::SetBlockCell(EventSetBlockCell { area, block_x, block_y, block_z, value: r.u32()? })
            }
            19 => {
                let sound = Sound::try_from(r.bits(8)?)?;
                ServerEvent::PhoneSound(EventPhoneSound { sound, item_id: r.bits(16)? as i32, volume: r.f32()?, pitch: r.f32()? })
            }
            20 => ServerEvent::Explosion(EventExplosion { size: r.bits(8)? as i32, pos: r.vec3()? }),
            21 => {
                let (player, kind, role, value) = (r.bits(10)? as i32, r.bits(4)? as i32, r.bits(4)? as i32, r.bits(30)? as i32);
                ServerEvent::Mission(EventMission { player, kind, role, value, pos: [r.f32()?, r.f32()?, r.f32()?] })
            }
            22 => {
                let a = r.bits(16)? as i32;
                let line = r.bits(6)? as i32;
                let len = r.bits(7)? as usize;
                ServerEvent::MissionText(EventMissionText { corporation: a & 0xff, mission: a >> 8, line, text: r.bytes(len)?.to_vec() })
            }
            other => return Err(CodecError::BadEnum(other)),
        };
        Ok(Self { tick_created, kind })
    }
}
