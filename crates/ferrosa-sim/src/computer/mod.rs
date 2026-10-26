pub mod decrypt;
pub mod fs;
pub mod links;
pub mod shell;

pub const LINES: usize = 32;
pub const COLUMNS: usize = 64;
pub const SCREEN_LINES: usize = 24;
pub const WHITE: u8 = 0xf;
pub const DRIVE_A: i32 = 0;
pub const DRIVE_C: i32 = 2;
pub const DRIVES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Mode {
    #[default]
    Shell = 0,
    Decrypt = 1,
    Mission = 2,
    Plugin = 4,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DirStack {
    pub depth: i32,
    pub path: [i32; 17],
}

impl DirStack {
    pub(crate) fn current(&self) -> i32 {
        self.path.get(self.depth as usize).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiskOp {
    pub kind: i32,
    pub timer: i32,
    pub drive: i32,
    pub dir: i32,
    pub next_dir: i32,
    pub next_file: i32,
    pub name: String,
    pub data: i32,
    pub keys: Vec<i32>,
}

pub const DISK_OP_COPY: i32 = 0;
pub const DISK_OP_DIR: i32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct Computer {
    pub current_line: i32,
    pub top_line: i32,
    /// line * 64 + column
    pub cursor: i32,
    pub lines: [[u8; COLUMNS]; LINES],
    pub colors: [[u8; COLUMNS]; LINES],
    pub prompt: [u8; COLUMNS],
    pub input: [u8; COLUMNS],
    pub drive: i32,
    pub dirs: [DirStack; DRIVES],
    pub op: DiskOp,
    pub team: i32,
    pub mode: Mode,
    pub program: String,
    pub args: String,
    pub decrypt: decrypt::Decrypter,
}

impl Default for Computer {
    fn default() -> Self {
        Self {
            current_line: 0,
            top_line: 0,
            cursor: 0,
            lines: [[0; COLUMNS]; LINES],
            colors: [[0; COLUMNS]; LINES],
            prompt: [0; COLUMNS],
            input: [0; COLUMNS],
            drive: 0,
            dirs: [DirStack::default(); DRIVES],
            op: DiskOp::default(),
            team: 0,
            mode: Mode::Shell,
            program: String::new(),
            args: String::new(),
            decrypt: decrypt::Decrypter::default(),
        }
    }
}

pub fn text_len(s: &[u8]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}

pub fn put_text(line: &mut [u8; COLUMNS], s: &[u8]) {
    let n = s.len().min(COLUMNS - 1);
    line[..n].copy_from_slice(&s[..n]);
    line[n] = 0;
}

impl Computer {
    /// A row written across the first `width` columns only, the line ending there, as the game's own list screens
    /// highlight a row.
    pub(crate) fn field(&mut self, l: usize, text: &[u8], width: usize, color: u8) {
        let n = text_len(text);

        for k in 0..width {
            self.lines[l][k] = if k < n { text[k] } else { b' ' };
            self.colors[l][k] = color;
        }

        self.lines[l][width] = 0;
    }


    pub(crate) fn line_mut(&mut self) -> &mut [u8; COLUMNS] {
        &mut self.lines[self.current_line as usize & (LINES - 1)]
    }

    pub(crate) fn banner(&mut self, l: usize, text: &[u8], color: u8) {
        let n = text_len(text);

        for k in 0..62 {
            self.lines[l][k] = if k < n { text[k] } else { b' ' };
            self.colors[l][k] = color;
        }

        self.lines[l][62] = 0;
    }
}
