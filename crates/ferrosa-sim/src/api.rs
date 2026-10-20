use ferrosa_physics::RigidBody;
use ferrosa_protocol::clientbound::game::GameState;
use glam::Vec3;

use crate::{plugin::Plugins, sim::Sim};

impl Sim {
    pub fn gamestate(&self) -> GameState {
        self.gamestate
    }

    pub fn set_gamestate(&mut self, state: GameState) {
        self.gamestate = state;
    }

    pub fn mode_name(&self) -> &'static str {
        self.mode.name()
    }

    pub fn set_plugins(&mut self, plugins: Plugins) {
        self.plugins = plugins;
        plugins.on_start(self);
    }

    pub fn gravity_scale(&self) -> f32 {
        self.bodies.gravity_scale
    }

    pub fn set_gravity_scale(&mut self, scale: f32) {
        self.bodies.gravity_scale = scale;
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
