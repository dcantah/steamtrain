use chrono::{DateTime, Utc};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, Mode, SortKey};
use crate::util::{format_playtime, human_bytes};

const BANNER: &str = r#"███████╗████████╗███████╗ █████╗ ███╗   ███╗████████╗██████╗  █████╗ ██╗███╗   ██╗
██╔════╝╚══██╔══╝██╔════╝██╔══██╗████╗ ████║╚══██╔══╝██╔══██╗██╔══██╗██║████╗  ██║
███████╗   ██║   █████╗  ███████║██╔████╔██║   ██║   ██████╔╝███████║██║██╔██╗ ██║
╚════██║   ██║   ██╔══╝  ██╔══██║██║╚██╔╝██║   ██║   ██╔══██╗██╔══██║██║██║╚██╗██║
███████║   ██║   ███████╗██║  ██║██║ ╚═╝ ██║   ██║   ██║  ██║██║  ██║██║██║ ╚████║
╚══════╝   ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝"#;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();

    // Mode area height depends on current mode
    let mode_area_height = match app.mode {
        Mode::PickSort => 5,
        Mode::SelectLibrary => app.libraries.len().min(10) as u16,
        _ => 3,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Banner
            Constraint::Length(2),  // Library info
            Constraint::Length(1),  // Help
            Constraint::Min(5),     // Table
            Constraint::Length(1),  // Status
            Constraint::Length(mode_area_height),  // Mode-specific area
        ])
        .split(size);

    draw_banner(f, chunks[0]);
    draw_library_info(f, chunks[1], app);
    draw_help(f, chunks[2]);
    draw_table(f, chunks[3], app);
    draw_status(f, chunks[4], app);
    draw_mode_area(f, chunks[5], app);
}

fn draw_banner<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let banner_lines: Vec<Spans> = BANNER
        .lines()
        .map(|line| Spans::from(Span::styled(line, Style::default().fg(Color::Cyan))))
        .collect();

    let banner = Paragraph::new(banner_lines).alignment(Alignment::Center);
    f.render_widget(banner, area);
}

