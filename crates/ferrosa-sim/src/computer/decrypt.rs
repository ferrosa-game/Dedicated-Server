use super::{Computer, Mode, SCREEN_LINES};
use crate::{sim::Sim, time::game_seconds};

const HEADER: &[u8] = b"DECRYPTER     Ctrl-R RESET   Ctrl-P PRINT    Ctrl-X EXIT";

/// DECRYPT's state: the wrapped plain text, the file's content, its cipher and the inverse, how
/// often each enciphered letter appears, the player's guess for each, the letter picked and the auto reveal timer.
#[derive(Clone, Debug, PartialEq)]
pub struct Decrypter {
    pub rows: [[u8; 64]; 16],
    pub content: i32,
    pub cipher: [i32; 26],
    pub inverse: [i32; 26],
    pub counts: [i32; 26],
    pub guesses: [i32; 26],
    pub selected: i32,
    pub timer: i32,
}

impl Default for Decrypter {
    fn default() -> Self {
        Self { rows: [[0; 64]; 16], content: 0, cipher: [0; 26], inverse: [0; 26], counts: [0; 26], guesses: [0; 26], selected: 0, timer: 0 }
    }
}

/// The wrap width for text starting at `from`: up to 60, backed off to a space but not below 30.
fn wrap(s: &[u8], from: i32, left: i32) -> i32 {
    let at = |k: i32| s.get(k as usize).copied().unwrap_or(0);
    let mut w = if left > 58 { 59 } else { left };

    if !(left > 58 || w > 30) || at(from + w) == b' ' {
        return w;
    }

    loop {
        w -= 1;

        if w == 30 || at(from + w) == b' ' {
            return w;
        }
    }
}

/// A cipher or guess as an index into the 26 letters; the binary's values can run past Z.
fn index(value: i32) -> usize {
    value.rem_euclid(26) as usize
}

fn letter(value: i32) -> u8 {
    (value + b'A' as i32) as u8
}

/// A guess as it is shown: blank when there is none.
fn shown(guess: i32) -> u8 {
    if guess == -1 { b' ' } else { letter(guess) }
}

impl Sim {
    /// computer_decrypt_init: the file's text, wrapped and in capitals, its letters counted. Only the 8 shown rows are
    /// kept (the binary writes further rows on over the content slot and cipher).
    pub(crate) fn decrypt_init(&mut self, id: usize, c: &mut Computer) {
        let (text, cipher) = self.fs.content(c.decrypt.content).map_or((Vec::new(), [0; 26]), |x| (x.text.clone(), x.cipher));
        let s: Vec<u8> = text.iter().copied().take_while(|&b| b != 0).take(512).collect();
        let len = s.len() as i32;
        let d = &mut c.decrypt;

        for row in d.rows.iter_mut().take(8) {
            row[0] = 0;
        }

        d.timer = 0;
        let mut w = wrap(&s, 0, len);
        let (mut row, mut col) = (0usize, 0usize);
        let mut k = 1i32;

        while k <= 256 {
            let ch = s.get(k as usize - 1).copied().unwrap_or(0);

            if ch == 0 {
                break;
            }

            if row < 16 {
                d.rows[row][col] = ch.to_ascii_uppercase();
            }

            col += 1;

            if col as i32 <= w {
                k += 1;
                continue;
            }

            if row < 16 {
                d.rows[row][col.min(63)] = 0;
            }

            row += 1;
            w = wrap(&s, k, len - k);
            k += 1;
            col = 0;
        }

        if row < 16 {
            d.rows[row][col.min(63)] = 0;
        }

        d.cipher = cipher;
        d.selected = -1;
        d.counts = [0; 26];
        d.guesses = [-1; 26];

        for i in 0..26 {
            d.inverse[index(d.cipher[i])] = i as i32;
        }

        for r in 0..8 {
            for k in 0..62 {
                let ch = d.rows[r][k];

                if ch == 0 {
                    break;
                }

                let up = ch.to_ascii_uppercase();
                d.rows[r][k] = up;

                if up.is_ascii_uppercase() {
                    d.counts[index(d.cipher[(up - b'A') as usize])] += 1;
                }
            }
        }

        c.current_line = 0;
        c.top_line = 0;

        c.banner(0, HEADER, 31);

        for line in c.lines.iter_mut().take(24).skip(1) {
            line[0] = 0;
        }

        self.decrypt_render(id, c);

        for l in 0..24_i32 {
            self.register_line(id, c, l);
        }
    }

    /// computer_decrypt_render: each shown row enciphered above as guessed so far, then the key table.
    pub(crate) fn decrypt_render(&mut self, id: usize, c: &mut Computer) {
        for (row, line) in (2..17).step_by(3).enumerate() {
            let text = c.decrypt.rows[row];
            self.decrypt_row(c, line, &text, false);
            self.register_line(id, c, line as i32);
            self.decrypt_row(c, line + 1, &text, true);
            self.register_line(id, c, line as i32 + 1);
        }

        self.decrypt_footer(id, c);
    }

    /// computer_decrypt_render_row: a row's letters as the player's guesses (blank when none) or enciphered, the
    /// picked letter in red.
    fn decrypt_row(&self, c: &mut Computer, line: usize, src: &[u8; 64], guessed: bool) {
        let d = &c.decrypt;
        let mut n = 0;

        while n < 62 {
            let ch = src[n];

            if ch == 0 {
                break;
            }

            if !ch.is_ascii_uppercase() {
                c.lines[line][n] = ch;
            } else {
                let e = d.cipher[(ch - b'A') as usize];

                if guessed {
                    c.lines[line][n] = shown(d.guesses[index(e)]);
                } else {
                    c.lines[line][n] = letter(e);
                    c.colors[line][n] = if d.selected == e { 12 } else { 15 };
                }
            }

            n += 1;
        }

        c.lines[line][n] = 0;
    }

