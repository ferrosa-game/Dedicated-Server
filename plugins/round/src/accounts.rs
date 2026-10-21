use std::collections::BTreeMap;

pub(crate) struct Account {
    pub(crate) money: i32,
    pub(crate) name: String,
}

#[derive(Default)]
pub(crate) struct Accounts(BTreeMap<u32, Account>);

impl Accounts {
    pub(crate) fn load() -> Self {
        let Ok(text) = std::fs::read_to_string("accounts.txt") else { return Self::default() };
        let mut accounts = BTreeMap::new();

        for line in text.lines() {
            let mut parts = line.splitn(3, ' ');
            let (Some(id), Some(money)) = (parts.next().and_then(|i| i.parse().ok()), parts.next().and_then(|m| m.parse().ok())) else { continue };

            accounts.insert(id, Account { money, name: parts.next().unwrap_or("").to_string() });
        }

        Self(accounts)
    }

    pub(crate) fn save(&self) {
        let text: String = self.0.iter().map(|(id, a)| format!("{id} {} {}\n", a.money, a.name)).collect();

        if let Err(e) = std::fs::write("accounts.txt", text) {
            eprintln!("[round] Could not save accounts: {e}");
        }
    }

    pub(crate) fn join(&mut self, id: u32, name: &str) -> &Account {
        let account = self.0.entry(id).or_insert_with(|| Account { money: 1000, name: String::new() });
        account.name = name.to_string();
        account
    }

    pub(crate) fn money(&self, id: u32) -> Option<i32> {
        self.0.get(&id).map(|a| a.money)
    }

    pub(crate) fn set_money(&mut self, id: u32, money: i32) {
        if let Some(a) = self.0.get_mut(&id) {
            a.money = money;
        }
    }

    pub(crate) fn add_money(&mut self, id: u32, money: i32) {
        if let Some(a) = self.0.get_mut(&id) {
            a.money += money;
        }
    }

    pub(crate) fn top(&self, n: usize) -> Option<&Account> {
        let mut ranked: Vec<&Account> = self.0.values().collect();
        ranked.sort_by_key(|a| std::cmp::Reverse(a.money));

        ranked.get(n).copied()
    }
}

pub(crate) fn with_commas(n: i32) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();

    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }

        out.push(c);
    }

    if n < 0 { format!("-{out}") } else { out }
}
