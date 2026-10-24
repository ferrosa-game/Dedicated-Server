//! The client's side of the game packet (client_recv_game_packet?, client_recv_object_packet): what a client
//! must remember between packets to read the next one, and the reading itself.

use glam::Vec3;

use crate::{
    CharacterCustomization, Gender, Model, SuitColor,
    clientbound::game::{
        GameState, HumanAppearance, ItemHeader, ItemKind, ItemTail, LineLinkEntry, MenuButton, MenuType, ObjectPack, OwnHumanData, ServerGamePacket,
        ServerHumanObject, ServerItemObject, ServerVehicleObject, ServerVoiceData, TrafficEntry, VehicleKind, events::Event,
    },
    codec::{CodecError, Reader, WireRead},
    serverbound::game::voice::VoiceFrame,
};

/// Object slots, pack ring entries and the packets remembered as bases (data_1860cff4, data_18610ffc, the frames at
/// data_f712a20).
const SLOTS: usize = 0x400;
const PACKS: usize = 0x800;
const HISTORY: usize = 0x100;
pub const HUMANS: usize = 0x100;
pub const ITEMS: usize = 0x400;
pub const VEHICLES: usize = 0x200;
pub const TRAFFIC: usize = 0x400;
/// A human's bones past the root.
const BONES: usize = 15;
const HUMAN_INTS: usize = 7 + 4 * BONES;
/// The item types whose objects carry more after their rotation.
const CASH_WORLD: u32 = ItemKind::CashWorld as u32;
const RADIO: u32 = ItemKind::Radio as u32;
const COMPUTER: u32 = ItemKind::Computer as u32;
/// Packed positions are in 4096ths of a unit, x and z offset by 4096.
const POSITION_SCALE: f32 = 1.0 / 4096.0;
const POSITION_OFFSET: f32 = 4096.0;

/// An object slot (data_1860cff4): whether it is filled, by a human (0) or item (1), and which.
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    active: bool,
    kind: u8,
    index: u16,
}

/// A pack ring entry applied: a slot filled with a human or item (an item of the type given), or emptied of what
/// was in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotChange {
    Pack { kind: u8, item_type: u16, index: u16 },
    Unpack { kind: u8, index: u16 },
}

/// One packet's objects as their packed integers, the base later packets are sent as changes from.
#[derive(Clone)]
struct Frame {
    humans: Vec<[i32; HUMAN_INTS]>,
    vehicles: Vec<[i32; 20]>,
    items: Vec<[i32; 7]>,
    traffic: Vec<[i32; 4]>,
}

impl Frame {
    fn new() -> Self {
        Self { humans: vec![[0; HUMAN_INTS]; HUMANS], vehicles: vec![[0; 20]; VEHICLES], items: vec![[0; 7]; ITEMS], traffic: vec![[0; 4]; TRAFFIC] }
    }
}

/// What a client remembers between game packets to read the next: its object slots and the pack ring that fills
/// them, each item's type (which says what follows its rotation), the last 256 packets' objects as bases for
/// changes, the bit counts changes are sent in, and the counts it acknowledges.
pub struct ObjectDecoder {
    slots: Vec<Slot>,
    packs: Vec<ObjectPack>,
    /// Pack entries received in order up to here (data_18610ff4), and applied up to here (data_18610ff8), which the
    /// client sends back.
    pub packs_received: u16,
    pub packs_applied: u16,
    /// Line links received in order up to here (data_1861affc), sent back.
    pub links_received: u8,
    /// The server's object packet count (sent_object_packets), sent back.
    pub object_packet: u32,
    item_kinds: Vec<u32>,
    history: Vec<Option<Box<Frame>>>,
    small: u32,
    large: u32,
    keep_flag: bool,
    /// The slot changes the last packet read applied, in order.
    pub changes: Vec<SlotChange>,
}

