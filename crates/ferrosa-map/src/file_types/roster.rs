use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct RosterPlayer {
    pub account_id: u32,
    pub phone_number: u32,
    pub steam_id: u64,
    pub eye_color: u32,
    pub hair_color: u32,
    pub skin_color: u32,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Roster {
    pub server_id: u32,
    pub players: Vec<RosterPlayer>,
}

impl Roster {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(roster) => roster,
                Err(e) => {
                    println!("[Sim] {}: {e}", path.display());

                    Roster::default()
                }
            },
            Err(_) => {
                let roster = Roster { server_id: rand::random_range(1..=u32::MAX), players: Vec::new() };

                roster.save(path);
                roster
            }
        }
    }

    pub fn save(&self, path: &Path) {
        if self.server_id == 0 {
            return;
        }

        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(path, text) {
                    println!("[Sim] Could not save the player roster: {e:?}");
                }
            }
            Err(e) => println!("[Sim] Could not write the player roster: {e}"),
        }
    }

    pub fn get(&self, account_id: u32) -> Option<&RosterPlayer> {
        self.players.iter().find(|p| p.account_id == account_id)
    }

    pub fn by_phone(&self, phone: i32) -> Option<&RosterPlayer> {
        self.players.iter().find(|p| p.phone_number as i32 == phone)
    }

    pub fn cache_player(&mut self, account_id: u32, name: &str, phone_number: u32, steam_id: u64) -> RosterPlayer {
        if self.get(account_id).is_none() {
            self.players.push(RosterPlayer {
                account_id,
                eye_color: rand::random_range(0..3),
                hair_color: rand::random_range(0..8),
                skin_color: rand::random_range(0..4),
                ..RosterPlayer::default()
            });
        }

        let Some(p) = self.players.iter_mut().find(|p| p.account_id == account_id) else { unreachable!() };

        p.phone_number = phone_number;
        p.steam_id = steam_id;
        p.name = name.to_string();

        p.clone()
    }
}
