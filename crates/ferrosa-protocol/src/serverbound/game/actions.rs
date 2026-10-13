use crate::codec::{CodecError, Reader, WireWrite, Writer};

#[derive(Debug, Clone, PartialEq)]
pub enum GameAction {
    Menu(MenuAction),
    Chat(ChatAction),
    Item(ItemAction),
    Inventory(InventoryAction),
    Admin(AdminAction),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Menu {
    Lobby = 2,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuAction {
    pub menu: Menu,
    pub button: u32,
    pub c: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatAction {
    pub message: String,
    pub volume: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemAction {
    pub a: u16,
    pub b: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InventoryAction {
    pub a: u16,
    pub b: u16,
    pub c: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdminAction {
    /// 4 sets account `a`'s ban time to `b`, 5 its criminal rating.
    pub kind: u8,
    pub a: u32,
    pub b: u32,
}

impl WireWrite for GameAction {
    /// client_send's action: its type in 4 bits, then what that type carries.
    fn write(&self, w: &mut Writer) {
        match self {
            GameAction::Menu(a) => {
                w.bits(0, 4);
                w.bits(
                    match a.menu {
                        Menu::Lobby => 2,
                        Menu::Other(m) => m as i32,
                    },
                    8,
                );
                w.u32(a.button);
                let mut text = a.c.clone();
                text.resize(16, 0);
                w.bytes(&text);
            }
            GameAction::Chat(a) => {
                let bytes = &a.message.as_bytes()[..a.message.len().min(63)];
                w.bits(1, 4);
                w.bits(bytes.len() as i32, 6);
                w.bytes(bytes);
                w.bits(a.volume as i32, 4);
            }
            GameAction::Item(a) => {
                w.bits(2, 4);
                w.bits(a.a as i32, 16);
                w.bits(a.b as i32, 16);
            }
            GameAction::Inventory(a) => {
                w.bits(3, 4);
                w.bits(a.a as i32, 16);
                w.bits(a.b as i32, 16);
                w.bits(a.c as i32, 16);
            }
            GameAction::Admin(a) => {
                w.bits(a.kind as i32, 4);
                w.u32(a.a);
                w.u32(a.b);
            }
            GameAction::Unknown => w.bits(15, 4),
        }
    }
}

pub fn read_actions(r: &mut Reader, num_actions: u32) -> Result<Vec<GameAction>, CodecError> {
    let mut actions = Vec::with_capacity(num_actions as usize);

    for _ in 0..num_actions {
        let action_type = r.bits(4)? as u8;

        let action = Some(match action_type {
            0 => {
                let menu_type = match r.bits(8)? as u8 {
                    2 => Menu::Lobby,
                    other => Menu::Other(other),
                };

                GameAction::Menu(MenuAction { menu: menu_type, button: r.u32()?, c: r.bytes(16)?.to_vec() })
            }
            1 => {
                let len = r.bits(6)? as usize;

                let message = if len > 0 { r.string(len)? } else { "".to_string() };

                GameAction::Chat(ChatAction { message, volume: r.bits(4)? as u8 })
            }
            2 => GameAction::Item(ItemAction { a: r.bits(16)? as u16, b: r.bits(16)? as u16 }),
            3 => GameAction::Inventory(InventoryAction { a: r.bits(16)? as u16, b: r.bits(16)? as u16, c: r.bits(16)? as u16 }),
            4 | 5 => GameAction::Admin(AdminAction { kind: action_type, a: r.u32()?, b: r.u32()? }),
            _ => {
                println!("Received invalid action type {action_type:?}");

                GameAction::Unknown
            }
        });

        if let Some(action) = action {
            actions.push(action)
        }
    }

    Ok(actions)
}
