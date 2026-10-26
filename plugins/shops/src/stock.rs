use ferrosa_wit::{Building, BuildingKind, ItemKind, ShopEntry, VehicleKind, server};

/// The guns, magazines, bandages and grenades a gun store sells, in the order it lists them.
const GUN_STOCK: [ItemKind; 12] = [
    ItemKind::Pistol,
    ItemKind::PistolMag,
    ItemKind::Mp5,
    ItemKind::Mp5Mag,
    ItemKind::Ak47,
    ItemKind::Ak47Mag,
    ItemKind::M16,
    ItemKind::M16Mag,
    ItemKind::Uzi,
    ItemKind::UziMag,
    ItemKind::Bandage,
    ItemKind::Grenade,
];

const BURGER_PRICE: i32 = 20;

/// Burger shops close at seven in the evening.
const CLOSING_TIME: i32 = 19 * 60 * 60;

/// The type a dealership stocks: mostly town cars, hatchbacks, minivans and vans, now and then a beamer, rarely a
/// turbo and very rarely a Turbo S.
fn random_stock_vehicle() -> VehicleKind {
    match rand::random_range(0..16) {
        0..=1 => VehicleKind::Van,
        2..=5 => VehicleKind::TownCar,
        6..=9 => VehicleKind::Hatchback,
        10..=12 => VehicleKind::Minivan,
        13..=14 => VehicleKind::Beamer,
        _ if rand::random_ratio(1, 16) => VehicleKind::TurboS,
        _ => VehicleKind::Turbo,
    }
}

/// of_kind: get every building of a type
fn of_kind(kind: BuildingKind) -> impl Iterator<Item = Building> {
    server::buildings().into_iter().filter(move |b| b.kind() == kind)
}

/// restock_dealerships: for every car dealer, restock it with new random vehicles
pub fn restock_dealerships() {
    for building in of_kind(BuildingKind::CarDealer) {
        let mut shop = building.shop();
        let mut j = 0;

        while j < shop.len() {
            if rand::random_ratio(1, 8) {
                let last = shop.pop().unwrap();

                if j < shop.len() {
                    shop[j] = last;
                }
            } else {
                j += 1;
            }
        }

        while shop.len() < 8 {
            let kind = random_stock_vehicle();
            let price = server::vehicle_price(kind as u32).unwrap_or(0);

            shop.push(ShopEntry { kind: kind as i32, price, extra: rand::random_range(0..6) });
        }

        building.set_shop(&shop);
    }
}

/// The gun stores' fixed stock.
pub fn stock_gun_stores() {
    let stock: Vec<ShopEntry> = GUN_STOCK.iter().map(|&kind| ShopEntry { kind: kind as i32, price: server::item_price(kind as u32).unwrap_or(0), extra: 0 }).collect();

    for building in of_kind(BuildingKind::GunStore) {
        building.set_shop(&stock);
    }
}

/// The burger shops' one line, until they close for the day.
pub fn stock_burger_shops() {
    if server::world_time() > CLOSING_TIME - 1 {
        return;
    }

    let stock = [ShopEntry { kind: 0, price: BURGER_PRICE, extra: 0 }];

    for building in of_kind(BuildingKind::BurgerShop) {
        building.set_shop(&stock);
    }
}
