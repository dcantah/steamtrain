mod vdf;

#[cfg(windows)]
mod platform_windows;
#[cfg(target_os = "macos")]
mod platform_darwin;

/// Platform-specific functions exposed for TUI use.
pub mod platform {
    #[cfg(windows)]
    pub use crate::platform_windows::disk_space;
    #[cfg(target_os = "macos")]
    pub use crate::platform_darwin::disk_space;
}

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub use vdf::{VdfError, VdfObject};

#[cfg(windows)]
use platform_windows::{file_allocated_size, launch_app_id_platform, steam_root};
#[cfg(target_os = "macos")]
use platform_darwin::{file_allocated_size, launch_app_id_platform, steam_root};

#[derive(Debug)]
pub enum SteamTrainError {
    SteamRootNotFound,
    SteamAppsNotFound(PathBuf),
    Io(std::io::Error),
    Vdf(VdfError),
    MissingField(&'static str),
    AppNotFound(String),
}

impl fmt::Display for SteamTrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SteamTrainError::SteamRootNotFound => write!(f, "Steam root not found"),
            SteamTrainError::SteamAppsNotFound(p) => {
                write!(f, "steamapps not found at {}", p.display())
            }
            SteamTrainError::Io(e) => write!(f, "IO error: {}", e),
            SteamTrainError::Vdf(e) => write!(f, "VDF parse error: {}", e),
            SteamTrainError::MissingField(field) => write!(f, "Missing required field: {}", field),
            SteamTrainError::AppNotFound(id) => write!(f, "App not found: {}", id),
        }
    }
}

impl Error for SteamTrainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SteamTrainError::Io(e) => Some(e),
            SteamTrainError::Vdf(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SteamTrainError {
    fn from(e: std::io::Error) -> Self {
        SteamTrainError::Io(e)
    }
}

impl From<VdfError> for SteamTrainError {
    fn from(e: VdfError) -> Self {
        SteamTrainError::Vdf(e)
    }
}

pub type Result<T> = std::result::Result<T, SteamTrainError>;

/// Represents the Steam installation directory.
#[derive(Debug, Clone)]
pub struct Install {
    pub root: PathBuf,
    pub steamapps_path: PathBuf,
    pub libraries: Vec<Library>,
    /// Playtime data for all apps (app_id -> minutes).
    playtime: HashMap<String, u64>,
    /// Currently logged-in user's display name.
    pub current_user: Option<String>,
}

/// Represents a single library instance where Steam games are stored.
#[derive(Debug, Clone)]
pub struct Library {
    pub path: PathBuf,
    steamapps_path: PathBuf,
}

/// Represents a game.
#[derive(Debug, Clone)]
pub struct App {
    pub app_id: String,
    pub name: String,
    pub install_dir: String,
    pub library: Library,
    pub game_path: PathBuf,
    pub size_on_disk_manifest: i64,
    pub last_updated: Option<SystemTime>,
    pub last_played: Option<SystemTime>,
    /// Playtime in minutes (from localconfig.vdf).
    pub playtime_minutes: Option<u64>,
}

/// AppIDs for non-game entries that should be filtered out.
const EXCLUDED_APP_IDS: &[&str] = &[
    "228980", // Steamworks Common Redistributables
];

/// Steam64 ID base - subtract this to get the userdata folder ID.
const STEAM64_BASE: u64 = 76561197960265728;

/// Info about the currently logged-in Steam user.
struct CurrentUser {
    folder_id: String,
    persona_name: String,
}

/// Finds the currently logged-in Steam user by parsing loginusers.vdf.
fn find_current_user(steam_root: &Path) -> Option<CurrentUser> {
    let loginusers_path = steam_root.join("config").join("loginusers.vdf");
    let data = std::fs::read_to_string(&loginusers_path).ok()?;
    let root = vdf::parse_vdf(&data).ok()?;
    let users = root.require_object("users").ok()?;

    for (steam64_str, value) in users.raw().iter() {
        if let Some(user_obj) = value.as_object() {
            if let Some(most_recent) = user_obj.get("MostRecent").and_then(|v| v.as_str()) {
                if most_recent == "1" {
                    // Convert Steam64 ID to folder ID
                    if let Ok(steam64) = steam64_str.parse::<u64>() {
                        let folder_id = steam64.saturating_sub(STEAM64_BASE);
                        let persona_name = user_obj
                            .get("PersonaName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        return Some(CurrentUser {
                            folder_id: folder_id.to_string(),
                            persona_name,
                        });
                    }
                }
            }
        }
    }
    None
}

