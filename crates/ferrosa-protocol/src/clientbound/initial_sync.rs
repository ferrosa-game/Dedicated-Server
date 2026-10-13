use crate::{
    GameMode,
    codec::{CodecError, Reader, WireRead, WireWrite},
};

#[derive(Debug, Clone, PartialEq)]
pub struct InitialSync {
    pub round_number: u32,
    pub gamemode: GameMode,
    pub map_name: String,
    pub weekly_enabled: bool,
    pub weekday: u8,
    pub sun_angle: u16,
    pub sun_axial_tilt: u16,
    pub versus_movedelay: Option<u8>,
}

impl WireWrite for InitialSync {
    fn write(&self, w: &mut crate::codec::Writer) {
        w.byte(6);
        w.u32(self.round_number);
        w.bits(self.gamemode as i32, 4);
        w.bits(self.weekly_enabled as i32, 4);
        w.byte(self.weekday);
        w.string(&self.map_name, 32);
        w.u16(self.sun_angle);
        w.u16(self.sun_axial_tilt);

        if self.gamemode == GameMode::Versus {
            w.byte(self.versus_movedelay.unwrap_or(0))
        }
    }
}

impl WireRead for InitialSync {
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let round_number = r.u32()?;
        let gamemode = GameMode::try_from(r.bits(4)?)?;
        let weekly_enabled = r.bits(4)? != 0;
        let weekday = r.bits(8)? as u8;
        let map_name = String::from_utf8_lossy(&r.text(32)?).into_owned();
        let sun_angle = r.u16()?;
        let sun_axial_tilt = r.u16()?;
        let versus_movedelay = if gamemode == GameMode::Versus { Some(r.bits(8)? as u8) } else { None };
        Ok(Self { round_number, gamemode, map_name, weekly_enabled, weekday, sun_angle, sun_axial_tilt, versus_movedelay })
    }
}
