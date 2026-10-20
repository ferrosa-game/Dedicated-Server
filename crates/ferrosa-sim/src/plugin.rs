use crate::{PlayerId, sim::Sim};

pub type Plugins = &'static dyn PluginHooks;

pub trait PluginHooks: Sync {
    fn on_start(&self, _sim: &mut Sim) {}

    fn reset(&self, _sim: &mut Sim) {}

    fn tick(&self, _sim: &mut Sim) {}

    fn tick_after_traffic(&self, _sim: &mut Sim) {}

    fn menus_done(&self, _sim: &mut Sim) {}

    fn on_menu(&self, _sim: &mut Sim, _pid: PlayerId, _menu: u8, _button: u32) -> bool {
        false
    }

    fn joining_money(&self, _sim: &Sim, money: i32) -> i32 {
        money
    }

    fn before_physics(&self, _sim: &mut Sim) {}

    fn after_physics(&self, _sim: &mut Sim) {}

    fn on_chat(&self, _sim: &mut Sim, _pid: PlayerId, _message: &str) -> bool {
        false
    }

    fn on_join(&self, _sim: &mut Sim, _pid: PlayerId) {}

    fn on_leave(&self, _sim: &mut Sim, _pid: PlayerId) {}

    fn on_death(&self, _sim: &mut Sim, _pid: PlayerId) {}
}

pub struct NoPlugins;

impl PluginHooks for NoPlugins {}