/// Loads playtime data for all apps from localconfig.vdf.
/// Returns a map of app_id -> playtime in minutes.
fn load_playtime_data(steam_root: &Path, user_folder: &str) -> HashMap<String, u64> {
    let mut result = HashMap::new();

    let localconfig_path = steam_root
        .join("userdata")
        .join(user_folder)
        .join("config")
        .join("localconfig.vdf");

    let data = match std::fs::read_to_string(&localconfig_path) {
        Ok(d) => d,
        Err(_) => return result,
    };

    let root = match vdf::parse_vdf(&data) {
        Ok(r) => r,
        Err(_) => return result,
    };

    // Navigate: UserLocalConfigStore -> Software -> valve -> Steam -> apps
    let apps = root
        .require_object("UserLocalConfigStore")
        .and_then(|u| u.require_object("Software"))
        .and_then(|s| s.require_object("valve"))
        .and_then(|v| v.require_object("Steam"))
        .and_then(|st| st.require_object("apps"));

    if let Ok(apps_obj) = apps {
        for (app_id, value) in apps_obj.raw().iter() {
            if let Some(app_obj) = value.as_object() {
                if let Some(playtime_str) = app_obj.get("Playtime").and_then(|v| v.as_str()) {
                    if let Ok(minutes) = playtime_str.parse::<u64>() {
                        result.insert(app_id.clone(), minutes);
                    }
                }
            }
        }
    }

    result
}

impl Install {
    /// Locates the primary Steam install and loads all libraries.
    pub fn find() -> Result<Self> {
        let root = steam_root()?;
        let steamapps = root.join("steamapps");

        if !steamapps.exists() {
            return Err(SteamTrainError::SteamAppsNotFound(steamapps));
        }

        let libs = read_libraries(&steamapps)?;

        // Deduplicate by steamapps path
        let mut seen = HashSet::new();
        let mut unique: Vec<Library> = libs
            .into_iter()
            .filter(|l| seen.insert(l.steamapps_path.clone()))
            .collect();

        unique.sort_by(|a, b| a.steamapps_path.cmp(&b.steamapps_path));

        let current_user_info = find_current_user(&root);
        let playtime = current_user_info
            .as_ref()
            .map(|u| load_playtime_data(&root, &u.folder_id))
            .unwrap_or_default();
        let current_user = current_user_info.map(|u| u.persona_name);

        Ok(Install {
            root,
            steamapps_path: steamapps,
            libraries: unique,
            playtime,
            current_user,
        })
    }

    /// Scans all libraries for appmanifest_*.acf and returns the parsed apps.
    pub fn apps(&self) -> Result<Vec<App>> {
        let mut out = Vec::new();
        for lib in &self.libraries {
            let entries = lib.apps_with_playtime(&self.playtime)?;
            out.extend(entries);
        }
        Ok(out)
    }

    /// Returns the playtime data map.
    pub fn playtime(&self) -> &HashMap<String, u64> {
        &self.playtime
    }

    /// Launches the application by invoking a steam URI.
    pub fn launch_app_id(&self, app_id: &str, args: &[&str]) -> Result<std::process::Child> {
        launch_app_id_platform(app_id, args)
    }
}

impl Library {
    /// Returns the path to the steamapps directory for the library.
    pub fn steamapps_path(&self) -> &Path {
        &self.steamapps_path
    }

    /// Attempts to find any games by the name given. Case-insensitive substring match.
    pub fn search_by_name(&self, name: &str) -> Result<Vec<App>> {
        let apps = self.apps()?;
        let name_lower = name.to_lowercase();
        Ok(apps
            .into_iter()
            .filter(|app| app.name.to_lowercase().contains(&name_lower))
            .collect())
    }

    /// Attempts to find a game in the library by ID.
    pub fn search_by_id(&self, id: &str) -> Result<App> {
        let apps = self.apps()?;
        apps.into_iter()
            .find(|app| app.app_id == id)
            .ok_or_else(|| SteamTrainError::AppNotFound(id.to_string()))
    }

    /// Scans for appmanifest_*.acf and returns the parsed apps (without playtime).
    pub fn apps(&self) -> Result<Vec<App>> {
        self.apps_with_playtime(&HashMap::new())
    }

    /// Scans for appmanifest_*.acf and returns the parsed apps with playtime data.
    pub fn apps_with_playtime(&self, playtime: &HashMap<String, u64>) -> Result<Vec<App>> {
        let mut out = Vec::new();

        let entries = std::fs::read_dir(&self.steamapps_path)?;

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if !name_str.starts_with("appmanifest_") || !name_str.ends_with(".acf") {
                continue;
            }

            let manifest_path = self.steamapps_path.join(&name);
            let mut app = parse_app_manifest(&manifest_path, self.clone())?;

            if EXCLUDED_APP_IDS.contains(&app.app_id.as_str()) {
                continue;
            }

            // Attach playtime if available
            app.playtime_minutes = playtime.get(&app.app_id).copied();

            out.push(app);
        }

