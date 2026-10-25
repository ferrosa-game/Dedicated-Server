use ferrosa_wit::server;

pub fn everything() {
    server::items().iter().for_each(|i| i.delete());
    server::vehicles().iter().for_each(|v| v.delete());
    server::humans().iter().filter(|h| h.player().is_none()).for_each(|h| h.delete());
    server::clear_bullets();

    server::announce("Cleared");
}
