use ferrosa_wit::Plugin;

mod stock;

#[derive(Default)]
struct Shops;

impl Plugin for Shops {
    fn reset(&mut self) {
        stock::restock_dealerships();
        stock::stock_gun_stores();
        stock::stock_burger_shops();
    }
}

ferrosa_wit::plugin!(Shops);
