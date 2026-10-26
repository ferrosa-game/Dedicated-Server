use crate::sim::traffic::TRAFFIC_CARS;

/// What a game mode turns on and off, set by reset_game from the mode's hooks. Code outside the modes asks these rather
/// than which mode it is. The default is a plain mode; each built-in mode changes what it needs from there, and a plugin
/// mode changes a few with the server's setters until the next reset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModeRules {
    /// The share of a teammate's damage that comes back on the attacker's head.
    pub team_damage: i32,
    /// The share of the price a sale of shares pays.
    pub stock_sell_rate: f32,
    /// The traffic cars reset_game asks for; modes without them make theirs when they start.
    pub reset_traffic: Option<u32>,
    /// Some traffic is parked in the third lane of flat three lane streets.
    pub parked_cars: bool,
    /// The traffic drives as in co-op.
    pub coop_traffic: bool,
    /// Dead humans stay on the ground.
    pub keep_bodies: bool,
    /// Loose items never despawn.
    pub keep_items: bool,
    /// Hurting people raises criminal ratings, by these rates.
    pub crime: bool,
    /// What hurting someone adds to the attacker's criminal rating, by their teams: a civilian hurting a civilian or
    /// a corporation's member, a member hurting a civilian, another member, or one inside their own base. A victim
    /// who is already criminal adds less, and a player at `crimekick` has their human killed.
    pub crimecivciv: i32,
    pub crimecivteam: i32,
    pub crimeteamciv: i32,
    pub crimeteamteam: i32,
    pub crimeteamteaminbase: i32,
    pub crimekick: i32,
    /// Hurting a teammate hurts the attacker too.
    pub punish_team_kills: bool,
    /// World mode's corporate ratings: the manager's rating is the account's credit and scales the missions, and ratings
    /// are settled when a manager leaves. Only world mode sets it; plugins can't.
    pub world_corporations: bool,
    /// The base buy menus (14 to 19) open inside a corporation's base.
    pub corp_buy_menus: bool,
    /// Corporation members count towards the player counts without a human.
    pub count_humanless_members: bool,
    /// The base computers and labs get their mission volumes.
    pub mission_spots: bool,
    /// Garage doors open and close when a teammate knocks on them.
    pub door_knocks: bool,
    /// Joining players get their saved money and ratings back, and new accounts start with the world config's
    /// startcash.
    pub saved_accounts: bool,
    /// Shares are sold when the human dies and bought back when a returning player takes it over.
    pub shares_follow_human: bool,
    /// A saved key keeps its own vehicle for the next day, rather than a new vehicle of its type and colour.
    pub keys_keep_vehicles: bool,
    /// A death waits out a respawn timer that grows with criminal rating, spectating unless respawnteam is set.
    pub respawn_wait: bool,
    /// A player without a human sees the lobby menu during intermission.
    pub intermission_lobby: bool,
    /// The round starts once everyone is ready.
    pub ready_start: bool,
    /// Nobody can walk for this many ticks after the round starts.
    pub move_delay: Option<i32>,
}

impl Default for ModeRules {
    fn default() -> Self {
        Self {
            team_damage: 0,
            stock_sell_rate: 0.95,
            reset_traffic: Some(TRAFFIC_CARS),
            parked_cars: false,
            coop_traffic: false,
            keep_bodies: false,
            keep_items: false,
            crime: false,
            crimecivciv: 0,
            crimecivteam: 0,
            crimeteamciv: 0,
            crimeteamteam: 0,
            crimeteamteaminbase: 0,
            crimekick: 0,
            punish_team_kills: true,
            world_corporations: false,
            corp_buy_menus: false,
            count_humanless_members: false,
            mission_spots: false,
            door_knocks: true,
            saved_accounts: false,
            shares_follow_human: false,
            keys_keep_vehicles: false,
            respawn_wait: false,
            intermission_lobby: false,
            ready_start: true,
            move_delay: None,
        }
    }
}