        Ok(out)
    }
}

impl App {
    /// Launches the app via the registered launcher.
    pub fn launch(&self, extra_args: &[&str]) -> Result<std::process::Child> {
        launch_app_id_platform(&self.app_id, extra_args)
    }

    /// Walks GamePath and sums allocated bytes on disk.
    pub fn compute_disk_size(&self) -> Result<i64> {
        dir_allocated_size(&self.game_path)
    }

    #[cfg(windows)]
    pub fn uninstall_via_steam(&self) -> Result<()> {
        platform_windows::uninstall_via_steam(&self.app_id)
    }

    #[cfg(windows)]
    pub fn open_install_path(&self) -> Result<()> {
        platform_windows::open_install_path(&self.game_path)
    }

    #[cfg(target_os = "macos")]
    pub fn uninstall_via_steam(&self) -> Result<()> {
        platform_darwin::uninstall_via_steam(&self.app_id)
    }

    #[cfg(target_os = "macos")]
    pub fn open_install_path(&self) -> Result<()> {
        platform_darwin::open_install_path(&self.game_path)
    }
}

fn parse_app_manifest(path: &Path, lib: Library) -> Result<App> {
    let data = std::fs::read_to_string(path)?;
    let root = vdf::parse_vdf(&data)?;

    let app_state = root.require_object("AppState")?;

    let app_id = app_state.require_string("appid")?;
    let install_dir = app_state.require_string("installdir")?;
    let name = app_state.require_string("name")?;

    if app_id.is_empty() || install_dir.is_empty() || name.is_empty() {
        return Err(SteamTrainError::MissingField("appid, installdir, or name"));
    }

    let size_on_disk_manifest = app_state.require_integer("SizeOnDisk").unwrap_or(0);

    let last_updated = app_state
        .require_integer("LastUpdated")
        .ok()
        .filter(|&v| v > 0)
        .map(|v| UNIX_EPOCH + Duration::from_secs(v as u64));

    let last_played = app_state
        .require_integer("LastPlayed")
        .ok()
        .filter(|&v| v > 0)
        .map(|v| UNIX_EPOCH + Duration::from_secs(v as u64));

    let game_path = lib.steamapps_path.join("common").join(&install_dir);

    Ok(App {
        app_id,
        name,
        install_dir,
        library: lib,
        game_path,
        size_on_disk_manifest,
        last_updated,
        last_played,
        playtime_minutes: None,
    })
}

fn read_libraries(primary_steamapps: &Path) -> Result<Vec<Library>> {
    let lib_vdf = primary_steamapps.join("libraryfolders.vdf");

    let data = match std::fs::read_to_string(&lib_vdf) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // If not found, return just the primary
            let parent = primary_steamapps.parent().unwrap_or(primary_steamapps);
            return Ok(vec![Library {
                path: parent.to_path_buf(),
                steamapps_path: primary_steamapps.to_path_buf(),
            }]);
        }
        Err(e) => return Err(e.into()),
    };

    let root = vdf::parse_vdf(&data)?;

    let lf = match root.require_object("libraryfolders") {
        Ok(lf) => lf,
        Err(_) => {
            let parent = primary_steamapps.parent().unwrap_or(primary_steamapps);
            return Ok(vec![Library {
                path: parent.to_path_buf(),
                steamapps_path: primary_steamapps.to_path_buf(),
            }]);
        }
    };

    let mut libs = Vec::new();

    for (_key, value) in lf.raw().iter() {
        if let Some(entry) = value.as_object() {
            if let Some(path_str) = entry.get("path").and_then(|v| v.as_str()) {
                let sp = path_str.trim();
                let steamapps = PathBuf::from(sp).join("steamapps");
                if steamapps.exists() {
                    libs.push(Library {
                        path: PathBuf::from(sp),
                        steamapps_path: steamapps,
                    });
                }
            }
        }
    }

    Ok(libs)
}

fn dir_allocated_size(root: &Path) -> Result<i64> {
    let mut total: i64 = 0;

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            if let Ok(alloc) = file_allocated_size(entry.path()) {
                if alloc > 0 {
                    total += alloc;
                }
            }
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excluded_apps() {
        assert!(EXCLUDED_APP_IDS.contains(&"228980"));
    }
}
