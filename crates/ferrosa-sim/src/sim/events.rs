use std::net::SocketAddr;

use ferrosa_protocol::{
    Team,
    clientbound::game::{
        GameState, MenuType,
        events::{Event, ServerEvent, chat::ChatType, update_player::EventUpdatePlayer},
    },
    serverbound::game::actions::{ChatAction, Menu, MenuAction},
};

use super::Sim;
use crate::{Client, ConnId, PlayerId, SimJoinMsg};

impl Sim {
    pub(crate) fn on_join(&mut self, conn: ConnId, src: SocketAddr, j: SimJoinMsg) {
        println!("[Sim] Player {} is joining... from {}... FERROSA: {}", j.auth_packet.player_name, src, j.join_packet.is_ferrosa);

        let accout_data = self.join_account(&j.auth_packet);

        if accout_data.ban_time > 0 {
            return self.kick(conn, src, "You are banned from this server");
        }

        let admin = self.is_admin_phone(accout_data.phone_number);

        if admin {
            println!("[Sim] Admin {} is joining!", j.auth_packet.player_name);
        }

        let people = self.players.iter().filter(|(_, p)| !p.is_bot).count();

        if people >= self.max_players as usize && !admin {
            return self.kick(conn, src, "Server is full");
        }

        let Some(player_id) = self.alloc_player(accout_data.account_id, j) else {
            return self.kick(conn, src, "Server is full");
        };

        if let Some(p) = self.players.get_mut(player_id.idx()) {
            p.username = accout_data.name();
        }

        self.clients.insert(
            conn,
            Client {
                addr: src,
                event_cursor: 0,
                round_number: u32::MAX,
                timeout: 0,
                admin_visible: admin,
                last_sdl_tick: 0,
                earshots: [None; 8],
                spectating: None,
                pack_ring: Vec::new(),
                pack_count: 0,
                pack_ack: 0,
                packed: Default::default(),
                traffic_priority: Vec::new(),
                vehicle_priority: Vec::new(),
                packet_sequence: 0,
                client_packet_count: 0,
                signal_cursor: 0,
                link_sent: Default::default(),
                link_ring: vec![0; 256],
                link_count: 0,
                link_ack: 0,
                player_id,
            },
        );

        self.restore_account(player_id);
        let player = self.players.get_mut(player_id.idx()).unwrap();
        player.username = super::economy::sanitize_name(&accout_data.player_name);
        player.is_admin = admin;
        self.events.push(player.make_update_player_event(self.tick));
        self.events.push(player.make_update_round_event(self.tick));

        if self.gamestate == GameState::Intermission {
            player.menu = MenuType::Lobby
        } else {
            player.menu = MenuType::Empty
        }

        self.send_initial_sync(src);

        let (mode, plugins) = (self.mode, self.plugins);
        mode.on_join(self, player_id);
        plugins.on_join(self, player_id);
    }

    pub(crate) fn on_leave(&mut self, conn: ConnId) {
        let client = self.clients.remove(&conn).unwrap();
        self.drop_conn(conn);
        let (mode, plugins) = (self.mode, self.plugins);
        mode.on_leave(self, client.player_id);
        plugins.on_leave(self, client.player_id);
        self.settle_leave(client.player_id);

        let player = self.players.remove(client.player_id.idx());

        self.events.push(Event {
            tick_created: self.tick,
            kind: ServerEvent::UpdatePlayer(EventUpdatePlayer {
                active: false,
                client_id: player.player_id.idx() as u32,
                is_bot: false,
                human_id: -1,
                team: player.team,
                customization: player.customization,
                name: player.username.clone(),
            }),
        });
    }

    pub(crate) fn on_menu_action(&mut self, player_id: PlayerId, action: MenuAction) {
        let id = match action.menu {
            Menu::Lobby => 2,
            Menu::Other(m) => m,
        };

        if id == 1 && self.enter_city_menu(player_id, action.button) {
            return;
        }

        let (mode, plugins) = (self.mode, self.plugins);

        if mode.on_menu(self, player_id, id, action.button) || plugins.on_menu(self, player_id, id, action.button) {
            return;
        }

        let Some(player) = self.players.get_mut(player_id.idx()) else {
            return;
        };

        if player.menu as u8 == id && (player.human.is_some() || player.ghost_human) && matches!(id, 14..=19) {
            return self.round_buy_menu(player_id, action.button);
        }

        if player.menu as u8 == id && matches!(id, 9..=11) {
            return self.shop_menu_action(player_id, action.button);
        }

        if player.menu as u8 == id && id == 13 {
            return self.bank_menu_action(player_id, action.button);
        }

        match action.menu {
            Menu::Lobby => {
                match action.button {
                    1 => player.team = Team::Goldmen,
                    2 => player.team = Team::Monsota,
                    3 => player.team = Team::OXS,
                    4 => player.team = Team::Spectator,
                    5 => player.is_ready = !player.is_ready,
                    other => println!("Got other {other}"),
                }

                self.update_player_event(player_id);
            }

            Menu::Other(other) => println!("Unknown menu option! {}", other),
        }
    }

    pub(crate) fn on_chat_action(&mut self, player_id: PlayerId, action: ChatAction) {
        let plugins = self.plugins;

        if !action.message.starts_with("/admin") && plugins.on_chat(self, player_id, &action.message) {
            return;
        }

        if action.message.trim() == "/tps" {
            return self.tps_command(player_id);
        }

        if action.message.starts_with('/') {
            return self.admin_message(player_id, &action.message);
        }

        let speaker = self
            .players
            .get(player_id.idx())
            .and_then(|p| p.human)
            .filter(|&h| self.gamestate != GameState::Restarting && self.humans.get(h).is_some_and(|h| h.health.vitality > 0));
        let volume = action.volume as i32;

        match speaker {
            None => self.send_chat(&action.message, ChatType::Announce, player_id.idx() as i32, volume),
            Some(human) => self.send_chat(&action.message, ChatType::Chat, human as i32, volume),
        }
    }
}
