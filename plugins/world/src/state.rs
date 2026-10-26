use ferrosa_wit::{PlayerId, Team};
use std::collections::HashMap;

pub const CORPORATIONS: [Team; 6] = [Team::Goldmen, Team::Monsota, Team::Oxs, Team::Nexaco, Team::Pentacom, Team::Prodocon];

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

#[derive(Default)]
pub struct Corp {
    pub manager: Option<PlayerId>,
    pub applicants: Vec<PlayerId>,
    pub money: i32,
    pub credit: i32,
    pub computers: Vec<i32>,
}

pub struct SavedBody {
    pub pos: ferrosa_wit::Vec3,
    pub yaw: f32,
    pub vehicle: Option<u32>,
    pub seat: u32,
}

pub struct SavedItem {
    pub slot: u32,
    pub kind: u32,
    pub uses: i32,
    pub magazine: Option<(u32, i32)>,
    pub cash: Option<ferrosa_wit::Cash>,
    pub key_vehicle: Option<u32>,
}

pub struct SavedVehicle {
    pub old: u32,
    pub new: Option<u32>,
    pub kind: u32,
    pub colour: i32,
    pub pos: ferrosa_wit::Vec3,
    pub rot: ferrosa_wit::Mat3,
}

pub struct World {
    pub cfg: Config,
    /// How much the clock moves each tick; 0 outside a day.
    pub clock_rate: i32,
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
    pub corps: [Corp; 6],
    pub missions: crate::missions::Missions,
    /// Where each player stood when the day ended, and what they held.
    pub saved_bodies: HashMap<PlayerId, SavedBody>,
    pub saved_items: HashMap<PlayerId, Vec<SavedItem>>,
    /// How many days have been played; the managers only clear before the first.
    pub round_number: u32,
    /// The saved money, ratings and play times, which this plugin keeps in its own accounts.txt.
    pub accounts: crate::store::Accounts,
}

impl Default for World {
    fn default() -> Self {
        Self {
            cfg: Config::default(),
            clock_rate: 0,
            max_time: 108_000,
            memos: 0,
            memo_timers: [0; CORPORATIONS.len()],
            newspaper: Vec::new(),
            bills_withdrawn: HashMap::new(),
            saved_vehicles: Vec::new(),
            corps: Default::default(),
            missions: Default::default(),
            saved_bodies: HashMap::new(),
            saved_items: HashMap::new(),
            round_number: 0,
            accounts: crate::store::Accounts::load(),
        }
    }
}

impl World {
    pub fn team_index(team: Team) -> Option<usize> {
        CORPORATIONS.iter().position(|&t| t == team)
    }
}
