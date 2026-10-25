use ferrosa_wit::{ChatType, Player, server};

pub fn toggle(p: &Player) {
    let on = !p.god_mode();

    p.set_god_mode(on);
    server::send_chat(ChatType::AdminChat, -1, 0, if on { "godmode on" } else { "godmode off" });
}
