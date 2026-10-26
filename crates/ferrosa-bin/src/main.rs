use std::{net::SocketAddr, path::Path, sync::Arc};

use ferrosa_map::file_types::roster::Roster;
use ferrosa_net::{Edge, ServerListing, masterserver::MasterServer};
use ferrosa_sim::{ConnId, Inbound, Sim};
use tokio::{net::UdpSocket, sync::mpsc};

use crate::config::ConfigMain;

pub mod config;
pub mod check_for_updates;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[Main] Ferrosa version v{}", env!("CARGO_PKG_VERSION"));

    check_for_updates::print_update_status();

    let config = ConfigMain::read_from_file();
    let server_id = Roster::load(Path::new("players.json")).server_id;
    let ferrosa_plugins::LoadedPlugin { plugins, modes } = ferrosa_plugins::load(Path::new("plugins"));

    let Some(&first) = modes.first() else {
        eprintln!("[Config] no game mode: build a mode plugin into plugins/ (round, world, eliminator or sandbox)");

        return Ok(());
    };

    let mode = ferrosa_sim::find(&modes, &config.gamemode).unwrap_or_else(|| {
        let fallback = ferrosa_sim::find(&modes, "round").unwrap_or(first);
        eprintln!("[Config] unknown gamemode {:?} - using {}", config.gamemode, fallback.name());
        fallback
    });

    let (in_tx, in_rx) = mpsc::unbounded_channel::<Inbound>();
    let (out_tx, out_rx) = mpsc::unbounded_channel::<(Vec<u8>, SocketAddr)>();
    let (drop_tx, drop_rx) = mpsc::unbounded_channel::<ConnId>();

    let socket = Arc::new(UdpSocket::bind(("0.0.0.0", config.port)).await?);
    println!("[Net] listening on {}", socket.local_addr()?);

    tokio::spawn(ferrosa_net::send_all(socket.clone(), out_rx));

    let masterserver = MasterServer::connect(&config.master_server_url, config.master_server_ip.as_deref(), out_tx.clone()).await?;

    {
        let out_tx = out_tx.clone();
        let map_name = config.map_name.clone();

        std::thread::Builder::new()
            .name("sim".into())
            .spawn(move || {
                let mut sim = Sim::new(in_rx, out_tx, drop_tx, map_name, mode, modes, plugins, config.max_players);
                sim.set_admin_password(config.admin_password.clone());
                sim.run()
            })
            .unwrap();
    }

    let server_listing = ServerListing {
        build: 38,
        address: [0, 0, 0, 0],
        gamemode: mode.client_mode(),
        max_players: config.max_players,
        password_protected: !config.server_password.is_empty(),
        server_password: config.server_password.clone(),
        port: config.port,
        server_id,
        server_name: config.server_name.clone(),
    };

    Edge::new(socket, in_tx, out_tx, drop_rx, masterserver, server_listing).run().await;
    Ok(())
}
