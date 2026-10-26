use ferrosa_protocol::clientbound::game::MenuType;

use crate::{PlayerId, sim::Sim, world::skeleton::BoneId};

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

    fn on_hurt(&self, _sim: &mut Sim, _attacker: PlayerId, _victim: PlayerId, _damage: i32) {}

    fn on_death(&self, _sim: &mut Sim, _pid: PlayerId) {}


    fn on_damage(&self, _sim: &mut Sim, _human: usize, _cause: DamageCause, _bone: Option<BoneId>, damage: i32) -> i32 {
        damage
    }

    fn programs(&self) -> &[&'static str] {
        &[]
    }

    fn on_computer_run(&self, _sim: &mut Sim, _item: usize, _program: &str, _args: &str) -> bool {
        false
    }

    fn on_computer_key(&self, _sim: &mut Sim, _item: usize, _key: i32) -> bool {
        false
    }

    fn on_computer_tick(&self, _sim: &mut Sim, _item: usize) {}

    fn on_computer_exit(&self, _sim: &mut Sim, _item: usize) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCause {
    Bullet(Option<PlayerId>),
    Explosion(Option<PlayerId>),
    Fall,
    Vehicle(Option<PlayerId>),
}

pub struct NoPlugins;

impl PluginHooks for NoPlugins {}
