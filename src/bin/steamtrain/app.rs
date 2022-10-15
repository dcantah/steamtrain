use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use steamtrain::{App as SteamApp, Install, Library};

/// Tracks a pending uninstall to watch for completion.
struct UninstallWatch {
    /// Path to the appmanifest_*.acf file we're watching for deletion.
    manifest_path: PathBuf,
    /// When the watch started (for timeout).
    started: Instant,
}

impl UninstallWatch {
    const TIMEOUT_SECS: u64 = 180;

    fn new(manifest_path: PathBuf) -> Self {
        Self {
            manifest_path,
            started: Instant::now(),
        }
    }

    /// Returns true if the manifest file has been deleted.
    fn is_deleted(&self) -> bool {
        !self.manifest_path.exists()
    }

    /// Returns true if we've been watching too long.
    fn is_expired(&self) -> bool {
        self.started.elapsed().as_secs() >= Self::TIMEOUT_SECS
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filtering,
    ConfirmDelete,
    PickSort,
    SelectLibrary,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Size,
    Playtime,
    LastPlayed,
    AppID,
}

impl SortKey {
    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Size => "Size",
            SortKey::Playtime => "Play Time",
            SortKey::LastPlayed => "Last Played",
            SortKey::AppID => "App ID",
        }
    }

    pub fn all() -> [SortKey; 5] {
        [SortKey::Name, SortKey::Size, SortKey::Playtime, SortKey::LastPlayed, SortKey::AppID]
    }
}

pub struct App {
    pub apps: Vec<SteamApp>,
    pub libraries: Vec<Library>,
    pub cur_lib_idx: usize,

    pub mode: Mode,
    pub filter: String,
    pub filter_input: String,

    pub table_state: TableState,
    pub filtered_apps: Vec<SteamApp>,

    pub sort_by: SortKey,
    sort_asc: bool,
    pub pick_sort_idx: usize,
    pub pick_lib_idx: usize,

    pub status: String,
    pub error: String,

    pub lib_total: u64,
    pub lib_free: u64,

    /// Currently logged-in user's display name.
    pub current_user: Option<String>,

    /// Playtime data (app_id -> minutes).
    playtime: HashMap<String, u64>,

    /// Active uninstall watch, if any.
    uninstall_watch: Option<UninstallWatch>,
}

pub struct TableState {
    pub selected: usize,
    pub offset: usize,
}

impl TableState {
    fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
        }
    }

    fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(len - 1);
    }

    fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn page_down(&mut self, page_size: usize, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + page_size).min(len - 1);
    }

    fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
    }

    pub fn ensure_visible(&mut self, height: usize) {
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = self.selected - height + 1;
        }
    }
}

impl App {
    pub fn new(install: Install, apps: Vec<SteamApp>) -> Self {
        let libraries = install.libraries.clone();
        let playtime = install.playtime().clone();
        let current_user = install.current_user.clone();
        let mut app = App {
            apps,
            libraries,
            cur_lib_idx: 0,
            mode: Mode::Normal,
            filter: String::new(),
            filter_input: String::new(),
            table_state: TableState::new(),
            filtered_apps: Vec::new(),
            sort_by: SortKey::Name,
            sort_asc: true,
            pick_sort_idx: 0,
            pick_lib_idx: 0,
            status: String::new(),
            error: String::new(),
            lib_total: 0,
            lib_free: 0,
            current_user,
            playtime,
            uninstall_watch: None,
        };
        app.update_disk_space();
        app.refresh_filtered();
        app
    }

    fn update_disk_space(&mut self) {
        if self.libraries.is_empty() {
            return;
        }
        let lib_path = &self.libraries[self.cur_lib_idx].path;
        #[cfg(windows)]
        {
            if let Ok((total, free)) = steamtrain::platform::disk_space(lib_path) {
                self.lib_total = total;
                self.lib_free = free;
            }
        }
        #[cfg(not(windows))]
        {
            // TODO: implement for other platforms
            let _ = lib_path;
        }
    }

