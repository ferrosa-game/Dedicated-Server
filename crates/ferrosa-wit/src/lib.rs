wit_bindgen::generate!({
    path: "wit",
    world: "plugin",
    pub_export_macro: true,
    export_macro_name: "export_bindings",
    default_bindings_module: "ferrosa_wit",
});

pub use ferrosa::plugin::{server, types::*};

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

        impl $crate::exports::ferrosa::plugin::hooks::Guest for __FerrosaPlugin {
            fn init() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::init(p))
            }

            fn on_start() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_start(p))
            }

            fn get_rules() -> $crate::Rules {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::rules(p))
            }

            fn reset() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::reset(p))
            }

            fn tick() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::tick(p))
            }

            fn tick_after_traffic() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::tick_after_traffic(p))
            }

            fn before_physics() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::before_physics(p))
            }

            fn after_physics() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::after_physics(p))
            }

            fn menus_done() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::menus_done(p))
            }

            fn on_menu(player: $crate::PlayerId, menu: u8, button: u32) -> bool {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_menu(p, player, menu, button))
            }

            fn on_join(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_join(p, player))
            }

            fn on_leave(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_leave(p, player))
            }

            fn on_death(player: $crate::PlayerId) {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_death(p, player))
            }

            fn joining_money(money: i32) -> i32 {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::joining_money(p, money))
            }

            fn on_admin_select() {
                __FERROSA.with_borrow_mut(|p| $crate::Plugin::on_admin_select(p))
            }
        }

        $crate::export_bindings!(__FerrosaPlugin with_types_in $crate);
    };
}
