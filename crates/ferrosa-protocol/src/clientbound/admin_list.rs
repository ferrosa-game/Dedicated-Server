use crate::codec::WireWrite;

#[derive(Debug, Clone, PartialEq)]
pub struct AdminListEntry {
    pub account_id: i32,
    pub name: String,
    pub phone_number: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdminList {
    pub rows: Vec<Option<AdminListEntry>>,
}

impl WireWrite for AdminList {
    fn write(&self, w: &mut crate::codec::Writer) {
        w.byte(9);

        for row in &self.rows {
            w.bits(row.is_some() as i32, 1);

            if let Some(e) = row {
                w.i32(e.account_id);
                w.string(&e.name, 32);
                w.i32(e.phone_number);
            }
        }
    }
}