    /// computer_decrypt_render_footer: each enciphered letter, how often it appears and the guess for it.
    fn decrypt_footer(&mut self, id: usize, c: &mut Computer) {
        for (line, guess_line, keys) in [(17, 18, 0..15), (20, 21, 15..26)] {
            let end = keys.len() * 4;

            for (j, i) in keys.enumerate() {
                let p = j * 4;
                let n = c.decrypt.counts[i];
                c.lines[line][p] = b'A' + i as u8;
                c.colors[line][p] = if c.decrypt.selected != i as i32 { 15 } else { 12 };
                c.lines[line][p + 1] = if n > 9 { b'0' + (n / 10) as u8 } else { b' ' };
                c.lines[line][p + 2] = b'0' + (n % 10) as u8;
                c.colors[line][p + 1] = 143;
                c.colors[line][p + 2] = 143;
                c.lines[line][p + 3] = b' ';
                c.lines[guess_line][p] = shown(c.decrypt.guesses[i]);
                c.lines[guess_line][p + 1..p + 4].fill(b' ');
            }

            c.lines[line][end] = 0;
            c.lines[guess_line][end] = 0;
            self.register_line(id, c, line as i32);
            self.register_line(id, c, guess_line as i32);

            if line == 17 {
                self.register_line(id, c, 19);
            }
        }
    }

    /// computer_decrypt_progress: the share of letters guessed right.
    fn decrypt_progress(c: &Computer) -> f32 {
        let d = &c.decrypt;
        let (mut right, mut total) = (0, 0);

        for i in 0..26 {
            if d.counts[i] > 0 {
                total += 1;
            }

            let g = d.guesses[index(d.cipher[i])];

            if g != i as i32 {
                total += (g != -1) as i32;
            } else {
                right += 1;
            }
        }

        if total == 0 { 1.0 } else { right as f32 / total as f32 }
    }

    /// computer_decrypt_handle_key: a letter picks an enciphered letter then guesses it (space clears the guess);
    /// enter starts or stops the auto reveal; Ctrl-R forgets every guess, Ctrl-P prints the screen and Ctrl-X quits.
    pub(crate) fn decrypt_key(&mut self, id: usize, c: &mut Computer, key: i32) {
        let mut k = (key as u8).to_ascii_uppercase() as i32;

        if key & 0x100 != 0 {
            match k as u8 {
                b'R' => {
                    c.decrypt.guesses = [-1; 26];
                    k = 0;
                }
                b'P' => {
                    self.print_to_memo(id, c);
                    return;
                }
                b'X' => {
                    c.mode = Mode::Shell;

                    for l in 0..SCREEN_LINES {
                        c.lines[l][0] = 0;
                        self.register_line(id, c, l as i32);
                    }

                    return;
                }
                _ => {}
            }
        }

        let t = c.decrypt.timer;

        if k == 10 {
            if t != 0 {
                c.lines[1][0] = 0;
                self.register_line(id, c, 1);
                c.decrypt.timer = 0;
                c.decrypt.selected = -1;
                self.decrypt_render(id, c);

                return;
            }

            super::put_text(&mut c.lines[1], b"AUTO DECRYPTING...");
            self.register_line(id, c, 1);
            let p = Self::decrypt_progress(c);
            c.decrypt.timer = ((1.0 - p) * game_seconds(10) as f32 + 20.0) as i32;

            if c.decrypt.timer > 0 {
                return;
            }
        } else if t > 0 {
            return;
        }

        let letter = (k as u8).is_ascii_uppercase();
        let sel = c.decrypt.selected;

        if sel == -1 {
            if letter {
                c.decrypt.selected = k - b'A' as i32;
            }
        } else {
            if letter {
                c.decrypt.guesses[sel as usize] = k - b'A' as i32;
            } else if k == b' ' as i32 {
                c.decrypt.guesses[sel as usize] = -1;
            }

            c.decrypt.selected = -1;
        }

        self.decrypt_render(id, c);
    }

    /// decrypt_tick_reveal_timer: when the auto timer runs out a letter is revealed.
    pub(crate) fn decrypt_tick(&mut self, id: usize, c: &mut Computer) {
        if c.decrypt.timer > 0 {
            c.decrypt.timer -= 1;

            if c.decrypt.timer == 0 {
                self.decrypt_reveal(id, c);
            }
        }
    }

    /// decrypt_reveal_random_letter: a random letter that appears and is not guessed gets its true guess.
    fn decrypt_reveal(&mut self, id: usize, c: &mut Computer) {
        let mut p = rand::random_range(0..26) as usize;

        for _ in 0..10_000 {
            let e = index(c.decrypt.cipher[p]);

            if c.decrypt.counts[e] != 0 && c.decrypt.guesses[e] == -1 {
                break;
            }

            p = rand::random_range(0..26) as usize;
        }

        c.decrypt.guesses[index(c.decrypt.cipher[p])] = p as i32;
        self.decrypt_render(id, c);
        c.lines[1][0] = 0;
        self.register_line(id, c, 1);
    }
}
