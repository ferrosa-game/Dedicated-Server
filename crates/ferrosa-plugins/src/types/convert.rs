use ferrosa_physics::RotMatrix;
use ferrosa_protocol::{
    GameMode, SuitColor, Team,
    clientbound::game::{
        GameState, ItemKind, MenuType, VehicleKind,
        events::{chat::ChatType, sound::Sound},
    },
};
use ferrosa_sim::{BoneId, DamageCause};

use crate::bindings::ferrosa::plugin::types::{self, ClientMode, Mat3, Mission, Vec3};

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

impl From<types::Team> for Team {
    fn from(team: types::Team) -> Self {
        match team {
            types::Team::Goldmen => Self::Goldmen,
            types::Team::Monsota => Self::Monsota,
            types::Team::Oxs => Self::OXS,
            types::Team::Nexaco => Self::Nexaco,
            types::Team::Pentacom => Self::Pentacom,
            types::Team::Prodocon => Self::Prodocon,
            types::Team::Mission => Self::Mission,
            types::Team::Spectator => Self::Spectator,
        }
    }
}

impl From<types::Sound> for Sound {
    fn from(sound: types::Sound) -> Self {
        match sound {
            types::Sound::CarCrash => Self::CarCrash,
            types::Sound::BodyHit => Self::BodyHit,
            types::Sound::PhoneRing => Self::PhoneRing,
            types::Sound::PhoneKey0 => Self::PhoneKey0,
            types::Sound::PhoneKey1 => Self::PhoneKey1,
            types::Sound::PhoneKey2 => Self::PhoneKey2,
            types::Sound::PhoneKey3 => Self::PhoneKey3,
            types::Sound::PhoneKey4 => Self::PhoneKey4,
            types::Sound::PhoneKey5 => Self::PhoneKey5,
            types::Sound::PhoneKey6 => Self::PhoneKey6,
            types::Sound::PhoneKey7 => Self::PhoneKey7,
            types::Sound::PhoneKey8 => Self::PhoneKey8,
            types::Sound::PhoneKey9 => Self::PhoneKey9,
            types::Sound::PhoneBusy => Self::PhoneBusy,
            types::Sound::Reload => Self::Reload,
            types::Sound::GearShift => Self::GearShift,
            types::Sound::Modem => Self::Modem,
            types::Sound::Floppy => Self::Floppy,
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

impl From<ferrosa_sim::Mission> for Mission {
    fn from(m: ferrosa_sim::Mission) -> Self {
        Self {
            active: m.active,
            done: m.done,
            id: m.id,
            kind: m.kind,
            item: id_of(m.item),
            item2: id_of(m.item2),
            team1: m.team1,
            team2: m.team2,
            disk_type: m.disk_type,
            value: m.value,
            deadline: m.deadline,
            location: id_of(m.location),
            provided_cash: m.provided_cash,
            rate: m.rate,
        }
    }
}

/// A mission's item or spot: the binary's -1 for none.
fn id_of(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
}

fn index_of(id: Option<u32>) -> i32 {
    id.and_then(|v| i32::try_from(v).ok()).unwrap_or(-1)
}

impl From<Mission> for ferrosa_sim::Mission {
    fn from(m: Mission) -> Self {
        Self {
            active: m.active,
            done: m.done,
            id: m.id,
            kind: m.kind,
            item: index_of(m.item),
            item2: index_of(m.item2),
            team1: m.team1,
            team2: m.team2,
            disk_type: m.disk_type,
            value: m.value,
            deadline: m.deadline,
            location: index_of(m.location),
            provided_cash: m.provided_cash,
            rate: m.rate,
            result: None,
        }
    }
}

impl From<BoneId> for types::Bone {
    fn from(b: BoneId) -> Self {
        match b {
            BoneId::Pelvis => Self::Pelvis,
            BoneId::Stomach => Self::Stomach,
            BoneId::Torso => Self::Torso,
            BoneId::Head => Self::Head,
            BoneId::ShoulderLeft => Self::ShoulderLeft,
            BoneId::ForearmLeft => Self::ForearmLeft,
            BoneId::HandLeft => Self::HandLeft,
            BoneId::ShoulderRight => Self::ShoulderRight,
            BoneId::ForearmRight => Self::ForearmRight,
            BoneId::HandRight => Self::HandRight,
            BoneId::ThighLeft => Self::ThighLeft,
            BoneId::ShinLeft => Self::ShinLeft,
            BoneId::FootLeft => Self::FootLeft,
            BoneId::ThighRight => Self::ThighRight,
            BoneId::ShinRight => Self::ShinRight,
            BoneId::FootRight => Self::FootRight,
        }
    }
}

impl From<DamageCause> for types::DamageCause {
    fn from(c: DamageCause) -> Self {
        let id = |p: Option<ferrosa_sim::PlayerId>| p.map(|p| p.0);

        match c {
            DamageCause::Bullet(p) => Self::Bullet(id(p)),
            DamageCause::Explosion(p) => Self::Explosion(id(p)),
            DamageCause::Fall => Self::Fall,
            DamageCause::Vehicle(p) => Self::Vehicle(id(p)),
        }
    }
}

impl From<ferrosa_sim::HumanHealth> for types::Health {
    fn from(h: ferrosa_sim::HumanHealth) -> Self {
        Self {
            health: h.health,
            blood: h.blood_level,
            vitality: h.vitality,
            chest: h.chest,
            head: h.head,
            left_arm: h.left_arm,
            right_arm: h.right_arm,
            left_leg: h.left_leg,
            right_leg: h.right_leg,
        }
    }
}

impl From<types::Health> for ferrosa_sim::HumanHealth {
    fn from(h: types::Health) -> Self {
        Self {
            health: h.health,
            blood_level: h.blood,
            vitality: h.vitality,
            chest: h.chest,
            head: h.head,
            left_arm: h.left_arm,
            right_arm: h.right_arm,
            left_leg: h.left_leg,
            right_leg: h.right_leg,
        }
    }
}

impl From<types::Bone> for BoneId {
    fn from(b: types::Bone) -> Self {
        match b {
            types::Bone::Pelvis => Self::Pelvis,
            types::Bone::Stomach => Self::Stomach,
            types::Bone::Torso => Self::Torso,
            types::Bone::Head => Self::Head,
            types::Bone::ShoulderLeft => Self::ShoulderLeft,
            types::Bone::ForearmLeft => Self::ForearmLeft,
            types::Bone::HandLeft => Self::HandLeft,
            types::Bone::ShoulderRight => Self::ShoulderRight,
            types::Bone::ForearmRight => Self::ForearmRight,
            types::Bone::HandRight => Self::HandRight,
            types::Bone::ThighLeft => Self::ThighLeft,
            types::Bone::ShinLeft => Self::ShinLeft,
            types::Bone::FootLeft => Self::FootLeft,
            types::Bone::ThighRight => Self::ThighRight,
            types::Bone::ShinRight => Self::ShinRight,
            types::Bone::FootRight => Self::FootRight,
        }
    }
}

impl From<ferrosa_protocol::Gender> for types::Gender {
    fn from(g: ferrosa_protocol::Gender) -> Self {
        match g {
            ferrosa_protocol::Gender::Female => Self::Female,
            ferrosa_protocol::Gender::Male => Self::Male,
        }
    }
}

impl From<types::Gender> for ferrosa_protocol::Gender {
    fn from(g: types::Gender) -> Self {
        match g {
            types::Gender::Female => Self::Female,
            types::Gender::Male => Self::Male,
        }
    }
}

impl From<ferrosa_protocol::Model> for types::Model {
    fn from(m: ferrosa_protocol::Model) -> Self {
        match m {
            ferrosa_protocol::Model::Casual => Self::Casual,
            ferrosa_protocol::Model::Suit => Self::Suit,
            ferrosa_protocol::Model::Suit2 => Self::Suit2,
        }
    }
}

impl From<types::Model> for ferrosa_protocol::Model {
    fn from(m: types::Model) -> Self {
        match m {
            types::Model::Casual => Self::Casual,
            types::Model::Suit => Self::Suit,
            types::Model::Suit2 => Self::Suit2,
        }
    }
}

impl From<SuitColor> for types::SuitColour {
    fn from(c: SuitColor) -> Self {
        match c {
            SuitColor::Brown => Self::Brown,
            SuitColor::Black => Self::Black,
            SuitColor::Grey => Self::Grey,
            SuitColor::Navy => Self::Navy,
            SuitColor::Teal => Self::Teal,
            SuitColor::Purple => Self::Purple,
            SuitColor::Tan => Self::Tan,
            SuitColor::White => Self::White,
            SuitColor::Red => Self::Red,
            SuitColor::Indigo => Self::Indigo,
            SuitColor::DarkNavy => Self::DarkNavy,
            SuitColor::DarkGrey => Self::DarkGrey,
        }
    }
}

impl From<types::SuitColour> for SuitColor {
    fn from(c: types::SuitColour) -> Self {
        match c {
            types::SuitColour::Brown => Self::Brown,
            types::SuitColour::Black => Self::Black,
            types::SuitColour::Grey => Self::Grey,
            types::SuitColour::Navy => Self::Navy,
            types::SuitColour::Teal => Self::Teal,
            types::SuitColour::Purple => Self::Purple,
            types::SuitColour::Tan => Self::Tan,
            types::SuitColour::White => Self::White,
            types::SuitColour::Red => Self::Red,
            types::SuitColour::Indigo => Self::Indigo,
            types::SuitColour::DarkNavy => Self::DarkNavy,
            types::SuitColour::DarkGrey => Self::DarkGrey,
        }
    }
}

impl From<MenuType> for types::Menu {
    fn from(menu: MenuType) -> Self {
        match menu {
            MenuType::Empty => Self::Empty,
            MenuType::EnterCity => Self::EnterCity,
            MenuType::Lobby => Self::Lobby,
            MenuType::EmptyBase => Self::EmptyBase,
            MenuType::WorldCarShop => Self::WorldCarShop,
            MenuType::WorldStore => Self::WorldStore,
            MenuType::WorldStoreDone => Self::WorldStoreDone,
            MenuType::WorldBank => Self::WorldBank,
            MenuType::WorldBank2 => Self::WorldBank2,
            MenuType::RoundCorpWeapons => Self::RoundCorpWeapons,
            MenuType::RoundCorpAmmo => Self::RoundCorpAmmo,
            MenuType::RoundCorpEquip => Self::RoundCorpEquip,
            MenuType::RoundCorpVehicle => Self::RoundCorpVehicle,
            MenuType::RoundCorpStock => Self::RoundCorpStock,
            MenuType::WorldEmptyCorp => Self::WorldEmptyCorp,
            MenuType::WorldCorpApplication => Self::WorldCorpApplication,
            MenuType::WorldCorpTabs => Self::WorldCorpTabs,
            MenuType::WorldCorpHiring => Self::WorldCorpHiring,
            MenuType::WorldCorpFiring => Self::WorldCorpFiring,
            MenuType::WorldCorpTeam => Self::WorldCorpTeam,
            MenuType::WorldCorpRequistion => Self::WorldCorpRequisition,
        }
    }
}

impl From<types::Menu> for MenuType {
    fn from(menu: types::Menu) -> Self {
        match menu {
            types::Menu::Empty => Self::Empty,
            types::Menu::EnterCity => Self::EnterCity,
            types::Menu::Lobby => Self::Lobby,
            types::Menu::EmptyBase => Self::EmptyBase,
            types::Menu::WorldCarShop => Self::WorldCarShop,
            types::Menu::WorldStore => Self::WorldStore,
            types::Menu::WorldStoreDone => Self::WorldStoreDone,
            types::Menu::WorldBank => Self::WorldBank,
            types::Menu::WorldBank2 => Self::WorldBank2,
            types::Menu::RoundCorpWeapons => Self::RoundCorpWeapons,
            types::Menu::RoundCorpAmmo => Self::RoundCorpAmmo,
            types::Menu::RoundCorpEquip => Self::RoundCorpEquip,
            types::Menu::RoundCorpVehicle => Self::RoundCorpVehicle,
            types::Menu::RoundCorpStock => Self::RoundCorpStock,
            types::Menu::WorldEmptyCorp => Self::WorldEmptyCorp,
            types::Menu::WorldCorpApplication => Self::WorldCorpApplication,
            types::Menu::WorldCorpTabs => Self::WorldCorpTabs,
            types::Menu::WorldCorpHiring => Self::WorldCorpHiring,
            types::Menu::WorldCorpFiring => Self::WorldCorpFiring,
            types::Menu::WorldCorpTeam => Self::WorldCorpTeam,
            types::Menu::WorldCorpRequisition => Self::WorldCorpRequistion,
        }
    }
}

/// A kind's number on the wire is its place in the WIT enum, so the two lists must stay in the same order: plugins
/// pass a kind as that number. These check it at compile time, and the COUNT checks catch a kind added to the
/// protocol without a name in the WIT.
const _: () = {
    const ITEMS: [(types::ItemKind, ItemKind); 46] = [
        (types::ItemKind::Auto5, ItemKind::Auto5),
        (types::ItemKind::Ak47, ItemKind::Ak47),
        (types::ItemKind::Ak47Mag, ItemKind::Ak47Mag),
        (types::ItemKind::M16, ItemKind::M16),
        (types::ItemKind::M16Mag, ItemKind::M16Mag),
        (types::ItemKind::Magnum, ItemKind::Magnum),
        (types::ItemKind::MagnumMag, ItemKind::MagnumMag),
        (types::ItemKind::Mp5, ItemKind::Mp5),
        (types::ItemKind::Mp5Mag, ItemKind::Mp5Mag),
        (types::ItemKind::Uzi, ItemKind::Uzi),
        (types::ItemKind::UziMag, ItemKind::UziMag),
        (types::ItemKind::Pistol, ItemKind::Pistol),
        (types::ItemKind::PistolMag, ItemKind::PistolMag),
        (types::ItemKind::Grenade, ItemKind::Grenade),
        (types::ItemKind::Bandage, ItemKind::Bandage),
        (types::ItemKind::Briefcase, ItemKind::Briefcase),
        (types::ItemKind::BriefcaseOpen, ItemKind::BriefcaseOpen),
        (types::ItemKind::CashRound, ItemKind::CashRound),
        (types::ItemKind::CashWorld, ItemKind::CashWorld),
        (types::ItemKind::DiskBlack, ItemKind::DiskBlack),
        (types::ItemKind::DiskGreen, ItemKind::DiskGreen),
        (types::ItemKind::DiskBlue, ItemKind::DiskBlue),
        (types::ItemKind::DiskWhite, ItemKind::DiskWhite),
        (types::ItemKind::DiskGold, ItemKind::DiskGold),
        (types::ItemKind::DiskRed, ItemKind::DiskRed),
        (types::ItemKind::Phone, ItemKind::Phone),
        (types::ItemKind::Radio, ItemKind::Radio),
        (types::ItemKind::Key, ItemKind::Key),
        (types::ItemKind::Door, ItemKind::Door),
        (types::ItemKind::PaperWorld, ItemKind::PaperWorld),
        (types::ItemKind::Burger, ItemKind::Burger),
        (types::ItemKind::Desk, ItemKind::Desk),
        (types::ItemKind::Lamp, ItemKind::Lamp),
        (types::ItemKind::PhonePay, ItemKind::PhonePay),
        (types::ItemKind::Paper, ItemKind::Paper),
        (types::ItemKind::SoccerBall, ItemKind::SoccerBall),
        (types::ItemKind::Rope, ItemKind::Rope),
        (types::ItemKind::Box, ItemKind::Box),
        (types::ItemKind::BigBox, ItemKind::BigBox),
        (types::ItemKind::Computer, ItemKind::Computer),
        (types::ItemKind::Arcade, ItemKind::Arcade),
        (types::ItemKind::Table, ItemKind::Table),
        (types::ItemKind::TableTest, ItemKind::TableTest),
        (types::ItemKind::Wall, ItemKind::Wall),
        (types::ItemKind::Bottle, ItemKind::Bottle),
        (types::ItemKind::Watermelon, ItemKind::Watermelon),
    ];

    const VEHICLES: [(types::VehicleKind, VehicleKind); 18] = [
        (types::VehicleKind::TownCar, VehicleKind::TownCar),
        (types::VehicleKind::TownCar2, VehicleKind::TownCar2),
        (types::VehicleKind::Metro, VehicleKind::Metro),
        (types::VehicleKind::Limo, VehicleKind::Limo),
        (types::VehicleKind::Turbo, VehicleKind::Turbo),
        (types::VehicleKind::TurboS, VehicleKind::TurboS),
        (types::VehicleKind::Beamer, VehicleKind::Beamer),
        (types::VehicleKind::Van, VehicleKind::Van),
        (types::VehicleKind::Van2, VehicleKind::Van2),
        (types::VehicleKind::Minivan, VehicleKind::Minivan),
        (types::VehicleKind::Truck, VehicleKind::Truck),
        (types::VehicleKind::Trailer, VehicleKind::Trailer),
        (types::VehicleKind::Helicopter, VehicleKind::Helicopter),
        (types::VehicleKind::Train, VehicleKind::Train),
        (types::VehicleKind::NoClip, VehicleKind::NoClip),
        (types::VehicleKind::Hatchback, VehicleKind::Hatchback),
        (types::VehicleKind::Test, VehicleKind::Test),
        (types::VehicleKind::Pickup, VehicleKind::Pickup),
    ];

    assert!(ItemKind::COUNT == ITEMS.len());
    assert!(VehicleKind::COUNT == VEHICLES.len());

    let mut k = 0;

    while k < ITEMS.len() {
        assert!(ITEMS[k].0 as u32 == ITEMS[k].1 as u32);
        k += 1;
    }

    let mut k = 0;

    while k < VEHICLES.len() {
        assert!(VEHICLES[k].0 as u32 == VEHICLES[k].1 as u32);
        k += 1;
    }
};
