<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/ferrosa-logo-dark.svg">
    <img src="assets/ferrosa-logo.svg" alt="Ferrosa" width="600">
  </picture>
</p>

# Ferrosa Dedicated Server

A Sub Rosa dedicated server written in Rust: a port of the original Sub Rosa dedicated server.

The goal is a faithful port. Most systems were checked against the original server under gdb (physics, humans,
vehicles, traffic, items, the economy, corporations, missions and computers), so the game should behave the way it
does on an official server.

## Game modes

| Mode | Notes |
| --- | --- |
| `round` | Three corporations, a lobby, buy menus at the bases, deals and car chases, optional weekly play. |
| `eliminator` | Hidden targets, eliminators and protectors, with position reveals and supply drops. |
| `world` | The persistent city: a day cycle, crime ratings, corporations with managers and projects, computers and world missions. |
| `sandbox` | No game rules, always in game, and everyone can use the test commands below. Clients see it as round mode. |

Driving, racing, co-op and versus exist in the client but are not ported. Because no one really plays these game modes.

## Running

**The simplest way is to download it from our latest release for your platform! [Here. Or on the right.](https://github.com/ferrosa-game/Dedicated-Server/releases/latest)**

The only thing you need installed is the [Rust toolchain.](https://rustup.rs/)

1. `git submodule update --init --recursive`
2. `cargo run --release`
3. Enjoy! Its pretty simple!

The server listens on UDP port 27584 by default and registers with the master server so it shows in the server list.

## Configuration

Files the server reads from the working directory:

| File | What it holds |
| --- | --- |
| `config.toml` | The main settings. Written with defaults on first run. |
| `config_round.txt` | Round mode: `roundtime` (minutes), `startcash`, `weekly`, `bonusratio`, `teamdamage`. |
| `config_world.txt` | World mode: `traffic`, `startcash`, `mincash`, the `crime*` rates and limits, `showjoinexit`, `respawnteam`. Written with defaults if missing. |
| `config_versus.txt` | `movedelay`. |
| `serveradmin.txt` | `adminphone=<number>` lines, the phone numbers allowed to log in as admin. |
| `server.srk` | Saved accounts: money, ratings, play time, bans. Created on first run. |

`config.toml`:

| Key | Meaning |
| --- | --- |
| `gamemode` | `round`, `eliminator`, `world` or `sandbox` (case doesn't matter). |
| `server_name`, `max_players`, `port` | How the server is listed and reached. |
| `server_password` | Empty for a public server. |
| `admin_password` | The password for `/admin` in chat. |
| `round_time` | The length of a world mode day, in seconds. |
| `map_name` | The map sandbox starts on; the other modes choose their own. |
| `master_server_url`, `master_server_ip` | Where the server registers. |

Keep `config.toml`, `serveradmin.txt` and `server.srk` out of version control: they hold passwords and player data.

## Chat commands

Admins log in with `/admin <password>`. They then have the original server's commands, among them `/kick`, `/ban`,
`/list`, `/message`, `/cash`, `/setrating`, `/give` and `/resetgame [round|eliminator]`.

In sandbox mode, or as an admin, there are test commands as well:

| Command | Does |
| --- | --- |
| `/human` | Gives you a human. |
| `/car [type] [colour]` | Spawns a vehicle in front of you; `/car` alone lists the types. |
| `/item [name]` | Spawns an item; `/item` alone lists the names. |
| `/guns` | Spawns a set of guns. |
| `/phone [number]` | Spawns a phone. |
| `/godmode`, `/kill`, `/heal` | What they say. |
| `/gravity`, `/clear`, `/tps` | World gravity, clear the world, and tick timing. |

## Workspace

| Crate | Contents |
| --- | --- |
| `ferrosa-bin` | The executable: config, networking setup and the master server registration. |
| `ferrosa-sim` | The game: the simulation, game modes, players, humans, vehicles, traffic, items and computers. |
| `ferrosa-physics` | Rigid bodies, bonds and the collision solver. |
| `ferrosa-map` | Parsers for the game's files (maps, vehicle bodies, item sets, accounts). |
| `ferrosa-protocol` | The client/server packet formats. |
| `ferrosa-net` | The UDP connection layer and the master server. |

## The future

We intend to add WASM into the server, so that way plugins can create custom game modes, this will be fleshed out in further updates. But as it stand right now, the server works.

## AI Disclosure

Im gonna be frank, I did *not* want to port all of that weirdly decompiled physics code by hand, AI has gotten to a point where it can write competent code, so I entrusted it with the work, making sure it verified the output of the physics functions via GDB with the real binary.

If you dont like this, dont use it. I dont really know what to tell you.

"Vibecoding" is something I do not partake in, I have audited all of this code myself, and am actively cleaning it up and removing useless code as I see fit. If you see something that should be changed, or that is bad, pipe up! As this project goes further the code will change and I intend to eliminate code written entirely by AI. But as it is just me with a limited schedule im doing the best I can.
