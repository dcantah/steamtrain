# SteamTrain

A terminal UI for managing your Steam game libraries (without SteamCMD). Browse, launch, and uninstall games from your Steam libraries directly from the command line.

[!NOTE]
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

- Rust 1.59+
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
