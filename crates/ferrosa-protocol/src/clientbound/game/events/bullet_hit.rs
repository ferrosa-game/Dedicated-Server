use glam::Vec3;

use crate::codec::WireWrite;

#[derive(Clone, Debug, PartialEq)]
pub struct EventBulletHit {
    pub pos: Vec3,
    pub normal: Vec3,
    pub hit_type: i32,
    pub unk: i32,
}

impl WireWrite for EventBulletHit {
    fn write(&self, w: &mut crate::codec::Writer) {
        w.bits(self.unk, 4);
        w.bits(self.hit_type, 6);
        self.pos.write(w);
        self.normal.write(w);
    }
}
