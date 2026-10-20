use ferrosa_physics::RotMatrix;
use ferrosa_protocol::{
    GameMode, Team,
    clientbound::game::{GameState, events::chat::ChatType},
};
use ferrosa_sim::ModeRules;

use crate::bindings::ferrosa::plugin::types::{self, ClientMode, Mat3, Rules, Vec3};

impl From<glam::Vec3> for Vec3 {
    fn from(v: glam::Vec3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

impl From<Vec3> for glam::Vec3 {
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<GameState> for types::GameState {
    fn from(state: GameState) -> Self {
        match state {
            GameState::Idle => Self::Idle,
            GameState::Intermission => Self::Intermission,
            GameState::InGame => Self::InGame,
            GameState::Restarting => Self::Restarting,
            GameState::Paused => Self::Paused,
        }
    }
}

impl From<types::GameState> for GameState {
    fn from(state: types::GameState) -> Self {
        match state {
            types::GameState::Idle => Self::Idle,
            types::GameState::Intermission => Self::Intermission,
            types::GameState::InGame => Self::InGame,
            types::GameState::Restarting => Self::Restarting,
            types::GameState::Paused => Self::Paused,
        }
    }
}

impl From<Team> for types::Team {
    fn from(team: Team) -> Self {
        match team {
            Team::Goldmen => Self::Goldmen,
            Team::Monsota => Self::Monsota,
            Team::OXS => Self::Oxs,
            Team::Nexaco => Self::Nexaco,
            Team::Pentacom => Self::Pentacom,
            Team::Prodocon => Self::Prodocon,
            Team::Mission => Self::Mission,
            Team::Spectator => Self::Spectator,
        }
    }
}

impl From<types::ChatType> for ChatType {
    fn from(kind: types::ChatType) -> Self {
        match kind {
            types::ChatType::Chat => Self::Chat,
            types::ChatType::ItemSpeak => Self::ItemSpeak,
            types::ChatType::EliminatorAnnouncement => Self::EliminatorAnnouncement,
            types::ChatType::AdminChat => Self::AdminChat,
            types::ChatType::Notice => Self::Notice,
        }
    }
}

impl From<RotMatrix> for Mat3 {
    fn from([x, y, z]: RotMatrix) -> Self {
        Self { x: x.into(), y: y.into(), z: z.into() }
    }
}

impl From<Mat3> for RotMatrix {
    fn from(m: Mat3) -> Self {
        [m.x.into(), m.y.into(), m.z.into()]
    }
}

impl From<ClientMode> for GameMode {
    fn from(mode: ClientMode) -> Self {
        match mode {
            ClientMode::Driving => Self::Driving,
            ClientMode::Racing => Self::Racing,
            ClientMode::Round => Self::Round,
            ClientMode::World => Self::World,
            ClientMode::Eliminator => Self::Eliminator,
            ClientMode::Coop => Self::CoOp,
            ClientMode::Versus => Self::Versus,
            ClientMode::None => Self::None,
        }
    }
}

impl From<ModeRules> for Rules {
    fn from(r: ModeRules) -> Self {
        Self {
            team_damage: r.team_damage,
            stock_sell_rate: r.stock_sell_rate,
            reset_traffic: r.reset_traffic,
            parked_cars: r.parked_cars,
            coop_traffic: r.coop_traffic,
            keep_bodies: r.keep_bodies,
            keep_items: r.keep_items,
            crime: r.crime,
            punish_team_kills: r.punish_team_kills,
            world_corporations: r.world_corporations,
            corp_buy_menus: r.corp_buy_menus,
            count_humanless_members: r.count_humanless_members,
            mission_spots: r.mission_spots,
            door_knocks: r.door_knocks,
            saved_accounts: r.saved_accounts,
            world_start_cash: r.world_start_cash,
            shares_follow_human: r.shares_follow_human,
            keys_keep_vehicles: r.keys_keep_vehicles,
            respawn_wait: r.respawn_wait,
            free_team_switch: r.free_team_switch,
            intermission_lobby: r.intermission_lobby,
            bot_lobby: r.bot_lobby,
            ready_start: r.ready_start,
            move_delay: r.move_delay,
            round_money_updates: r.round_money_updates,
        }
    }
}

impl From<Rules> for ModeRules {
    fn from(r: Rules) -> Self {
        Self {
            team_damage: r.team_damage,
            stock_sell_rate: r.stock_sell_rate,
            reset_traffic: r.reset_traffic,
            parked_cars: r.parked_cars,
            coop_traffic: r.coop_traffic,
            keep_bodies: r.keep_bodies,
            keep_items: r.keep_items,
            crime: r.crime,
            punish_team_kills: r.punish_team_kills,
            world_corporations: r.world_corporations,
            corp_buy_menus: r.corp_buy_menus,
            count_humanless_members: r.count_humanless_members,
            mission_spots: r.mission_spots,
            door_knocks: r.door_knocks,
            saved_accounts: r.saved_accounts,
            world_start_cash: r.world_start_cash,
            shares_follow_human: r.shares_follow_human,
            keys_keep_vehicles: r.keys_keep_vehicles,
            respawn_wait: r.respawn_wait,
            free_team_switch: r.free_team_switch,
            intermission_lobby: r.intermission_lobby,
            bot_lobby: r.bot_lobby,
            ready_start: r.ready_start,
            move_delay: r.move_delay,
            round_money_updates: r.round_money_updates,
        }
    }
}
