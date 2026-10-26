use ferrosa_wit::{Computer, Key, Program, ScreenColour, ScreenShade};

use crate::ComputerGames;

const SIDE: usize = 4;
const TILE_WIDTH: usize = 12;
const TILE_HEIGHT: usize = 4;
const COLUMNS: usize = 62;
const LEFT: usize = (COLUMNS - SIDE * TILE_WIDTH) / 2;

const TITLE: ScreenColour = ScreenColour { foreground: ScreenShade::Black, background: ScreenShade::White };
const PLAIN: ScreenColour = ScreenColour { foreground: ScreenShade::White, background: ScreenShade::Black };
const EMPTY: ScreenColour = ScreenColour { foreground: ScreenShade::White, background: ScreenShade::DarkGrey };

fn colour(value: u32) -> ScreenColour {
    let shades = [
        (ScreenShade::Black, ScreenShade::LightGrey),
        (ScreenShade::Black, ScreenShade::White),
        (ScreenShade::White, ScreenShade::Brown),
        (ScreenShade::White, ScreenShade::Red),
        (ScreenShade::Black, ScreenShade::BrightRed),
        (ScreenShade::White, ScreenShade::Magenta),
        (ScreenShade::Black, ScreenShade::BrightMagenta),
        (ScreenShade::Black, ScreenShade::Yellow),
        (ScreenShade::Black, ScreenShade::BrightGreen),
        (ScreenShade::Black, ScreenShade::BrightCyan),
        (ScreenShade::White, ScreenShade::BrightBlue),
    ];
    let step = value.max(2).ilog2() as usize - 1;
    let (foreground, background) = shades[step.min(shades.len() - 1)];

    ScreenColour { foreground, background }
}

fn slide(line: [u32; SIDE]) -> ([u32; SIDE], u32) {
    let mut out = [0; SIDE];
    let mut score = 0;
    let mut filled = 0;
    let mut held = None;

    for value in line.into_iter().filter(|&v| v != 0) {
        match held {
            Some(h) if h == value => {
                out[filled] = h + value;
                score += h + value;
                filled += 1;
                held = None;
            }
            Some(h) => {
                out[filled] = h;
                filled += 1;
                held = Some(value);
            }
            None => held = Some(value),
        }
    }

    if let Some(h) = held {
        out[filled] = h;
    }

    (out, score)
}

pub struct Tiles {
    board: [[u32; SIDE]; SIDE],
    score: u32,
    over: bool,
    won: bool,
}

impl Tiles {
    pub fn new(computer: &Computer) -> Self {
        let mut game = Self { board: [[0; SIDE]; SIDE], score: 0, over: false, won: false };

        computer.set_cursor(None);
        computer.clear();
        game.place();
        game.place();
        game.draw(computer);

        game
    }

    fn place(&mut self) {
        let free: Vec<(usize, usize)> = (0..SIDE).flat_map(|y| (0..SIDE).map(move |x| (y, x))).filter(|&(y, x)| self.board[y][x] == 0).collect();

        if let Some(&(y, x)) = free.get(rand::random_range(0..free.len().max(1))) {
            self.board[y][x] = if rand::random_bool(0.1) { 4 } else { 2 };
        }
    }

    fn square(heading: Key, i: usize, j: usize) -> (usize, usize) {
        match heading {
            Key::Left => (i, j),
            Key::Right => (i, SIDE - 1 - j),
            Key::Up => (j, i),
            _ => (SIDE - 1 - j, i),
        }
    }

    fn shift(&mut self, heading: Key) -> bool {
        let mut moved = false;

        for i in 0..SIDE {
            let line = std::array::from_fn(|j| {
                let (y, x) = Self::square(heading, i, j);

                self.board[y][x]
            });
            let (slid, score) = slide(line);

            self.score += score;

            for (j, &value) in slid.iter().enumerate() {
                let (y, x) = Self::square(heading, i, j);

                if self.board[y][x] != value {
                    self.board[y][x] = value;
                    moved = true;
                }
            }
        }

        moved
    }

    fn stuck(&self) -> bool {
        let pairs = (0..SIDE).flat_map(|y| (0..SIDE).map(move |x| (y, x))).any(|(y, x)| {
            let value = self.board[y][x];

            value == 0 || (x + 1 < SIDE && self.board[y][x + 1] == value) || (y + 1 < SIDE && self.board[y + 1][x] == value)
        });

        !pairs
    }

    fn draw(&self, computer: &Computer) {
        let state = if self.over {
            "  NO MOVES LEFT"
        } else if self.won {
            "  YOU WIN"
        } else {
            ""
        };

        computer.write_line(0, &format!("  2048    SCORE:{:06}{state}", self.score), TITLE);

        for line in 0..SIDE * TILE_HEIGHT {
            let (row, inner) = (line / TILE_HEIGHT, line % TILE_HEIGHT);
            let mut text = vec![b' '; COLUMNS];
            let mut colours = vec![PLAIN; COLUMNS];

            if inner != TILE_HEIGHT - 1 {
                for column in 0..SIDE * TILE_WIDTH {
                    let value = self.board[row][column / TILE_WIDTH];

                    if column % TILE_WIDTH == TILE_WIDTH - 1 {
                        continue;
                    }

                    colours[LEFT + column] = if value == 0 { EMPTY } else { colour(value) };
                }
            }

            if inner == TILE_HEIGHT / 2 {
                for (tile, &value) in self.board[row].iter().enumerate() {
                    if value == 0 {
                        continue;
                    }

                    let label = value.to_string();
                    let at = LEFT + tile * TILE_WIDTH + (TILE_WIDTH - 1 - label.len()) / 2;

                    text[at..at + label.len()].copy_from_slice(label.as_bytes());
                }
            }

            computer.write_cells((2 + line) as u32, &String::from_utf8_lossy(&text), &colours);
        }

        computer.write_line(ferrosa_wit::SHOWN_LINES - 1, "  arrows slide, R starts again, ctrl-X quits", TITLE);
    }
}

impl Program<ComputerGames> for Tiles {
    fn key(&mut self, _plugin: &mut ComputerGames, computer: &Computer, key: Key, ctrl: bool) -> bool {
        if ctrl {
            return key != Key::Character('X');
        }

        if key == Key::Character('R') {
            *self = Self::new(computer);
            return true;
        }

        if self.over || !matches!(key, Key::Left | Key::Right | Key::Up | Key::Down) {
            return true;
        }

        if self.shift(key) {
            self.place();
            self.won |= self.board.iter().flatten().any(|&v| v >= 2048);
            self.over = self.stuck();
        }

        self.draw(computer);

        true
    }
}
