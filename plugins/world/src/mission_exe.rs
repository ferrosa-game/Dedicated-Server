use ferrosa_wit::{Computer, Key, Program, ScreenColour, ScreenShade, Sound, server};

use crate::{World, missions::DAILY_MISSIONS, state::World as State};

/// The screen a plugin program draws on.
const SCREEN_LINES: u32 = 24;

/// The colour bytes the game's own MISSION.EXE draws in, as a background nibble and a foreground one.
const WHITE: u8 = 0x0f;
const SELECTED: u8 = 232;
const BUTTON: u8 = 184;
const HEADING: u8 = 240;

/// How long connecting takes, and an upload.
const CONNECT_TICKS: i32 = 256;
const UPLOAD_TICKS: i32 = 255;

/// The intel a corporation can ask for: 1 to 8 can request, 0 requested, 11 faxed.
const INTEL_FAXED: i32 = 11;

fn shade(n: u8) -> ScreenShade {
    match n & 0xf {
        0 => ScreenShade::Black,
        1 => ScreenShade::Blue,
        2 => ScreenShade::Green,
        3 => ScreenShade::Cyan,
        4 => ScreenShade::Red,
        5 => ScreenShade::Magenta,
        6 => ScreenShade::Brown,
        7 => ScreenShade::LightGrey,
        8 => ScreenShade::DarkGrey,
        9 => ScreenShade::BrightBlue,
        10 => ScreenShade::BrightGreen,
        11 => ScreenShade::BrightCyan,
        12 => ScreenShade::BrightRed,
        13 => ScreenShade::BrightMagenta,
        14 => ScreenShade::Yellow,
        _ => ScreenShade::White,
    }
}

/// The game's colour byte: the background in the high nibble, the foreground in the low one.
fn colour(byte: u8) -> ScreenColour {
    ScreenColour { foreground: shade(byte), background: shade(byte >> 4) }
}

/// A row written across `width` columns, the line ending there, as the game's own list screens do.
fn field(c: &Computer, line: u32, text: &str, width: u32, byte: u8) {
    c.write_field(line, text, width, colour(byte));
}

/// A line emptied: the game writes a 0 at the start, which `write-field` of no width does.
fn blank(c: &Computer, line: u32) {
    c.write_field(line, "", 0, colour(WHITE));
}

/// MISSION.EXE: the corporation's view of the day's missions, with intel to request and a file to upload.
pub struct MissionExe {
    /// 0 connecting, 1 uploading.
    kind: i32,
    timer: i32,
    /// 0 the mission list, 1 one mission, 2 the file list.
    page: i32,
    selected: i32,
    button: i32,
    file: i32,
    files: Vec<u32>,
    /// Counts on so the idle redraw only happens every sixteenth tick, as the server's own tick did.
    ticks: u32,
}

impl MissionExe {
    pub fn new(c: &Computer) -> Self {
        c.clear();

        for l in 0..SCREEN_LINES {
            blank(c, l);
        }

        field(c, 0, "CONNECTING...", 13, WHITE);

        if let Some(item) = server::get_item(c.item()) {
            server::play_sound(Sound::Modem, item.pos(), 1.0, 1.0);
        }

        Self { kind: 0, timer: CONNECT_TICKS, page: 0, selected: -1, button: 0, file: 0, files: Vec::new(), ticks: 0 }
    }

    /// The corporation whose base the computer is in, and the volume it reads; neither means the program quits.
    fn team_volume(&self, c: &Computer) -> Option<(usize, i32)> {
        let team = c.team().and_then(State::team_index)?;
        let volume = server::get_item(c.item()).and_then(|i| i.volume())?;

        Some((team, volume.id() as i32))
    }

