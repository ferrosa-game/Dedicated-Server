use ferrosa_wit::{Mat3, Mission, Team, Vec3, clock_time, server};
use rand::seq::SliceRandom;

use crate::{
    Round, TEAMS,
    bases::{Area, base},
    chase,
    corps::Corps,
    team_counts,
};

/// The disks come in turn, black to red; gold missions are worth four times as much.
const DISKS: [&str; 6] = ["diskblack", "diskgreen", "diskblue", "diskwhite", "diskgold", "diskred"];
const GOLD: usize = 4;
/// Steps along the table: half a unit between items, three quarters around a briefcase.
const ITEM_STEP: f32 = 0.5;
const CASE_STEP: f32 = 0.75;

/// Where a disk can be left for a transaction.
const MISSION_SPOTS: [Vec3; 8] = [
    // Hondo Park
    Vec3::new(1718.0, 46.0, 1262.0),
    // Rio Granary
    Vec3::new(1778.0, 30.0, 1006.0),
    // Isle of Burgers
    Vec3::new(1470.0, 50.0, 1494.0),
    // Park above the Kamel Building
    Vec3::new(1282.0, 62.0, 1534.0),
    // Bobson Dugnutt Museum
    Vec3::new(1674.0, 66.0, 1510.0),
    // The Mall
    Vec3::new(1346.0, 38.0, 1298.0),
    // Gas Station
    Vec3::new(1606.0, 26.0, 1166.0),
    // Red Cube Park
    Vec3::new(1546.0, 38.0, 1278.0),
];

enum MissionTypes {
    Acquisition = 1,
    Sell = 2,
    BuyFunded = 3,
    Buy = 4,
    TwoTeamSell = 7,
    TwoTeamBuy = 8
}

#[derive(Clone, Copy)]
enum Deal {
    Transaction(usize),
    TwoTeam,
    CarChase(usize),
}

impl Deal {
    fn size(self) -> usize {
        match self {
            Self::Transaction(n) | Self::CarChase(n) => n,
            Self::TwoTeam => 3,
        }
    }
}

/// The deals for two corporations and for three, with how often each comes up (out of their total). A deal for fewer
/// than all of them leaves the rest to be dealt again; one corporation alone always gets a car chase.
const FOR_TWO: [(u32, Deal); 3] = [(12, Deal::Transaction(2)), (3, Deal::CarChase(2)), (1, Deal::CarChase(1))];
const FOR_THREE: [(u32, Deal); 6] = [
    (540, Deal::Transaction(3)),
    (244, Deal::TwoTeam),
    (108, Deal::CarChase(3)),
    (48, Deal::Transaction(2)),
    (36, Deal::CarChase(2)),
    (48, Deal::CarChase(1)),
];

/// One of `deals` at random, by their weights.
fn pick(deals: &[(u32, Deal)]) -> Deal {
    let mut roll = rand::random_range(0..deals.iter().map(|d| d.0).sum::<u32>());

    for &(weight, deal) in deals {
        if roll < weight {
            return deal;
        }

        roll -= weight;
    }

    deals[0].1
}

/// Round's mission bookkeeping: what each corporation spent on its missions, and the deals' counters. The missions
/// themselves are the corporations' mission slots on the server.
pub(crate) struct Missions {
    pub(crate) spent: Corps<i32>,
    pub(crate) counter: i32,
    disk_index: usize,
    last_disks: [i32; 2],
}

impl Default for Missions {
    fn default() -> Self {
        Self { spent: Corps::default(), counter: 0, disk_index: 0, last_disks: [-1; 2] }
    }
}

/// A corporation's mission slots.
pub(crate) fn missions(team: Team) -> Vec<Mission> {
    server::get_corporation(team).map_or_else(Vec::new, |c| c.missions())
}

fn set_mission(team: Team, slot: usize, mission: Mission) {
    if let Some(c) = server::get_corporation(team) {
        c.set_mission(slot as u32, mission);
    }
}

