//! Windows-specific Steam detection and operations.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use winreg::enums::*;
use winreg::RegKey;

use crate::{Result, SteamTrainError};

pub fn steam_root() -> Result<PathBuf> {
    // Try HKCU first
    if let Ok(path) = reg_get_string(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath") {
        let steamapps = PathBuf::from(&path).join("steamapps");
        if steamapps.exists() {
            return Ok(PathBuf::from(path));
        }
    }

    // Try HKLM (WOW6432Node for 32-bit apps on 64-bit Windows)
    if let Ok(path) = reg_get_string(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\WOW6432Node\Valve\Steam",
        "InstallPath",
    ) {
        let steamapps = PathBuf::from(&path).join("steamapps");
        if steamapps.exists() {
            return Ok(PathBuf::from(path));
        }
    }

    // Fallbacks
    let candidates = [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"];

    for candidate in &candidates {
        let steamapps = PathBuf::from(candidate).join("steamapps");
        if steamapps.exists() {
            return Ok(PathBuf::from(*candidate));
        }
    }

    Err(SteamTrainError::SteamRootNotFound)
}

fn reg_get_string(root: winreg::HKEY, path: &str, name: &str) -> Result<String> {
    let key = RegKey::predef(root)
        .open_subkey(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;

    let value: String = key
        .get_value(name)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;

    Ok(value)
}

/// Get allocated (compressed) file size on Windows.
pub fn file_allocated_size(path: &Path) -> Result<i64> {
    use winapi::um::fileapi::GetCompressedFileSizeW;

    const INVALID_FILE_SIZE: u32 = 0xFFFFFFFF;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut high: u32 = 0;
    let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };

    if low == INVALID_FILE_SIZE {
        // Fallback to regular file size
        let metadata = std::fs::metadata(path)?;
        return Ok(metadata.len() as i64);
    }

    Ok(((high as i64) << 32) | (low as i64))
}

pub fn launch_app_id_platform(app_id: &str, extra_args: &[&str]) -> Result<Child> {
    let root = steam_root()?;

    let mut exe = root.join("steam.exe");
    if !exe.exists() {
        exe = root.join("Steam.exe");
    }

    let mut args = vec!["-applaunch", app_id];
    args.extend(extra_args);

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    Command::new(exe)
        .args(&args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(Into::into)
}

pub fn uninstall_via_steam(app_id: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let uri = format!("steam://uninstall/{}", app_id);

    Command::new("cmd")
        .args(&["/c", "start", "", &uri])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?
        .wait()?;

    Ok(())
}

pub fn open_install_path(path: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    Command::new("explorer")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    Ok(())
}

/// Get total and free disk space for a path.
/// Returns (total_bytes, free_bytes).
pub fn disk_space(path: &Path) -> Result<(u64, u64)> {
    use winapi::um::fileapi::GetDiskFreeSpaceExW;
    use winapi::um::winnt::ULARGE_INTEGER;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes_available: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
    let mut total_bytes: ULARGE_INTEGER = unsafe { std::mem::zeroed() };
    let mut total_free_bytes: ULARGE_INTEGER = unsafe { std::mem::zeroed() };

    let ret = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_bytes_available,
            &mut total_bytes,
            &mut total_free_bytes,
        )
    };

    if ret == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let total = unsafe { *total_bytes.QuadPart() };
    let free = unsafe { *total_free_bytes.QuadPart() };

    Ok((total, free))
}