fn draw_library_info<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let lib_path = if !app.libraries.is_empty() {
        app.libraries[app.cur_lib_idx].path.display().to_string()
    } else {
        "(none)".to_string()
    };

    let games_size: i64 = app.apps.iter().map(|a| a.size_on_disk_manifest).sum();

    // Format: "1.1 TiB/1.8 TiB (Games: 792.5 GiB)"
    let disk_info = if app.lib_total > 0 {
        let used = app.lib_total.saturating_sub(app.lib_free);
        format!(
            "{}/{} (Games: {})",
            human_bytes(used as i64),
            human_bytes(app.lib_total as i64),
            human_bytes(games_size)
        )
    } else {
        format!("Games: {}", human_bytes(games_size))
    };

    let user_str = app.current_user.as_deref().unwrap_or("Unknown");

    let info = vec![
        Spans::from(vec![
            Span::styled("User: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(user_str),
            Span::raw("  "),
            Span::styled("Library: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(&lib_path),
        ]),
        Spans::from(vec![
            Span::styled("Disk Space: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(disk_info),
        ]),
    ];

    let paragraph = Paragraph::new(info);
    f.render_widget(paragraph, area);
}

fn draw_help<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let help = Paragraph::new(Spans::from(vec![
        Span::raw("[↑/↓] Move  [l] Choose Lib  [Enter] Launch  [/] Filter  [d] Delete  [r] Rescan  [s] Sort  [o] Order  [p] Open Folder  [q] Quit"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, area);
}

fn draw_table<B: Backend>(f: &mut Frame<B>, area: Rect, app: &mut App) {
    let header_cells = ["APP ID", "NAME", "SIZE", "PLAY TIME", "LAST UPDATED", "LAST PLAYED"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let selected_style = Style::default()
        .bg(Color::Cyan)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);

    let rows: Vec<Row> = app
        .filtered_apps
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let size_str = if a.size_on_disk_manifest > 0 {
                human_bytes(a.size_on_disk_manifest)
            } else {
                "-".to_string()
            };

            let playtime_str = a
                .playtime_minutes
                .map(format_playtime)
                .unwrap_or_else(|| "-".to_string());

            let last_updated = a
                .last_updated
                .map(|t| format_time(t))
                .unwrap_or_else(|| "-".to_string());

            let last_played = a
                .last_played
                .map(|t| format_time(t))
                .unwrap_or_else(|| "-".to_string());

            let cells = vec![
                Cell::from(a.app_id.clone()),
                Cell::from(a.name.clone()),
                Cell::from(size_str),
                Cell::from(playtime_str),
                Cell::from(last_updated),
                Cell::from(last_played),
            ];

            let row = Row::new(cells);
            if i == app.table_state.selected {
                row.style(selected_style)
            } else {
                row
            }
        })
        .collect();

    // Ensure visibility
    let table_height = area.height.saturating_sub(2) as usize;
    app.table_state.ensure_visible(table_height);

    // Calculate name column width: total width minus fixed columns and spacing
    // Fixed: APP ID(10) + SIZE(12) + PLAYTIME(10) + LAST UPDATED(18) + LAST PLAYED(18) = 68
    // Spacing: 5 gaps * 2 chars = 10
    let fixed_width: u16 = 10 + 12 + 10 + 18 + 18 + 10;
    let name_width = area.width.saturating_sub(fixed_width).max(20);

    let widths = [
        Constraint::Length(10),         // APP ID
        Constraint::Length(name_width), // NAME - fills remaining space
        Constraint::Length(12),         // SIZE
        Constraint::Length(10),         // PLAYTIME
        Constraint::Length(18),         // LAST UPDATED
        Constraint::Length(18),         // LAST PLAYED
    ];

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::TOP | Borders::BOTTOM).border_style(Style::default().fg(Color::Cyan)))
        .widths(&widths)
        .column_spacing(2);

    f.render_widget(table, area);
}

fn draw_status<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    // In certain modes, show the mode header in the status area
    let (text, style) = match app.mode {
        Mode::PickSort => (
            "Sort by (↑/↓, Enter to confirm, Esc to cancel):".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        Mode::SelectLibrary => (
            "Select library (↑/↓, Enter to confirm, Esc to cancel):".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        _ if !app.error.is_empty() => (app.error.clone(), Style::default().fg(Color::Red)),
        _ => (app.status.clone(), Style::default().fg(Color::Green)),
    };

    let paragraph = Paragraph::new(text).style(style);
    f.render_widget(paragraph, area);
}

fn draw_mode_area<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    match app.mode {
        Mode::Normal => {}
        Mode::Filtering => {
            let input = Paragraph::new(format!("Filter: {}_", app.filter_input))
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(input, area);
        }
        Mode::ConfirmDelete => {
            let confirm = Paragraph::new("Delete selected app? y/N")
                .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(confirm, area);
        }
        Mode::PickSort => {
            let options = SortKey::all();
            let mut lines = Vec::new();

            for (i, opt) in options.iter().enumerate() {
                let style = if i == app.pick_sort_idx {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                let prefix = if i == app.pick_sort_idx { "> " } else { "  " };
                lines.push(Spans::from(Span::styled(
                    format!("{}{}", prefix, opt.label()),
                    style,
                )));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, area);
        }
        Mode::SelectLibrary => {
            let mut lines = Vec::new();

            for (i, lib) in app.libraries.iter().enumerate() {
                let style = if i == app.pick_lib_idx {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                let prefix = if i == app.pick_lib_idx { "> " } else { "  " };
                lines.push(Spans::from(Span::styled(
                    format!("{}{}", prefix, lib.path.display()),
                    style,
                )));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, area);
        }
    }
}

fn format_time(time: std::time::SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}
