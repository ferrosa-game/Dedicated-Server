use super::{Computer, Mode, SCREEN_LINES, WHITE, text_len};
use crate::sim::Sim;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MissionScreen {
    pub kind: i32,
    pub timer: i32,
    pub page: i32,
    pub selected: i32,
    pub button: i32,
    pub file: i32,
    pub dir: i32,
    pub files: Vec<i32>,
}

impl Computer {
    pub(crate) fn field(&mut self, l: usize, text: &[u8], width: usize, color: u8) {
        let n = text_len(text);

        for k in 0..width {
            self.lines[l][k] = if k < n { text[k] } else { b' ' };
            self.colors[l][k] = color;
        }

        self.lines[l][width] = 0;
    }
}

impl Sim {
    pub(crate) fn mission_init(&mut self, id: usize, c: &mut Computer) {
        c.current_line = 0;
        c.top_line = 0;
        c.mission = MissionScreen { selected: -1, ..Default::default() };

        for l in 0..SCREEN_LINES {
            c.lines[l][0] = 0;

            if l == 0 {
                super::put_text(&mut c.lines[0], b"CONNECTING...");
            }

            c.colors[l][..63].fill(WHITE);
            self.register_line(id, c, l as i32);
        }

        c.mission.kind = 0;
        c.mission.timer = 256;
        self.phone_sound(ferrosa_protocol::clientbound::game::events::sound::Sound::Modem, id, 1.0);
    }

