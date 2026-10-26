use ferrosa_map::file_types::roster::RosterPlayer;
use ferrosa_protocol::{
    Model, SuitColor, Team,
    clientbound::game::events::{Event, ServerEvent, update_stock::EventUpdateStock},
};

use super::Sim;
use crate::{PlayerId, player::Player};

pub const CORPORATIONS: usize = 6;

/// A corporation's share market.
#[derive(Clone, Copy, Debug)]
pub struct Corporation {
    pub shares: i32,
    pub price: f32,
}

impl Default for Corporation {
    fn default() -> Self {
        Self { shares: 100, price: 100.0 }
    }
}

impl Sim {
    /// sell_all_stocks: sells all of a players stocks
    pub(crate) fn sell_all_stocks(&mut self, pid: PlayerId) {
        let held = self.players.get(pid.idx()).map_or(0, |p| p.stocks);
        self.sell_stocks(pid, held);
    }

    /// player_sell_stocks: sells `n` of the player's shares at `stock_sell_rate` of the price less what the
    /// corporation's running projects have spent per share.
    pub(crate) fn sell_stocks(&mut self, pid: PlayerId, n: i32) {
        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        let Some(c) = corporation_of(player.team) else { return };

        let held = player.stocks;

        if held < n {
            return;
        }

        let corp = &mut self.corporations[c];

        let shares = corp.shares;
        let mut price = corp.price;

        for m in self.corp_state.get(c).map_or(&[][..], |p| &p.missions[..]) {
            if m.active && m.provided_cash > 0 && shares > 0 {
                price -= m.provided_cash.max(0) as f32 / shares as f32;
            }
        }

        player.stocks = held - n;
        player.money = ((price * self.rules.stock_sell_rate) * n as f32 + player.money as f32) as i32;
        corp.shares = shares - n;
    }

    /// purchase_stocks: buys up to `n` shares (never past 9999 held) if the player has more money than they cost.
    pub(crate) fn purchase_stocks(&mut self, pid: PlayerId, n: i32) {
        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        let Some(c) = corporation_of(player.team) else { return };

        let held = player.stocks;
        let (n, count) = if held > 9999 {
            (0, 0.0)
        } else {
            let n = if held + n > 9999 { (9999 + 1) - held } else { n };
            (n, n as f32)
        };

        let money = player.money as f32;
        let cost = count * self.corporations[c].price;

        if money > cost {
            self.corporations[c].shares += n;
            player.stocks = held + n;
            player.money = (money - cost) as i32;
        }
    }
}

/// The corporation a player's team trades in, if it is one.
pub fn corporation_of(team: Team) -> Option<usize> {
    let t = team as usize;
    (t < 6).then_some(t)
}

/// wealth_tax: takes 5% of the money over 10000 and sells off a twentieth of the shares worth more than 10000.
pub fn wealth_tax(player: &mut Player, corps: &mut [Corporation; 6]) {
    let money = player.money;
    let excess = money - 10_000;

    if excess > 0 {
        player.money = money - (excess as f32 * 0.05) as i32;
    }

    let Some(c) = corporation_of(player.team) else { return };
    let price = corps[c].price;
    let over = ((player.stocks as f32 * price - 10_000.0) / price) as i32;

    if over > 0 {
        let sold = over / 20;
        corps[c].shares -= sold;
        player.stocks -= sold;
    }
}

/// create_event_update_stock: the first five share prices in tenths.
pub fn stock_event(corps: &[Corporation; 6], tick: u32) -> Event {
    let tenths: Vec<i32> = corps.iter().map(|c| (c.price * 10.0) as i32).collect();
    Event {
        tick_created: tick,
        kind: ServerEvent::UpdateStock(EventUpdateStock { prices: [(tenths[1] << 16) + tenths[0], (tenths[3] << 16) + tenths[2], tenths[4]] }),
    }
}

/// sanitize_name: slashes become spaces, the spaces at either end go, and an empty name is "Noname".
pub fn sanitize_name(raw: &[u8]) -> String {
    let end = raw.iter().take(31).position(|&b| b == 0).unwrap_or(raw.len().min(31));
    let bytes: Vec<u8> = raw[..end].iter().map(|&b| if b == b'/' { b' ' } else { b }).collect();
    let start = bytes.iter().position(|&b| b != b' ').unwrap_or(bytes.len());
    let stop = bytes.iter().rposition(|&b| b != b' ').map_or(start, |i| i + 1);
    let name = String::from_utf8_lossy(&bytes[start..stop.max(start)]).into_owned();
    if name.is_empty() { "Noname".into() } else { name }
}



