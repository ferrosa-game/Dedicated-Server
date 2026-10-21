use ferrosa_physics::RigidBody;
use glam::Vec3;

use crate::sim::Sim;

impl Sim {
    pub fn gravity_scale(&self) -> f32 {
        self.bodies.gravity_scale
    }

    pub fn set_gravity_scale(&mut self, scale: f32) {
        self.bodies.gravity_scale = scale;

        for (_, item) in self.items.iter_mut() {
            item.physics_settled = false;
            item.settled_timer = 0;
        }
    }

    pub fn body_ids(&self) -> Vec<usize> {
        self.bodies.ids()
    }

    pub fn body(&self, id: usize) -> Option<&RigidBody> {
        self.bodies.get(id)
    }

    pub fn body_mut(&mut self, id: usize) -> Option<&mut RigidBody> {
        self.bodies.get_mut(id)
    }

    pub fn add_impulse(&mut self, id: usize, offset: Vec3, impulse: Vec3) {
        self.bodies.add_impulse(id, offset, impulse);
    }
}