    fn intel_text(world: &World, team: usize) -> &'static str {
        match server::get_corporation(crate::state::CORPORATIONS[team]).map_or(0, |c| c.intel()) {
            1..=8 => "REQUEST INTEL",
            0 => "INTEL REQUESTED",
            INTEL_FAXED => "INTEL FAXED",
            _ => {
                let _ = world;

                "NO INTEL AVAILABLE"
            }
        }
    }

    fn menu(&self, world: &World, c: &Computer, team: usize) {
        for row in 0..SCREEN_LINES {
            if row == 1 {
                field(c, 1, "MISSION SELECT", 16, WHITE);
            } else if (3..=12).contains(&row) {
                let r = row as i32 - 5;
                let text = match r {
                    -2 => Self::intel_text(world, team).to_string(),
                    -1 => "UPLOAD FILE".to_string(),
                    7 => "EXIT".to_string(),
                    _ => {
                        let m = world.state.missions.missions.get(r as usize);
                        let named = m.is_some_and(|m| m.state != 0);
                        let mut s = match (named, m) {
                            (true, Some(m)) => format!("{}. PROJECT {}                ", r + 1, m.names[3]),
                            _ => format!("{}. UNKNOWN                   ", r + 1),
                        };

                        s.truncate(24);
                        s += m.map_or("", |m| m.teams[team].word.as_str());
                        s
                    }
                };

                field(c, row, &text, 32, if self.selected == r { SELECTED } else { WHITE });
            } else {
                blank(c, row);
            }
        }
    }

    fn page(&self, world: &World, c: &Computer, team: usize) {
        let Ok(m) = usize::try_from(self.selected) else { return };
        let mission = world.state.missions.missions.get(m);

        for row in 0..SCREEN_LINES {
            match row {
                0 => {
                    let named = mission.is_some_and(|m| m.state != 0);
                    let t = match (named, mission) {
                        (true, Some(m)) => format!("PROJECT {}", m.names[3]),
                        _ => "UNKNOWN                     ".to_string(),
                    };

                    field(c, 0, &t, 20, WHITE);
                }
                3 => field(c, 3, "STATUS", 20, WHITE),
                4..=7 => match mission.and_then(|m| m.teams[team].lines.get(row as usize - 4)) {
                    Some(s) => field(c, row, s, 60, WHITE),
                    None => blank(c, row),
                },
                12 => field(c, 12, "BACK", 16, if self.button == 0 { BUTTON } else { WHITE }),
                _ => blank(c, row),
            }
        }
    }

    fn upload(&self, c: &Computer, volume: i32) {
        if self.timer > 0 {
            for row in 0..SCREEN_LINES {
                match row {
                    0 => field(c, 0, "UPLOADING", 16, HEADING),
                    1 => {
                        let done = (UPLOAD_TICKS - self.timer) >> 4;
                        let bar: String = (0..16).map(|k| if done < k { ' ' } else { '\u{7f}' }).collect();

                        field(c, 1, &bar, 16, HEADING);
                    }
                    _ => blank(c, row),
                }
            }

            return;
        }

        let names: Vec<String> = server::get_volume(volume).map_or(Vec::new(), |v| v.files().iter().map(|f| f.name()).collect());

        for row in 0..SCREEN_LINES {
            if row == 0 {
                field(c, 0, "UPLOAD FILE", 20, HEADING);
                continue;
            }

            let r = row as i32 - 4;

            if !(0..16).contains(&r) {
                blank(c, row);
                continue;
            }

            let k = r + (self.file & !15);
            let count = self.files.len() as i32;
            let byte = if k == self.file { BUTTON } else { WHITE };

            if k < count {
                let name = self.files.get(k as usize).and_then(|&f| names.get(f as usize)).cloned().unwrap_or_default();

                field(c, row, &format!(" {name}"), 16, byte);
            } else if k == count {
                field(c, row, " CANCEL", 16, byte);
            } else {
                blank(c, row);
            }
        }
    }

    fn redraw(&self, world: &World, c: &Computer, team: usize, volume: i32, menu_first: bool) {
        if menu_first && self.page == 0 {
            self.menu(world, c, team);
        }

        if self.page == 1 {
            self.page(world, c, team);
        }

        if self.page == 2 {
            self.upload(c, volume);
        }
    }

    /// The files on the computer's own volume, which the upload page lists.
    fn list_files(&mut self, volume: i32) {
        self.files = server::get_volume(volume).map_or(Vec::new(), |v| (0..v.files().len() as u32).collect());
    }
}

impl Program<World> for MissionExe {
    fn key(&mut self, world: &mut World, c: &Computer, key: Key, ctrl: bool) -> bool {
        let Some((team, volume)) = self.team_volume(c) else { return false };

        if ctrl {
            return key != Key::Character('X');
        }

        match self.page {
            0 => match key {
                Key::Up if self.selected >= -1 => self.selected -= 1,
                Key::Down if self.selected <= 6 => self.selected += 1,
                Key::Enter => match self.selected {
                    -1 => {
                        self.list_files(volume);
                        self.page = 2;
                        self.button = 0;
                        self.upload(c, volume);

                        return true;
                    }
                    -2 => {
                        if let Some(corp) = server::get_corporation(crate::state::CORPORATIONS[team])
                            && corp.intel() <= 8
                        {
                            corp.set_intel(0);
                        }
                    }
                    0..7 => {
                        self.page = 1;
                        self.button = 0;
                        self.redraw(world, c, team, volume, false);

                        return true;
                    }
                    _ => return false,
                },
                _ => {}
            },
            1 => {
                match key {
                    Key::Up if self.button > 0 => self.button -= 1,
                    Key::Down if self.button < 0 => self.button += 1,
                    Key::Enter if self.button == 0 => self.page = 0,
                    _ => {}
                }

                if self.page == 1 {
                    self.page(world, c, team);

                    return true;
                }
            }
            _ => {
                match key {
                    Key::Up if self.file > 0 => self.file -= 1,
                    Key::Down if self.file < self.files.len() as i32 => self.file += 1,
                    Key::Enter => {
                        if self.file >= self.files.len() as i32 {
                            self.page = 0;
                        } else {
                            self.kind = 1;
                            self.timer = UPLOAD_TICKS;
                        }
                    }
                    Key::Backspace => self.page = 0,
                    _ => {}
                }

                if self.page == 2 {
                    self.upload(c, volume);

                    return true;
                }
            }
        }

        self.redraw(world, c, team, volume, true);

        true
    }

    fn tick(&mut self, world: &mut World, c: &Computer) {
        let Some((team, volume)) = self.team_volume(c) else { return };
        self.ticks = self.ticks.wrapping_add(1);

        if self.timer <= 0 {
            if self.ticks & 0xf != 0 {
                return;
            }

            return self.redraw(world, c, team, volume, true);
        }

        self.timer -= 1;

        if self.kind == 1 && self.timer & 0xf == 0 {
            self.upload(c, volume);
        }

        if self.timer != 0 {
            return;
        }

        match self.kind {
            0 => self.menu(world, c, team),
            1 => {
                self.page = 0;
                let f = self.files.get(self.file as usize).copied().unwrap_or(0);
                let data = server::get_volume(volume).and_then(|v| v.files().get(f as usize).map(|f| f.content())).unwrap_or(-1);

                for m in 0..DAILY_MISSIONS {
                    if world.state.missions.missions.get(m).is_some_and(|x| x.document == data) {
                        if !world.state.missions.missions[m].teams[team].uploaded {
                            world.file_trade(m, team);
                        }

                        self.selected = m as i32;
                        self.page = 1;
                        self.button = 0;
                    }
                }

                self.redraw(world, c, team, volume, true);
            }
            _ => {}
        }
    }
}
