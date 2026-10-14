use std::{collections::HashMap, net::SocketAddr};

/// send_packet_to_server: the datagrams `data` goes out as: itself, or over 1200 bytes "7DFF" fragments under `id`,
/// `(len >> 10) + 1` of them (the last one empty when the length is a multiple of 1024).
pub fn split(data: &[u8], id: u16) -> Vec<Vec<u8>> {
    if data.len() <= 1200 {
        return vec![data.to_vec()];
    }

    let count = (data.len() >> 10) + 1;

    (0..count)
        .map(|i| {
            let body = &data[(i * 1024).min(data.len())..((i + 1) * 1024).min(data.len())];

            let mut f = Vec::with_capacity(8 + body.len());
            f.extend_from_slice(b"7DFF");
            f.extend_from_slice(&id.to_le_bytes());
            f.push(i as u8);
            f.push(count as u8);
            f.extend_from_slice(body);

            f
        })
        .collect()
}

/// A packet being put back together from its fragments.
#[derive(Clone, Default)]
struct Slot {
    id: u16,
    received: u32,
    len: usize,
    data: Vec<u8>,
}

/// One sender's reassembly ring (data_1d7f08e8/ec).
#[derive(Default)]
struct Ring {
    slots: Vec<Slot>,
    first: usize,
    next: usize,
}

/// process_recv_frag, kept per sender so fragments from different clients under the same id are never mixed.
#[derive(Default)]
pub struct Reassembler {
    rings: HashMap<SocketAddr, Ring>,
}

impl Reassembler {
    /// A "7DFF" datagram's fragment filed under its packet's id; the whole packet once every fragment is in. A
    /// packet is handed over once: its slot then starts over, so a repeated fragment does not deliver it again.
    pub fn add(&mut self, from: SocketAddr, datagram: &[u8]) -> Option<Vec<u8>> {
        let f = datagram.get(4..).filter(|f| f.len() >= 4)?;
        let id = u16::from_le_bytes([f[0], f[1]]);
        let (index, count) = (f[2] as usize, f[3] as usize);

        if count == 0 || count > 16 || index > 16 - 1 {
            return None;
        }

        let ring = self.rings.entry(from).or_default();

        if ring.slots.is_empty() {
            ring.slots = vec![Slot::default(); 64];
        }

        let mut k = ring.first;

        let slot = loop {
            if k == ring.next {
                let s = ring.next;
                ring.slots[s] = Slot { id, received: 0, len: 0, data: Vec::new() };
                ring.next = (s + 1) % 64;

                if ring.next == ring.first {
                    ring.first = (ring.first + 1) % 64;
                }

                break s;
            }

            if ring.slots[k].id == id {
                break k;
            }

            k = (k + 1) % 64;
        };

        let s = &mut ring.slots[slot];
        let body = &f[4..f.len().min(4 + 1024)];
        let at = index * 1024;

        if s.data.len() < at + body.len() {
            s.data.resize(at + body.len(), 0);
        }
        s.data[at..at + body.len()].copy_from_slice(body);
        s.len = s.len.max(at + body.len());
        s.received |= 1 << index;

        let all = (1u32 << count) - 1;

        if s.received & all != all {
            return None;
        }

        let whole = s.data[..s.len].to_vec();
        *s = Slot { id, ..Slot::default() };

        Some(whole)
    }

    /// Drops the rings of senders `keep` turns down.
    pub fn retain(&mut self, keep: impl Fn(&SocketAddr) -> bool) {
        self.rings.retain(|a, _| keep(a));
    }
}