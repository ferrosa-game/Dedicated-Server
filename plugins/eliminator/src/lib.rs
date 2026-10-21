use std::collections::HashMap;

use ferrosa_wit::{
    Bone, ChatType, ClientMode, DamageCause, GameState, HumanId, Intersection, Mat3, Menu, Model, Player, PlayerId, Plugin, SuitColour, Team,
    Vec3, game_minutes, game_seconds, server,
};

mod immortal;
mod supplies;

const BLOCKED_CORNERS: [usize; 4] = [18, 40, 50, 51];
pub(crate) const TEAMS: [Team; 3] = [Team::Goldmen, Team::Monsota, Team::Oxs];

const ROLE: i32 = 0;
const POSITION: i32 = 1;

#[derive(Clone, Copy)]
struct Group {
    target: Option<PlayerId>,
    eliminator: Option<PlayerId>,
    protector: Option<PlayerId>,
    revealed: Vec3,
    current: Vec3,
}

fn id(player: Option<PlayerId>) -> i32 {
    player.map_or(0, |p| p as i32)
}

fn position(player: Option<PlayerId>) -> Option<Vec3> {
    Some(server::get_player(player?)?.human()?.pos())
}

fn alive(player: Option<PlayerId>) -> bool {
    player.and_then(server::get_player).is_some_and(|p| p.human().is_some())
}

fn tell(player: Option<PlayerId>, kind: i32, role: i32, value: i32, pos: Option<Vec3>) {
    if let Some(p) = player.and_then(server::get_player) {
        p.send_mission_event(kind, role, value, pos);
    }
}

fn deal(mut list: Vec<PlayerId>) -> Vec<Group> {
    let least = rand::random_range(3..=5);
    let mut groups = Vec::new();

    loop {
        let mut take = || (!list.is_empty()).then(|| list.remove(rand::random_range(0..list.len())));
        let (eliminator, target, protector) = (take(), take(), take());

        groups.push(Group { target, eliminator, protector, revealed: Vec3::ZERO, current: Vec3::ZERO });

        if groups.len() >= 8 || list.len() < least {
            return groups;
        }
    }
}

#[derive(Default)]
struct Eliminator {
    timer: i32,
    elapsed: i32,
    groups: Vec<Group>,
    last_target: Option<Vec3>,
    ready: i32,
    switch_timers: HashMap<PlayerId, i32>,
    immortals: immortal::Immortals,
}

impl Plugin for Eliminator {
    fn init(&mut self) {
        server::register_mode("eliminator", &["e".to_string()], ClientMode::Eliminator, Some("round"));
    }

    fn reset(&mut self) {
        server::set_game_state(GameState::Intermission);
        self.timer = game_seconds(30);
        self.elapsed = 0;
        self.groups.clear();
        self.switch_timers.clear();
        self.immortals.clear();

        for corp in TEAMS.into_iter().filter_map(server::get_corporation) {
            corp.set_funds(0);
        }

        server::reset_lobby();
        server::set_keep_bodies(true);
        server::set_keep_items(true);
        server::set_door_knocks(false);
        server::set_game_timer(self.timer);
    }

    fn tick(&mut self) {
        self.switch_timers.retain(|_, t| {
            *t -= 1;
            *t > 0
        });

        match server::get_game_state() {
            GameState::Intermission => self.lobby(),
            GameState::InGame => self.hunt(),
            GameState::Restarting => {
                self.timer -= 1;

                if self.timer <= 0 {
                    server::request_reset();
                }
            }
            _ => {}
        }

        self.immortals.tick();
        server::set_game_timer(self.timer);
    }

    fn menus_done(&mut self) {
        let lobby = server::get_game_state() == GameState::Intermission;

        for p in server::players() {
            match p.human() {
                None => p.set_menu(if lobby { Menu::Lobby } else { Menu::Empty }),
                Some(_) if p.menu() != Menu::Empty => p.set_menu(Menu::Empty),
                Some(_) => {}
            }
        }
    }

    fn on_menu(&mut self, player: PlayerId, menu: Menu, button: u32) -> bool {
        if menu != Menu::Lobby {
            return false;
        }

        self.lobby_button(player, button);
        true
    }

    fn on_damage(&mut self, human: HumanId, cause: DamageCause, _bone: Option<Bone>, damage: i32) -> i32 {
        self.immortals.on_damage(human, cause, damage)
    }

    fn on_death(&mut self, player: PlayerId) {
        let Some(p) = server::get_player(player) else { return };
        server::send_chat(ChatType::EliminatorAnnouncement, -1, 0, &format!("{} has died", p.name()));

        let thirds = self.ready / 3;

        if self.timer <= game_minutes(14) && thirds > 0 {
            self.timer = (game_minutes(5) / thirds + self.timer).min(game_minutes(14));
        }
    }
}

impl Eliminator {
    fn lobby(&mut self) {
        let people: Vec<bool> = server::players().iter().filter(|p| !p.bot()).map(|p| p.ready()).collect();

        self.timer -= 1;

        if people.iter().filter(|&&r| r).count() <= 2 {
            self.timer = game_seconds(30);
            return;
        }

        if self.timer > game_seconds(3) && people.iter().all(|&r| r) {
            self.timer = game_seconds(3);
            return;
        }

        if self.timer <= 0 {
            self.setup();
        }
    }

