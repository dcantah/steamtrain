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
            let install = Install::find()?;
            return cmd_size(&install, walk);
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
    println!("    steamtrain                  Launch the TUI");
    println!("    steamtrain size [--walk]    Print disk usage per library");
    println!("    steamtrain help             Show this help");
    println!();
    println!("SIZE OPTIONS:");
    println!("    -w, --walk    Also walk each game directory to report real on-disk usage");
    println!("                  (slower; manifest totals are reported either way)");
}

fn cmd_size(install: &Install, walk: bool) -> Result<(), Box<dyn std::error::Error>> {
    if install.libraries.is_empty() {
        println!("No Steam libraries found.");
        return Ok(());
    }

    let mut grand_manifest: i64 = 0;
    let mut grand_walk: i64 = 0;
    let mut grand_count: usize = 0;

    for lib in &install.libraries {
        let apps = lib.apps()?;
        let manifest_total: i64 = apps.iter().map(|a| a.size_on_disk_manifest).sum();
        grand_manifest += manifest_total;
        grand_count += apps.len();

        println!("Library: {} ({} games)", lib.path.display(), apps.len());
        println!("  Manifest:  {}", human_bytes(manifest_total));

        if walk {
            let mut walk_total: i64 = 0;
            let stderr = io::stderr();
            for (i, app) in apps.iter().enumerate() {
                let mut h = stderr.lock();
                let _ = write!(h, "\r  walking [{}/{}] {:.50}", i + 1, apps.len(), app.name);
                let _ = h.flush();
                drop(h);

                if let Ok(sz) = app.compute_disk_size() {
                    walk_total += sz;
                }
            }
            // Clear the progress line.
            eprint!("\r{:80}\r", "");
            grand_walk += walk_total;
            println!("  On disk:   {}", human_bytes(walk_total));
        }

        println!();
    }

    if install.libraries.len() > 1 {
        println!("Total ({} games):", grand_count);
        println!("  Manifest:  {}", human_bytes(grand_manifest));
        if walk {
            println!("  On disk:   {}", human_bytes(grand_walk));
        }
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
