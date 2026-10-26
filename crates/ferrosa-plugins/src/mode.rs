use ferrosa_protocol::GameMode;
use ferrosa_sim::{ModeHooks, ModeRules, PlayerId, Sim};
use glam::Vec3;
use wasmtime::Store;

use crate::{
    host::{HostState, RegisteredMode, SimPtr},
    plugin::{Hooks, WasmPlugin},
};

pub(crate) struct WasmGameMode {
    name: &'static str,
    who: &'static str,
    aliases: &'static [&'static str],
    client_mode: GameMode,
    map: Option<&'static str>,
    plugin: &'static WasmPlugin,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

impl WasmGameMode {
    pub(crate) fn new(plugin: &'static WasmPlugin, mode: RegisteredMode) -> Self {
        Self {
            who: leak(format!("{}.{}", plugin.name, mode.name)),
            name: leak(mode.name),
            aliases: Box::leak(mode.aliases.into_iter().map(leak).collect()),
            client_mode: mode.client_mode,
            map: mode.map.map(leak),
            plugin,
        }
    }

    fn call<R>(&self, sim: SimPtr, hook: &str, f: impl FnOnce(&Hooks, &mut Store<HostState>) -> Option<wasmtime::Result<R>>) -> Option<R> {
        self.plugin.call_hook(self.who, Some(sim), hook, f)
    }
}

impl ModeHooks for WasmGameMode {
    fn name(&self) -> &'static str {
        self.name
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }

    fn client_mode(&self) -> GameMode {
        self.client_mode
    }

    fn map(&self) -> Option<&'static str> {
        self.map
    }

    /// A plugin mode does for itself what the rules would turn on: it asks for traffic, starts the round, opens the
    /// lobby and base menus and deals team damage. The rest it changes with the server's setters.
    fn rules(&self, _sim: &Sim) -> ModeRules {
        ModeRules { reset_traffic: None, ready_start: false, punish_team_kills: false, ..ModeRules::default() }
    }

    fn base_menus(&self, sim: &mut Sim, pid: PlayerId, outside: Option<Vec3>) {
        let outside = outside.map(Into::into);

        self.call(SimPtr(sim), "base-menus", |h, s| h.base_menus.as_ref().map(|g| g.call_base_menus(s, pid.0, outside)));
    }

    fn joining_money(&self, sim: &Sim, money: i32) -> i32 {
        let ptr = SimPtr(sim as *const Sim as *mut Sim);

        self.call(ptr, "joining-money", |h, s| h.joining_money.as_ref().map(|g| g.call_joining_money(s, money))).unwrap_or(money)
    }

    fn on_admin_select(&self, sim: &mut Sim) {
        self.call(SimPtr(sim), "on-admin-select", |h, s| h.on_admin_select.as_ref().map(|g| g.call_on_admin_select(s)));
    }
}