/// Sends every active mission to its corporation again, as the round starts.
pub(crate) fn show_all() {
    for team in TEAMS {
        for (slot, m) in missions(team).into_iter().enumerate().filter(|(_, m)| m.active) {
            set_mission(team, slot, m);
        }
    }
}

fn step(p: Vec3, r: Vec3, s: f32) -> Vec3 {
    r * s + p
}

/// The world clock time a mission of `time` is due by, as its briefing shows it.
fn deadline(time: i32) -> i32 {
    server::world_time() + time
}

fn item_kind(id: i32) -> i32 {
    u32::try_from(id).ok().and_then(server::get_item).map_or(0, |i| i.kind() as i32)
}

fn spawn(name: &str, pos: Vec3, rot: Mat3) -> Option<ferrosa_wit::Item> {
    server::spawn_item(name, pos, rot)
}

fn value_step() -> i32 {
    rand::random_range(0..5) * 250
}

impl Round {
    /// generate_round_missions: the round corporations with players, shuffled, then dealt out from the front: each deal
    /// takes some of them, and the rest are dealt again. Missions are only dealt at a round's setup, after the reset has
    /// cleared every mission, so none are still running.
    pub(crate) fn generate_missions(&mut self) {
        self.missions.spent = Corps::default();

        let counts = team_counts();
        let mut list: Vec<Team> = TEAMS.into_iter().filter(|&team| counts[team] > 0).collect();

        list.shuffle(&mut rand::rng());

        let mut rest = &list[..];

        while !rest.is_empty() {
            let deal = match rest.len() {
                1 => Deal::CarChase(1),
                2 => pick(&FOR_TWO),
                _ => pick(&FOR_THREE),
            };

            let taken = self.deal(deal, rest);
            rest = &rest[taken..];
        }
    }

    /// Makes `deal`'s missions for the corporations at the front of `list`; how many it took.
    fn deal(&mut self, deal: Deal, list: &[Team]) -> usize {
        match deal {
            Deal::Transaction(n) => self.transaction(&list[..n]),
            Deal::TwoTeam => self.two_team(&list[..3]),
            Deal::CarChase(n) => self.car_chase(&list[..n]),
        }

        deal.size()
    }

    /// Takes the next disk colour, black to red and over again.
    fn next_disk(&mut self) -> usize {
        let old = self.missions.disk_index;

        self.missions.disk_index = (old + 1) % DISKS.len();
        old
    }

    /// The table frame of a corporation and the place half a unit along it the items are laid out from.
    fn table_start(team: Team) -> Option<(Mat3, Vec3)> {
        let base = base(team)?;
        let rot = base.frame();

        Some((rot, step(base.table, rot.x, ITEM_STEP)))
    }

    /// A buyer's round cash: `count` bundles along the table, a briefcase before every fourth, each taken from the
    /// corporation's spending.
    fn lay_out_cash(&mut self, team: Team, count: i32) {
        let Some((rot, mut p)) = Self::table_start(team) else { return };

        for j in 0..count {
            if j & 3 == 0 {
                p = step(p, rot.x, CASE_STEP);
                spawn("briefcase", p, rot);
                p = step(p, rot.x, CASE_STEP);
            }

            spawn("cashround", p, rot);
            self.missions.spent[team] -= 250;
            p = step(p, rot.x, ITEM_STEP);
        }
    }

    /// A disk on a corporation's table with a briefcase beside it.
    fn lay_out_disk(&mut self, team: Team, disk: &str, value: i32) -> i32 {
        let Some((rot, p)) = Self::table_start(team) else { return -1 };

        let id = spawn(disk, p, rot).map_or(-1, |i| i.id() as i32);

        self.missions.spent[team] -= value;
        spawn("briefcase", step(step(p, rot.x, ITEM_STEP), rot.x, CASE_STEP), rot);

        id
    }

