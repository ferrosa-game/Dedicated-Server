use ferrosa_protocol::clientbound::game::MenuType;

use crate::{PlayerId, sim::Sim};

pub type Plugins = &'static dyn PluginHooks;

pub trait PluginHooks: Sync {
    fn on_start(&self, _sim: &mut Sim) {}

    fn reset(&self, _sim: &mut Sim) {}

    fn tick(&self, _sim: &mut Sim) {}

    fn tick_after_traffic(&self, _sim: &mut Sim) {}

    fn menus_done(&self, _sim: &mut Sim) {}

    fn on_menu(&self, _sim: &mut Sim, _pid: PlayerId, _menu: MenuType, _button: u32) -> bool {
        false
    }

    fn before_physics(&self, _sim: &mut Sim) {}

    fn after_physics(&self, _sim: &mut Sim) {}

    fn on_chat(&self, _sim: &mut Sim, _pid: PlayerId, _message: &str) -> bool {
        false
    }

    fn on_join(&self, _sim: &mut Sim, _pid: PlayerId) {}

    fn on_leave(&self, _sim: &mut Sim, _pid: PlayerId) {}

    fn on_death(&self, _sim: &mut Sim, _pid: PlayerId) {}

    fn on_hurt(&self, _sim: &mut Sim, _attacker: PlayerId, _victim: PlayerId, _damage: i32) {}
}

pub struct NoPlugins;

impl PluginHooks for NoPlugins {}
