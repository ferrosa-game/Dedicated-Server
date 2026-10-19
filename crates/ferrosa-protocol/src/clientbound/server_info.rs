use crate::{
    GameMode,
    codec::{CodecError, Reader, WireRead, WireWrite, Writer},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ServerInfo {
    pub server_name: String,
    pub timestamp: u32,
    pub current_players: u8,
    pub max_players: u8,
    pub build: u8,
    pub gamemode: GameMode,
    pub address: [u8; 4],
    pub port: u16,
    pub password_protected: bool,
    /// The server's id in the list (always 80085 here).
    pub id: i32,
    /// The letter after the version (0 for a), from build 34 on.
    pub sub_version: u8,
}

impl WireWrite for ServerInfo {
    fn write(&self, w: &mut Writer) {
        w.byte(1);
        w.byte(self.build);
        w.bytes(&self.timestamp.to_le_bytes());

        w.bits(self.gamemode as i32, 4);
        w.bits(Writer::least_significant(self.current_players) as i32, 4);
        w.bits(Writer::most_significant(self.current_players) as i32, 4);
        w.bits(Writer::least_significant(self.max_players) as i32, 4);
        w.bits(Writer::most_significant(self.max_players) as i32, 4);
        w.bits(9, 4);

        w.string(&self.server_name, 32);
        w.bytes(&self.id.to_le_bytes());
        w.bytes(&self.address);
        w.bytes(&self.port.to_le_bytes());

        w.bits(self.password_protected as i32, 1);
        w.bits(self.sub_version as i32, 8);
        w.bits(71, 8);
    }
}

impl WireRead for ServerInfo {
    /// After the packet's type byte: what the server list reads, the address from build 27 on, the password flag from
    /// build 30 and the sub-version from build 34.
    fn read(r: &mut Reader) -> Result<Self, CodecError> {
        let build = r.bits(8)? as u8;
        let timestamp = r.u32()?;
        let gamemode = GameMode::try_from(r.bits(4)?)?;
        let current_players = r.bits(8)? as u8;
        let max_players = r.bits(8)? as u8;
        let server_name = String::from_utf8_lossy(&r.text(32)?).into_owned();
        let id = r.i32()?;
        let (mut address, mut port) = ([0; 4], 0);

        if build > 0x1a {
            address = r.bytes(4)?.try_into().unwrap();
            port = r.u16()?;
        }

        let password_protected = build > 0x1d && r.bits(1)? == 1;
        let mut sub_version = 0;

        if build > 0x21 {
            sub_version = r.bits(8)? as u8;
            r.bits(8)?;
        }

        Ok(Self { server_name, timestamp, current_players, max_players, build, gamemode, address, port, password_protected, id, sub_version })
    }
}
