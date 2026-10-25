use ferrosa_wit::{Mat3, Menu, Player, server};

pub fn spawn(p: &Player) {
    if let Some(h) = server::spawn_human(p.camera(), Mat3::IDENTITY, Some(p.id())) {
        p.set_human(&h);
        p.set_menu(Menu::Empty);
    }
}
