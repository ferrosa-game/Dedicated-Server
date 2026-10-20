use ferrosa_protocol::GameMode;
use ferrosa_sim::{ModeHooks, ModeRules, Sim};
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

    fn rules(&self, sim: &Sim) -> ModeRules {
        self.call(SimPtr::Read(sim), "get-rules", |h, s| h.get_rules.as_ref().map(|g| g.call_get_rules(s))).map_or_else(ModeRules::default, Into::into)
    }

    fn on_admin_select(&self, sim: &mut Sim) {
        self.call(SimPtr::Write(sim), "on-admin-select", |h, s| h.on_admin_select.as_ref().map(|g| g.call_on_admin_select(s)));
    }
}
