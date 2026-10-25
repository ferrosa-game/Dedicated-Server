wit_bindgen::generate!({
    path: "wit",
    world: "plugin",
    pub_export_macro: true,
    export_macro_name: "export_bindings",
    default_bindings_module: "ferrosa_wit",
});

use std::ops::{Add, Mul, Neg, Sub};

pub use ferrosa::plugin::{server, types::*};

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Mat3 {
    /// No rotation.
    pub const IDENTITY: Self = Self { x: Vec3::new(1.0, 0.0, 0.0), y: Vec3::new(0.0, 1.0, 0.0), z: Vec3::new(0.0, 0.0, 1.0) };
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// The server runs 60 ticks a second; timers and hooks count in ticks.
pub const TICKS_PER_SECOND: i32 = 60;

/// `seconds` of real time in ticks.
pub const fn game_seconds(seconds: i32) -> i32 {
    seconds * TICKS_PER_SECOND
}

/// `minutes` of real time in ticks.
pub const fn game_minutes(minutes: i32) -> i32 {
    minutes * 60 * TICKS_PER_SECOND
}

/// `hours` and `minutes` on the world clock (server::world-time), which counts sixtieths of a world second from
/// midnight: a time of day, or a stretch of one. It only runs in world mode, where a world minute passes faster than a
/// real one.
pub const fn clock_time(hours: i32, minutes: i32) -> i32 {
    (hours * 60 + minutes) * 60 * TICKS_PER_SECOND
}

/// A chat message as a command and its arguments, if it is one: `/give ak47 2` is `("give", ["ak47", "2"])`.
pub fn command(message: &str) -> Option<(&str, Vec<&str>)> {
    let mut words = message.trim().strip_prefix('/')?.split_whitespace();

    Some((words.next()?, words.collect()))
}

/// The lines of a computer's screen a program can draw on: it keeps 24, but the game only ever draws the first 23,
/// so anything written on line 23 is never seen.
pub const SHOWN_LINES: u32 = 23;

/// A key a computer program is given, and whether ctrl was held with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Up,
    Down,
    Left,
    Right,
}

impl Key {
    /// Reads the code the server sends: a character's own byte, 8 backspace, 10 enter, 16 to 19 the arrows, with
    /// 0x100 set when ctrl is held. Letters always arrive upper case.
    pub fn new(code: i32) -> (Self, bool) {
        let key = match code & 0xff {
            8 => Self::Backspace,
            10 => Self::Enter,
            16 => Self::Up,
            17 => Self::Down,
            18 => Self::Left,
            19 => Self::Right,
            c => Self::Char(c as u8 as char),
        };

        (key, code & 0x100 != 0)
    }
}

/// A program running on one computer, made by Plugin::program. The server keeps it for as long as it runs, so its
/// fields are that computer's own state, and drops it once it ends.
pub trait Program: 'static {
    /// A key pressed on it; false ends the program and gives the shell back. Ctrl-X is the key the game's own
    /// programs quit on.
    fn key(&mut self, computer: &Computer, key: Key, ctrl: bool) -> bool;

    /// Each tick while it runs. A program can end itself here with computer.exit().
    fn tick(&mut self, _computer: &Computer) {}
}

/// A plugin. Every hook does nothing unless written. The server makes it once, with Default, and keeps it for as long
/// as it runs. A plugin that calls server::register_mode in init is that game mode: it only gets hooks while its mode
/// is the server's, and before the other plugins.
pub trait Plugin: Default + 'static {
    /// Once, when the plugin is loaded, before the server starts. A game mode registers itself here; nothing else in
    /// server can be used yet.
    fn init(&mut self) {}

    /// Once, when the server starts.
    fn on_start(&mut self) {}

    /// A reset, after the corporations' missions are cleared.
    fn reset(&mut self) {}

    /// Each tick, before the traffic and NPCs move.
    fn tick(&mut self) {}

    /// Each tick, after the traffic and NPCs move.
    fn tick_after_traffic(&mut self) {}

    /// Each physics tick, before the bodies move.
    fn before_physics(&mut self) {}

    /// Each physics tick, once the bonds are solved and the leftover items and vehicles cleaned up.
    fn after_physics(&mut self) {}

    /// Once every player's menus are done for the tick.
    fn menus_done(&mut self) {}

    /// A menu button pressed; true takes it. The mode is asked first, then each plugin, then the shared menus.
    fn on_menu(&mut self, _player: PlayerId, _menu: Menu, _button: u32) -> bool {
        false
    }

    /// A chat message from a player, before the server's own commands; true takes it, and it goes no further. The
    /// mode is asked first, then each plugin. /admin messages never reach plugins.
    fn on_chat(&mut self, _player: PlayerId, _message: &str) -> bool {
        false
    }

    /// A player has joined, their account restored and the initial sync sent: where a mode sets their starting money.
    fn on_join(&mut self, _player: PlayerId) {}

    /// A player is leaving, before their shares and account are settled.
    fn on_leave(&mut self, _player: PlayerId) {}

    /// A real player's human has died.
    fn on_death(&mut self, _player: PlayerId) {}

    /// A human is about to be hurt by a bullet, a grenade, a fall or a vehicle; returns the damage to deal instead (0
    /// to refuse it). The mode is asked first, then each plugin with the last answer.
    fn on_damage(&mut self, _human: HumanId, _cause: DamageCause, _bone: Option<Bone>, damage: i32) -> i32 {
        damage
    }

    /// /resetgame has named this plugin's mode. Only asked of the mode.
    fn on_admin_select(&mut self) {}

    /// A program this plugin registered with server::register_program, run at `computer`'s shell with the rest of
    /// the typed line as `args`. None refuses it, and the computer answers BAD COMMAND OR FILE NAME as it does for
    /// any other unknown command; a program that refuses should draw nothing. The screen is then the program's to
    /// draw on, but nothing clears it first, so a program paints every line it wants to own.
    fn program(&mut self, _name: &str, _computer: &Computer, _args: &str) -> Option<Box<dyn Program>> {
        None
    }
}

