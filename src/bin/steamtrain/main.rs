mod app;
mod ui;
mod util;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

use steamtrain::Install;

use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
