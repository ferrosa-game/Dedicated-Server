use ferrosa_wit::{Computer, Plugin, Program, server};

mod snake;
mod tiles;

#[derive(Default)]
struct ComputerGames;

impl Plugin for ComputerGames {
    fn init(&mut self) {
        server::register_program("SNAKE");
        server::register_program("2048");
    }

    fn program(&mut self, name: &str, computer: &Computer, _args: &str) -> Option<Box<dyn Program>> {
        match name {
            "SNAKE" => Some(Box::new(snake::Snake::new(computer))),
            "2048" => Some(Box::new(tiles::Tiles::new(computer))),
            _ => None,
        }
    }
}

ferrosa_wit::plugin!(ComputerGames);