impl Default for ObjectDecoder {
    fn default() -> Self {
        Self {
            slots: vec![Slot::default(); SLOTS],
            packs: vec![ObjectPack { slot: 0, unpack: false, kind: 0, item_type: 0, index: 0 }; PACKS],
            packs_received: 0,
            packs_applied: 0,
            links_received: 0,
            object_packet: 0,
            item_kinds: vec![0; ITEMS],
            history: (0..HISTORY).map(|_| None).collect(),
            small: 0,
            large: 0,
            keep_flag: false,
            changes: Vec::new(),
        }
    }
}

/// A packed position: 4096ths of a unit, x and z offset by 4096.
fn unpack_position(v: &[i32]) -> Vec3 {
    Vec3::new(v[0] as f32 * POSITION_SCALE - POSITION_OFFSET, v[1] as f32 * POSITION_SCALE, v[2] as f32 * POSITION_SCALE - POSITION_OFFSET)
}

/// decode_quaternion_14bit and decode_quaternion_bits: which component is largest, then the other three (in the
/// order 3, 1, 2, 0) as fractions of 1/sqrt 2 in `bits` signed bits; the largest is what makes it unit length,
/// then the whole normalised.
pub fn decode_quaternion(v: &[i32], bits: u32) -> [f32; 4] {
    let largest = v[0] as usize & 3;
    let scale = (1i32 << (bits - 1)) as f32 as f64 - 1.0;
    let mut q = [0f32; 4];
    let mut left = 1.0f64;
    let mut k = 1;

    for i in [3usize, 1, 2, 0] {
        if i != largest {
            let c = v[k] as f64 / scale / std::f64::consts::FRAC_1_SQRT_2;
            q[i] = c as f32;
            left -= c * c;
            k += 1;
        }
    }

    q[largest] = if left > 0.0 { left.sqrt() as f32 } else { 0.0 };
    let len = q.iter().map(|c| c * c).sum::<f32>().sqrt();
    if len > 0.0 { q.map(|c| c / len) } else { q }
}

impl ObjectDecoder {
    fn base(&self, index: u32) -> Option<&Frame> {
        self.history[index as usize & (HISTORY - 1)].as_deref()
    }

    fn frame(&mut self) -> &mut Frame {
        let k = self.object_packet as usize & (HISTORY - 1);
        self.history[k].get_or_insert_with(|| Box::new(Frame::new()))
    }

    /// An item's type as the client knows it, for event 5 (the client applies those events after reading).
    pub fn set_item_kind(&mut self, item: usize, kind: u32) {
        if let Some(k) = self.item_kinds.get_mut(item) {
            *k = kind;
        }
    }

    fn delta_bits(&self, r: &mut Reader, previous: i32, count: u32) -> Result<i32, CodecError> {
        r.delta_bits(previous, count, self.small, self.large, self.keep_flag)
    }

    /// The pack entries from the applied one to the newest received in order, each filling or emptying its slot.
    fn apply_packs(&mut self) {
        let mut k = self.packs_applied;

        while k != self.packs_received {
            let pk = self.packs[k as usize];
            let slot = &mut self.slots[pk.slot as usize & (SLOTS - 1)];

            if !pk.unpack {
                if pk.kind == 1
                    && let Some(t) = self.item_kinds.get_mut(pk.index as usize)
                {
                    *t = pk.item_type as u32;
                }
                self.changes.push(SlotChange::Pack { kind: pk.kind, item_type: pk.item_type, index: pk.index });
                *slot = Slot { active: true, kind: pk.kind, index: pk.index };
            } else {
                self.changes.push(SlotChange::Unpack { kind: slot.kind, index: slot.index });
                slot.active = false;
            }

            k = (k + 1) & (PACKS as u16 - 1);
        }

        self.packs_applied = k;
    }