pub const WORLD_TIME_START: i32 = 1728010;

impl Sim {

    /// The death bookkeeping of human_simulation for a dead human's player: world mode sells their shares and sets
    /// the respawn wait, every mode taxes their wealth, then the player loses the human.
    pub(crate) fn settle_death(&mut self, pid: PlayerId) {
        let tick = self.tick;
        let real = pid.idx() <= 255;

        if self.rules.shares_follow_human && real {
            self.sell_all_stocks(pid);
        }

        let Some(player) = self.players.get_mut(pid.idx()) else { return };

        if real {
            wealth_tax(player, &mut self.corporations);
            self.events.push(player.make_update_financials_event(tick));
        }

        if real {
            self.mode.on_death(self, pid);

            let plugins = self.plugins;
            plugins.on_death(self, pid);
        }

        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        player.human = None;

        self.update_player_event(pid);
    }

    /// create_account_from_join_ticket: the joining player's account, made new (1000 money, startcash in world mode,
    /// 1000 corporate rating and a random look) if this server has not seen them, and brought up to date with what the
    /// master server says about them.
    pub(crate) fn join_account(&mut self, auth: &ferrosa_protocol::masterserver::AuthPacket) -> RosterPlayer {
        let name = sanitize_name(auth.player_name.as_bytes());
        let player = self.roster.cache_player(auth.account_id, &name, auth.phone_number, auth.steam_id);

        self.roster.save(std::path::Path::new("players.json"));

        player
    }

    /// The account part of server_recv_and_dispatch's join and restore_account_data: round and world mode players
    /// get their saved money and ratings back, then the mode's starting money applies.
    pub(crate) fn restore_account(&mut self, pid: PlayerId) {
        self.reclaim_humans(pid);
        let Some(money) = self.players.get(pid.idx()).map(|p| p.money) else { return };
        let money = self.mode.joining_money(self, money);

        if let Some(player) = self.players.get_mut(pid.idx()) {
            player.money = money;
        }
    }

    /// A returning player takes back every human still holding their account: its team becomes theirs, and in world
    /// mode they buy back the shares it remembers.
    fn reclaim_humans(&mut self, pid: PlayerId) {
        let Some(account) = self.players.get(pid.idx()).map(|p| p.account_id) else { return };

        for id in self.humans.ids() {
            let Some(h) = self.humans.get_mut(id).filter(|h| h.account == Some(account)) else { continue };

            h.player = Some(pid);

            let (team, stocks) = (h.team.unwrap_or(Team::Spectator), h.stocks);

            let Some(player) = self.players.get_mut(pid.idx()) else { return };
            player.human = Some(id);
            player.ghost_human = false;
            player.team = team;

            if self.rules.shares_follow_human {
                self.purchase_stocks(pid, stocks);
            } else {
                player.menu = ferrosa_protocol::clientbound::game::MenuType::Empty;
                player.customization.model = Model::Suit;
                player.customization.suit_color = SuitColor::try_from(player.team as u8).unwrap_or_default();
            }
        }
    }

    /// The economy part of delete_player: the leaving player's shares are sold and their account updated, and their
    /// human remembers the account.
    pub(crate) fn settle_leave(&mut self, pid: PlayerId) {
        if let Some(p) = self.players.get(pid.idx()).filter(|p| p.is_bot) {
            let (human, account) = (p.human, p.account_id);

            if let Some(h) = human.and_then(|h| self.humans.get_mut(h)).filter(|h| h.player == Some(pid)) {
                h.player = None;
                h.account = Some(account);
            }

            return;
        }

        self.remove_player_from_corps(pid);

        self.sell_all_stocks(pid);

        for k in 0..self.corp_state.len() {
            if self.corp_state[k].manager != Some(pid) {
                continue;
            }

            self.corp_state[k].manager = None;
            self.corp_state[k].fired.clear();
        }

        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        let account = player.account_id;
        if let Some(h) = player.human.and_then(|id| self.humans.get_mut(id))
            && h.player == Some(pid)
        {
            h.player = None;
            h.account = Some(account);
        }
    }
}
