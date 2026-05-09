mod app;
mod ui;
mod util;

use std::io::{self, Write};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

use steamtrain::Install;

use app::App;
use util::human_bytes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("-h") | Some("--help") | Some("help") => {
            print_usage();
            return Ok(());
        }
        Some("size") => {
            let walk = args[1..]
                .iter()
                .any(|a| a == "--walk" || a == "-w");
            let per_game = args[1..]
                .iter()
                .any(|a| a == "--per-game" || a == "-g");
            let install = Install::find()?;
            return cmd_size(&install, walk, per_game);
        }
        Some(other) if other.starts_with('-') => {
            // No top-level flags supported; fall through to launch TUI for bare invocation only.
            eprintln!("unknown option: {}", other);
            print_usage();
            std::process::exit(2);
        }
        Some(other) => {
            eprintln!("unknown command: {}", other);
            print_usage();
            std::process::exit(2);
        }
        None => {}
    }

    let install = Install::find()?;
    // Load apps from first library with playtime data
    let apps = if !install.libraries.is_empty() {
        install.libraries[0].apps_with_playtime(install.playtime())?
    } else {
        Vec::new()
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(install, apps);
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {}", err);
    }

    Ok(())
}

fn print_usage() {
    println!("steamtrain - Steam library management TUI");
    println!();
    println!("USAGE:");
    println!("    steamtrain                          Launch the TUI");
    println!("    steamtrain size [--walk] [-g]       Print disk usage per library");
    println!("    steamtrain help                     Show this help");
    println!();
    println!("SIZE OPTIONS:");
    println!("    -w, --walk        Walk each game directory for real on-disk usage instead");
    println!("                      of reading the manifest tally (slower, but accurate)");
    println!("    -g, --per-game    Also list every game with its individual size, sorted");
    println!("                      largest-first");
}

fn cmd_size(install: &Install, walk: bool, per_game: bool) -> Result<(), Box<dyn std::error::Error>> {
    if install.libraries.is_empty() {
        println!("No Steam libraries found.");
        return Ok(());
    }

    let mut grand_total: i64 = 0;
    let mut grand_count: usize = 0;
    let label = if walk { "On disk" } else { "Manifest" };

    for lib in &install.libraries {
        let apps = lib.apps()?;
        grand_count += apps.len();

        println!("Library: {} ({} games)", lib.path.display(), apps.len());

        let mut entries: Vec<(String, i64)> = if walk {
            let mut v = Vec::with_capacity(apps.len());
            let stderr = io::stderr();
            for (i, app) in apps.iter().enumerate() {
                let mut h = stderr.lock();
                let _ = write!(h, "\r  walking [{}/{}] {:.50}", i + 1, apps.len(), app.name);
                let _ = h.flush();
                drop(h);

                let sz = app.compute_disk_size().unwrap_or(0);
                v.push((app.name.clone(), sz));
            }
            // Clear the progress line.
            eprint!("\r{:80}\r", "");
            v
        } else {
            apps.iter()
                .map(|a| (a.name.clone(), a.size_on_disk_manifest))
                .collect()
        };

        let lib_total: i64 = entries.iter().map(|(_, s)| *s).sum();
        grand_total += lib_total;

        if per_game {
            entries.sort_by(|a, b| b.1.cmp(&a.1));
            for (name, sz) in &entries {
                println!("  {:>11}  {}", human_bytes(*sz), name);
            }
        }

        println!("  {}: {}", label, human_bytes(lib_total));
        println!();
    }

    if install.libraries.len() > 1 {
        println!("Total ({} games):", grand_count);
        println!("  {}: {}", label, human_bytes(grand_total));
    }

    Ok(())
}

fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.handle_key(key) {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse.kind);
                }
                Event::Resize(_, _) => {
                    // Terminal will auto-resize on next draw
                }
            }
        }

        app.tick();
    }
}
