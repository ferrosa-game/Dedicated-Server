//! Real time in server ticks, and the world clock.

/// The server runs 60 ticks a second.
pub const TICKS_PER_SECOND: i32 = 60;

pub const TICKS_PER_SECOND_F32: f32 = TICKS_PER_SECOND as f32;
//pub const TICKS_PER_MINUTE_F32: f32 = 60.0 * TICKS_PER_SECOND_F32;

/// The world clock counts sixtieths of a world second.
pub const WORLD_MINUTE: i32 = 3600;
pub const WORLD_HOUR: i32 = 60 * WORLD_MINUTE;

#[inline]
pub fn game_seconds(time: i32) -> i32 {
    time * 60
}

#[inline]
pub fn game_minutes(time: i32) -> i32 {
    time * 60 * TICKS_PER_SECOND
}
