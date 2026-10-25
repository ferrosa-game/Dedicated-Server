use ferrosa_wit::{Computer, Key, Program, ScreenColour, ScreenShade, server};

use crate::Sandbox;

const TITLE: ScreenColour = ScreenColour { foreground: ScreenShade::Black, background: ScreenShade::White };
const HIGHLIGHT: ScreenColour = ScreenColour { foreground: ScreenShade::DarkGrey, background: ScreenShade::Yellow };
const BODY: ScreenColour = ScreenColour { foreground: ScreenShade::White, background: ScreenShade::Black };
const FIELD: u32 = 20;
const ROWS: usize = ferrosa_wit::SHOWN_LINES as usize - 2;

pub struct PhoneBook {
    numbers: Vec<i32>,
    at: usize,
    top: usize,
}

impl PhoneBook {
    pub fn new(computer: &Computer, args: &str) -> Self {
        let mut numbers: Vec<i32> = server::items().iter().filter_map(|i| i.phone_number()).collect();

        numbers.sort_unstable();

        if args.eq_ignore_ascii_case("/R") {
            numbers.reverse();
        }

        let book = Self { numbers, at: 0, top: 0 };

        computer.set_cursor(None);
        book.draw(computer);

        book
    }

    fn draw(&self, computer: &Computer) {
        computer.write_line(0, &format!(" PHONE BOOK  {} of {}", (self.at + 1).min(self.numbers.len()), self.numbers.len()), TITLE);

        for row in 0..ROWS {
            let k = self.top + row;
            let line = self.numbers.get(k).map_or_else(String::new, |n| format!("  {n:07}"));
            let colour = if k == self.at && !line.is_empty() { HIGHLIGHT } else { BODY };

            computer.write_field(row as u32 + 1, &line, FIELD, colour);
        }

        computer.write_line(ferrosa_wit::SHOWN_LINES - 1, " arrows pick, ctrl-X quits", TITLE);
    }

    fn follow(&mut self) {
        self.top = self.top.min(self.at).max((self.at + 1).saturating_sub(ROWS));
    }
}

impl Program<Sandbox> for PhoneBook {
    fn key(&mut self, _plugin: &mut Sandbox, computer: &Computer, key: Key, ctrl: bool) -> bool {
        if ctrl && key == Key::Char('X') {
            return false;
        }

        let last = self.numbers.len().saturating_sub(1);

        match key {
            Key::Up => self.at = self.at.saturating_sub(1),
            Key::Down => self.at = (self.at + 1).min(last),
            Key::Left => self.at = self.at.saturating_sub(ROWS),
            Key::Right => self.at = (self.at + ROWS).min(last),
            _ => {}
        }

        self.follow();
        self.draw(computer);

        true
    }
}