    /// read_human_object_data and its unpacking in client_recv_object_packet.
    fn read_human(&mut self, r: &mut Reader, slot: u16, h: usize) -> Result<ServerHumanObject, CodecError> {
        let appearance = if r.bits(1)? == 1 {
            let player_id = r.nullable(8)?;
            let mut c = CharacterCustomization { gender: Gender::from_bit(r.bits(1)?), ..Default::default() };

            for (f, n) in [(&mut c.head, 5), (&mut c.skin, 3), (&mut c.hair_style, 4), (&mut c.eye_color, 3), (&mut c.hair_color, 4)] {
                *f = r.bits(n)? as u8;
            }

            c.model = Model::try_from(r.bits(5)? as u8).unwrap_or_default();
            c.suit_color = SuitColor::try_from(r.bits(4)? as u8).unwrap_or_default();

            for (f, n) in [(&mut c.tie_color, 4), (&mut c.necklace, 4)] {
                *f = r.bits(n)? as u8;
            }
            Some(HumanAppearance { player_id, customization: c, wanted: r.bits(2)? as i32 })
        } else {
            None
        };

        let base_index = r.bits(8)?;
        let vehicle = r.signed(8)?;
        let vehicle_seat = r.bits(4)? as i32;
        let alive = r.bits(1)? != 0;
        let bleeding = r.bits(1)? != 0;
        let base = self.base(base_index).map_or([0; HUMAN_INTS], |f| f.humans[h]);
        let mut v = [0i32; HUMAN_INTS];

        for i in 0..3 {
            v[i] = r.delta(base[i], 28)?;
        }

        v[3] = r.bits(2)? as i32;

        for i in 4..7 {
            v[i] = self.delta_bits(r, base[i], 14)?;
        }

        for b in 0..BONES {
            let at = 7 + b * 4;

            if r.bits(1)? == 1 {
                v[at] = r.bits(2)? as i32;

                for i in at + 1..at + 4 {
                    v[i] = self.delta_bits(r, base[i], 12)?;
                }
            } else {
                v[at..at + 4].copy_from_slice(&base[at..at + 4]);
            }
        }

        self.frame().humans[h] = v;
        Ok(ServerHumanObject {
            slot,
            human_id: h as u16,
            appearance,
            vehicle,
            vehicle_seat,
            alive,
            bleeding,
            pos: unpack_position(&v[0..3]),
            rot: decode_quaternion(&v[3..7], 14),
            bones: std::array::from_fn(|b| decode_quaternion(&v[7 + b * 4..11 + b * 4], 12)),
        })
    }

    /// object_decode_item and object_decode_item_posdata.
    fn read_item(&mut self, r: &mut Reader, slot: u16, i: usize) -> Result<ServerItemObject, CodecError> {
        let header = if r.bits(1)? == 1 {
            self.item_kinds[i] = r.bits(8)?;
            let (parent_item, parent_human, parent_slot) = (r.nullable(10)?, r.nullable(10)?, r.bits(4)? as i32);
            let screen = if self.item_kinds[i] == COMPUTER { Some((r.bits(5)? as i32, r.bits(5)? as i32)) } else { None };
            Some(ItemHeader { parent_item, parent_human, parent_slot, screen })
        } else {
            None
        };
        let kind = self.item_kinds[i];
        let base_index = r.bits(8)?;
        let base = self.base(base_index).map_or([0; 7], |f| f.items[i]);
        let mut v = [0i32; 7];

        for k in 0..3 {
            v[k] = r.delta(base[k], 28)?;
        }

        v[3] = r.bits(2)? as i32;

        for k in 4..7 {
            v[k] = self.delta_bits(r, base[k], 14)?;
        }

        let tail = match kind {
            CASH_WORLD => ItemTail::Cash { bills: r.bits(4)? as i32, spread: r.bits(4)? as i32, codes: r.bits(30)? },
            RADIO => ItemTail::Radio(r.bits(1)? != 0),
            COMPUTER => ItemTail::Computer(r.nullable(12)?),
            _ => ItemTail::None,
        };
        self.frame().items[i] = v;
        let item_type = ItemKind::try_from(kind as u8).ok();
        Ok(ServerItemObject { slot, item_id: i as u16, item_type, pos: unpack_position(&v[0..3]), rot: decode_quaternion(&v[3..7], 14), header, tail })
    }

