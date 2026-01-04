## SteamTrain

A terminal UI for managing your Steam game libraries (without SteamCMD). Browse, launch, and uninstall games from your Steam libraries directly from the command line.

```
                        ███████╗████████╗███████╗ █████╗ ███╗   ███╗████████╗██████╗  █████╗ ██╗███╗   ██╗
                        ██╔════╝╚══██╔══╝██╔════╝██╔══██╗████╗ ████║╚══██╔══╝██╔══██╗██╔══██╗██║████╗  ██║
                        ███████╗   ██║   █████╗  ███████║██╔████╔██║   ██║   ██████╔╝███████║██║██╔██╗ ██║
                        ╚════██║   ██║   ██╔══╝  ██╔══██║██║╚██╔╝██║   ██║   ██╔══██╗██╔══██║██║██║╚██╗██║
                        ███████║   ██║   ███████╗██║  ██║██║ ╚═╝ ██║   ██║   ██║  ██║██║  ██║██║██║ ╚████║
                        ╚══════╝   ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝
 User: SteamTrainUser  Library: E:\SteamLibrary
 Disk Space: 1.1 TiB/1.8 TiB (Games: 793.4 GiB)
 [↑/↓] Move  [l] Choose Lib  [Enter] Launch  [/] Filter  [d] Delete  [r] Rescan  [s] Sort  [o] Order  [p] Open Folder  [q] Quit
 ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
 APP ID    NAME                                                      SIZE         PLAY TIME     LAST UPDATED        LAST PLAYED
 1903340   Clair Obscur: Expedition 33                               40.7 GiB     69h 15m       2025-08-13 23:01    2025-06-29 03:55
 730       Counter-Strike 2                                          58.4 GiB     951h 30m      2025-08-19 19:19    2025-08-16 23:29
 236430    DARK SOULS™ II                                            13.8 GiB     81h 15m       2025-03-30 16:46    2022-04-16 04:29
 374320    DARK SOULS™ III                                           24.9 GiB     125h 12m      2025-03-30 19:49    2023-07-16 08:12
 211420    DARK SOULS™: Prepare To Die Edition                       3.7 GiB      102h 5m       2025-03-30 16:24    2024-07-16 08:12
 3017860   DOOM: The Dark Ages                                       76.7 GiB     5h 42m        2025-08-17 03:29    2025-05-16 19:22
 1245620   ELDEN RING                                                67.0 GiB     294h 5m       2025-08-28 03:51    2025-08-03 15:58
 39210     FINAL FANTASY XIV Online                                  52.4 MiB     550h 20m      2025-07-30 19:46    2025-08-06 21:24
 3240220   Grand Theft Auto V Enhanced                               93.3 GiB     -             2025-07-20 07:22    -
 1145350   Hades II                                                  10.2 GiB     18h 50m       2025-11-16 02:10    2024-05-14 22:49
 367520    Hollow Knight                                             7.4 GiB      32h 33m       2025-03-30 17:18    2021-10-10 20:15
 1030300   Hollow Knight: Silksong                                   7.6 GiB      50h 10m       2025-11-22 19:38    2025-10-11 20:20
 1627720   Lies of P                                                 59.3 GiB     29h 55m       2025-06-21 02:30    2025-06-15 04:13
 1809540   Nine Sols                                                 6.1 GiB      25h 3m        2025-03-30 16:14    2025-01-23 06:28
 1687950   Persona 5 Royal                                           39.4 GiB     12h 3m        2024-10-26 11:35    2024-10-31 22:39
 620       Portal 2                                                  11.9 GiB     21h 13m       2025-03-30 16:40    2024-08-12 05:39
 2751000   Prince of Persia The Lost Crown                           28.6 GiB     18h 10m       2025-03-30 15:56    2025-03-28 12:56
 1174180   Red Dead Redemption 2                                     119.5 GiB    70h 30m       2025-03-31 06:48    2025-04-01 01:07
 814380    Sekiro™: Shadows Die Twice                                14.9 GiB     107h 50m      2024-05-15 22:18    2024-06-28 18:46
 2680010   The First Berserker: Khazan                               52.4 GiB     24h 12m       2025-08-13 23:03    2025-05-04 04:16
 292030    The Witcher 3: Wild Hunt                                  57.3 GiB     2h 3m         2025-03-30 22:42    2025-03-30 22:46
 ─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
```

> [!NOTE]
> 1. This program relies heavily on Steam's internal file formats (VDF files like `libraryfolders.vdf`, `appmanifest_*.acf`, `loginusers.vdf`, and `localconfig.vdf`). These are undocumented formats that Valve could change at any time, which would break this program's functionality.
>
> 2. I used this project to learn Rust, so likely not the most idiomatic :)
>
> 3. It's unlikely I'll be taking contributions, but feel free to fork!

## Features

- Browse installed games across all Steam libraries
- Launch games directly from the TUI
- Uninstall games via Steam
- Filter games by name or App ID
- Sort by name, size, last played, or App ID
- View disk space usage per library
- Open game installation folders in file explorer

## Requirements

- Rust 1.78+
- Steam installed on your system
- Windows or macOS

## Building

```bash
cargo build --release
```

## Usage

```bash
steamtrain
```

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `PgUp` / `b` | Page up |
| `PgDn` / `f` | Page down |
| `Enter` | Launch selected game |
| `/` | Filter games |
| `s` | Sort options |
| `o` | Toggle sort order (asc/desc) |
| `l` | Switch library |
| `d` | Delete/uninstall game |
| `r` | Rescan library |
| `p` | Open install folder |
| `q` / `Ctrl+C` | Quit |

## Library Structure

```
steamtrain-rs/
├── src/
│   ├── lib.rs              # Core library (Install, Library, App)
│   ├── vdf.rs              # Valve Data Format parser
│   ├── platform_windows.rs # Windows-specific code
│   ├── platform_darwin.rs  # macOS-specific code
│   └── bin/steamtrain/
│       ├── main.rs         # Entry point
│       ├── app.rs          # TUI application state
│       ├── ui.rs           # TUI rendering
│       └── util.rs         # Helpers (human_bytes, etc.)
├── Cargo.toml
└── README.md
```

## Library API

The core library can be used independently of the TUI:

```rust
use steamtrain::{Install, Library, App};

// Find Steam installation and all libraries
let install = Install::find()?;

// List all games in a library
for lib in &install.libraries {
    let apps = lib.apps()?;
    for app in apps {
        println!("{}: {} ({})", app.app_id, app.name, app.game_path.display());
    }
}

// Search for a game
let results = install.libraries[0].search_by_name("Dark Souls")?;

// Launch a game
results[0].launch(&[])?;
```
