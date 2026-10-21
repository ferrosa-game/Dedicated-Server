<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/ferrosa-logo-dark.svg">
    <img src="assets/ferrosa-logo.svg" alt="Ferrosa" width="600">
  </picture>
</p>

# Links

<div class="header" align="center">  
  
[Download](https://github.com/ferrosa-game/Dedicated-Server/releases/latest) | [Discord](https://discord.gg/yZJWc6MsMZ)
  
</div>

# Ferrosa Dedicated Server

A Sub Rosa dedicated server written in Rust: a port of the original Sub Rosa dedicated server.

The goal is a faithful port. Most systems were checked against the original server under gdb (physics, humans,
vehicles, traffic, items, the economy, corporations, missions and computers), so the game should behave the way it
does on an official server.

## Game modes

| Mode | Notes |
| --- | --- |
| `round` | Three corporations, a lobby, buy menus at the bases, deals and car chases, optional weekly play. A plugin (`plugins/round`). |
| `eliminator` | Hidden targets, eliminators and protectors, with position reveals and supply drops. A plugin (`plugins/eliminator`). |
| `world` | The persistent city: a day cycle, crime ratings, corporations with managers and projects, computers and world missions. |
| `sandbox` | No game rules, always in game, and everyone can use the test commands below. Clients see it as round mode. A plugin (`plugins/sandbox`). |

Driving, racing, co-op and versus exist in the client but are not ported. Because no one really plays these game modes.

## Running

**The simplest way is to download it from our latest release for your platform! [Here. Or on the right.](https://github.com/ferrosa-game/Dedicated-Server/releases/latest)**

The only thing you need installed is the [Rust toolchain.](https://rustup.rs/)

1. `git submodule update --init --recursive`
2. `rustup target add wasm32-wasip2`, then `cargo build --release` in each folder in `plugins/`
3. `cargo run --release`
4. Enjoy! Its pretty simple!

The server listens on UDP port 27584 by default and registers with the master server so it shows in the server list.

## Configuration

Files the server reads from the working directory:

| File | What it holds |
| --- | --- |
| `config.toml` | The main settings. Written with defaults on first run. |
| `config_world.txt` | World mode: `traffic`, `startcash`, `mincash`, the `crime*` rates and limits, `showjoinexit`, `respawnteam`. Written with defaults if missing. |
| `serveradmin.txt` | `adminphone=<number>` lines, the phone numbers allowed to log in as admin. |
| `server.srk` | Accounts: world mode's saved money and ratings, play time, bans. Created on first run. |
| `plugins/round/config_round.txt` | Round mode: `roundtime` (minutes), `startcash`, `weekly`, `bonusratio`, `teamdamage`. |
| `plugins/round/accounts.txt` | Round mode's saved money. |

`config.toml`:

| Key | Meaning |
| --- | --- |
| `gamemode` | `round`, `eliminator`, `world`, `sandbox` or any plugin's mode (case doesn't matter). |
| `server_name`, `max_players`, `port` | How the server is listed and reached. |
| `server_password` | Empty for a public server. |
| `admin_password` | The password for `/admin` in chat. |
| `round_time` | The length of a world mode day, in seconds. |
| `map_name` | The map for modes that don't choose their own, like sandbox. |
| `master_server_url`, `master_server_ip` | Where the server registers. |

Keep `config.toml`, `serveradmin.txt`, `server.srk` and `plugins/round/accounts.txt` out of version control: they hold passwords and player data.

## Chat commands

Admins log in with `/admin <password>`. They then have the original server's commands, among them `/kick`, `/ban`,
`/list`, `/message`, `/cash`, `/setrating`, `/give` and `/resetgame [mode]`. Anyone can use `/tps` for tick timing.

Sandbox mode has test commands for everyone:

| Command | Does |
| --- | --- |
| `/human` | Gives you a human. |
| `/car [type] [colour]` | Spawns a vehicle in front of you; `/car` alone lists the types. |
| `/item [name]` | Spawns an item; `/item` alone lists the names. |
| `/guns`, `/watermelon` | Spawns a set of guns, or a pile of watermelons. |
| `/phone [number]` | Spawns a phone. |
| `/godmode`, `/kill`, `/heal` | What they say. |
| `/gravity`, `/clear`, `/stocks` | World gravity, clear the world, and the share menu. |

## Plugins

Plugins are WebAssembly components the server loads from `plugins/<name>/`: a `plugin.toml` (`name`, `version`,
`enabled`, and `entry`, the `.wasm` file, `plugin.wasm` by default) and the component itself. A plugin can register a
game mode, hook into the tick, menus, chat, joins and deaths, and use the server's players, humans, items, vehicles,
corporations and physics. It can read and write files in its own folder, and nothing else.

The API is in `crates/ferrosa-wit/wit`. In Rust, `ferrosa-wit` is the SDK: implement its `Plugin` trait, end with
`ferrosa_wit::plugin!(MyPlugin);`, and build for `wasm32-wasip2`. `plugins/sandbox` is a small example and
`plugins/round` and `plugins/eliminator` whole game modes.

```rust
use ferrosa_wit::{PlayerId, Plugin, server};

#[derive(Default)]
struct Hello;

impl Plugin for Hello {
    fn on_join(&mut self, player: PlayerId) {
        if let Some(p) = server::get_player(player) {
            p.message("Hello!");
        }
    }
}

ferrosa_wit::plugin!(Hello);
```

## Workspace

| Crate | Contents |
| --- | --- |
| `ferrosa-bin` | The executable: config, networking setup and the master server registration. |
| `ferrosa-sim` | The game: the simulation, game modes, players, humans, vehicles, traffic, items and computers. |
| `ferrosa-world` | The loaded world: the map and its collision, buildings and bases, streets, item and vehicle types, the skeleton. |
| `ferrosa-physics` | Rigid bodies, bonds and the collision solver. |
| `ferrosa-map` | Parsers for the game's files (maps, vehicle bodies, item sets, accounts). |
| `ferrosa-protocol` | The client/server packet formats. |
| `ferrosa-net` | The UDP connection layer and the master server. |
| `ferrosa-plugins` | Loads and runs the WASM plugins (wasmtime). |
| `ferrosa-wit` | The plugin API (WIT) and the Rust plugin SDK. |

## AI Disclosure

Im gonna be frank, I did *not* want to port all of that weirdly decompiled physics code by hand, AI has gotten to a point where it can write competent code, so I entrusted it with the work, making sure it verified the output of the physics functions via GDB with the real binary.

If you dont like this, dont use it. I dont really know what to tell you.

"Vibecoding" is something I do not partake in, I have audited all of this code myself, and am actively cleaning it up and removing useless code as I see fit. If you see something that should be changed, or that is bad, pipe up! As this project goes further the code will change and I intend to eliminate code written entirely by AI. But as it is just me with a limited schedule im doing the best I can.
