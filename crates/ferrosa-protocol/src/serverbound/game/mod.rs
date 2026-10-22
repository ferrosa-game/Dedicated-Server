use std::f32::consts::{FRAC_PI_2, PI};

use bitflags::bitflags;
use glam::Vec3;

pub mod actions;
pub mod voice;

use crate::{
    codec::{Reader, WireRead, WireWrite, Writer},
    serverbound::game::{
        actions::{GameAction, read_actions},
        voice::{VoiceData, decode_voice_data},
    },
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InputFlags: u32 {
        const LEFT_MOUSE = 1 << 0;
        const RIGHT_MOUSE = 1 << 1;
        const JUMP = 1 << 2;
        const CROUCH = 1 << 3;
        const SHIFT = 1 << 4;
        const DROP_ITEM = 1 << 5;
        const MODE_KEYS = 31 << 6;
        const GRAB = 1 << 11;
        const RELOAD = 1 << 12;
        const SWITCH_HANDS = 1 << 13;
        const DELETE = 1 << 18;
        const RAGDOLL = 1 << 19;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientGamePacket {
    pub round_num: u32,

    pub gear_x: f32,
    pub left_right: f32,
    pub gear_y: f32,
    pub forward_back: f32,
    pub view_yaw_delta: f32,
    pub view_pitch: f32,
    pub free_look_yaw: f32,
    pub free_look_pitch: f32,
    pub view_yaw: f32,
    pub spin_yaw: f32,
    pub view_pitch_delta: f32,

    pub input_flags: u32,
    pub input_type: u8,
    pub zoom_level: u8,
    pub received_events: u32,

    pub num_sent_objects: u32,
    pub camera_pos: Vec3,

    pub packet_action_count: u8,
    pub total_actions: u8,

    pub actions: Vec<GameAction>,
    pub voice_data: VoiceData,

    pub spectating_human_id: u8,
    pub pack_ack: u16,
    pub link_ack: u8,
    pub packet_count: u32,
    pub sdl_tick: u32,
}

impl WireRead for ClientGamePacket {
    fn read(r: &mut Reader) -> Result<Self, crate::codec::CodecError> {
        let round_num = r.u32()?;
        let gear_x = r.fixed_float()?.clamp(-1.0, 1.0);
        let left_right = r.fixed_float()?.clamp(-1.0, 1.0);
        let gear_y = r.fixed_float()?.clamp(-1.0, 1.0);
        let forward_back = r.fixed_float()?.clamp(-1.0, 1.0);
        let view_yaw_delta = r.fixed_float()?.clamp(-PI, PI);
        let view_pitch = r.fixed_float()?.clamp(-FRAC_PI_2, FRAC_PI_2);
        let free_look_yaw = r.fixed_float()?.clamp(-PI, PI);
        let free_look_pitch = r.fixed_float()?.clamp(-FRAC_PI_2, FRAC_PI_2);
        let view_yaw = r.fixed_float()?.clamp(-PI, PI);
        let spin_yaw = r.fixed_float()?.clamp(-PI, PI);
        let view_pitch_delta = r.fixed_float()?.clamp(-FRAC_PI_2, FRAC_PI_2);

        let input_flags = r.u32()? & InputFlags::all().bits();
        let input_type = r.bits(8)? as u8;
        let zoom_level = (r.bits(4)? as u8).min(2);
        let received_events = r.bits(16)?;

        let num_sent_objects = r.u32()?;

        let camera = Vec3::new(r.f32()?, r.f32()?, r.f32()?);
        let camera_pos = if camera.is_finite() { camera.clamp(Vec3::splat(-4096.0), Vec3::splat(4096.0)) } else { Vec3::ZERO };

        let packet_action_count = r.bits(4)?;
        let total_actions = r.bits(8)?;

        let actions = read_actions(r, packet_action_count)?;

        let voice_data = decode_voice_data(r)?;

        let spectating_human_id = r.bits(8)?;
        let pack_ack = r.bits(11)?;
        let link_ack = r.bits(8)?;
        let packet_count = r.u32()?;
        let sdl_tick = r.u32()?;

        Ok(Self {
            round_num,
            gear_x,
            left_right,
            gear_y,
            forward_back,
            view_yaw_delta,
            view_pitch,
            free_look_yaw,
            free_look_pitch,
            view_yaw,
            spin_yaw,
            view_pitch_delta,
            input_flags,
            input_type,
            zoom_level,
            received_events,
            num_sent_objects,
            camera_pos,
            packet_action_count: packet_action_count as u8,
            total_actions: total_actions as u8,
            actions,
            voice_data,
            spectating_human_id: spectating_human_id as u8,
            pack_ack: pack_ack as u16,
            link_ack: link_ack as u8,
            packet_count,
            sdl_tick,
        })
    }
}

impl WireWrite for ClientGamePacket {
    fn write(&self, w: &mut Writer) {
        w.byte(4);
        w.u32(self.round_num);
        for v in [
            self.gear_x,
            self.left_right,
            self.gear_y,
            self.forward_back,
            self.view_yaw_delta,
            self.view_pitch,
            self.free_look_yaw,
            self.free_look_pitch,
            self.view_yaw,
            self.spin_yaw,
            self.view_pitch_delta,
        ] {
            w.fixed_float(v, 24);
        }
        w.u32(self.input_flags);
        w.bits(self.input_type as i32, 8);
        w.bits(self.zoom_level as i32, 4);
        w.bits(self.received_events as i32, 16);
        w.u32(self.num_sent_objects);
        w.f32(self.camera_pos.x);
        w.f32(self.camera_pos.y);
        w.f32(self.camera_pos.z);
        w.bits(self.packet_action_count as i32, 4);
        w.bits(self.total_actions as i32, 8);

        for a in &self.actions {
            a.write(w);
        }

        self.voice_data.write(w);
        w.bits(self.spectating_human_id as i32, 8);
        w.bits(self.pack_ack as i32, 11);
        w.bits(self.link_ack as i32, 8);
        w.u32(self.packet_count);
        w.u32(self.sdl_tick);
    }
}
