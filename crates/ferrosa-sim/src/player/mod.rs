use ferrosa_protocol::{
    CharacterCustomization, Model, SuitColor, Team,
    clientbound::game::{
        MenuButton, MenuType,
        events::{Event, ServerEvent, update_player::EventUpdatePlayer, update_player_round::EventUpdatePlayerRound},
    },
    serverbound::game::{ClientGamePacket, InputFlags},
};

use glam::Vec3;

use crate::{
    PlayerId, SimJoinMsg,
    player::{actions::ActionQueue, voice::PlayerVoice},
};

pub mod actions;
pub mod voice;

/// A player's movement and view controls, from their game packet or set by the bot AI.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub gear_x: f32,
    pub strafe: f32,
    pub gear_y: f32,
    pub walk: f32,
    pub look_yaw: f32,
    pub look_pitch: f32,
    pub free_look_yaw: f32,
    pub free_look_pitch: f32,
    pub body_yaw: f32,
    pub spin_yaw: f32,
    pub spin_pitch: f32,
}

pub struct Player {
    pub player_id: PlayerId,
    pub username: String,
    pub team: Team,
    #[allow(unused)]
    pub steam_id: u64,
    pub actions: ActionQueue,
    pub menu: MenuType,
    /// The building whose shop or bank menu is open.
    pub menu_tab: i32,
    /// How many things the player has bought from gun stores and burger shops, cleared when they get a
    /// new human, at reset_game, and every 3600 ticks in round mode.
    pub items_bought: i32,
    /// The buttons of the open menu, rebuilt every tick.
    pub menu_buttons: Vec<MenuButton>,
    /// The extra value of each button slot, which stays from the last button that set one.
    pub button_extras: Vec<i32>,
    /// Whether the player sees the round manager page, given to each corporation's richest member at the
    /// end of a round.
    pub manager_tab: bool,
    /// A player fired with their human: the binary keeps the deleted human's id until they get a new one, so
    /// logic_player keeps closing their menu (they are out of every corporation) and the buy pages still take tab
    /// buttons.
    pub ghost_human: bool,
    pub team_switch_timer: i32,
    pub vehicles_bought: i32,
    pub saved_inventory: [Vec<crate::sim::lobby::SavedItem>; 7],
    pub corp_money: i32,
    pub corp_credit: i32,
    pub account_id: u32,
    pub input: InputFlags,
    pub money: i32,
    pub stocks: i32,
    pub corp_rating: i32,
    pub crim_rating: i32,
    pub spawn_timer: i32,
    pub vitality: i32,
    pub phone_number: u32,
    pub is_ready: bool,
    pub voice: PlayerVoice,
    pub customization: CharacterCustomization,
    pub camera_pos: Vec3,
    pub view_yaw: f32,
    pub view_pitch: f32,
    pub human: Option<usize>,
    pub controls: Controls,
    pub input_bits: u32,
    pub control_mode: u32,
    pub zoom_level: u32,
    pub is_bot: bool,
    pub god_mode: bool,
    pub is_admin: bool,
    pub admin_tries: i32,
    /// A mission bot's deadline on the world clock and the deal it belongs to.
    pub bot_deadline: i32,
    pub bot_mission: i32,
    pub bot: BotBrain,
    #[allow(unused)]
    pub ferrosa_client: bool
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BotTarget {
    pub human: i32,
    pub pos: Vec3,
    pub awareness: f32,
    pub timer: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BotBrain {
    pub is_zombie: bool,
    pub idle_yaw: f32,
    pub waypoint: i32,
    pub waypoint_count: i32,
    pub waypoints: Vec<Vec3>,
    pub delay: i32,
    pub wrecked_ticks: i32,
    pub targets: Vec<BotTarget>,
    pub target: i32,
}

impl BotBrain {
    pub(crate) fn waypoint_pos(&self, i: i32) -> Vec3 {
        usize::try_from(i).ok().and_then(|i| self.waypoints.get(i)).copied().unwrap_or(Vec3::ZERO)
    }

    pub(crate) fn set_waypoint(&mut self, i: usize, p: Vec3) {
        if self.waypoints.len() <= i {
            self.waypoints.resize(i + 1, Vec3::ZERO);
        }

        self.waypoints[i] = p;
    }
}

impl Player {
    pub(crate) fn new_from_join(player_id: PlayerId, account_id: u32, j: SimJoinMsg) -> Self {
        let avatar_info = j.join_packet.avatar_info;

        Self {
            player_id,
            steam_id: j.auth_packet.steam_id,
            actions: ActionQueue::default(),
            account_id,
            menu: MenuType::Empty,
            menu_tab: 0,
            items_bought: 0,
            menu_buttons: Vec::new(),
            button_extras: Vec::new(),
            manager_tab: false,
            ghost_human: false,
            team_switch_timer: 0,
            vehicles_bought: 0,
            saved_inventory: Default::default(),
            corp_money: 0,
            corp_credit: 0,
            input: InputFlags::empty(),
            team: Team::Spectator,
            username: j.auth_packet.player_name,
            money: 0,
            stocks: 0,
            corp_rating: 0,
            crim_rating: 0,
            spawn_timer: 0,
            vitality: 0,
            phone_number: j.auth_packet.phone_number,
            is_ready: false,
            voice: PlayerVoice::new(),
            customization: CharacterCustomization {
                eye_color: avatar_info.eye_color,
                gender: avatar_info.gender,
                hair_color: avatar_info.hair_color,
                hair_style: avatar_info.hair,
                head: avatar_info.head,
                model: Model::Casual,
                necklace: 0,
                skin: avatar_info.skin_color,
                suit_color: SuitColor::Brown,
                tie_color: 0,
            },
            camera_pos: Default::default(),
            view_yaw: 0.0,
            view_pitch: 0.0,
            human: None,
            controls: Controls::default(),
            input_bits: 0,
            control_mode: 0,
            zoom_level: 0,
            is_bot: false,
            god_mode: false,
            is_admin: false,
            admin_tries: 0,
            bot_deadline: 0,
            bot_mission: 0,
            bot: BotBrain::default(),
            ferrosa_client: j.join_packet.is_ferrosa
        }
    }

    pub(crate) fn new_bot(player_id: PlayerId, customization: CharacterCustomization, menu: MenuType) -> Self {
        Self {
            player_id,
            steam_id: 0,
            actions: ActionQueue::default(),
            account_id: u32::MAX,
            menu,
            menu_tab: 0,
            items_bought: 0,
            menu_buttons: Vec::new(),
            button_extras: Vec::new(),
            manager_tab: false,
            ghost_human: false,
            team_switch_timer: 0,
            vehicles_bought: 0,
            saved_inventory: Default::default(),
            corp_money: 0,
            corp_credit: 0,
            input: InputFlags::empty(),
            team: Team::Spectator,
            username: String::new(),
            money: 50_000,
            stocks: 0,
            corp_rating: 0,
            crim_rating: 0,
            spawn_timer: 0,
            vitality: 0,
            phone_number: 0,
            is_ready: false,
            voice: PlayerVoice::new(),
            customization,
            camera_pos: Default::default(),
            view_yaw: 0.0,
            view_pitch: 0.0,
            human: None,
            controls: Controls::default(),
            input_bits: 0,
            control_mode: 0,
            zoom_level: 0,
            is_bot: true,
            god_mode: false,
            is_admin: false,
            admin_tries: 0,
            bot_deadline: 0,
            bot_mission: 0,
            bot: BotBrain::default(),
            ferrosa_client: false
        }
    }

    /// Adds a menu button; without an extra value it shows whatever its slot last held.
    pub(crate) fn push_button(&mut self, id: i32, text: &str, extra: Option<i32>) {
        let button_count = self.menu_buttons.len();

        if self.button_extras.len() <= button_count {
            self.button_extras.resize(button_count + 1, 0);
        }

        if let Some(e) = extra {
            self.button_extras[button_count] = e;
        }

        self.menu_buttons.push(MenuButton { id, text: text.to_string(), extra: self.button_extras[button_count] });
    }

    pub(crate) fn process_game_packet(&mut self, msg: Box<ClientGamePacket>) {
        self.actions.ingest(msg.total_actions, msg.actions);
        self.voice.ingest(msg.voice_data);

        self.input = InputFlags::from_bits_retain(msg.input_flags);
        self.controls = Controls {
            gear_x: msg.gear_x,
            strafe: msg.left_right,
            gear_y: msg.gear_y,
            walk: msg.forward_back,
            look_yaw: msg.view_yaw_delta,
            look_pitch: msg.view_pitch,
            free_look_yaw: msg.free_look_yaw,
            free_look_pitch: msg.free_look_pitch,
            body_yaw: msg.view_yaw,
            spin_yaw: msg.spin_yaw,
            spin_pitch: msg.view_pitch_delta,
        };

        self.input_bits = msg.input_flags;
        self.control_mode = msg.input_type as u32;
        self.zoom_level = msg.zoom_level as u32;
        self.camera_pos = msg.camera_pos;
        self.view_yaw = msg.view_yaw;
        self.view_pitch = msg.view_pitch;
    }

    pub(crate) fn make_update_player_event(&self, tick: u32) -> Event {
        Event {
            tick_created: tick,
            kind: ServerEvent::UpdatePlayer(EventUpdatePlayer {
                active: true,
                client_id: self.player_id.0,
                customization: self.customization,
                human_id: self.human.map_or(-1, |h| h as i32),
                is_bot: self.is_bot,
                team: self.team,
                name: self.username.clone(),
            }),
        }
    }

    pub(crate) fn make_update_financials_event(&self, tick: u32) -> Event {
        Event {
            tick_created: tick,
            kind: ServerEvent::UpdatePlayerRound(EventUpdatePlayerRound {
                client_id: self.player_id.0,
                money: self.money,
                phone_number: self.phone_number,
                stocks: self.stocks,
            }),
        }
    }
}

#[derive(Default)]
pub struct PlayerTable {
    slots: Vec<Option<Player>>,
    count: usize,
}

impl PlayerTable {
    pub(crate) fn next_key(&self) -> usize {
        self.slots.iter().position(Option::is_none).unwrap_or(self.slots.len())
    }

    /// insert: insert a player into slot, returning the slot ID
    pub(crate) fn insert(&mut self, p: Player) -> usize {
        let k = self.next_key();

        if k == self.slots.len() {
            self.slots.push(Some(p));
        } else {
            self.slots[k] = Some(p);
        }

        self.count += 1;

        k
    }

    /// remove: remove a player from slot `k`
    pub(crate) fn remove(&mut self, k: usize) -> Player {
        let player = self.slots.get_mut(k).and_then(Option::take).expect("no player in that slot");

        self.count -= 1;

        player
    }

    pub(crate) fn get(&self, k: usize) -> Option<&Player> {
        self.slots.get(k)?.as_ref()
    }

    pub(crate) fn get_mut(&mut self, k: usize) -> Option<&mut Player> {
        self.slots.get_mut(k)?.as_mut()
    }

    pub(crate) fn len(&self) -> usize {
        self.count
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (usize, &Player)> {
        self.slots.iter().enumerate().filter_map(|(k, p)| Some((k, p.as_ref()?)))
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut Player)> {
        self.slots.iter_mut().enumerate().filter_map(|(k, p)| Some((k, p.as_mut()?)))
    }
}
