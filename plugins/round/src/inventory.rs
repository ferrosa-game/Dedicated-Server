use std::collections::HashMap;

use ferrosa_wit::{Cash, Mat3, Player, PlayerId, Team, server};

use crate::spawn_corp_vehicle;

enum Kept {
    Item { kind: u32, uses: i32, magazine: Option<(u32, i32)> },
    Key { kind: u32, colour: i32 },
    Cash(Cash),
}

#[derive(Default)]
pub(crate) struct Inventories(HashMap<PlayerId, Vec<(u32, Kept)>>);

impl Inventories {
    pub(crate) fn save(&mut self, p: &Player) {
        let Some(human) = p.human() else {
            self.0.remove(&p.id());
            return;
        };

        let (last_kept, key, cash) = (server::find_item_kind("bandage"), server::find_item_kind("key"), server::find_item_kind("cashworld"));
        let mut kept = Vec::new();

        for slot in 0..7 {
            for item in human.slot(slot) {
                let kind = item.kind();

                let thing = if last_kept.is_some_and(|last| kind <= last) {
                    let magazine = item.children().first().map(|m| (m.kind(), m.uses()));
                    Kept::Item { kind, uses: item.uses(), magazine }
                } else if Some(kind) == key {
                    let Some(car) = item.key_vehicle().filter(|v| v.health() > 0) else { continue };
                    Kept::Key { kind: car.kind(), colour: car.colour() }
                } else if Some(kind) == cash
                    && let Some(c) = item.cash()
                {
                    Kept::Cash(c)
                } else {
                    continue;
                };

                kept.push((slot, thing));
            }
        }

        self.0.insert(p.id(), kept);
    }

    /// init_player_inventory: the kept items back on the player's new human, a gun with its magazine in and a key's
    /// car parked in `team`'s base.
    pub(crate) fn restore(&self, p: &Player, team: Team) {
        let (Some(human), Some(kept)) = (p.human(), self.0.get(&p.id())) else { return };
        let pos = human.pos();
        let spawn = |kind: u32| server::spawn_item_by_id(kind, pos, Mat3::IDENTITY);

        for (slot, thing) in kept {
            let item = match thing {
                Kept::Item { kind, uses, magazine } => {
                    let Some(item) = spawn(*kind) else { continue };
                    item.set_uses(*uses);

                    if let Some((kind, rounds)) = magazine
                        && let Some(mag) = spawn(*kind)
                    {
                        mag.set_uses(*rounds);
                        item.attach(&mag);
                    }

                    item
                }
                Kept::Key { kind, colour } => {
                    let Some(car) = spawn_corp_vehicle(*kind, team, *colour) else { continue };
                    let Some(key) = server::find_item_kind("key").and_then(spawn) else { continue };

                    key.set_key_vehicle(Some(&car));
                    key
                }
                Kept::Cash(c) => {
                    let Some(item) = server::find_item_kind("cashworld").and_then(spawn) else { continue };

                    item.set_cash(c);
                    item
                }
            };

            human.put_item(&item, *slot);
        }
    }

    pub(crate) fn forget(&mut self, player: PlayerId) {
        self.0.remove(&player);
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}
