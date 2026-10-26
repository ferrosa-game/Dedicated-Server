use ferrosa_wit::{ItemKind, Mat3, Team, Vec3, server};
use crate::World;

impl World {
    pub(super) fn spawn_mission_intel_car(&mut self, file: &str, content: i32, expiry: i32) -> Option<u32> {
        let vehicle = server::spawn_route_car(Some(768.0))?;
        let car = vehicle.traffic_car();
        let (vpos, vrot) = (vehicle.pos(), vehicle.rot());
        let (driver, driver_human) = server::create_mission_bot(expiry, vpos, vrot, &vehicle, 0)?;

        if let Some(key) = server::spawn_item_by_id(ItemKind::Key as u32, Vec3::new(62.0, 10.0, 60.0), Mat3::IDENTITY) {
            key.set_key_vehicle(Some(&vehicle));

            if let Some(h) = &driver_human {
                h.put_item(&key, 4);
            }
        }

        if let Some(disk) = server::spawn_item_by_id(ItemKind::DiskRed as u32, Vec3::new(62.0, 10.0, 60.0), Mat3::IDENTITY)
            && let Some(volume) = server::new_volume(360)
        {
            disk.set_volume(&volume);
            volume.add_file_of_content(file, content);

            if let Some(h) = &driver_human {
                h.put_item(&disk, 3);
            }
        }

        if let Some(p) = server::get_player(driver) {
            p.set_team(Team::Mission);
        }

        server::create_guards(expiry, vpos, vrot, &vehicle);

        car
    }
}
