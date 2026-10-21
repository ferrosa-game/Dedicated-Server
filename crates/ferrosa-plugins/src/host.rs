use std::{path::Path, sync::OnceLock, time::Duration};

use ferrosa_protocol::GameMode;
use ferrosa_sim::Sim;
use wasmtime::{Config, Engine, StoreLimits, StoreLimitsBuilder, bail};
use wasmtime_wasi::{FsPerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub(crate) fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();

    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.epoch_interruption(true);

        let engine = Engine::new(&config).expect("the plugin engine's config is valid");
        let ticker = engine.clone();

        std::thread::Builder::new()
            .name("plugin epoch".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(10));
                    ticker.increment_epoch();
                }
            })
            .unwrap();

        engine
    })
}

#[derive(Clone, Copy)]
pub(crate) struct SimPtr(pub(crate) *mut Sim);

unsafe impl Send for SimPtr {}

pub(crate) struct RegisteredMode {
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) client_mode: GameMode,
    pub(crate) map: Option<String>,
}

pub(crate) struct HostState {
    pub(crate) sim: Option<SimPtr>,
    pub(crate) registering: bool,
    pub(crate) registered: Option<RegisteredMode>,
    wasi: WasiCtx,
    pub(crate) table: ResourceTable,
    pub(crate) limits: StoreLimits,
}

impl HostState {
    pub(crate) fn new(folder: &Path) -> wasmtime::Result<Self> {
        let wasi = WasiCtxBuilder::new().inherit_stdout().inherit_stderr().preopened_dir(folder, ".", FsPerms::ReadWrite)?.build();

        Ok(Self {
            sim: None,
            registering: false,
            registered: None,
            wasi,
            table: ResourceTable::new(),
            limits: StoreLimitsBuilder::new().memory_size(64 << 20).build(),
        })
    }

    pub(crate) fn sim(&self) -> wasmtime::Result<&Sim> {
        match self.sim {
            Some(SimPtr(sim)) => Ok(unsafe { &*sim }),
            None => bail!("the server can only be used from a hook"),
        }
    }

    pub(crate) fn sim_mut(&mut self) -> wasmtime::Result<&mut Sim> {
        match self.sim {
            Some(SimPtr(sim)) => Ok(unsafe { &mut *sim }),
            None => bail!("the server can only be used from a hook"),
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}
