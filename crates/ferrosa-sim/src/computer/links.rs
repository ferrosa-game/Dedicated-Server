use super::{COLUMNS, Computer, LINES, text_len};
use crate::sim::Sim;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Link {
    pub active: bool,
    pub kind: i32,
    pub tick: i32,
    pub item: i32,
    pub line: i32,
    pub text: Vec<u8>,
    pub colors: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct LinkPool {
    pub links: Vec<Link>,
}

impl Default for LinkPool {
    fn default() -> Self {
        Self { links: vec![Link::default(); 16384] }
    }
}

impl LinkPool {
    /// A link in use.
    pub(crate) fn get(&self, l: i32) -> Option<&Link> {
        self.record(l).filter(|k| k.active)
    }

    /// A link record whether in use or not (a freed one keeps what it last held).
    pub(crate) fn record(&self, l: i32) -> Option<&Link> {
        usize::try_from(l).ok().and_then(|l| self.links.get(l))
    }

    pub(crate) fn free(&mut self, l: i32) {
        if let Some(k) = usize::try_from(l).ok().and_then(|l| self.links.get_mut(l)) {
            k.active = false;
        }
    }
}

impl Sim {
    /// computer_register_line_link for a computer line: kept when unchanged, otherwise a new link replaces it.
    pub(crate) fn register_line(&mut self, id: usize, c: &Computer, line: i32) {
        let l = line as usize & (LINES - 1);
        let text = c.lines[l];
        let colors = c.colors[l];
        self.register_link(id, 1, line, &text, Some(&colors));
    }

    /// Registers line `line` of item `id` (its text and, for computers, colours). Returns the link, -1 when the pool
    /// is full.
    pub(crate) fn register_link(&mut self, id: usize, kind: i32, line: i32, text: &[u8; COLUMNS], colors: Option<&[u8; COLUMNS]>) -> i32 {
        let real = text_len(text);
        let len = real.min(COLUMNS - 1);
        let colors = colors.copied().unwrap_or([0; COLUMNS]);
        let slot = line as usize & 63;
        let Some(item) = self.items.get(id) else { return -1 };

        if item.link_mask & (1 << slot) != 0 {
            let old = item.links[slot];
            let same = self.links.get(old).is_some_and(|k| k.text.len() == len && (real == 0 || (k.text[..] == text[..len] && k.colors[..] == colors[..len])));

            if same {
                return old;
            }

            self.links.free(old);
        }

        let Some(free) = self.links.links.iter().position(|k| !k.active) else { return -1 };
        self.links.links[free] =
            Link { active: true, kind, tick: self.tick as i32, item: id as i32, line, text: text[..len].to_vec(), colors: colors[..len].to_vec() };
        self.set_link_slot(id, free as i32, line);
        free as i32
    }

    /// computer_set_line_link_slot: the item's slot holds the link, and every connection is due to be sent it.
    fn set_link_slot(&mut self, id: usize, link: i32, line: i32) {
        let slot = line as usize & 63;

        if let Some(item) = self.items.get_mut(id) {
            item.links[slot] = link;
            item.link_mask |= 1 << slot;
        }

        for client in self.clients.values_mut() {
            if let Some(m) = client.link_sent.get_mut(&id) {
                *m &= !(1u64 << slot);
            }
        }
    }

    /// The delete_item part: the item's links go.
    pub(crate) fn free_item_links(&mut self, id: usize) {
        let Some(item) = self.items.get(id) else { return };
        let (mask, links) = (item.link_mask, item.links);

        for (k, &link) in links.iter().enumerate() {
            if mask & (1 << k) != 0 {
                self.links.free(link);
            }
        }

        for client in self.clients.values_mut() {
            client.link_sent.remove(&id);
        }
    }
}
