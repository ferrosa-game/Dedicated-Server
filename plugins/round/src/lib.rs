use std::collections::HashMap;

use bases::{Area, base};
use corps::Corps;
use ferrosa_wit::{Bone, ClientMode, GameState, Mat3, Menu, Model, Player, PlayerId, Plugin, SuitColour, Team, Vec3, game_minutes, game_seconds, server};

mod accounts;
mod bases;
mod chase;
mod corps;
mod inventory;
mod menus;
mod missions;

/// The round corporations: Goldmen, Monsota and OXS.
pub(crate) const ROUND_CORPS: usize = 3;
pub(crate) const TEAMS: [Team; ROUND_CORPS] = [Team::Goldmen, Team::Monsota, Team::Oxs];
pub(crate) const TEAM_NAMES: Corps<&str> = Corps(["Goldmen Inc", "Monsota", "OXS International"]);
const ROUND_START: i32 = game_seconds(90);
/// With nobody on a team the intermission timer rests at 6900.
const IDLE_TIME: i32 = game_seconds(115);
/// Ticks in a round day, and how many elapsed ticks pass before loot in the open starts to matter.
pub(crate) const DAY: i32 = game_minutes(1);
const LOOT_GRACE: i32 = game_minutes(2) - 1;

const LOBBY_TEAM_TIME: i32 = game_seconds(10) - 1;
const LOBBY_UNREADY_TIME: i32 = game_seconds(3);
/// The lobby lasts two minutes.
const LOBBY_TIME: i32 = game_minutes(2);
/// The phones on each round corporation's table: 7.75 along its back, the second 0.5 beside it, numbered from 1111.
const PHONE_BACK: f32 = 7.75;
const PHONE_SIDE: f32 = 0.5;

#[derive(Clone, Copy)]
pub(crate) struct RoundConfig {
    pub(crate) roundtime: i32,
    pub(crate) startcash: i32,
    pub(crate) weekly: bool,
    pub(crate) bonusratio: i32,
    pub(crate) teamdamage: i32,
}

impl Default for RoundConfig {
    fn default() -> Self {
        Self { roundtime: 10, startcash: 250, weekly: false, bonusratio: 1, teamdamage: 50 }
    }
}

impl RoundConfig {
    fn load() -> Self {
        let mut c = Self::default();
        let Ok(text) = std::fs::read_to_string("config_round.txt") else { return c };
        let num = |key: &str| text.lines().find_map(|l| l.trim().strip_prefix(key)).and_then(|v| v.trim().parse::<i32>().ok());

        c.roundtime = num("roundtime=").unwrap_or(c.roundtime).clamp(1, 60);
        c.startcash = num("startcash=").unwrap_or(c.startcash);
        c.weekly = num("weekly=").map_or(c.weekly, |v| v != 0);
        c.bonusratio = num("bonusratio=").unwrap_or(c.bonusratio);
        c.teamdamage = num("teamdamage=").unwrap_or(c.teamdamage).clamp(0, 200);
        c
    }
}

/// The team, if it is a round corporation: Goldmen, Monsota or OXS.
pub(crate) fn round_corp(team: Team) -> Option<Team> {
    ((team as usize) < ROUND_CORPS).then_some(team)
}

/// A corporation's share price.
pub(crate) fn share_price(team: Team) -> f32 {
    server::get_corporation(team).map_or(0.0, |c| c.price())
}

/// A turn about the up axis, as the server builds one from nothing.
pub(crate) fn yaw(angle: f32) -> Mat3 {
    let (s, c) = angle.sin_cos();

    Mat3 { x: Vec3::new(c, 0.0, s), y: Vec3::new(0.0, 1.0, 0.0), z: Vec3::new(-s, 0.0, c) }
}

/// How many players are on each round corporation, with a human or without.
pub(crate) fn team_counts() -> Corps<usize> {
    let mut counts = Corps::default();

    for p in server::players() {
        if let Some(team) = round_corp(p.team()) {
            counts[team] += 1;
        }
    }

    counts
}

/// A vehicle in the first free car space of the corporation's base.
pub(crate) fn spawn_corp_vehicle(kind: u32, team: Team, colour: i32) -> Option<ferrosa_wit::Vehicle> {
    server::get_corporation(team)?.spawn_vehicle(kind, colour)
}

/// Round mode: the corporations' rounds with a lobby, buy menus and weekly play.
#[derive(Default)]
pub(crate) struct Round {
    pub(crate) cfg: RoundConfig,
    pub(crate) accounts: accounts::Accounts,
    weekday: i32,
    pub(crate) game_timer: i32,
    round_elapsed: i32,
    round_max_time: i32,
    occupied: bool,
    /// Whether a reset has set the share prices yet.
    priced: bool,
    switch_timers: HashMap<PlayerId, i32>,
    pub(crate) managers: Corps<Option<PlayerId>>,
    pub(crate) fired: Corps<Vec<u32>>,
    pub(crate) items_bought: HashMap<PlayerId, i32>,
    pub(crate) vehicles_bought: HashMap<PlayerId, i32>,
    pub(crate) vehicle_stock: [menus::VehicleOffer; 8],
    pub(crate) missions: missions::Missions,
    inventories: inventory::Inventories,
}

