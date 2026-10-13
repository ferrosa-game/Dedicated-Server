use ferrosa_physics::RotMatrix;
use ferrosa_protocol::{CharacterCustomization, Team};
use glam::Vec3;

use crate::PlayerId;

pub mod arms;
pub mod bones;
pub mod create;
pub mod damage;
pub mod grab;
pub mod ik;
pub mod inventory;
pub mod locomotion;
pub mod physics;
pub mod seated;
pub mod simulation;

pub use bones::{BONE_COUNT, BoneId};
pub use locomotion::Locomotion;

pub const MAX_HUMANS: usize = 256;

#[derive(Clone, Debug)]
pub struct Bone {
    pub body: usize,
    pub pos: Vec3,
    pub pos2: Vec3,
    pub vel: Vec3,
    pub rot: RotMatrix,
    pub ang_vel: Vec3,
    pub mass: f32,
    pub min_inertia: f32,
    pub angles: Vec3,
    pub networked_rot: RotMatrix,
    pub limit_angles: [f32; 3],
    pub capsule: [Vec3; 2],
    pub joint: Option<usize>,
    pub ground_contact: i32,
    /// How hard the bone holds its pose: the human's strength, nothing for the pelvis and legs of a
    /// seated human.
    pub strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub bond: usize,
}

#[derive(Clone, Debug, Default)]
pub struct AttachmentGroup {
    pub points: Vec<(usize, Vec3)>,
    pub centroid: Vec3,
    pub prev_centroid: Vec3,
}

#[derive(Clone, Debug, Default)]
pub struct Attachments {
    pub groups: Vec<AttachmentGroup>,
}

pub struct HumanHealth {
    pub health: i32,
    pub blood_level: i32,
    /// Whether the human is alive (the binary's oldHealth): 100 at spawn, dead at 0 or below. Fatal injuries set it
    /// to 0, a crush takes 120, at 24 or below the human weakens and at 9 or below a bandage can save them.
    pub vitality: i32,
    pub chest: i32,
    pub head: i32,
    pub left_arm: i32,
    pub right_arm: i32,
    pub left_leg: i32,
    pub right_leg: i32,
}

impl Default for HumanHealth {
    fn default() -> Self {
        Self { health: 100, blood_level: 100, vitality: 100, chest: 100, head: 100, left_arm: 100, right_arm: 100, left_leg: 100, right_leg: 100 }
    }
}

pub struct Human {
    pub player: Option<PlayerId>,
    /// The account of the player this human belongs to, kept after the player leaves.
    pub account: Option<u32>,
    /// The player's share count, copied every tick so a returning player can buy them back.
    pub stocks: i32,
    pub team: Option<Team>,
    pub customization: CharacterCustomization,
    pub health: HumanHealth,
    pub stamina: i32,
    pub max_stamina: i32,
    pub vehicle: Option<usize>,
    /// The seat taken in `vehicle`.
    pub seat: usize,
    /// Ticks the human has been getting out of its seat (0 when not), which slides it outwards.
    pub seat_exit: i32,
    /// The gear stick a driver moves (controls 0 and 2).
    pub gear_x_input: f32,
    pub gear_y_input: f32,
    pub last_vehicle_cooldown: i32,
    /// Whether the traffic car this human sits in names it as its driver, set before each tick.
    pub traffic_driver: bool,
    /// An immortal human loses no blood, cannot die of its wounds and is not hurt by falls or
    /// vehicles; it recovers quickly while knocked down.
    pub is_immortal: bool,
    pub is_on_ground: bool,
    pub spawn_protection: i32,
    pub input_flags: u32,
    pub last_input_flags: u32,
    pub movement_state: i32,
    pub getup_ticks: i32,
    pub movement_mode: i32,
    pub is_standing: bool,
    pub view_yaw: f32,
    pub view_pitch: f32,
    pub yaw_offset: f32,
    pub strafe_input: f32,
    pub walk_input: f32,
    pub stance: f32,
    pub lean_forward: f32,
    pub lean_side: f32,
    pub pain: i32,
    /// The spin a jump starts with, about the head's up and side axes (from the player's
    /// controls).
    pub jump_spin_yaw: f32,
    pub jump_spin_pitch: f32,
    /// Where the player looks around without turning (controls 6 and 7): the waist and
    /// chest turn part of the way, and a pointing arm follows them.
    pub free_look_yaw: f32,
    pub free_look_pitch: f32,
    pub hand_sway: Vec3,
    pub hand_sway_vel: Vec3,
    pub inventory: [InventorySlot; INVENTORY_SLOTS],
    pub actions: [QueuedAction; 8],
    pub actions_queued: i32,
    pub actions_finished: i32,
    pub action_type: i32,
    pub action_progress: f32,
    pub action_duration: i32,
    pub action_hand: i32,
    pub action_slot: i32,
    pub throw_pitch: f32,
    pub despawn_ticks: i32,
    pub last_vehicle: i32,
    pub strength: f32,
    pub eat_cooldown: i32,
    pub progress_bar: i32,
    /// Ticks an immortal human knocked below 50 health is kept from being shot (1800 when knocked down).
    pub down_timer: i32,
    /// How far the body still has to turn to face the view.
    pub view_turn: f32,
    /// The body's heading, turned towards the view a step at a time.
    pub body_yaw: f32,
    /// The pitch counterpart of `yaw_offset`: the seated human's look pitch.
    pub pitch_offset: f32,
    /// Where the player looks (controls 4 and 5): the view's yaw from the body and its pitch,
    /// or the look around a seat.
    pub look_yaw: f32,
    pub look_pitch: f32,
    /// The body heading the client last saw, which the look yaw is relative to.
    pub client_body_yaw: f32,
    /// The vehicles whose bounds overlap the human's, max 8.
    pub nearby_vehicles: Vec<usize>,
    pub bleeding: bool,
    pub pos: Vec3,
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    pub grid_min: [i32; 3],
    pub grid_max: [i32; 3],
    pub bones: Vec<Bone>,
    pub joints: Vec<Joint>,
    pub locomotion: Locomotion,
    pub grabs: [grab::Grab; 2],
}

pub const INVENTORY_SLOTS: usize = 7;

/// One inventory slot: the two hands (0 right, 1 left) and five pockets.
#[derive(Clone, Copy, Debug, Default)]
pub struct InventorySlot {
    pub count: i32,
    pub items: [i32; 8],
}

/// An inventory action waiting in the human's queue (human_action_simulation).
#[derive(Clone, Copy, Debug, Default)]
pub struct QueuedAction {
    pub kind: i32,
    pub progress: f32,
    pub slot: i32,
    pub arg: i32,
}
