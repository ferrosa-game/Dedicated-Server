use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use ferrosa_sim::{Mode, NoPlugins, Plugins};
use serde::Deserialize;
use wasmtime::{
    Store,
    component::{Component, HasSelf, Instance, InstancePre, Linker},
};

use crate::{
    bindings,
    hooks::WasmPlugins,
    host::{HostState, RegisteredMode, SimPtr, engine},
    mode::WasmGameMode,
};

#[derive(Deserialize, Default)]
#[serde(default)]
struct PluginManifest {
    name: Option<String>,
    version: Option<String>,
    enabled: Option<bool>,
    entry: Option<String>,
}

pub(crate) struct WasmPlugin {
    pub(crate) name: &'static str,
    pub(crate) mode: Option<&'static str>,
    store: Mutex<Store<HostState>>,
    hooks: Hooks,
    stopped: AtomicBool,
}

macro_rules! hooks {
    ($($hook:ident),* $(,)?) => {
        pub(crate) struct Hooks {
            $(pub(crate) $hook: Option<bindings::exports::ferrosa::plugin::$hook::Guest>,)*
        }

        impl Hooks {
            fn load(pre: &InstancePre<HostState>, store: &mut Store<HostState>, instance: &Instance) -> wasmtime::Result<Self> {
                Ok(Self {
                    $($hook: bindings::exports::ferrosa::plugin::$hook::GuestIndices::new(pre).ok().map(|i| i.load(&mut *store, instance)).transpose()?,)*
                })
            }
        }
    };
}

hooks!(init, on_start, reset, tick, tick_after_traffic, before_physics, after_physics, menus_done, on_menu, on_chat, on_join, on_leave, on_death, on_damage, on_admin_select);

pub struct LoadedPlugin {
    pub plugins: Plugins,
    pub modes: Vec<Mode>,
}

fn folders(dir: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<_> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.join("plugin.toml").is_file()).collect()).unwrap_or_default();
    folders.sort();
    folders
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

pub fn load(dir: &Path) -> LoadedPlugin {
    let dir = dir.to_path_buf();

    std::thread::spawn(move || load_from(&dir)).join().unwrap_or_else(|e| std::panic::resume_unwind(e))
}

fn load_from(dir: &Path) -> LoadedPlugin {
    let mut plugins = Vec::new();
    let mut modes = Vec::new();

    for folder in folders(dir) {
        let manifest = match std::fs::read_to_string(folder.join("plugin.toml")).map_err(|e| e.to_string()).and_then(|t| toml::from_str::<PluginManifest>(&t).map_err(|e| e.to_string())) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[Plugins] {}: {e}", folder.join("plugin.toml").display());
                continue;
            }
        };

        if manifest.enabled == Some(false) {
            continue;
        }

        let (plugin, registered) = match WasmPlugin::load(&folder, &manifest) {
            Ok(Some(loaded)) => loaded,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("[Plugins] {}: {e:?}", folder.display());
                continue;
            }
        };
        let plugin: &'static WasmPlugin = Box::leak(Box::new(plugin));

        match &manifest.version {
            Some(version) => println!("[Plugins] loaded {} {version}", plugin.name),
            None => println!("[Plugins] loaded {}", plugin.name),
        }

        if let Some(registered) = registered {
            println!("[Plugins] {} is the mode {}", plugin.name, registered.name);
            modes.push(&*Box::leak(Box::new(WasmGameMode::new(plugin, registered))) as Mode);
        }

        plugins.push(plugin);
    }

    let plugins: Plugins = if plugins.is_empty() { &NoPlugins } else { Box::leak(Box::new(WasmPlugins(plugins))) };

    LoadedPlugin { plugins, modes }
}

impl WasmPlugin {
    fn load(folder: &Path, manifest: &PluginManifest) -> wasmtime::Result<Option<(Self, Option<RegisteredMode>)>> {
        let path = folder.join(manifest.entry.as_deref().unwrap_or("plugin.wasm"));

        let component = Component::from_file(engine(), &path)?;
        let mut linker = Linker::new(engine());

        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        bindings::Plugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

        let mut store = Store::new(engine(), HostState::new(folder)?);
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(10);

        let pre = linker.instantiate_pre(&component)?;
        let instance = pre.instantiate(&mut store)?;
        let hooks = Hooks::load(&pre, &mut store, &instance)?;
        let name = match &manifest.name {
            Some(name) => name.clone(),
            None => folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };

        let mut plugin = Self { name: leak(name), mode: None, store: Mutex::new(store), hooks, stopped: AtomicBool::new(false) };
        plugin.call_hook(plugin.name, None, "init", |h, s| {
            let init = h.init.as_ref()?;
            s.data_mut().registering = true;

            let result = init.call_init(&mut *s);

            s.data_mut().registering = false;
            Some(result)
        });

        if plugin.stopped.load(Ordering::Relaxed) {
            return Ok(None);
        }

        let registered = plugin.store.get_mut().unwrap().data_mut().registered.take();
        plugin.mode = registered.as_ref().map(|m| leak(m.name.clone()));

        Ok(Some((plugin, registered)))
    }

    pub(crate) fn call_hook<R>(
        &self,
        who: &str,
        sim: Option<SimPtr>,
        hook: &str,
        f: impl FnOnce(&Hooks, &mut Store<HostState>) -> Option<wasmtime::Result<R>>,
    ) -> Option<R> {
        if self.stopped.load(Ordering::Relaxed) {
            return None;
        }

        let Ok(mut store) = self.store.try_lock() else {
            eprintln!("[Plugins] {who}.{hook}: called while another of its hooks runs");
            return None;
        };

        store.data_mut().sim = sim;
        store.set_epoch_deadline(10);

        let result = f(&self.hooks, &mut store);
        store.data_mut().sim = None;

        if let Some(SimPtr(sim)) = sim {
            unsafe { (*sim).flush_updates() };
        }

        result?
            .map_err(|e| {
                eprintln!("[Plugins] {who}.{hook}: {e:?}");
                eprintln!("[Plugins] {} is stopped", self.name);
                self.stopped.store(true, Ordering::Relaxed);
            })
            .ok()
    }
}
