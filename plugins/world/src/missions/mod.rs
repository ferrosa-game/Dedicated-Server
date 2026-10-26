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

#[derive(Clone, Debug, Default)]
pub struct MissionSpot {
    pub building: u32,
    pub used: i32,
    pub volumes: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct MissionTeam {
    pub value: i32,
    pub status: i32,
    pub lines: Vec<String>,
    pub word: String,
    pub uploaded: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Mission {
    pub active: bool,
    pub state: i32,
    pub reward: i32,
    pub teams: [MissionTeam; 6],
    pub document: i32,
    pub intel: i32,
    pub filename: String,
    pub names: [String; 4],
    pub high_value: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Missions {
    pub spots: Vec<MissionSpot>,
    pub missions: Vec<Mission>,
    pub names_used: Vec<bool>,
    pub boosted: i32,
    pub intel_files: i32,
}
