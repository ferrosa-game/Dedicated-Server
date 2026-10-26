use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// A banned account: who it was when the ban was made, so an admin can read the file, and the second it lifts.
#[derive(Clone, Debug)]
pub struct Ban {
    pub account_id: u32,
    pub steam_id: u64,
    pub phone_number: u32,
    pub name: String,
    /// The second the ban lifts, or none for one that never does.
    pub until: Option<u64>,
}

/// A ban of a day or more never lifts.
pub const PERMANENT: u64 = 86400;

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Ban {
    fn lifted(&self, now: u64) -> bool {
        self.until.is_some_and(|u| u <= now)
    }

    fn line(&self) -> String {
        let until = match self.until {
            Some(u) => u.to_string(),
            None => "never".to_string(),
        };

        format!("account={} steam={} phone={} until={until} name={}", self.account_id, self.steam_id, self.phone_number, self.name)
    }

    /// One line of bans.txt: `account=`, `steam=`, `phone=` and `until=` in any order, then `name=` and the rest of
    /// the line. A line missing a field, or with one that does not read as a number, is dropped.
    fn parse(line: &str) -> Option<Self> {
        let (fields, name) = match line.split_once("name=") {
            Some((fields, name)) => (fields, name.trim_end().to_string()),
            None => (line, String::new()),
        };
        let field = |key: &str| fields.split_whitespace().find_map(|t| t.strip_prefix(key));
        let until = match field("until=")? {
            "never" => None,
            s => Some(s.parse().ok()?),
        };

        Some(Ban {
            account_id: field("account=")?.parse().ok()?,
            steam_id: field("steam=")?.parse().ok()?,
            phone_number: field("phone=")?.parse().ok()?,
            name,
            until,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bans {
    bans: Vec<Ban>,
    steam: Vec<u64>,
}

impl Bans {
    /// bans.txt and the serversteamban.txt beside it, with the bans that have run out left behind.
    pub fn load(path: &Path) -> Self {
        let steam = std::fs::read_to_string(path.with_file_name("serversteamban.txt"))
            .map(|text| text.split("steamban=").skip(1).filter_map(leading_u64).collect())
            .unwrap_or_default();
        let now = now();

        let bans = std::fs::read_to_string(path).map(|text| text.lines().filter_map(Ban::parse).filter(|b| !b.lifted(now)).collect()).unwrap_or_default();

        Bans { bans, steam }
    }

    pub fn save(&mut self, path: &Path) {
        let now = now();
        self.bans.retain(|b| !b.lifted(now));
        let mut out = String::new();

        for b in &self.bans {
            out += &b.line();
            out.push('\n');
        }

        if let Err(e) = std::fs::write(path, out) {
            println!("[Sim] Could not save bans: {e:?}");
        }
    }

    /// Whether this account may not play: a Steam id an admin listed, or a ban that has not run out.
    pub fn is_banned(&self, account_id: u32, steam_id: u64) -> bool {
        if steam_id != 0 && self.steam.contains(&steam_id) {
            return true;
        }

        let now = now();

        self.bans.iter().any(|b| b.account_id == account_id && !b.lifted(now))
    }

    /// Bans an account until x second, or for good with none; an account already banned is banned anew.
    pub fn ban(&mut self, account_id: u32, steam_id: u64, phone_number: u32, name: &str, until: Option<u64>) {
        self.bans.retain(|b| b.account_id != account_id);
        self.bans.push(Ban { account_id, steam_id, phone_number, name: name.to_string(), until });
    }

    /// Lifts an account's ban, and says whether there was one. A Steam id in serversteamban.txt stays banned.
    pub fn unban(&mut self, account_id: u32) -> bool {
        let before = self.bans.len();
        self.bans.retain(|b| b.account_id != account_id);

        self.bans.len() != before
    }
}

fn leading_u64(s: &str) -> Option<u64> {
    let t = s.trim_start();
    let end = t.char_indices().find(|&(_, c)| !c.is_ascii_digit()).map_or(t.len(), |(i, _)| i);

    t[..end].parse::<u64>().ok()
}
