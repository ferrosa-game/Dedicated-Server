use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub money: i32,
    pub corp_rating: i32,
    pub crim_rating: i32,
    pub play_time: u32,
    pub spawn_timer: i32,
    pub name: String,
}

#[derive(Default)]
pub struct Accounts(BTreeMap<u32, Account>);

fn field(fields: &str, key: &str) -> Option<String> {
    fields.split_whitespace().find_map(|t| t.strip_prefix(key)).map(str::to_string)
}

fn number<T: std::str::FromStr + Default>(fields: &str, key: &str) -> T {
    field(fields, key).and_then(|v| v.parse().ok()).unwrap_or_default()
}

impl Accounts {
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string("accounts.txt") else { return Self::default() };
        let mut accounts = BTreeMap::new();

        for line in text.lines() {
            let (fields, name) = match line.split_once("name=") {
                Some((fields, name)) => (fields, name.trim_end().to_string()),
                None => (line, String::new()),
            };
            let Some(id) = field(fields, "account=").and_then(|v| v.parse().ok()) else { continue };

            accounts.insert(
                id,
                Account {
                    money: number(fields, "money="),
                    corp_rating: number(fields, "corp="),
                    crim_rating: number(fields, "crim="),
                    play_time: number(fields, "time="),
                    spawn_timer: number(fields, "timer="),
                    name,
                },
            );
        }

        Self(accounts)
    }

    pub fn save(&self) {
        let text: String = self
            .0
            .iter()
            .map(|(id, a)| format!("account={id} money={} corp={} crim={} time={} timer={} name={}\n", a.money, a.corp_rating, a.crim_rating, a.play_time, a.spawn_timer, a.name))
            .collect();

        if let Err(e) = std::fs::write("accounts.txt", text) {
            eprintln!("[world] Could not save accounts: {e}");
        }
    }

    pub fn entry(&mut self, id: u32) -> &mut Account {
        self.0.entry(id).or_default()
    }

    pub fn open(&mut self, id: u32, money: i32) -> &mut Account {
        self.0.entry(id).or_insert(Account { money, corp_rating: 1000, ..Account::default() })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &Account)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}
