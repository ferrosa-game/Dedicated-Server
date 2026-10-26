use ferrosa_wit::{Bill, Cash, ItemKind, Menu, PlayerId, server};

use crate::World;

const NOTES: [Bill; 7] = [Bill::None, Bill::One, Bill::Five, Bill::Ten, Bill::Twenty, Bill::Fifty, Bill::Hundred];

impl World {
    pub fn bank_menu_action(&mut self, pid: PlayerId, button: i32) {
        let Some(p) = server::get_player(pid) else { return };

        if p.human().is_none() {
            return;
        }

        match button as u32 {
            b @ 1..=6 => self.bank_withdraw(pid, b),
            7 => self.bank_deposit(pid),
            16 => p.set_menu(Menu::EmptyBase),
            _ => {}
        }
    }

    fn bank_withdraw(&mut self, pid: PlayerId, code: u32) {
        let Some(note) = NOTES.get(code as usize).copied() else { return };
        let Some(p) = server::get_player(pid) else { return };
        let Some(human) = p.human() else { return };
        let value = server::bill_value(note);
        let withdrawn = self.state.bills_withdrawn.get(&pid).copied().unwrap_or(0);

        if p.money() < value || (code <= 2 && withdrawn > 39) {
            return;
        }

        match human.slot(0).into_iter().next() {
            None => {
                let Some(item) = human.give_item(ItemKind::CashWorld as u32, 0, 1) else { return };

                item.set_cash(&Cash { bills: vec![note] });
            }
            Some(item) => {
                if item.kind() != ItemKind::CashWorld as u32 || !item.insert_bill(0, note) {
                    return;
                }
            }
        }

        p.set_money(p.money() - value);

        if code <= 2 {
            *self.state.bills_withdrawn.entry(pid).or_default() += 1;
        }
    }

    fn bank_deposit(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let Some(human) = p.human() else { return };
        let Some(item) = human.slot(0).into_iter().next().filter(|i| i.kind() == ItemKind::CashWorld as u32) else { return };
        let Some(value) = item.cash_value() else { return };

        p.set_money(p.money() + value);
        item.delete();
    }
}
