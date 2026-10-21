use ferrosa_wit::{Base, Mat3, Team, Vec3, server};

pub(crate) fn base(team: Team) -> Option<Base> {
    server::get_corporation(team)?.base()
}

pub(crate) trait Area {
    fn contains(&self, pos: Vec3) -> bool;

    fn in_vault(&self, pos: Vec3) -> bool;

    fn frame(&self) -> Mat3;
}

impl Area for Base {
    fn contains(&self, pos: Vec3) -> bool {
        pos.x >= self.interior_min.x && pos.x < self.interior_max.x && pos.z >= self.interior_min.z && pos.z < self.interior_max.z
    }

    fn in_vault(&self, pos: Vec3) -> bool {
        let (min, max) = (self.vault_min, self.vault_max);

        pos.x >= min.x && pos.x < max.x && pos.y >= min.y && pos.y < max.y && pos.z >= min.z && pos.z < max.z
    }

    fn frame(&self) -> Mat3 {
        crate::yaw(self.table_orientation + std::f32::consts::PI)
    }
}