    /// client_recv_object_packet's vehicles.
    fn read_vehicle(&mut self, r: &mut Reader) -> Result<ServerVehicleObject, CodecError> {
        let id = r.bits(10)? as usize & (VEHICLES - 1);
        let tier = r.bits(2)? as i32;
        let traffic_car = r.nullable(10)?;
        let base_index = r.bits(8)?;
        let base = self.base(base_index).map_or([0; 20], |f| f.vehicles[id]);
        let mut v = [0i32; 20];

        for k in 0..3 {
            v[k] = r.delta(base[k], 28)?;
        }

        v[3] = r.bits(2)? as i32;

        for k in 4..7 {
            v[k] = self.delta_bits(r, base[k], 14)?;
        }

        v[7] = self.delta_bits(r, base[7], 9)?;

        for w in 0..4 {
            let at = 8 + w * 3;
            v[at] = r.delta(base[at], 8)?;
            v[at + 1] = self.delta_bits(r, base[at + 1], 9)?;
            v[at + 2] = r.delta(base[at + 2], 8)?;
        }

        let engine_rpm = r.bits(13)? as i32;
        self.frame().vehicles[id] = v;
        Ok(ServerVehicleObject {
            vehicle_id: id as u16,
            traffic_car,
            pos: unpack_position(&v[0..3]),
            rot: decode_quaternion(&v[3..7], 14),
            steer: v[7],
            wheels: std::array::from_fn(|w| [v[8 + w * 3], v[9 + w * 3], v[10 + w * 3]]),
            engine_rpm,
            tier,
        })
    }

    /// client_recv_object_packet's traffic cars: runs of unsent ones skipped.
    fn read_traffic(&mut self, r: &mut Reader, count: usize) -> Result<Vec<TrafficEntry>, CodecError> {
        let mut out = Vec::new();
        let mut skip = 0i32;

        for i in 0..count.min(TRAFFIC) {
            if skip > 0 {
                skip -= 1;
                continue;
            }

            if r.bits(1)? != 1 {
                skip = r.bits(10)? as i32 - 1;
                continue;
            }

            let base_index = r.bits(8)?;
            let vehicle = r.bits(1)? != 0;
            let kind_color = if r.bits(1)? == 1 {
                let kind = VehicleKind::try_from(r.bits(6)? as u8).unwrap_or_default();
                Some((kind, r.bits(4)? as i32))
            } else {
                None
            };
            let base = self.base(base_index).map_or([0; 4], |f| f.traffic[i]);
            let mut v = [0i32; 4];

            for k in 0..3 {
                v[k] = r.delta(base[k], 28)?;
            }

            v[3] = self.delta_bits(r, base[3], 9)?;
            self.frame().traffic[i] = v;
            let yaw = (v[3] as f32 as f64 * std::f64::consts::PI / 255.0) as f32;
            out.push(TrafficEntry { index: i as u16, vehicle, kind_color, pos: unpack_position(&v[0..3]), yaw });
        }

        Ok(out)
    }

