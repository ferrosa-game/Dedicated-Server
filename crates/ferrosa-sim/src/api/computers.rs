const FIELD_WIDTH: usize = 62;

use crate::{
    computer::{COLUMNS, Computer, LINES, Mode, SCREEN_LINES, WHITE, fs, put_text, text_len},
    items::state::ItemState,
    sim::Sim,
};

impl Sim {
    pub(crate) fn computer(&self, item: usize) -> Option<&Computer> {
        match self.items.get(item).map(|i| &i.state) {
            Some(ItemState::Computer(c)) => Some(c),
            _ => None,
        }
    }

    /// Every item that is a computer.
    pub fn computer_items(&self) -> Vec<usize> {
        self.items.ids().into_iter().filter(|&i| self.computer(i).is_some()).collect()
    }

    pub fn is_computer(&self, item: usize) -> bool {
        self.computer(item).is_some()
    }

    pub fn computer_mode(&self, item: usize) -> Option<Mode> {
        self.computer(item).map(|c| c.mode)
    }

    pub fn computer_team(&self, item: usize) -> Option<i32> {
        self.computer(item).map(|c| c.team).filter(|&t| t >= 0)
    }

    pub fn computer_program(&self, item: usize) -> Option<&str> {
        self.computer(item).filter(|c| c.mode == Mode::Plugin).map(|c| c.program.as_str())
    }

    pub fn computer_print(&mut self, item: usize, text: &str) {
        let text = text.as_bytes().to_vec();

        self.with_computer(item, |sim, c| {
            put_text(c.line_mut(), &text);
            sim.commit_line(item, c);
        });
    }

    pub fn computer_write_line(&mut self, item: usize, line: u32, text: &str, colour: u8) {
        self.computer_write_field(item, line, text, FIELD_WIDTH as u32, colour);
    }

    pub fn computer_write_field(&mut self, item: usize, line: u32, text: &str, width: u32, colour: u8) {
        if line as usize >= SCREEN_LINES {
            return;
        }

        let text = text.as_bytes().to_vec();
        let width = (width as usize).min(FIELD_WIDTH);

        self.with_computer(item, |sim, c| {
            c.field(line as usize, &text, width, colour);
            sim.register_line(item, c, line as i32);
        });
    }

    pub fn computer_write_cells(&mut self, item: usize, line: u32, text: &str, colours: &[u8]) {
        if line as usize >= SCREEN_LINES {
            return;
        }

        let line = line as usize;
        let text = text.as_bytes().to_vec();
        let colours = colours[..colours.len().min(FIELD_WIDTH)].to_vec();

        self.with_computer(item, |sim, c| {
            for (k, &colour) in colours.iter().enumerate() {
                c.lines[line][k] = text.get(k).copied().filter(|&b| b != 0).unwrap_or(b' ');
                c.colors[line][k] = colour;
            }

            c.lines[line][colours.len()] = 0;
            sim.register_line(item, c, line as i32);
        });
    }

    pub fn computer_read_line(&self, item: usize, line: u32) -> Option<String> {
        let c = self.computer(item)?;
        let l = c.lines.get(line as usize & (LINES - 1))?;

        Some(String::from_utf8_lossy(&l[..text_len(l)]).into_owned())
    }

    pub fn computer_clear(&mut self, item: usize) {
        self.with_computer(item, |sim, c| {
            for line in 0..SCREEN_LINES {
                c.lines[line][0] = 0;
                c.colors[line][..COLUMNS - 1].fill(WHITE);
                sim.register_line(item, c, line as i32);
            }
        });
    }

    pub fn computer_set_cursor(&mut self, item: usize, at: Option<(u32, u32)>) {
        self.with_computer(item, |_, c| {
            c.cursor = at.map_or(-1, |(line, column)| ((line as i32 & 0x1f) << 6) | (column as i32 & 0x3f));
        });
    }

    pub fn computer_drive_volume(&self, item: usize, drive: u32) -> Option<i32> {
        let volume = self.drive_volume(item, i32::try_from(drive).ok()?);

        self.fs.volume(volume).map(|_| volume)
    }

    pub(crate) fn name_program(&self, c: &mut Computer, token: &[u8], line: &[u8; COLUMNS], pos: i32) -> bool {
        let typed = String::from_utf8_lossy(token).into_owned();
        let Some(name) = self.plugins.programs().iter().find(|p| p.eq_ignore_ascii_case(&typed)) else { return false };
        let rest = String::from_utf8_lossy(&line[..text_len(line)]).into_owned();
        let args = rest.get(pos.max(0) as usize..).unwrap_or("").trim();

        c.program = (*name).to_string();
        c.args = args.strip_prefix(".EXE").unwrap_or(args).trim().to_string();

        true
    }