/// Makes a type implementing Plugin the component's plugin: `ferrosa_wit::plugin!(MyPlugin);`.
#[macro_export]
macro_rules! plugin {
    ($plugin:ty) => {
        struct __FerrosaPlugin;

        thread_local! {
            static __FERROSA: ::std::cell::RefCell<$plugin> = ::std::cell::RefCell::new(::std::default::Default::default());
        }

        impl $crate::exports::ferrosa::plugin::init::Guest for __FerrosaPlugin {
            fn init() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::init(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_start::Guest for __FerrosaPlugin {
            fn on_start() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_start(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::reset::Guest for __FerrosaPlugin {
            fn reset() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::reset(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::tick::Guest for __FerrosaPlugin {
            fn tick() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::tick(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::tick_after_traffic::Guest for __FerrosaPlugin {
            fn tick_after_traffic() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::tick_after_traffic(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::before_physics::Guest for __FerrosaPlugin {
            fn before_physics() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::before_physics(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::after_physics::Guest for __FerrosaPlugin {
            fn after_physics() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::after_physics(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::menus_done::Guest for __FerrosaPlugin {
            fn menus_done() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::menus_done(p))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_menu::Guest for __FerrosaPlugin {
            fn on_menu(player: $crate::PlayerId, menu: $crate::Menu, button: u32) -> bool {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_menu(p, player, menu, button))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_chat::Guest for __FerrosaPlugin {
            fn on_chat(player: $crate::PlayerId, message: ::std::string::String) -> bool {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_chat(p, player, &message))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_join::Guest for __FerrosaPlugin {
            fn on_join(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_join(p, player))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_leave::Guest for __FerrosaPlugin {
            fn on_leave(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_leave(p, player))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_death::Guest for __FerrosaPlugin {
            fn on_death(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_death(p, player))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_damage::Guest for __FerrosaPlugin {
            fn on_damage(human: $crate::HumanId, cause: $crate::DamageCause, bone: ::std::option::Option<$crate::Bone>, damage: i32) -> i32 {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_damage(p, human, cause, bone, damage))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_admin_select::Guest for __FerrosaPlugin {
            fn on_admin_select() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_admin_select(p))
            }
        }

        struct __FerrosaProgram(::std::cell::RefCell<::std::boxed::Box<dyn $crate::Program>>);

        impl $crate::exports::ferrosa::plugin::programs::Guest for __FerrosaPlugin {
            type Program = __FerrosaProgram;
        }

        impl $crate::exports::ferrosa::plugin::programs::GuestProgram for __FerrosaProgram {
            fn start(
                name: ::std::string::String,
                computer: $crate::ItemId,
                args: ::std::string::String,
            ) -> ::std::option::Option<$crate::exports::ferrosa::plugin::programs::Program> {
                let at = $crate::server::get_computer(computer)?;
                let program = __FERROSA.with_borrow_mut(|p| $crate::Plugin::program(p, &name, &at, &args))?;

                ::std::option::Option::Some($crate::exports::ferrosa::plugin::programs::Program::new(__FerrosaProgram(::std::cell::RefCell::new(program))))
            }

            fn key(&self, computer: $crate::ItemId, key: i32) -> bool {
                let ::std::option::Option::Some(at) = $crate::server::get_computer(computer) else { return false };
                let (key, ctrl) = $crate::Key::new(key);

                self.0.borrow_mut().key(&at, key, ctrl)
            }

            fn tick(&self, computer: $crate::ItemId) {
                if let ::std::option::Option::Some(at) = $crate::server::get_computer(computer) {
                    self.0.borrow_mut().tick(&at);
                }
            }
        }

        $crate::export_bindings!(__FerrosaPlugin with_types_in $crate);
    };
}