    /// client_recv_object_packet: the delta settings and the server's object packet count, the pack entries (those
    /// received in order then applied to the slots), the line links, every filled slot's human or item, the
    /// vehicles, the traffic cars and an intersection's lights. Fills the packet's object fields.
    fn read_objects(&mut self, r: &mut Reader, p: &mut ServerGamePacket) -> Result<(), CodecError> {
        self.small = r.bits(4)?;
        self.large = r.bits(4)?;
        r.bits(1)?;
        self.keep_flag = r.bits(1)? != 0;
        self.object_packet = r.u32()?;
        p.network_tick = self.object_packet;
        let count = r.bits(11)?;
        let mut at = r.bits(11)? as u16;
        p.pack_offset = at;

        for _ in 0..count {
            let pk = ObjectPack {
                slot: r.bits(10)? as u16,
                unpack: r.bits(2)? != 0,
                kind: r.bits(3)? as u8,
                item_type: r.bits(10)? as u16,
                index: r.bits(16)? as u16,
            };
            self.packs[at as usize] = pk;
            p.object_packs.push(pk);

            if self.packs_received == at {
                self.packs_received = (self.packs_received + 1) & (PACKS as u16 - 1);
            }

            at = (at + 1) & (PACKS as u16 - 1);
        }

        self.apply_packs();
        let count = r.bits(8)?;
        let mut at = r.bits(8)? as u8;
        p.link_offset = at;

        for _ in 0..count {
            let index = r.bits(16)? as u16;
            let kind = r.bits(6)? as i32;
            let tick = r.bits(28)? as i32;
            let mut link = LineLinkEntry { index, kind, tick, item: 0, line: 0, text: Vec::new(), colors: Vec::new() };

            if kind == 0 || kind == 1 {
                link.item = r.bits(12)? as i32;
                link.line = r.bits(if kind == 0 { 6 } else { 5 })? as i32;
                let len = r.bits(7)? as usize;
                link.text = r.bytes(len)?.to_vec();

                if kind == 1 {
                    link.colors = r.bytes(len)?.to_vec();
                }
            }

            p.line_links.push(link);

            if self.links_received == at {
                self.links_received = self.links_received.wrapping_add(1);
            }

            at = at.wrapping_add(1);
        }

        for s in 0..SLOTS {
            let slot = self.slots[s];

            if !slot.active || r.bits(1)? != 1 {
                continue;
            }

            let index = slot.index as usize;

            match slot.kind {
                0 if index < HUMANS => {
                    let h = self.read_human(r, s as u16, index)?;
                    p.humans.push(h);
                }
                1 if index < ITEMS => {
                    let it = self.read_item(r, s as u16, index)?;
                    p.items.push(it);
                }
                _ => {}
            }
        }

        let count = r.bits(8)?;

        for _ in 0..count {
            let v = self.read_vehicle(r)?;
            p.vehicles.push(v);
        }

        r.bits(8)?;
        p.traffic_count = r.bits(10)? as i32;
        r.bits(8)?;
        p.traffic = self.read_traffic(r, p.traffic_count as usize)?;
        p.signal = (r.bits(10)? as i32, [r.bits(2)? as i32, r.bits(2)? as i32, r.bits(2)? as i32, r.bits(2)? as i32]);
        Ok(())
    }
}

impl ServerGamePacket {
    /// client_recv_game_packet?'s game packet, after its type byte: read only if `accept` takes its round and tick
    /// (the client's own round, and not older than its tick), so nothing of a skipped packet is remembered.
    pub fn read(r: &mut Reader, d: &mut ObjectDecoder, accept: impl FnOnce(u32, u32) -> bool) -> Result<Option<Self>, CodecError> {
        d.changes.clear();
        let round_number = r.u32()?;
        let ticks_since_reset = r.u32()?;

        if !accept(round_number, ticks_since_reset) {
            return Ok(None);
        }

        let gamestate = GameState::try_from(r.bits(4)?)?;
        let ready_states = if gamestate == GameState::Intermission {
            let mut v = [false; 32];

            for b in &mut v {
                *b = r.bits(1)? != 0;
            }

            Some(v)
        } else {
            None
        };
        let mut corp_round = [(0, 0); 3];

        if matches!(gamestate, GameState::Intermission | GameState::Restarting) {
            for c in &mut corp_round {
                *c = (r.signed(16)?, r.signed(16)?);
            }
        }

        let game_timer = r.bits(24)? as i32;
        let game_timer_extra = r.bits(16)? as i32;
        let sun_time = r.bits(30)? as i32;
        let mut team_counts = [0; 5];

        for n in &mut team_counts {
            *n = r.bits(6)? as i32;
        }

        let client_id = r.bits(8)?;
        let human_id = r.nullable(10)?;
        let mut own_human = if human_id >= 0 {
            let (view_yaw, view_pitch, yaw_offset, pitch_offset, body_yaw) = (r.f32()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?);
            let is_standing = r.bits(1)? != 0;
            let pain = r.bits(8)? as i32;
            let action_type = r.nullable(8)?;
            let (action_duration, action_hand, action_slot, progress_bar) = (r.bits(7)? as i32, r.bits(4)? as i32, r.bits(4)? as i32, r.bits(8)? as i32);
            let mut health = [0; 7];

            for hp in &mut health {
                *hp = r.bits(7)? as i32;
            }

            let (stamina, max_stamina) = (r.bits(8)? as i32, r.bits(8)? as i32);
            Some(OwnHumanData {
                human_id,
                view_yaw,
                view_pitch,
                yaw_offset,
                pitch_offset,
                body_yaw,
                is_standing,
                pain,
                action_type,
                action_duration,
                action_hand,
                action_slot,
                progress_bar,
                health,
                stamina,
                max_stamina,
                head_vel: Vec3::ZERO,
            })
        } else {
            None
        };
        let head_vel = r.vec3()?;

        if let Some(h) = own_human.as_mut() {
            h.head_vel = head_vel;
        }

        let manager_tab = r.bits(1)? != 0;
        let menu_type = MenuType::try_from(r.bits(8)?).unwrap_or(MenuType::Empty);
        let menu_tab = r.bits(16)? as i32;
        let shop = if menu_type == MenuType::WorldStore {
            let n = r.bits(8)?;
            (0..n).map(|_| Ok((r.bits(8)? as i32, r.bits(24)? as i32, r.bits(4)? as i32))).collect::<Result<_, CodecError>>()?
        } else {
            Vec::new()
        };
        let (money, corp_money, corp_credit, corp_rating, crim_rating) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?, r.i32()?);
        let spawn_timer = r.bits(16)? as i32;
        let received_actions = r.bits(8)?;
        let vitality = r.bits(10)? as i32;
        r.bits(6)?;
        r.bits(8)?;
        let mut display_inventory: [Vec<i32>; 7] = Default::default();

