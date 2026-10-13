use glam::Vec3;

use crate::codec::WireWrite;

pub mod admin_list;
pub mod game;
pub mod initial_sync;
pub mod kick;
pub mod server_info;

impl WireWrite for Vec3 {
    fn write(&self, w: &mut crate::codec::Writer) {
        w.f32(self.x);
        w.f32(self.y);
        w.f32(self.z);
    }
}
