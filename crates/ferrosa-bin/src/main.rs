use std::{net::SocketAddr, path::Path, sync::Arc};

use ferrosa_map::file_types::srk::SrkData;
use ferrosa_net::{Edge, ServerListing, masterserver::MasterServer};
use ferrosa_sim::{ConnId, Inbound, Sim};
use tokio::{net::UdpSocket, sync::mpsc};

use crate::config::ConfigMain;

pub mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[Main] Ferrosa version v{}", env!("CARGO_PKG_VERSION"));

    let config = ConfigMain::read_from_file();
    let mode = ferrosa_sim::by_name(&config.gamemode).unwrap_or_else(|| {
        eprintln!("[Config] unknown gamemode {:?} - using round", config.gamemode);
        ferrosa_sim::by_name("round").unwrap()
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
                let mut sim = Sim::new(in_rx, out_tx, drop_tx, map_name, mode, config.max_players);
                sim.set_admin_password(config.admin_password.clone());
                sim.set_world_max_time(config.round_time as i32 * 60);
                sim.run()
            })
            .unwrap();
    }

    let srk = SrkData::load(Path::new("server.srk")).unwrap();
    let server_listing = ServerListing {
        build: 38,
        address: [0, 0, 0, 0],
        gamemode: mode.client_mode(),
        max_players: config.max_players,
        password_protected: !config.server_password.is_empty(),
        server_password: config.server_password.clone(),
        port: config.port,
        server_id: srk.server_id,
        server_name: config.server_name.clone(),
    };

    Edge::new(socket, in_tx, out_tx, drop_rx, masterserver, server_listing).run().await;
    Ok(())
}
