mod hooks;
mod host;
mod mode;
mod plugin;
mod server;
mod types;

mod bindings {
    wasmtime::component::bindgen!({
        path: "../ferrosa-wit/wit",
        world: "plugin",
        imports: { default: trappable },
    });
}

pub use plugin::{LoadedPlugin, load};