    fn setup(&mut self) {
        server::set_traffic(128, true);

        let ready: Vec<Player> = server::players().into_iter().filter(|p| p.ready() && !p.bot()).collect();
        self.ready = ready.len() as i32;
        self.groups = deal(ready.iter().map(|p| p.id()).collect());

        let corners = server::intersections();
        let mut free: Vec<usize> = (0..corners.len()).filter(|k| !BLOCKED_CORNERS.contains(k)).collect();

        for p in &ready {
            self.spawn(p, &corners, &mut free);
        }

        supplies::stock_tables();

        for team in TEAMS {
            server::set_team_door(team, true);
        }

        for g in &mut self.groups {
            g.current = position(g.target).unwrap_or(Vec3::ZERO);
        }

        server::set_game_state(GameState::InGame);
        self.timer = game_minutes(15);
        self.elapsed = 0;
    }

    fn spawn(&mut self, p: &Player, corners: &[Intersection], free: &mut Vec<usize>) {
        if free.is_empty() {
            return;
        }

        let corner = &corners[free.swap_remove(rand::random_range(0..free.len()))];
        let (_, _, west, north) = corner.lanes;
        let pos = Vec3::new(corner.pos.x + (west - 1) as f32 * 4.0, corner.pos.y + 1.0, corner.pos.z + (north - 1) as f32 * 4.0);

        p.set_team(Team::Goldmen);
        p.set_model(Model::Suit);
        p.set_suit_colour(SuitColour::Brown);

        let Some(human) = server::spawn_human(pos, Mat3::IDENTITY, Some(p.id())) else { return };

        p.set_human(&human);
        p.set_menu(Menu::Empty);
        human.set_view_yaw(0.0);

        let me = p.id();

        if self.groups.iter().any(|g| g.eliminator == Some(me)) {
            self.immortals.add(&human);
        }

        let mut told = false;

        for g in &self.groups {
            if g.eliminator == Some(me) {
                told = true;
                p.send_mission_event(ROLE, 1, 0, None);
            } else if g.protector == Some(me) {
                told = true;
                p.send_mission_event(ROLE, 2, ((me as i32) << 10) + id(g.target), None);
            }
        }

        if !told {
            p.send_mission_event(ROLE, 0, 0, None);
        }
    }

    fn hunt(&mut self) {
        self.timer -= 1;
        self.elapsed += 1;

        let ready = server::players().iter().filter(|p| !p.bot() && p.ready()).count();

        if let Some(pos) = self.groups.iter().filter_map(|g| position(g.target)).next_back() {
            self.last_target = Some(pos);
        }

        let interval = if self.elapsed < game_minutes(5) {
            game_seconds(45)
        } else if self.elapsed < game_minutes(8) {
            game_seconds(30)
        } else if self.elapsed < game_minutes(11) {
            game_seconds(15)
        } else {
            game_seconds(5)
        };

        if self.elapsed > game_minutes(2) && self.timer % interval == 0 {
            self.reveal();
        }

        if self.elapsed > game_minutes(3) && self.elapsed % game_minutes(1) == 0 {
            supplies::drop(ready);
        }

        let targets = self.groups.iter().any(|g| alive(g.target));
        let eliminators = self.groups.iter().any(|g| alive(g.eliminator));

        if !targets {
            self.end([0, 100, 0]);
        } else if !eliminators || self.timer <= 0 {
            self.end([100, 0, 100]);
        }
    }

    fn reveal(&mut self) {
        let late = self.elapsed > game_minutes(3);
        let last = self.last_target.unwrap_or(Vec3::ZERO);

        for g in &mut self.groups {
            g.revealed = g.current;

            if late {
                tell(g.eliminator, POSITION, 0, 0, Some(g.revealed));
            }

            g.current = position(g.target).unwrap_or(last);
            tell(g.protector, POSITION, 0, 0, Some(g.current));
        }
    }

    fn end(&mut self, scores: [i32; 3]) {
        for (team, score) in TEAMS.into_iter().zip(scores) {
            if let Some(corp) = server::get_corporation(team) {
                corp.set_funds(score);
            }
        }

        server::set_game_state(GameState::Restarting);
        self.timer = game_seconds(20);

        for p in server::players().iter().filter(|p| !p.bot()) {
            for (i, g) in self.groups.iter().enumerate() {
                p.send_mission_event(2, i as i32, (id(g.eliminator) << 10) + id(g.target) + (id(g.protector) << 20), None);
            }
        }
    }

    fn lobby_button(&mut self, player: PlayerId, button: u32) {
        if server::get_game_state() != GameState::Intermission {
            return;
        }

        let Some(p) = server::get_player(player) else { return };

        if self.timer > game_seconds(10) && !self.switch_timers.contains_key(&player) {
            match button {
                1..=3 => {
                    p.set_team(TEAMS[button as usize - 1]);
                    p.set_ready(false);
                    self.switch_timers.insert(player, game_seconds(1));
                }
                4 => {
                    p.sell_all_stocks();
                    p.set_team(Team::Spectator);
                    p.set_ready(false);
                    self.switch_timers.insert(player, game_seconds(1));
                }
                _ => {}
            }
        }

        if button == 5 {
            if !p.ready() {
                p.set_ready(true);
            } else if self.timer > game_seconds(3) {
                p.set_ready(false);
            }
        }
    }
}

ferrosa_wit::plugin!(Eliminator);
