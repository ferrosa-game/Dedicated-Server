use glam::Vec3;

use crate::codec::WireWrite;

/// Event 20: an explosion (size 0 for a grenade).
#[derive(Clone, Debug, PartialEq)]
pub struct EventExplosion {
    pub size: i32,
    pub pos: Vec3,
}

impl WireWrite for EventExplosion {
    fn write(&self, w: &mut crate::codec::Writer) {
        w.bits(self.size, 8);
        w.f32(self.pos.x);
        w.f32(self.pos.y);
        w.f32(self.pos.z);
    }
}