impl Plugin for Round {
    fn init(&mut self) {
        server::register_mode("round", &["r".to_string()], ClientMode::Round, Some("round"));
        self.cfg = RoundConfig::load();
        self.accounts = accounts::Accounts::load();
    }

    fn reset(&mut self) {
        self.reset_round();
    }

    fn tick(&mut self) {
        self.tick_round();
    }

    fn menus_done(&mut self) {
        self.base_menus();
        self.menu_buttons();
    }

    fn on_menu(&mut self, player: PlayerId, menu: Menu, button: u32) -> bool {
        if menu == Menu::Lobby {
            self.lobby(player, button);
            return true;
        }

        let Some(p) = server::get_player(player) else { return false };

        if menus::is_buy_menu(menu) && p.menu() == menu {
            return self.buy_menu(&p, button);
        }

        false
    }

    fn on_join(&mut self, player: PlayerId) {
        let Some(p) = server::get_player(player) else { return };
        let Some(id) = p.account_id() else { return };
        let money = self.accounts.join(id, &p.name()).money;

        p.set_money(if self.cfg.weekly { self.cfg.startcash } else { money });
    }

    fn on_leave(&mut self, player: PlayerId) {
        self.inventories.forget(player);
    }

    /// Team damage: hurting a teammate deals the config's share of it to the attacker's head.
    fn on_hurt(&mut self, attacker: PlayerId, victim: PlayerId, damage: i32) {
        let (Some(a), Some(v)) = (server::get_player(attacker), server::get_player(victim)) else { return };

        if a.team() != v.team() || a.god_mode() || damage <= 0 {
            return;
        }

        let Some(human) = a.human() else { return };

        println!("[round] {} team damage: {damage}", a.name());
        human.damage(Bone::Head, damage * self.cfg.teamdamage / 100);
    }

    fn on_admin_select(&mut self) {
        self.weekday = 5;
    }
}

impl Round {
    fn tick_round(&mut self) {
        self.switch_timers.retain(|_, t| {
            *t -= 1;
            *t > 0
        });

        self.occupied = server::players().iter().any(|p| !p.bot());

        match server::get_game_state() {
            GameState::Intermission => self.intermission(),
            GameState::InGame => self.in_game(),
            GameState::Restarting => self.restarting(),
            _ => {}
        }

        self.account_sync();
        server::set_game_timer(self.game_timer);
    }

    fn intermission(&mut self) {
        let mut on_teams = 0;
        let mut all_ready = true;

        for p in server::players().iter().filter(|p| !p.bot()) {
            let corp = round_corp(p.team());

            on_teams += corp.is_some() as i32;
            all_ready &= p.ready();

            if self.game_timer < ROUND_START
                && p.human().is_none()
                && let Some(team) = corp
            {
                self.spawn(p, team);
            }
        }

        let timer = self.game_timer;

        if self.round_max_time == timer {
            self.close_doors();
        }

        self.game_timer = timer - 1;

        if on_teams == 0 && (ROUND_START + 2..=IDLE_TIME).contains(&timer) {
            self.game_timer = IDLE_TIME;
            return;
        }

        self.countdown(on_teams != 0 && all_ready);
    }

    fn close_doors(&mut self) {
        for team in TEAMS {
            server::set_team_door(team, false);
        }
    }

    fn countdown(&mut self, all_ready: bool) {
        if all_ready && self.game_timer > ROUND_START {
            self.game_timer = ROUND_START;
        }

        if self.game_timer == ROUND_START {
            self.setup();
        }

        if self.game_timer > ROUND_START - 1 {
            return;
        }

        let all_driving = server::players().iter().all(|p| round_corp(p.team()).is_none() || p.human().is_none_or(|h| h.vehicle().is_some()));

        if all_driving {
            self.game_timer -= 7;
        }

        if self.game_timer > 0 {
            return;
        }

        missions::show_all();

        for team in TEAMS {
            server::set_team_door(team, true);
        }

        server::set_game_state(GameState::InGame);
        self.game_timer = self.cfg.roundtime * DAY;
    }