    /// The first free mission slot of a corporation, with a new active mission of this deal for it.
    fn open_mission(&self, team: Team) -> Option<(usize, Mission)> {
        let slot = missions(team).iter().position(|m| !m.active)?;
        let m = Mission {
            active: true,
            done: false,
            id: self.missions.counter,
            kind: 0,
            item: -1,
            item2: -1,
            team1: 0,
            team2: 0,
            disk_type: 0,
            value: 0,
            deadline: 0,
            location: -1,
            provided_cash: 0,
            rate: 0.0,
        };

        Some((slot, m))
    }

    /// generate_transaction_missions: the first corporation sells a disk (on its table, or three times in four in a
    /// briefcase at a mission spot), one or two others are given round cash to buy it and the rest may buy it unfunded.
    fn transaction(&mut self, list: &[Team]) {
        let n = list.len();
        let twice = rand::random_bool(0.25) as usize + 1;
        let funded = if twice < n { twice } else { 1 };
        let colour = self.next_disk();
        let disk = DISKS[colour];
        let location = if rand::random_bool(0.75) { rand::random_range(0..MISSION_SPOTS.len() as i32) } else { -1 };

        let values: Vec<i32> = (0..n)
            .map(|i| {
                if i == 0 {
                    return 100;
                }

                let mut v = value_step() + 1000;

                if funded < i {
                    v += value_step();
                }

                if colour == GOLD { v * 4 } else { v }
            })
            .collect();

        for (i, &team) in list.iter().enumerate() {
            if i == 0 {
                self.missions.last_disks[0] = if location == -1 {
                    self.lay_out_disk(team, disk, values[0])
                } else {
                    self.missions.spent[team] -= values[0];
                    self.disk_in_briefcase(disk, MISSION_SPOTS[location as usize])
                };
            } else if i <= funded {
                let mut count = rand::random_range(0..3) + 2;

                if colour == GOLD {
                    count += rand::random_range(0..3) + 2;
                }

                self.lay_out_cash(team, count);
            }
        }

        let item = self.missions.last_disks[0];
        let disk_type = item_kind(item);

        for (i, &team) in list.iter().enumerate() {
            let Some((slot, mut m)) = self.open_mission(team) else { continue };
            let spent = self.missions.spent[team];

            m.item = item;
            m.disk_type = disk_type;
            m.value = values[i];
            m.deadline = deadline(clock_time(1, 55));
            m.team1 = list[0] as i32;

            (m.kind, m.location, m.provided_cash) = match i {
                0 => (MissionTypes::Sell as i32, location, 0),
                _ if i <= funded => (MissionTypes::BuyFunded as i32, -1, -spent),
                _ => (MissionTypes::Buy as i32, -1, 0),
            };

            set_mission(team, slot, m);
        }

        self.missions.counter += 1;
    }

    /// A disk at a mission spot, inside a briefcase.
    fn disk_in_briefcase(&mut self, disk: &str, pos: Vec3) -> i32 {
        let rot = Mat3::IDENTITY;
        let Some(item) = spawn(disk, pos, rot) else { return -1 };

        if let Some(case) = spawn("briefcase", pos, rot) {
            case.attach(&item);
        }

        item.id() as i32
    }

    /// This mission is the one where two corps each have a disk, and they both try to sell it to the third
    /// Competing for the thirds money.
    fn two_team(&mut self, list: &[Team]) {
        let team_count = list.len();
        let colour = self.next_disk();
        let disk = DISKS[colour];

        let values: Vec<i32> = (0..team_count)
            .map(|team_idx| {
                if team_idx <= 1 {
                    return 100;
                }

                let v = value_step() + value_step() + 1000;
                (if colour == GOLD { v * 4 } else { v }) * 2
            })
            .collect();

        for id in 0..2 {
            self.missions.last_disks[id] = self.lay_out_disk(list[id], disk, values[id])
        }

        let mut count = rand::random_range(0..5) + 4;
        if colour == GOLD {
            count += rand::random_range(0..3) + 2;
        }

        self.lay_out_cash(list[2], count);


        let [item, item2] = self.missions.last_disks;
        let disk_type = item_kind(item);

        for (i, &team) in list.iter().enumerate() {
            let Some((slot, mut mission)) = self.open_mission(team) else { continue };
            let spent = self.missions.spent[team];

            mission.item = item;
            mission.item2 = item2;
            mission.disk_type = disk_type;
            mission.value = values[i];
            mission.deadline = deadline(clock_time(1, 55));

            if i <= 1 {
                mission.kind = MissionTypes::TwoTeamSell as i32;
                mission.team1 = list[2] as i32;
            } else {
                mission.kind = MissionTypes::TwoTeamBuy as i32;
                mission.team1 = list[0] as i32;
                mission.team2 = list[1] as i32;
                mission.provided_cash = -spent;
            }

            set_mission(team, slot, mission);
        }

        self.missions.counter += 1;
    }

