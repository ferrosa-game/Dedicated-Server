mod body;
mod convert;
mod corporation;
mod human;
mod item;
mod player;
mod vehicle;

pub use body::BodyRef;
pub use corporation::CorporationRef;
pub use human::HumanRef;
pub use item::ItemRef;
pub use player::PlayerRef;
pub use vehicle::VehicleRef;

use crate::{bindings::ferrosa::plugin::types, host::HostState};

impl types::Host for HostState {}
