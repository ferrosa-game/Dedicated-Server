//! World mode's own missions: a project whose document sits on a lab computer, which the corporations race to find
//! and upload. The corporations' own mission slots, which the server keeps, are in `corp`.

mod corp;
mod daily;
mod intel;
mod intel_car;
mod intel_disk;
mod names;
mod spots;
mod text;
mod trade;
mod value;

pub use names::clock_time;

pub const MISSIONS: usize = 256;

pub const DAILY_MISSIONS: usize = 7;

/// The INTEL file of mission k is content k + 200.
pub const INTEL_CONTENT: i32 = 200;

const DISK_CAPACITY: i32 = 360;

/// A lab's computers, where document missions are put: the building, how many missions it has been given and its
/// computers' volumes.
#[derive(Clone, Debug, Default)]
pub struct MissionSpot {
    pub building: u32,
    pub used: i32,
    pub volumes: Vec<i32>,
}

/// A corporation's view of a world mission: what it is worth to it, how often it has asked for intel, its status
/// lines and status word.
#[derive(Clone, Debug, Default)]
pub struct MissionTeam {
    pub value: i32,
    pub status: i32,
    pub lines: Vec<String>,
    pub word: String,
    /// Whether the corporation has uploaded a file for it.
    pub uploaded: bool,
}

/// A world mission: a project whose document sits on a lab computer.
#[derive(Clone, Debug, Default)]
pub struct Mission {
    pub active: bool,
    /// 0 untouched, 1 being traded, 2 done.
    pub state: i32,
    pub reward: i32,
    pub teams: [MissionTeam; 6],
    /// The content of its .DOC file.
    pub document: i32,
    /// Its INTEL file's content and name.
    pub intel: i32,
    pub filename: String,
    /// The document's name, the lab's streets and the project name.
    pub names: [String; 4],
    pub high_value: bool,
}

/// What world mode keeps for its missions.
#[derive(Clone, Debug, Default)]
pub struct Missions {
    pub spots: Vec<MissionSpot>,
    pub missions: Vec<Mission>,
    /// Which project names are taken today.
    pub names_used: Vec<bool>,
    /// The mission paying five times its reward, and the INTEL files made so far.
    pub boosted: i32,
    pub intel_files: i32,
}