    fn intel_text(&self, team: usize) -> &'static [u8] {
        match self.corp_state[team].intel {
            1..=8 => b"REQUEST INTEL",
            0 => b"INTEL REQUESTED",
            11 => b"INTEL FAXED",
            _ => b"NO INTEL AVAILABLE",
        }
    }

    fn mission_menu(&mut self, id: usize, c: &mut Computer) {
        let Some(team) = usize::try_from(c.team).ok() else {
            c.mission.page = 0;
            return;
        };

        for row in 0..24 {
            let l = row as usize;

            if row == 1 {
                c.field(1, b"MISSION SELECT", 16, WHITE);
            } else if (3..=12).contains(&row) {
                let r = row - 5;
                let text = match r {
                    -2 => self.intel_text(team).to_vec(),
                    -1 => b"UPLOAD FILE".to_vec(),
                    7 => b"EXIT".to_vec(),
                    _ => {
                        let m = self.modes.world.missions.missions.get(r as usize).cloned().unwrap_or_default();
                        let mut s = if m.state != 0 {
                            format!("{}. PROJECT {}                ", r + 1, m.names[3])
                        } else {
                            format!("{}. UNKNOWN                   ", r + 1)
                        };
                        s.truncate(24);
                        s += &m.teams[team].word;
                        s.into_bytes()
                    }
                };
                c.field(l, &text, 32, if c.mission.selected == r { 232 } else { WHITE });
            } else {
                c.lines[l][0] = 0;
            }
        }

        for l in 0..SCREEN_LINES as i32 {
            self.register_line(id, c, l);
        }
    }

    fn mission_page(&mut self, id: usize, c: &mut Computer) {
        let (Some(team), Ok(m)) = (usize::try_from(c.team).ok(), usize::try_from(c.mission.selected)) else {
            c.mission.page = 0;
            return;
        };
        let mission = self.modes.world.missions.missions.get(m).cloned().unwrap_or_default();

        for row in 0..SCREEN_LINES {
            match row {
                0 => {
                    let t = if mission.state != 0 { format!("PROJECT {}", mission.names[3]) } else { "UNKNOWN                     ".into() };
                    c.field(0, t.as_bytes(), 20, WHITE);
                }
                3 => c.field(3, b"STATUS", 20, WHITE),
                4..=7 => match mission.teams[team].lines.get(row - 4) {
                    Some(s) => c.field(row, s.as_bytes(), 60, WHITE),
                    None => c.lines[row][0] = 0,
                },
                12 => c.field(12, b"BACK", 16, if c.mission.button == 0 { 184 } else { WHITE }),
                _ => c.lines[row][0] = 0,
            }
        }

        for l in 0..SCREEN_LINES as i32 {
            self.register_line(id, c, l);
        }
    }

    fn mission_upload(&mut self, id: usize, c: &mut Computer) {
        let vol = self.items.get(id).map_or(-1, |i| i.volume);

        if c.mission.timer <= 0 {
            for row in 0..SCREEN_LINES {
                if row == 0 {
                    c.field(0, b"UPLOAD FILE", 20, 240);
                    continue;
                }

                let r = row as i32 - 4;

                if !(0..16).contains(&r) {
                    c.lines[row][0] = 0;
                    continue;
                }

                let k = r + (c.mission.file & !15);
                let count = c.mission.files.len() as i32;
                let color = if k == c.mission.file { 184 } else { WHITE };

                if k < count {
                    let f = c.mission.files[k as usize];
                    let name = self.fs.volume(vol).and_then(|v| v.files.get(f as usize)).map_or(String::new(), |f| f.name.clone());
                    c.field(row, format!(" {name}").as_bytes(), 16, color);
                } else if k == count {
                    c.field(row, b" CANCEL", 16, color);
                } else {
                    c.lines[row][0] = 0;
                }
            }
        } else {
            for row in 0..SCREEN_LINES {
                match row {
                    0 => c.field(0, b"UPLOADING", 16, 240),
                    1 => {
                        let done = (255 - c.mission.timer) >> 4;
                        let bar: Vec<u8> = (0..16).map(|k| if done < k { b' ' } else { 127 }).collect();
                        c.field(1, &bar, 16, 240);
                    }
                    _ => c.lines[row][0] = 0,
                }
            }
        }

        for l in 0..SCREEN_LINES as i32 {
            self.register_line(id, c, l);
        }
    }

    fn mission_exit(&mut self, id: usize, c: &mut Computer) {
        c.mode = Mode::Shell;

        for l in 0..SCREEN_LINES {
            c.lines[l][0] = 0;
            c.colors[l][..63].fill(WHITE);
            self.register_line(id, c, l as i32);
        }
    }

    fn mission_redraw(&mut self, id: usize, c: &mut Computer, menu_first: bool) {
        if menu_first && c.mission.page == 0 {
            self.mission_menu(id, c);
        }

        if c.mission.page == 1 {
            self.mission_page(id, c);
        }

        if c.mission.page == 2 {
            self.mission_upload(id, c);
        }
    }

    pub(crate) fn mission_key(&mut self, id: usize, c: &mut Computer, key: i32) {
        let vol = self.items.get(id).map_or(-1, |i| i.volume);
        let Some(team) = usize::try_from(c.team).ok().filter(|_| vol != -1) else { return self.mission_exit(id, c) };

        if key & 0x100 != 0 {
            if (key as u8).eq_ignore_ascii_case(&b'X') {
                self.mission_exit(id, c);
            }

            return;
        }

        let s = &mut c.mission;

        match s.page {
            0 => match key {
                16 => {
                    if s.selected >= -1 {
                        s.selected -= 1;
                    }
                }
                17 => {
                    if s.selected <= 6 {
                        s.selected += 1;
                    }
                }
                10 => match s.selected {
                    -1 => {
                        let dir = s.dir;
                        s.files = self
                            .fs
                            .volume(vol)
                            .map_or(Vec::new(), |v| v.files.iter().enumerate().filter(|(_, f)| f.dir == dir).map(|(k, _)| k as i32).collect());
                        s.page = 2;
                        s.button = 0;
                        return self.mission_upload(id, c);
                    }
                    -2 => {
                        if self.corp_state[team].intel <= 8 {
                            self.corp_state[team].intel = 0;
                        }
                    }
                    0..7 => {
                        s.page = 1;
                        s.button = 0;
                        return self.mission_redraw(id, c, false);
                    }
                    _ => return self.mission_exit(id, c),
                },
                _ => {}
            },
            1 => {
                match key {
                    16 => {
                        if s.button > 0 {
                            s.button -= 1;
                        }
                    }
                    17 => {
                        if s.button < 0 {
                            s.button += 1;
                        }
                    }
                    10 if s.button == 0 => s.page = 0,
                    _ => {}
                }

                if s.page == 1 {
                    return self.mission_page(id, c);
                }
            }
            _ => {
                match key {
                    16 => {
                        if s.file > 0 {
                            s.file -= 1;
                        }
                    }
                    17 => {
                        if s.file < s.files.len() as i32 {
                            s.file += 1;
                        }
                    }
                    10 => {
                        if s.file >= s.files.len() as i32 {
                            s.page = 0;
                        } else {
                            s.kind = 1;
                            s.timer = 255;
                        }
                    }
                    8 => s.page = 0,
                    _ => {}
                }

                if s.page == 2 {
                    return self.mission_upload(id, c);
                }
            }
        }

        self.mission_redraw(id, c, true);
    }

    pub(crate) fn mission_tick(&mut self, id: usize, c: &mut Computer) {
        let vol = self.items.get(id).map_or(-1, |i| i.volume);
        let Some(team) = usize::try_from(c.team).ok().filter(|_| vol != -1) else { return };

        if c.mission.timer <= 0 {
            if (self.tick ^ id as u32) & 0xf != 0 {
                return;
            }

            return self.mission_redraw(id, c, true);
        }

        c.mission.timer -= 1;

        if c.mission.kind == 1 {
            if c.mission.timer & 0xf != 0 {
                return;
            }

            self.mission_upload(id, c);
        }

        if c.mission.timer != 0 {
            return;
        }

        match c.mission.kind {
            0 => self.mission_menu(id, c),
            1 => {
                c.mission.page = 0;
                let f = c.mission.files.get(c.mission.file as usize).copied().unwrap_or(0);
                let data = self.fs.volume(vol).and_then(|v| v.files.get(f as usize)).map_or(-1, |f| f.data);

                for m in 0..7 {
                    if self.modes.world.missions.missions.get(m).is_some_and(|x| x.document == data) {
                        if !self.modes.world.missions.missions[m].teams[team].uploaded {
                            self.file_trade(m, team);
                        }

                        c.mission.selected = m as i32;
                        c.mission.page = 1;
                        c.mission.button = 0;
                    }
                }

                self.mission_redraw(id, c, true);
            }
            _ => {}
        }
    }
}
