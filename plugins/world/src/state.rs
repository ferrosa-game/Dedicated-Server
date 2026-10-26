use ferrosa_wit::{PlayerId, Team};
use std::collections::HashMap;

/// The six corporations, in the order their teams are numbered.
pub const CORPORATIONS: [Team; 6] = [Team::Goldmen, Team::Monsota, Team::Oxs, Team::Nexaco, Team::Pentacom, Team::Prodocon];

/// roundtime= (minutes) times 3600, 30 minutes when unset: what the world timer shows.
pub const DEFAULT_MAX_TIME: i32 = 108000;

/// config_world.txt (load_conf_world): the traffic, the crime rates and limits, the money and the join messages.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub traffic: i32,
    pub crimecivciv: i32,
    pub crimecivteam: i32,
    pub crimeteamciv: i32,
    pub crimeteamteam: i32,
    pub crimeteamteaminbase: i32,
    pub crimevscriminal: i32,
    pub crimenobuy: i32,
    pub crimenospawn: i32,
    pub crimekick: i32,
    pub startcash: i32,
    pub mincash: i32,
    pub showjoinexit: bool,
    pub respawnteam: bool,
    /// Shares are sold when the human dies, as world mode does.
    pub shares_follow_human: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            traffic: 256,
            crimecivciv: 100,
            crimecivteam: 200,
            crimeteamciv: 50,
            crimeteamteam: 0,
            crimeteamteaminbase: 100,
            crimevscriminal: 20,
            crimenobuy: 200,
            crimenospawn: 500,
            crimekick: 1000,
            startcash: 1000,
            mincash: 500,
            showjoinexit: true,
            respawnteam: false,
            shares_follow_human: true,
        }
    }
}

impl Config {
    /// The config read back out of config_world.txt's `key=value` lines; anything missing keeps its default.
    pub fn load() -> Self {
        let mut c = Self::default();
        let Ok(text) = std::fs::read_to_string("config_world.txt") else { return c };

        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            let Ok(n) = value.trim().parse::<i32>() else { continue };

            match key.trim() {
                "traffic" => c.traffic = n,
                "crimecivciv" => c.crimecivciv = n,
                "crimecivteam" => c.crimecivteam = n,
                "crimeteamciv" => c.crimeteamciv = n,
                "crimeteamteam" => c.crimeteamteam = n,
                "crimeteamteaminbase" => c.crimeteamteaminbase = n,
                "crimevscriminal" => c.crimevscriminal = n,
                "crimenobuy" => c.crimenobuy = n,
                "crimenospawn" => c.crimenospawn = n,
                "crimekick" => c.crimekick = n,
                "startcash" => c.startcash = n,
                "mincash" => c.mincash = n,
                "showjoinexit" => c.showjoinexit = n != 0,
                "respawnteam" => c.respawnteam = n != 0,
                _ => {}
            }
        }

        c
    }
}

/// What the world mode keeps for one corporation. The server holds the share market, the missions and the base; this
/// is the rest, which only world mode has.
#[derive(Default)]
pub struct Corp {
    /// Whoever is running it, if anyone.
    pub manager: Option<PlayerId>,
    /// Up to 32 players waiting for the manager to hire them.
    pub applicants: Vec<PlayerId>,
    /// Its bank: what requisitions are paid from, and what the manager draws bills against.
    pub money: i32,
    /// How far its bank may go below nothing, which is the manager's corporate rating.
    pub credit: i32,
    /// The volumes of its base's computers.
    pub computers: Vec<i32>,
}

/// Where a player's human stood when the day ended.
pub struct SavedBody {
    pub pos: ferrosa_wit::Vec3,
    pub yaw: f32,
    /// The vehicle it was in, as that vehicle's old number, and which seat.
    pub vehicle: Option<u32>,
    pub seat: u32,
}

/// One of a player's items kept for the next day.
pub struct SavedItem {
    pub slot: u32,
    pub kind: u32,
    pub uses: i32,
    /// A gun's magazine, with what is left in it.
    pub magazine: Option<(u32, i32)>,
    pub cash: Option<ferrosa_wit::Cash>,
}

/// A vehicle a key was saved with at the end of the day: respawned at the reset where it was, the key then pointing
/// at the new one.
pub struct SavedVehicle {
    pub old: u32,
    pub new: Option<u32>,
    pub kind: u32,
    pub colour: i32,
    pub pos: ferrosa_wit::Vec3,
    pub rot: ferrosa_wit::Mat3,
}

/// The world mode state beyond the clock: how fast it runs, the memos placed, each corporation's memo countdown and
/// the players' own figures, which the accounts keep across days.
pub struct World {
    pub cfg: Config,
    /// How much the clock moves each tick; 0 outside a day.
    pub rate: i32,
    /// What the world timer counts down from.
    pub max_time: i32,
    /// How many memos stand at the spawn.
    pub memos: i32,
    /// Quarter hours left before each corporation's next memo.
    pub memo_timers: [i32; CORPORATIONS.len()],
    /// What the memos at the spawn say.
    pub newspaper: Vec<u8>,
    /// How many small bills each player has drawn from the bank today.
    pub bills_withdrawn: HashMap<PlayerId, i32>,
    /// The vehicles saved keys point at, across the reset.
    pub saved_vehicles: Vec<SavedVehicle>,
    /// Each player's corporate rating.
    pub ratings: HashMap<PlayerId, i32>,
    /// How long each player must wait before entering the city again.
    pub spawn_timers: HashMap<PlayerId, i32>,
    pub corps: [Corp; 6],
    pub missions: crate::missions::Missions,
    /// Where each player stood when the day ended, and what they held.
    pub saved_bodies: HashMap<PlayerId, SavedBody>,
    pub saved_items: HashMap<PlayerId, Vec<SavedItem>>,
    /// How many days have been played; the managers only clear before the first.
    pub round_number: u32,
}

impl Default for World {
    fn default() -> Self {
        Self {
            cfg: Config::default(),
            rate: 0,
            max_time: DEFAULT_MAX_TIME,
            memos: 0,
            memo_timers: [0; CORPORATIONS.len()],
            newspaper: Vec::new(),
            bills_withdrawn: HashMap::new(),
            saved_vehicles: Vec::new(),
            ratings: HashMap::new(),
            spawn_timers: HashMap::new(),
            corps: Default::default(),
            missions: Default::default(),
            saved_bodies: HashMap::new(),
            saved_items: HashMap::new(),
            round_number: 0,
        }
    }
}

impl World {
    /// A player's corporate rating, which the accounts keep across days (the manager's becomes the corporation's
    /// credit).
    pub fn corp_rating(&self, player: PlayerId) -> i32 {
        self.ratings.get(&player).copied().unwrap_or(0)
    }

    /// How long until the player may enter the city again.
    pub fn spawn_timer(&self, player: PlayerId) -> i32 {
        self.spawn_timers.get(&player).copied().unwrap_or(0)
    }

    pub fn team_index(team: Team) -> Option<usize> {
        CORPORATIONS.iter().position(|&t| t == team)
    }
}
