use ferrosa_wit::{Computer, Key, Program, ScreenColour, ScreenShade};

const WIDTH: usize = 32;
const HEIGHT: usize = 24;
const RING: usize = 64;
const QUARTERS: [(i32, i32); 4] = [(1, 1), (0x11, 1), (0x11, 0xb), (1, 0xb)];
const GAME_OVER: &str = "                           GAME OVER!";
const LEVEL_WALLS: [&[usize]; 7] = [
    &[0x48, 0x68, 0x88, 0xa8, 0xc8, 0xe8, 0x108, 0x276, 0x256, 0x236, 0x216, 0x1f6, 0x1d6, 0x1b6],
    &[0x56, 0x76, 0x96, 0xb6, 0xd6, 0xf6, 0x116, 0x268, 0x248, 0x228, 0x208, 0x1e8, 0x1c8, 0x1a8],
    &[0x161, 0x162, 0x163, 0x164, 0x165, 0x166, 0x167, 0x168],
    &[0x17d, 0x17c, 0x17b, 0x17a, 0x179, 0x178, 0x177, 0x176],
    &[0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93],
    &[0x22b, 0x22c, 0x22d, 0x22e, 0x22f, 0x230, 0x231, 0x232, 0x233],
    &[0x196, 0x28],
];

const HEADER: ScreenColour = ScreenColour { foreground: ScreenShade::Black, background: ScreenShade::LightGrey };
const PLAIN: ScreenColour = ScreenColour { foreground: ScreenShade::White, background: ScreenShade::Black };

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Wall,
    Body,
    Head,
    Apple,
}

impl Cell {
    fn colour(self) -> ScreenColour {
        let background = match self {
            Self::Empty => ScreenShade::Black,
            Self::Wall => ScreenShade::LightGrey,
            Self::Body => ScreenShade::Blue,
            Self::Head => ScreenShade::BrightBlue,
            Self::Apple => ScreenShade::Green,
        };

        ScreenColour { foreground: ScreenShade::White, background }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Heading {
    Right,
    Down,
    Left,
    Up,
}

pub struct Snake {
    score: u16,
    apples: i32,
    level: i32,
    countdown: i32,
    over: bool,
    heading: Heading,
    head: usize,
    tail: usize,
    ring: [(i32, i32); RING],
    board: [[Cell; WIDTH]; HEIGHT],
}

impl Snake {
    pub fn new(computer: &Computer) -> Self {
        let mut snake = Self {
            score: 0,
            apples: 0,
            level: 0,
            countdown: 0,
            over: false,
            heading: Heading::Right,
            head: 0,
            tail: 0,
            ring: [(0, 0); RING],
            board: [[Cell::Empty; WIDTH]; HEIGHT],
        };

        computer.set_cursor(None);
        snake.new_level(computer);

        snake
    }

    fn cell(&mut self, (x, y): (i32, i32)) -> &mut Cell {
        &mut self.board[(y as usize).min(HEIGHT - 1)][(x as usize).min(WIDTH - 1)]
    }

    fn new_level(&mut self, computer: &Computer) {
        for (y, row) in self.board.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let wall = y == 21 || y == 0 || (y < 21 && (x == 0 || x == 30));
                *cell = if wall { Cell::Wall } else { Cell::Empty };
            }
        }

        for walls in LEVEL_WALLS.iter().take((self.level & 7) as usize) {
            for &k in walls.iter() {
                self.board[k / WIDTH][k % WIDTH] = Cell::Wall;
            }
        }

        for &(left, top) in &QUARTERS {
            for _ in 0..2 {
                if let Some(spot) = self.free_spot(left, top) {
                    *self.cell(spot) = Cell::Apple;
                    self.apples += 1;
                }
            }
        }

        self.ring = [(15, 10); RING];
        self.tail = 0;
        self.head = 2;
        *self.cell((15, 10)) = Cell::Head;
        self.heading = Heading::Right;
        self.countdown = 64;
        self.level += 1;

        self.render(computer);
    }

    fn free_spot(&mut self, left: i32, top: i32) -> Option<(i32, i32)> {
        let roll = || (rand::random_range(0..12) + left, rand::random_range(0..9) + top);
        let mut spot = roll();

        for _ in 0..512 {
            if *self.cell(spot) == Cell::Empty {
                return Some(spot);
            }

            spot = roll();
        }

        (*self.cell(spot) == Cell::Empty).then_some(spot)
    }

    fn ahead(&self, (mut x, mut y): (i32, i32)) -> (i32, i32) {
        match self.heading {
            Heading::Right => x = if x + 1 >= 31 { 0 } else { x + 1 },
            Heading::Down => y = if y + 1 >= 22 { 0 } else { y + 1 },
            Heading::Left => x = if x - 1 < 0 { 31 - 1 } else { x - 1 },
            Heading::Up => y = if y - 1 < 0 { 22 - 1 } else { y - 1 },
        }

        (x, y)
    }

    fn eat(&mut self, at: (i32, i32)) {
        *self.cell(at) = Cell::Empty;
        self.score = self.score.wrapping_add(5);
        self.apples -= 1;

        if self.apples <= 0 {
            self.countdown = 32;
            self.apples = 0;
        }

        let end = self.ring[self.tail];

        for _ in 0..4 {
            self.tail = (self.tail + RING - 1) & (RING - 1);
            self.ring[self.tail] = end;
        }
    }

    fn render(&self, computer: &Computer) {
        computer.write_line(0, &format!("  SNAKE    SCORE:{:05}  LEVEL:{:03}    Ctrl-X EXIT", self.score, self.level), HEADER);

        for (row, cells) in self.board.iter().enumerate().skip(1) {
            let colours: Vec<ScreenColour> = (0..62).map(|k| cells[k >> 1].colour()).collect();

            computer.write_cells(row as u32, "", &colours);
        }

        if self.over {
            computer.write_line(1, GAME_OVER, PLAIN);
        }
    }
}

impl Program for Snake {
    fn key(&mut self, _computer: &Computer, key: Key, ctrl: bool) -> bool {
        if ctrl {
            return key != Key::Char('X');
        }

        self.heading = match key {
            Key::Right if self.heading != Heading::Left => Heading::Right,
            Key::Left if self.heading != Heading::Right => Heading::Left,
            Key::Down if self.heading != Heading::Up => Heading::Down,
            Key::Up if self.heading != Heading::Down => Heading::Up,
            _ => self.heading,
        };

        true
    }

    fn tick(&mut self, computer: &Computer) {
        if self.over {
            return;
        }

        self.countdown -= 1;

        if self.countdown > 0 {
            return;
        }

        if self.apples == 0 {
            return self.new_level(computer);
        }

        self.countdown = (16 - (self.level >> 3 << 2)).max(4);

        let at = self.ahead(self.ring[self.head & (RING - 1)]);

        self.head = (self.head + 1) & (RING - 1);
        self.ring[self.head] = at;

        if *self.cell(at) == Cell::Apple {
            self.eat(at);
        }

        if *self.cell(at) != Cell::Empty {
            self.over = true;
        }

        let tail = self.ring[self.tail];

        *self.cell(tail) = Cell::Empty;
        self.tail = (self.tail + 1) & (RING - 1);

        let mut k = self.tail;

        while k != self.head {
            let p = self.ring[k];

            *self.cell(p) = Cell::Body;
            k = (k + 1) & (RING - 1);
        }

        *self.cell(at) = Cell::Head;

        self.render(computer);
    }
}