    pub fn set_library(&mut self, idx: usize) {
        if self.libraries.is_empty() {
            return;
        }
        self.cur_lib_idx = idx.min(self.libraries.len() - 1);

        match self.libraries[self.cur_lib_idx].apps_with_playtime(&self.playtime) {
            Ok(apps) => {
                self.apps = apps;
                self.error.clear();
            }
            Err(e) => {
                self.apps.clear();
                self.error = format!("load apps: {}", e);
            }
        }
        self.update_disk_space();
        self.refresh_filtered();
    }

    pub fn refresh_filtered(&mut self) {
        let needle = self.filter.to_lowercase();
        self.filtered_apps = self
            .apps
            .iter()
            .filter(|a| {
                if needle.is_empty() {
                    true
                } else {
                    a.name.to_lowercase().contains(&needle)
                        || a.app_id.to_lowercase().contains(&needle)
                }
            })
            .cloned()
            .collect();

        self.sort_apps();

        if self.table_state.selected >= self.filtered_apps.len() && !self.filtered_apps.is_empty() {
            self.table_state.selected = self.filtered_apps.len() - 1;
        }
    }

    fn sort_apps(&mut self) {
        self.filtered_apps.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Size => a.size_on_disk_manifest.cmp(&b.size_on_disk_manifest),
                SortKey::Playtime => a.playtime_minutes.unwrap_or(0).cmp(&b.playtime_minutes.unwrap_or(0)),
                SortKey::LastPlayed => {
                    let at = a.last_played.unwrap_or(std::time::UNIX_EPOCH);
                    let bt = b.last_played.unwrap_or(std::time::UNIX_EPOCH);
                    at.cmp(&bt)
                }
                SortKey::AppID => a.app_id.cmp(&b.app_id),
            };
            if self.sort_asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    pub fn selected_app(&self) -> Option<&SteamApp> {
        self.filtered_apps.get(self.table_state.selected)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Filtering => self.handle_filtering(key),
            Mode::ConfirmDelete => self.handle_confirm_delete(key),
            Mode::PickSort => self.handle_pick_sort(key),
            Mode::SelectLibrary => self.handle_select_library(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,

            KeyCode::Up | KeyCode::Char('k') => {
                self.table_state.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.table_state.select_next(self.filtered_apps.len());
            }
            KeyCode::PageUp | KeyCode::Char('b') => {
                self.table_state.page_up(10);
            }
            KeyCode::PageDown | KeyCode::Char('f') => {
                self.table_state.page_down(10, self.filtered_apps.len());
            }

            KeyCode::Enter => {
                if let Some(app) = self.selected_app() {
                    match app.launch(&[]) {
                        Ok(_) => {
                            self.status = "Launched.".to_string();
                            self.error.clear();
                        }
                        Err(e) => {
                            self.error = format!("launch: {}", e);
                        }
                    }
                }
            }

            KeyCode::Char('/') => {
                self.mode = Mode::Filtering;
                self.filter_input = self.filter.clone();
                self.status.clear();
                self.error.clear();
            }

            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.set_library(self.cur_lib_idx);
                self.status = "Refreshed.".to_string();
            }

            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.mode = Mode::SelectLibrary;
                self.pick_lib_idx = self.cur_lib_idx;
                self.status.clear();
                self.error.clear();
            }

