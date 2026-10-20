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
    component::{Component, HasSelf, Linker},
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
    bindings: bindings::Plugin,
    stopped: AtomicBool,
}

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

        let mut store = Store::new(engine(), HostState::new());
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(10);

        let bindings = bindings::Plugin::instantiate(&mut store, &component, &linker)?;
        let name = match &manifest.name {
            Some(name) => name.clone(),
            None => folder.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        };

        let mut plugin = Self { name: leak(name), mode: None, store: Mutex::new(store), bindings, stopped: AtomicBool::new(false) };
        let init = plugin.call_hook(plugin.name, None, "init", |b, s| {
            s.data_mut().registering = true;
            let result = b.ferrosa_plugin_hooks().call_init(&mut *s);
            s.data_mut().registering = false;
            result
        });

        if init.is_none() {
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
        f: impl FnOnce(&bindings::Plugin, &mut Store<HostState>) -> wasmtime::Result<R>,
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

        let result = f(&self.bindings, &mut store);
        store.data_mut().sim = None;

        result
            .map_err(|e| {
                eprintln!("[Plugins] {who}.{hook}: {e:?}");
                eprintln!("[Plugins] {} is stopped", self.name);
                self.stopped.store(true, Ordering::Relaxed);
            })
            .ok()
    }
}
