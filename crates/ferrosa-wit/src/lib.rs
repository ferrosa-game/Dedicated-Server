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

/// A chat message as a command and its arguments, if it is one: `/give ak47 2` is `("give", ["ak47", "2"])`.
pub fn command(message: &str) -> Option<(&str, Vec<&str>)> {
    let mut words = message.trim().strip_prefix('/')?.split_whitespace();

    Some((words.next()?, words.collect()))
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

    /// A mode's settings for the shared code, taken at every reset. Only asked of the mode; the server can't be
    /// changed from here.
    fn rules(&mut self) -> Rules {
        server::default_rules()
    }

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
    fn on_menu(&mut self, _player: PlayerId, _menu: u8, _button: u32) -> bool {
        false
    }

    /// A chat message from a player, before the server's own commands; true takes it, and it goes no further. The
    /// mode is asked first, then each plugin. /admin messages never reach plugins.
    fn on_chat(&mut self, _player: PlayerId, _message: &str) -> bool {
        false
    }

    /// A player has joined, their account restored and the initial sync sent.
    fn on_join(&mut self, _player: PlayerId) {}

    /// A player is leaving, before their shares and account are settled.
    fn on_leave(&mut self, _player: PlayerId) {}

    /// A real player's human has died.
    fn on_death(&mut self, _player: PlayerId) {}

    /// The money a joining player starts with: the mode is given what their account has, then each plugin the last
    /// answer. The server can't be changed from here.
    fn joining_money(&mut self, money: i32) -> i32 {
        money
    }

    /// /resetgame has named this plugin's mode. Only asked of the mode.
    fn on_admin_select(&mut self) {}
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

        impl $crate::exports::ferrosa::plugin::get_rules::Guest for __FerrosaPlugin {
            fn get_rules() -> $crate::Rules {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::rules(p))
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
            fn on_menu(player: $crate::PlayerId, menu: u8, button: u32) -> bool {
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

        impl $crate::exports::ferrosa::plugin::joining_money::Guest for __FerrosaPlugin {
            fn joining_money(money: i32) -> i32 {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::joining_money(p, money))
            }
        }

        impl $crate::exports::ferrosa::plugin::on_admin_select::Guest for __FerrosaPlugin {
            fn on_admin_select() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_admin_select(p))
            }
        }

        $crate::export_bindings!(__FerrosaPlugin with_types_in $crate);
    };
}