            KeyCode::Char('d') | KeyCode::Char('D') => {
                if self.selected_app().is_some() {
                    self.mode = Mode::ConfirmDelete;
                    self.status.clear();
                    self.error.clear();
                }
            }

            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.mode = Mode::PickSort;
                self.pick_sort_idx = SortKey::all()
                    .iter()
                    .position(|&k| k == self.sort_by)
                    .unwrap_or(0);
                self.status.clear();
                self.error.clear();
            }

            KeyCode::Char('o') | KeyCode::Char('O') => {
                self.sort_asc = !self.sort_asc;
                self.refresh_filtered();
                self.status = format!(
                    "Sort: {} ({})",
                    self.sort_by.label(),
                    if self.sort_asc { "asc" } else { "desc" }
                );
            }

            KeyCode::Char('p') | KeyCode::Char('P') => {
                if let Some(app) = self.selected_app() {
                    match app.open_install_path() {
                        Ok(_) => {
                            self.status = "Opened install folder.".to_string();
                            self.error.clear();
                        }
                        Err(e) => {
                            self.error = format!("open folder: {}", e);
                        }
                    }
                }
            }

            _ => {}
        }
        false
    }

    fn handle_filtering(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                self.filter = self.filter_input.clone();
                self.mode = Mode::Normal;
                self.refresh_filtered();
                self.table_state.selected = 0;
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
            }
            _ => {}
        }
        false
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(app) = self.selected_app().cloned() {
                    // Build the manifest path to watch
                    let manifest_path = self.libraries[self.cur_lib_idx]
                        .steamapps_path()
                        .join(format!("appmanifest_{}.acf", app.app_id));

                    match app.uninstall_via_steam() {
                        Ok(_) => {
                            self.status = "Uninstalling via Steam... (will auto-refresh when done)".to_string();
                            self.error.clear();
                            // Start watching for the manifest to be deleted
                            self.uninstall_watch = Some(UninstallWatch::new(manifest_path));
                        }
                        Err(e) => {
                            self.error = format!("uninstall: {}", e);
                        }
                    }
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        false
    }

    fn handle_pick_sort(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                if self.pick_sort_idx > 0 {
                    self.pick_sort_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.pick_sort_idx < 4 {
                    self.pick_sort_idx += 1;
                }
            }
            KeyCode::Enter => {
                self.sort_by = SortKey::all()[self.pick_sort_idx];
                self.mode = Mode::Normal;
                self.refresh_filtered();
                self.status = format!(
                    "Sort: {} ({})",
                    self.sort_by.label(),
                    if self.sort_asc { "asc" } else { "desc" }
                );
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        false
    }

    fn handle_select_library(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                if self.pick_lib_idx > 0 {
                    self.pick_lib_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.pick_lib_idx < self.libraries.len() - 1 {
                    self.pick_lib_idx += 1;
                }
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.set_library(self.pick_lib_idx);
                self.status = "Library changed".to_string();
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        false
    }

    pub fn handle_mouse(&mut self, kind: MouseEventKind) {
        match kind {
            MouseEventKind::ScrollUp => {
                match self.mode {
                    Mode::Normal => {
                        self.table_state.select_prev();
                    }
                    Mode::PickSort => {
                        if self.pick_sort_idx > 0 {
                            self.pick_sort_idx -= 1;
                        }
                    }
                    Mode::SelectLibrary => {
                        if self.pick_lib_idx > 0 {
                            self.pick_lib_idx -= 1;
                        }
                    }
                    _ => {}
                }
            }
            MouseEventKind::ScrollDown => {
                match self.mode {
                    Mode::Normal => {
                        self.table_state.select_next(self.filtered_apps.len());
                    }
                    Mode::PickSort => {
                        if self.pick_sort_idx < 4 {
                            self.pick_sort_idx += 1;
                        }
                    }
                    Mode::SelectLibrary => {
                        if self.pick_lib_idx < self.libraries.len().saturating_sub(1) {
                            self.pick_lib_idx += 1;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        // Check if we're watching for an uninstall to complete
        if let Some(ref watch) = self.uninstall_watch {
            if watch.is_deleted() {
                // The manifest was deleted - game was uninstalled!
                self.uninstall_watch = None;
                self.set_library(self.cur_lib_idx); // Rescan
                self.status = "Uninstall complete. Library refreshed.".to_string();
            } else if watch.is_expired() {
                // Timed out waiting (user probably cancelled in Steam)
                self.uninstall_watch = None;
                self.status = "Uninstall watch timed out.".to_string();
            }
        }
    }
}