    fn setup(&mut self) {
        server::set_traffic(128, false);
        menus::randomize_stock(&mut self.vehicle_stock);
        self.generate_missions();

        for team in TEAMS {
            if let Some(m) = self.managers[team].and_then(server::get_player) {
                server::announce(&format!("{} Manager: {}", TEAM_NAMES[team], m.name()));
            }
        }

        let counts = team_counts();
        let (town_car, minivan) = (menus::VehicleKind::TownCar.id(), menus::VehicleKind::Minivan.id());

        for team in TEAMS {
            if let Some(car) = town_car.and_then(|k| spawn_corp_vehicle(k, team, 0)) {
                car.park_in_traffic();
            }

            if counts[team] > 4 && let Some(k) = minivan {
                spawn_corp_vehicle(k, team, 0);
            }

            self.lay_out_phones(team);
        }
    }

    fn lay_out_phones(&mut self, team: Team) {
        let (Some(base), Some(phone)) = (base(team), server::find_item_kind("phone")) else { return };
        let rot = yaw(base.table_orientation + std::f32::consts::FRAC_PI_2);
        let first = rot.z * PHONE_BACK + base.table;
        let second = rot.x * PHONE_SIDE + first;
        let number = (team as i32 + 1) * 1111 + 1;

        for (pos, texture, number) in [(first, 0, number - 1), (second, 1, number)] {
            let Some(item) = server::spawn_item(phone, pos, rot) else { continue };

            item.set_phone_number(number);
            item.set_phone_texture(texture);
        }
    }

    fn spawn(&mut self, p: &Player, team: Team) {
        let Some(base) = base(team) else { return };
        let spread = || (rand::random_range(0..256) - 127) as f32 * 3.0 * 1.0 / 128.0;
        let pos = Vec3::new(spread() + base.spawn.x, base.spawn.y - 0.25, spread() + base.spawn.z);
        let angle = std::f32::consts::FRAC_PI_2 + base.table_orientation;

        p.set_model(Model::Suit);

        let suits = Corps([SuitColour::Brown, SuitColour::Black, SuitColour::Grey]);

        if suits.0.contains(&p.suit_colour()) {
            p.set_suit_colour(suits[team]);
        }

        let ties = Corps([2, 9, 8]);
        p.set_tie_colour(ties[team]);

        let Some(human) = server::spawn_human(pos, yaw(angle), Some(p.id())) else { return };

        p.set_human(&human);
        p.set_menu(Menu::Empty);
        human.set_view_yaw(angle);
        self.inventories.restore(p, team);
    }

    fn in_game(&mut self) {
        self.game_timer -= 1;
        self.round_elapsed += 1;

        if self.game_timer % DAY == 0 {
            self.items_bought.clear();

            for bought in self.vehicles_bought.values_mut() {
                *bought = (*bought - 1).max(0);
            }
        }

        if self.round_elapsed > LOOT_GRACE && !loot_all_in_bases() && all_members_home() {
            self.game_timer -= 10;
        }

        for team in TEAMS {
            if !server::team_door_open(team) && !server::players().iter().any(|p| p.team() == team && p.human().is_some()) {
                server::set_team_door(team, true);
            }
        }

        if loot_all_in_vaults() && self.game_timer > game_seconds(1) {
            self.game_timer = game_seconds(1);
        }

        if !server::players().iter().any(|p| !p.bot() && p.human().is_some()) && self.game_timer > game_seconds(1) {
            self.game_timer = game_seconds(1);
        }

        if self.game_timer <= 0 {
            self.complete();
            server::set_game_state(GameState::Restarting);
            self.game_timer = game_seconds(21);
        }
    }

    fn restarting(&mut self) {
        self.game_timer -= 1;

        if self.game_timer == game_seconds(21) - 10 {
            server::announce("Top 10 list");
        } else if self.game_timer <= game_seconds(20) && self.game_timer > 0 && (game_seconds(20) - self.game_timer) % game_seconds(2) == 0 {
            self.print_top10(((game_seconds(20) - self.game_timer) / game_seconds(2)) as usize);
        }

        if self.game_timer > 0 {
            return;
        }

        let players = server::players();

        for p in &players {
            p.set_manager_tab(false);
        }

        for team in TEAMS {
            let price = share_price(team);
            let mut best: Option<(i32, &Player)> = None;

            for p in players.iter().filter(|p| p.team() == team) {
                let worth = if self.cfg.weekly {
                    p.account_id().and_then(|id| self.accounts.money(id)).unwrap_or(0)
                } else {
                    (p.stocks() as f32 * price + p.money() as f32) as i32
                };

                if worth > best.map_or(0, |(w, _)| w) {
                    best = Some((worth, p));
                }
            }

            if let Some((_, p)) = best {
                p.set_manager_tab(true);

                if self.managers[team] != Some(p.id()) {
                    self.managers[team] = Some(p.id());
                    self.fired[team].clear();
                }
            }
        }

        for p in &players {
            self.inventories.save(p);
        }

        server::request_reset();
    }

