use ferrosa_protocol::clientbound::game::events::chat::ChatType;

use crate::{PlayerId, sim::Sim};

fn chat_lines(message: &str) -> impl Iterator<Item = &str> {
    message.lines().flat_map(|line| {
        let mut rest = line;

        std::iter::from_fn(move || {
            if rest.is_empty() {
                return None;
            }

            if rest.len() <= 63 {
                return Some(std::mem::take(&mut rest));
            }

            let limit = rest.floor_char_boundary(63);
            let cut = if rest[limit..].starts_with(' ') { limit } else { rest[..limit].rfind(' ').filter(|&k| k > 0).unwrap_or(limit) };
            let (head, tail) = rest.split_at(cut);
            rest = tail.trim_start();
            Some(head)
        })
    })
}

impl Sim {
    pub fn announce(&mut self, message: &str) {
        for line in chat_lines(message) {
            self.chat(ChatType::Announce, -1, 0, line);
        }
    }

    pub fn priv_chat(&mut self, player: PlayerId, message: &str) {
        for line in chat_lines(message) {
            self.chat(ChatType::PrivateMessage, player.0 as i32, 0, line);
        }
    }

    pub fn chat(&mut self, kind: ChatType, speaker: i32, volume: i32, message: &str) {
        for line in chat_lines(message) {
            self.send_chat(line, kind, speaker, volume);
        }
    }
}