    pub(crate) fn start_named_program(&mut self, item: usize) {
        let plugins = self.plugins;
        let Some(c) = self.computer(item).filter(|c| c.mode == Mode::Shell && !c.program.is_empty()) else { return };
        let (program, args, line, top) = (c.program.clone(), c.args.clone(), c.current_line, c.top_line);

        self.with_computer(item, |_, c| {
            c.mode = Mode::Plugin;
            c.current_line = 0;
            c.top_line = 0;
            c.cursor = -1;
        });

        if plugins.on_computer_run(self, item, &program, &args) {
            return;
        }

        self.with_computer(item, |sim, c| {
            c.mode = Mode::Shell;
            c.program.clear();
            c.args.clear();
            c.current_line = line;
            c.top_line = top;
            sim.say(item, c, b"BAD COMMAND OR FILE NAME");
            sim.draw_prompt(item, c);
        });
    }

    pub fn exit_program(&mut self, item: usize) {
        self.end_program(item, true);
    }

    pub(crate) fn drop_program(&mut self, item: usize) {
        self.end_program(item, false);
    }

    fn end_program(&mut self, item: usize, restore: bool) {
        let plugins = self.plugins;

        if self.computer_mode(item) != Some(Mode::Plugin) {
            return;
        }

        self.with_computer(item, |sim, c| {
            c.mode = Mode::Shell;
            c.program.clear();
            c.args.clear();

            if !restore {
                return;
            }

            for line in 0..SCREEN_LINES {
                c.lines[line][0] = 0;
                c.colors[line][..COLUMNS - 1].fill(WHITE);
                sim.register_line(item, c, line as i32);
            }

            sim.draw_prompt(item, c);
        });

        plugins.on_computer_exit(self, item);
    }

    pub(crate) fn exit_all_programs(&mut self) {
        for item in self.computer_items() {
            self.exit_program(item);
        }
    }

    pub fn new_volume(&mut self, capacity: i32) -> Option<i32> {
        let volume = self.fs.alloc_volume(capacity);

        (volume != -1).then_some(volume)
    }

    pub fn volume_capacity(&self, volume: i32) -> Option<i32> {
        self.fs.volume(volume).map(|v| v.capacity)
    }

    pub fn volume_files(&self, volume: i32) -> usize {
        self.fs.volume(volume).map_or(0, |v| v.files.len())
    }

    pub fn add_volume_file(&mut self, volume: i32, name: &str, text: &str) -> Option<usize> {
        self.fs.volume(volume)?;

        let data = self.fs.alloc_content();

        if data == -1 {
            return None;
        }

        if let Some(content) = self.fs.content_mut(data) {
            content.text = text.as_bytes().to_vec();
        }

        let file = self.fs.add_file(volume, 0, name, data);

        if file == -1 {
            if let Some(content) = self.fs.content_mut(data) {
                content.active = false;
            }

            return None;
        }

        Some(file as usize)
    }

    fn file(&self, volume: i32, file: usize) -> Option<&fs::File> {
        self.fs.volume(volume)?.files.get(file)
    }

    pub fn file_name(&self, volume: i32, file: usize) -> Option<String> {
        self.file(volume, file).map(|f| f.name.clone())
    }

    pub fn file_text(&self, volume: i32, file: usize) -> Option<String> {
        let data = self.file(volume, file)?.data;

        self.fs.content(data).map(|c| String::from_utf8_lossy(&c.text).into_owned())
    }

    pub fn set_file_text(&mut self, volume: i32, file: usize, text: &str) {
        let Some(data) = self.file(volume, file).map(|f| f.data) else { return };

        if let Some(content) = self.fs.content_mut(data) {
            content.text = text.as_bytes().to_vec();
        }
    }

    pub fn file_encrypted(&self, volume: i32, file: usize) -> bool {
        self.file(volume, file).and_then(|f| self.fs.content(f.data)).is_some_and(|c| c.encrypted)
    }

    pub fn set_file_encrypted(&mut self, volume: i32, file: usize, encrypted: bool) {
        let Some(data) = self.file(volume, file).map(|f| f.data) else { return };

        if encrypted {
            self.fs.encipher(data);
        } else if let Some(content) = self.fs.content_mut(data) {
            content.encrypted = false;
        }
    }
}
