use ferrosa_protocol::clientbound::game::MenuType;
use ferrosa_sim::{BoneId, ComputerMode, DamageCause, PlayerId, PluginHooks, Sim};
use wasmtime::{Store, component::ResourceAny};

use crate::{
    bindings::ferrosa::plugin::types,
    host::{HostState, SimPtr},
    plugin::{Hooks, WasmPlugin},
};

pub(crate) struct WasmPlugins {
    pub(crate) plugins: Vec<&'static WasmPlugin>,
    programs: Vec<&'static str>,
}

impl WasmPlugins {
    pub(crate) fn new(plugins: Vec<&'static WasmPlugin>, programs: Vec<&'static str>) -> Self {
        Self { plugins, programs }
    }

    fn running(&self, item: usize) -> Option<(&'static WasmPlugin, ResourceAny)> {
        self.plugins.iter().find_map(|p| {
            let running = p.running.lock().ok()?;
            let &(_, handle) = running.iter().find(|(at, _)| *at == item)?;

            Some((*p, handle))
        })
    }

    fn drop_running(&self, sim: &mut Sim, item: usize) {
        let Some((plugin, handle)) = self.running(item) else { return };

        if let Ok(mut running) = plugin.running.lock() {
            running.retain(|(at, _)| *at != item);
        }

        plugin.drop_program(sim, handle);
    }

    fn owner(&self, program: &str) -> Option<&'static WasmPlugin> {
        self.plugins.iter().copied().find(|p| p.programs.contains(&program))
    }

    fn targets(&self, sim: &Sim) -> Vec<&'static WasmPlugin> {
        let running = sim.mode_name();
        let mode = self.plugins.iter().copied().filter(|p| p.mode == Some(running));

        mode.chain(self.plugins.iter().copied().filter(|p| p.mode.is_none())).collect()
    }

    fn all_hooks(&self, sim: &mut Sim, hook: &str, f: impl Fn(&Hooks, &mut Store<HostState>) -> Option<wasmtime::Result<()>>) {
        for plugin in self.targets(sim) {
            plugin.call_hook(plugin.name, Some(SimPtr(&mut *sim)), hook, &f);
        }
    }
}

impl PluginHooks for WasmPlugins {
    fn on_start(&self, sim: &mut Sim) {
        self.all_hooks(sim, "on-start", |h, s| h.on_start.as_ref().map(|g| g.call_on_start(s)));
    }

    fn reset(&self, sim: &mut Sim) {
        self.all_hooks(sim, "reset", |h, s| h.reset.as_ref().map(|g| g.call_reset(s)));
    }

    fn tick(&self, sim: &mut Sim) {
        self.all_hooks(sim, "tick", |h, s| h.tick.as_ref().map(|g| g.call_tick(s)));
    }

    fn tick_after_traffic(&self, sim: &mut Sim) {
        self.all_hooks(sim, "tick-after-traffic", |h, s| h.tick_after_traffic.as_ref().map(|g| g.call_tick_after_traffic(s)));
    }

    fn before_physics(&self, sim: &mut Sim) {
        self.all_hooks(sim, "before-physics", |h, s| h.before_physics.as_ref().map(|g| g.call_before_physics(s)));
    }

    fn after_physics(&self, sim: &mut Sim) {
        self.all_hooks(sim, "after-physics", |h, s| h.after_physics.as_ref().map(|g| g.call_after_physics(s)));
    }

    fn menus_done(&self, sim: &mut Sim) {
        self.all_hooks(sim, "menus-done", |h, s| h.menus_done.as_ref().map(|g| g.call_menus_done(s)));
    }

    fn on_menu(&self, sim: &mut Sim, pid: PlayerId, menu: MenuType, button: u32) -> bool {
        self.targets(sim).into_iter().any(|plugin| {
            plugin.call_hook(plugin.name, Some(SimPtr(&mut *sim)), "on-menu", |h, s| h.on_menu.as_ref().map(|g| g.call_on_menu(s, pid.0, menu.into(), button))) == Some(true)
        })
    }

    fn on_chat(&self, sim: &mut Sim, pid: PlayerId, message: &str) -> bool {
        self.targets(sim).into_iter().any(|plugin| {
            plugin.call_hook(plugin.name, Some(SimPtr(&mut *sim)), "on-chat", |h, s| h.on_chat.as_ref().map(|g| g.call_on_chat(s, pid.0, message))) == Some(true)
        })
    }

    fn on_join(&self, sim: &mut Sim, pid: PlayerId) {
        self.all_hooks(sim, "on-join", |h, s| h.on_join.as_ref().map(|g| g.call_on_join(s, pid.0)));
    }

    fn on_leave(&self, sim: &mut Sim, pid: PlayerId) {
        self.all_hooks(sim, "on-leave", |h, s| h.on_leave.as_ref().map(|g| g.call_on_leave(s, pid.0)));
    }

    fn on_death(&self, sim: &mut Sim, pid: PlayerId) {
        self.all_hooks(sim, "on-death", |h, s| h.on_death.as_ref().map(|g| g.call_on_death(s, pid.0)));
    }

    fn programs(&self) -> &[&'static str] {
        &self.programs
    }

    fn on_computer_run(&self, sim: &mut Sim, item: usize, program: &str, args: &str) -> bool {
        let Some(plugin) = self.owner(program) else { return false };

        self.drop_running(sim, item);

        let started = plugin.call_hook(plugin.name, Some(SimPtr(&mut *sim)), "program.start", |h, s| {
            let programs = h.programs.as_ref()?;

            Some(programs.program().call_start(s, program, item as u32, args))
        });

        match started.flatten() {
            Some(handle) => {
                if sim.computer_mode(item) != Some(ComputerMode::Plugin) {
                    plugin.drop_program(sim, handle);

                    return true;
                }

                if let Ok(mut running) = plugin.running.lock() {
                    running.push((item, handle));
                }

                true
            }
            None => false,
        }
    }

    fn on_computer_key(&self, sim: &mut Sim, item: usize, key: i32) -> bool {
        let Some((plugin, handle)) = self.running(item) else { return false };

        plugin
            .call_hook(plugin.name, Some(SimPtr(&mut *sim)), "program.key", |h, s| {
                let programs = h.programs.as_ref()?;

                Some(programs.program().call_key(s, handle, item as u32, key))
            })
            .unwrap_or(false)
    }

    fn on_computer_tick(&self, sim: &mut Sim, item: usize) {
        let Some((plugin, handle)) = self.running(item) else { return };

        plugin.call_hook(plugin.name, Some(SimPtr(&mut *sim)), "program.tick", |h, s| {
            let programs = h.programs.as_ref()?;

            Some(programs.program().call_tick(s, handle, item as u32))
        });
    }

    fn on_computer_exit(&self, sim: &mut Sim, item: usize) {
        self.drop_running(sim, item);
    }

    fn on_damage(&self, sim: &mut Sim, human: usize, cause: DamageCause, bone: Option<BoneId>, damage: i32) -> i32 {
        let (cause, bone): (types::DamageCause, Option<types::Bone>) = (cause.into(), bone.map(Into::into));

        self.targets(sim).into_iter().fold(damage, |damage, plugin| {
            plugin
                .call_hook(plugin.name, Some(SimPtr(&mut *sim)), "on-damage", |h, s| h.on_damage.as_ref().map(|g| g.call_on_damage(s, human as u32, cause, bone, damage)))
                .unwrap_or(damage)
        })
    }
}
