//! Hands holding on to other humans: the grab part of human_update_hand_grab_and_inventory.
//!
//! The binary keeps a grab record per hand and, while the grab key is held, bonds the hand to the spot it records,
//! but nothing in it ever fills the record in, so no hand ever takes hold. Here an empty hand takes hold of the
//! nearest other human's bone it touches while the key is held.

use ferrosa_physics::rotation::{rotate, unrotate};
use ferrosa_physics::{Bond, ItemPoint, RigidBodies, RotMatrix, Table};
use glam::Vec3;

use super::{BoneId, Human, inventory::point_segment_distance};
use crate::input::Input;

#[derive(Clone, Copy, Debug, Default)]
pub struct Grab {
    pub holding: bool,
    pub human: usize,
    pub bone: usize,
    pub point: Vec3,
    pub spot: Vec3,
}

#[derive(Clone, Copy)]
struct HeldBone {
    body: usize,
    pos: Vec3,
    rot: RotMatrix,
    capsule: [Vec3; 2],
}

fn to_world(b: &HeldBone, l: Vec3) -> Vec3 {
    b.pos + rotate(&b.rot, l)
}

fn to_local(b: &HeldBone, p: Vec3) -> Vec3 {
    unrotate(&b.rot, p - b.pos)
}

/// grab_hands: let go without the grab key or with something in the hand, take hold
/// of the nearest other human's bone in reach, and while holding pull the hand and that bone together at the cost of
/// a point of stamina every 4 ticks, letting go once the stamina runs low or the hand slips off.
pub fn grab_hands(humans: &mut Table<Human>, bodies: &mut RigidBodies, ticks: u32) {
    let others: Vec<(usize, bool, Vec<HeldBone>)> = humans
        .iter()
        .map(|(id, h)| (id, h.vehicle.is_none(), h.bones.iter().map(|b| HeldBone { body: b.body, pos: b.pos, rot: b.rot, capsule: b.capsule }).collect()))
        .collect();
    let bone_of = |human: usize, bone: usize| others.iter().find(|o| o.0 == human).and_then(|o| o.2.get(bone).copied());

    for id in humans.ids() {
        let Some(h) = humans.get_mut(id) else { continue };

        for (hand, &bone) in [BoneId::HandRight as usize, BoneId::HandLeft as usize].iter().enumerate() {
            let free = h.health.vitality > 0 && h.vehicle.is_none() && h.inventory[hand].count == 0;

            if !free || !h.has_input(Input::Grab) {
                h.grabs[hand].holding = false;
                continue;
            }

            let palm = h.bones[bone].pos;

            if !h.grabs[hand].holding {
                let nearest = others
                    .iter()
                    .filter(|o| o.0 != id && o.1)
                    .flat_map(|o| o.2.iter().enumerate().map(move |(j, b)| (o.0, j, b)))
                    .map(|(human, j, b)| (point_segment_distance(b.capsule[0], b.capsule[1], palm), human, j, b))
                    .filter(|&(d, ..)| d <= 0.25)
                    .min_by(|a, b| a.0.total_cmp(&b.0));

                if let Some((_, human, j, b)) = nearest {
                    h.grabs[hand] = Grab { holding: true, human, bone: j, point: to_local(b, palm), spot: palm };
                }

                continue;
            }

            let g = h.grabs[hand];

            let Some(held) = bone_of(g.human, g.bone) else {
                h.grabs[hand].holding = false;
                continue;
            };

            let spot = to_world(&held, g.point);

            if palm.distance(spot) > 0.125 || h.stamina <= 7 {
                h.grabs[hand].holding = false;
                continue;
            }

            h.grabs[hand].spot = spot;
            bodies.create_bond(Bond::ItemPoint(ItemPoint::grab(h.bones[bone].body, held.body, g.point)));

            if ticks.is_multiple_of(4) {
                h.stamina -= 1;
            }
        }
    }
}
