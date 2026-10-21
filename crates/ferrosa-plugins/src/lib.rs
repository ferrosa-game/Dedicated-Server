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
        with: {
            "ferrosa:plugin/types.body": crate::types::BodyRef,
            "ferrosa:plugin/types.corporation": crate::types::CorporationRef,
            "ferrosa:plugin/types.human": crate::types::HumanRef,
            "ferrosa:plugin/types.player": crate::types::PlayerRef,
            "ferrosa:plugin/types.item": crate::types::ItemRef,
            "ferrosa:plugin/types.vehicle": crate::types::VehicleRef,
        },
    });
}

pub use plugin::{LoadedPlugin, load};
