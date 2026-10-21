use ferrosa_wit::{Gender, Human, Item, Mat3, Model, Player, SuitColour, Team, Vec3, Vehicle, server};

/// The chase's key and briefcase are made at a fixed spot and go straight into the driver's pockets.
const ITEMS_AT: Vec3 = Vec3::new(62.0, 10.0, 60.0);

fn spawn(name: &str, pos: Vec3) -> Option<Item> {
    server::spawn_item(name, pos, Mat3::IDENTITY)
}

/// create_round_traffic_car: a town car driving a route through the streets, its driver holding its key and a
/// briefcase with a disk of `kind`, and one to three armed guards. The disk.
pub(crate) fn car_chase(kind: u32) -> Option<Item> {
    let car = server::spawn_route_car(None)?;
    let (pos, rot) = (car.pos(), car.rot());
    let (driver, human) = mission_bot(&car, pos, rot, 0)?;

    if let Some(key) = spawn("key", ITEMS_AT) {
        key.set_key_vehicle(Some(&car));
        human.put_item(&key, 4);
    }

    let disk = server::spawn_item_by_id(kind, ITEMS_AT, Mat3::IDENTITY);

    if let Some(case) = spawn("briefcase", ITEMS_AT) {
        if let Some(disk) = &disk {
            case.attach(disk);
        }

        human.put_item(&case, 3);
    }

    driver.set_waypoints(&[]);

    for seat in 1..4 {
        if seat == 1 || !rand::random::<bool>() {
            guard(&car, pos, rot, seat);
        }
    }

    disk
}

fn mission_bot(car: &Vehicle, pos: Vec3, rot: Mat3, seat: u32) -> Option<(Player, Human)> {
    let bot = server::add_bot()?;

    bot.set_team(Team::Mission);
    bot.set_name("");
    bot.set_gender(Gender::Male);
    bot.set_model(Model::Suit2);
    bot.set_suit_colour(SuitColour::Black);

    let human = server::spawn_human(pos, rot, Some(bot.id()))?;

    bot.set_human(&human);
    human.enter_vehicle(car, seat);
    Some((bot, human))
}

fn guard(car: &Vehicle, pos: Vec3, rot: Mat3, seat: u32) {
    let Some((_, human)) = mission_bot(car, pos, rot, seat) else { return };
    let at = human.pos();

    if let Some(gun) = spawn("pistol", at) {
        human.put_item(&gun, 0);
    }

    for k in 0..4 {
        let Some(mag) = spawn("pistolmag", at) else { continue };
        let slot = if k == 0 { 1 } else { (k - 1) / 2 + 3 };

        human.put_item(&mag, slot);
    }
}