        for slot in &mut display_inventory {
            let n = r.bits(4)?;
            *slot = (0..n).map(|_| r.bits(16).map(|t| t as i32)).collect::<Result<_, _>>()?;
        }

        let n = r.bits(8)?;
        let mut menu_buttons = Vec::new();

        for _ in 0..n {
            let id = r.i32()?;
            let len = r.bits(6)? as usize;
            let text = String::from_utf8_lossy(r.bytes(len)?).into_owned();
            menu_buttons.push(MenuButton { id, text, extra: r.i32()? });
        }

        r.bits(8)?;
        let mut p = ServerGamePacket {
            client_id,
            received_actions,
            ticks_since_reset,
            sun_time,
            packet_sequence: 0,
            client_packet_count: 0,
            round_number,
            network_tick: 0,
            last_sdl_tick: 0,
            menu_type,
            menu_tab,
            shop,
            spawn_timer,
            vitality,
            money,
            corp_money,
            corp_credit,
            corp_rating,
            crim_rating,
            manager_tab,
            menu_buttons,
            display_inventory,
            gamestate,
            game_timer,
            game_timer_extra,
            corp_round,
            team_counts,
            ready_states,
            voice: Default::default(),
            follow_pos: Vec3::ZERO,
            own_human,
            humans: Vec::new(),
            items: Vec::new(),
            vehicles: Vec::new(),
            object_packs: Vec::new(),
            pack_offset: 0,
            line_links: Vec::new(),
            link_offset: 0,
            traffic_count: 0,
            traffic: Vec::new(),
            signal: (0, [0; 4]),
            global_event_count: 0,
            first_event: 0,
            events: Vec::new(),
        };
        d.read_objects(r, &mut p)?;

        for v in &mut p.voice {
            if r.bits(1)? != 1 {
                continue;
            }

            let (player_id, human_id, item_id) = (r.nullable(8)?, r.nullable(8)?, r.nullable(8)?);
            let mut frames: [VoiceFrame; 4] = Default::default();

            for f in &mut frames {
                f.index = r.bits(6)? as u8;
                f.size = r.bits(11)? as u16;
                f.volume = r.bits(2)? as u8;
                f.data = r.bytes(f.size as usize)?.to_vec();
            }

            *v = Some(ServerVoiceData { player_id, human_id, item_id, voice_frames: frames });
        }

        p.global_event_count = r.bits(16)?;
        let count = r.bits(6)?;
        p.first_event = r.bits(16)?;

        for k in 0..count {
            p.events.push((p.first_event.wrapping_add(k) & 0xffff, Event::read(r)?));
        }

        p.packet_sequence = r.u32()?;
        p.client_packet_count = r.u32()?;
        p.last_sdl_tick = r.u32()?;
        Ok(Some(p))
    }
}