    /// create_mission_aquisition (car chase): a disk carried through the streets in a guarded traffic car, every
    /// corporation of the group racing to take it.
    fn car_chase(&mut self, list: &[Team]) {
        let deadline = deadline(clock_time(1, 25));
        let colour = self.next_disk();
        let chase = server::find_item_kind(DISKS[colour]).and_then(chase::car_chase);
        let (location, item) = chase.map_or((-1, -1), |disk| (255, disk.id() as i32));

        self.missions.last_disks[0] = item;

        let disk_type = item_kind(item);

        for &team in list {
            let Some((slot, mut m)) = self.open_mission(team) else { continue };
            let mut value = value_step() + 1000;

            if colour == GOLD {
                value *= 4;
            }

            m.kind = MissionTypes::Acquisition as i32;
            m.item = item;
            m.disk_type = disk_type;
            m.value = value;
            m.deadline = deadline;
            m.location = location;
            m.rate = value as f32 * (1.0 / 40000.0);

            set_mission(team, slot, m);
        }

        self.missions.counter += 1;
    }

    /// round_complete: each round corporation earns the round cash in its base and the value of the missions whose
    /// disks are there, less what its missions were given; its members share the earnings (larger teams split them
    /// further with bonusratio on) and its share price moves with them.
    pub(crate) fn complete(&mut self) {
        let counts = team_counts();
        let least = counts.iter().map(|(_, &n)| n).filter(|&n| n > 0).min().unwrap_or(1) as i32;
        let divisors = counts.map(|n| if self.cfg.bonusratio != 0 && n > 0 { n as i32 / least } else { 1 });
        let items = server::items();
        let cash = server::find_item_kind("cashround");
        let players = server::players();

        for team in TEAMS {
            let (Some(base), Some(corp)) = (base(team), server::get_corporation(team)) else { continue };
            let pos = |id: i32| u32::try_from(id).ok().and_then(server::get_item).map(|i| i.pos());
            let mut earnings = self.missions.spent[team];

            earnings += items.iter().filter(|i| Some(i.kind()) == cash && base.contains(i.pos())).count() as i32 * 250;

            for m in corp.missions().iter().filter(|m| m.active && m.item != -1) {
                if !pos(m.item).is_some_and(|p| base.contains(p)) {
                    continue;
                }

                if (MissionTypes::TwoTeamSell as i32..=9).contains(&m.kind) && m.item2 != -1 && !pos(m.item2).is_some_and(|p| base.contains(p)) {
                    continue;
                }

                earnings += m.value;
            }

            let share = earnings / divisors[team];

            for p in players.iter().filter(|p| p.team() == team) {
                p.set_money(p.money() + share + 250);

                if self.cfg.weekly
                    && let Some(id) = p.account_id()
                {
                    self.accounts.add_money(id, share);
                }
            }

            corp.set_funds(earnings);

            let mut price = corp.price() * (earnings as f32 / 50000.0 + 1.0);

            if 50.0 > price {
                price = 50.0;
            } else if !self.cfg.weekly && price >= 500.0 {
                price *= 0.2;

                for p in players.iter().filter(|p| p.team() == team) {
                    p.set_stocks(p.stocks() * 5);
                }
            }

            corp.set_price(price);
        }
    }
}
