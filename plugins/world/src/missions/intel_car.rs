//! The other way: a town car driving the city with the intel aboard, its driver armed and its guards with it.

use ferrosa_wit::{ItemKind, Mat3, Team, Vec3, server};

use super::DISK_CAPACITY;
use crate::World;

/// How far the intel car must drive, and how long its mission lasts.
const INTEL_CAR_DISTANCE: f32 = 768.0;

pub(super) const INTEL_TIME: i32 = 216000;

/// The intel car's driver holds the key in slot 4 and the disk in slot 3.
const INTEL_KEY_SLOT: u32 = 4;

const INTEL_DISK_SLOT: u32 = 3;

/// The intel car's key and disk are made here before being handed to the driver.
const INTEL_ITEMS_POS: Vec3 = Vec3::new(62.0, 10.0, 60.0);

impl World {
    /// spawn_mission_intel_car: a town car driving to a street at least 768 away, its driver holding the key and a
    /// red disk with the intel, and its guards; all of them leave at `expiry`. The traffic car it is.
    pub(super) fn spawn_mission_intel_car(&mut self, file: &str, content: i32, expiry: i32) -> Option<u32> {
        let vehicle = server::spawn_route_car(Some(INTEL_CAR_DISTANCE))?;
        let car = vehicle.traffic_car();
        let (vpos, vrot) = (vehicle.pos(), vehicle.rot());
        let (driver, driver_human) = server::create_mission_bot(expiry, vpos, vrot, &vehicle, 0)?;

        if let Some(key) = server::spawn_item_by_id(ItemKind::Key as u32, INTEL_ITEMS_POS, Mat3::IDENTITY) {
            key.set_key_vehicle(Some(&vehicle));

            if let Some(h) = &driver_human {
                h.put_item(&key, INTEL_KEY_SLOT);
            }
        }

        if let Some(disk) = server::spawn_item_by_id(ItemKind::DiskRed as u32, INTEL_ITEMS_POS, Mat3::IDENTITY)
            && let Some(volume) = server::new_volume(DISK_CAPACITY)
        {
            disk.set_volume(&volume);
            volume.add_file_of_content(file, content);

            if let Some(h) = &driver_human {
                h.put_item(&disk, INTEL_DISK_SLOT);
            }
        }

        if let Some(p) = server::get_player(driver) {
            p.set_team(Team::Mission);
        }

        server::create_guards(expiry, vpos, vrot, &vehicle);

        car
    }
}
