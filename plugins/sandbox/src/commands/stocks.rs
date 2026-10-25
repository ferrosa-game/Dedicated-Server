use ferrosa_wit::{Menu, Player};

pub fn toggle(p: &Player) {
    p.set_menu(if p.menu() == Menu::RoundCorpStock { Menu::Empty } else { Menu::RoundCorpStock });
}