    fn print_top10(&self, n: usize) {
        if let Some(a) = self.accounts.top(n) {
            server::announce(&format!("{}. {}   ${}", n + 1, a.name, accounts::with_commas(a.money)));
        }
    }

    fn weekly_reset(&mut self) {
        for corp in TEAMS.into_iter().filter_map(server::get_corporation) {
            corp.set_shares(100);
            corp.set_price(100.0);
        }

        for p in server::players() {
            p.set_money(self.cfg.startcash);
            p.set_stocks(0);
        }

        self.inventories.clear();

        self.weekday = 1;
    }

    fn account_sync(&mut self) {
        if self.cfg.weekly {
            return;
        }

        for p in server::players() {
            let Some(id) = p.account_id() else { continue };
            let worth = match round_corp(p.team()) {
                Some(team) => (p.money() as f32 + p.stocks() as f32 * (0.98 * share_price(team))) as i32,
                None => p.money(),
            };

            self.accounts.set_money(id, worth);
        }
    }

    fn reset_round(&mut self) {
        let occupied = server::players().iter().any(|p| !p.bot());

        if occupied && !self.occupied {
            self.weekday = 5;
        }

        self.occupied = occupied;
        self.accounts.save();
        self.weekday += 1;

        if self.cfg.weekly && self.weekday > 5 {
            self.weekly_reset();
        }

        server::set_game_state(GameState::Intermission);
        self.round_elapsed = 0;
        self.round_max_time = LOBBY_TIME;
        self.game_timer = LOBBY_TIME;
        self.missions.counter = 0;

        self.items_bought.clear();
        self.vehicles_bought.clear();
        self.reset_corporations();
        server::reset_lobby();
        server::set_game_timer(self.game_timer);
        server::set_week(self.cfg.weekly, self.weekday as u8);
        server::set_keep_bodies(true);
        server::set_keep_items(true);
        server::set_stock_sell_rate(0.98);
        server::set_count_humanless_members(true);
    }

    /// reset_corporation_rounds: each round corporation's funds cleared and its shares counted again, and on the first
    /// reset its price set to 100.
    fn reset_corporations(&mut self) {
        let players = server::players();

        for corp in TEAMS.into_iter().filter_map(server::get_corporation) {
            let held: i32 = players.iter().filter(|p| p.team() == corp.team()).map(|p| p.stocks()).sum();

            corp.set_funds(0);
            corp.set_shares(100 + held);

            if !self.priced {
                corp.set_price(100.0);
            }
        }

        self.priced = true;
    }

    fn lobby(&mut self, player: PlayerId, button: u32) {
        if server::get_game_state() != GameState::Intermission {
            return;
        }

        let timer = self.game_timer;
        let Some(p) = server::get_player(player) else { return };

        if timer > LOBBY_TEAM_TIME && !self.switch_timers.contains_key(&player) {
            match button {
                1..=3 => {
                    let team = TEAMS[button as usize - 1];

                    if p.account_id().is_some_and(|a| self.fired[team].contains(&a)) {
                        return;
                    }

                    p.set_team(team);
                    p.set_ready(false);
                    self.switch_timers.insert(player, game_seconds(1));
                }
                4 => {
                    if let Some(team) = round_corp(p.team())
                        && self.managers[team] == Some(player)
                    {
                        self.managers[team] = None;
                        self.fired[team].clear();
                    }

                    p.sell_all_stocks();
                    p.set_team(Team::Spectator);
                    p.set_manager_tab(false);
                    p.set_ready(false);
                    self.switch_timers.insert(player, game_seconds(1));
                }
                _ => {}
            }
        }

        if button == 5 {
            if !p.ready() {
                p.set_ready(true);
            } else if timer > LOBBY_UNREADY_TIME {
                p.set_ready(false);
            }
        }
    }
}

fn is_loot(kind: u32) -> bool {
    let disks = server::find_item_kind("diskblack").zip(server::find_item_kind("diskred"));

    disks.is_some_and(|(first, last)| (first..=last).contains(&kind)) || server::find_item_kind("cashround") == Some(kind)
}

fn loot_all_in_bases() -> bool {
    let bases: Vec<_> = TEAMS.into_iter().filter_map(base).collect();

    server::items().iter().filter(|i| is_loot(i.kind())).all(|i| bases.iter().any(|b| b.contains(i.pos())))
}

fn loot_all_in_vaults() -> bool {
    let bases: Vec<_> = TEAMS.into_iter().filter_map(base).collect();

    server::items().iter().filter(|i| is_loot(i.kind())).all(|i| bases.iter().any(|b| b.in_vault(i.pos())))
}

fn all_members_home() -> bool {
    server::players().iter().all(|p| {
        let (Some(team), Some(h)) = (round_corp(p.team()), p.human()) else { return true };

        base(team).is_some_and(|b| b.contains(h.pos()))
    })
}

ferrosa_wit::plugin!(Round);
