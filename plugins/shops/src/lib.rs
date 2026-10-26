use ferrosa_wit::{Plugin, server};

mod stock;

#[derive(Default)]
struct Shops {
    clock: i32,
}

impl Plugin for Shops {
    fn reset(&mut self) {
        self.clock = server::world_time();
        stock::restock_dealerships();
        stock::stock_gun_stores();
        stock::stock_burger_shops();
    }

    fn tick(&mut self) {
        let clock = server::world_time();

        if clock == self.clock {
            return;
        }

        self.clock = clock;

        if clock % 18000 == 0 {
            stock::restock_dealerships();
            stock::stock_gun_stores();
        }
    }
}

ferrosa_wit::plugin!(Shops);
