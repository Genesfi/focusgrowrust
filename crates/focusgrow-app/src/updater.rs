use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Deserialize;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

pub const WM_APP_UPDATE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 25;
pub const VERSION_CHECK_URL: &str =
    "https://raw.githubusercontent.com/Genesfi/focusgrowrust/main/version.json";

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteVersionPayload {
    pub version: String,
    pub installer_url: String,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        current: String,
    },
    Available {
        current: String,
        latest: String,
        installer_url: String,
        changelog: String,
    },
    Downloading {
        latest: String,
    },
    ReadyToInstall {
        installer_path: PathBuf,
        latest: String,
    },
    Error(String),
}

pub struct AppUpdater {
    status: Arc<Mutex<UpdateStatus>>,
    is_busy: Arc<AtomicBool>,
    notify_hwnd: HWND,
}

impl AppUpdater {
    pub fn new(notify_hwnd: HWND) -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            is_busy: Arc::new(AtomicBool::new(false)),
            notify_hwnd,
        }
    }

    pub fn get_status(&self) -> UpdateStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or(UpdateStatus::Idle)
    }

    /// Triggers an asynchronous version check against GitHub Raw.
    pub fn check_for_updates(&self) {
        if self.is_busy.swap(true, Ordering::SeqCst) {
            return; // Already checking or downloading
        }

        let status_arc = Arc::clone(&self.status);
        let busy_arc = Arc::clone(&self.is_busy);
        let hwnd_raw = self.notify_hwnd.0 as isize;

        if let Ok(mut s) = status_arc.lock() {
            *s = UpdateStatus::Checking;
        }
        notify_ui(hwnd_raw);

        thread::spawn(move || {
            let current = env!("CARGO_PKG_VERSION").to_string();
            let result = fetch_version_payload(VERSION_CHECK_URL);

            let new_status = match result {
                Ok(payload) => {
                    if is_newer(&payload.version, &current) {
                        UpdateStatus::Available {
                            current,
                            latest: payload.version,
                            installer_url: payload.installer_url,
                            changelog: payload.changelog.unwrap_or_else(|| "New performance improvements and features".to_string()),
                        }
                    } else {
                        UpdateStatus::UpToDate { current }
                    }
                }
                Err(err) => UpdateStatus::Error(format!("Check failed: {}", err)),
            };

            if let Ok(mut s) = status_arc.lock() {
                *s = new_status;
            }
            busy_arc.store(false, Ordering::SeqCst);
            notify_ui(hwnd_raw);
        });
    }

    /// Triggers an asynchronous download of the installer package.
    pub fn start_download(&self, installer_url: String, latest: String) {
        if self.is_busy.swap(true, Ordering::SeqCst) {
            return;
        }

        let status_arc = Arc::clone(&self.status);
        let busy_arc = Arc::clone(&self.is_busy);
        let hwnd_raw = self.notify_hwnd.0 as isize;

        if let Ok(mut s) = status_arc.lock() {
            *s = UpdateStatus::Downloading { latest: latest.clone() };
        }
        notify_ui(hwnd_raw);

        thread::spawn(move || {
            let temp_dest = std::env::temp_dir().join(format!("FocusGrow_Setup_v{}.exe", latest));
            let dl_res = download_file(&installer_url, &temp_dest);

            let new_status = match dl_res {
                Ok(()) => UpdateStatus::ReadyToInstall {
                    installer_path: temp_dest,
                    latest,
                },
                Err(e) => UpdateStatus::Error(format!("Download failed: {}", e)),
            };

            if let Ok(mut s) = status_arc.lock() {
                *s = new_status;
            }
            busy_arc.store(false, Ordering::SeqCst);
            notify_ui(hwnd_raw);
        });
    }

    /// Launches the downloaded installer and terminates the current instance cleanly.
    pub fn launch_installer(installer_path: &Path) {
        use std::process::Command;
        let _ = Command::new(installer_path)
            .arg("/FORCECLOSEAPPLICATIONS")
            .spawn();
        std::process::exit(0);
    }
}

fn notify_ui(hwnd_raw: isize) {
    if hwnd_raw != 0 {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(hwnd_raw as *mut _)),
                WM_APP_UPDATE,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

fn fetch_version_payload(url: &str) -> Result<RemoteVersionPayload, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("curl.exe")
        .args(&[
            "-s",
            "-L",
            "--fail",
            "--max-time",
            "6",
            "-H",
            "Accept: application/json",
            "-A",
            "FocusGrow-Updater/1.0",
            url,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("curl execution error: {}", e))?;

    if !output.status.success() {
        return Err("No version.json published on GitHub yet (404 Not Found)".to_string());
    }

    let body = String::from_utf8(output.stdout)
        .map_err(|_| "Invalid response encoding from server".to_string())?;

    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed.starts_with("404") {
        return Err("No version.json published on GitHub yet (404 Not Found)".to_string());
    }

    serde_json::from_str::<RemoteVersionPayload>(trimmed)
        .map_err(|e| format!("Invalid version format: {}", e))
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let dest_str = dest.to_string_lossy();
    let output = std::process::Command::new("curl.exe")
        .args(&[
            "-s",
            "-L",
            "--max-time",
            "300",
            "-A",
            "FocusGrow-Updater/1.0",
            "-o",
            &dest_str,
            url,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Download execution error: {}", e))?;

    if !output.status.success() {
        return Err(format!("Download failed with status: {:?}", output.status.code()));
    }

    if let Ok(meta) = std::fs::metadata(dest) {
        if meta.len() > 100_000 {
            return Ok(());
        }
    }

    Err("Downloaded file is incomplete or corrupted".to_string())
}

pub fn parse_version(v: &str) -> (u32, u32, u32) {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = clean.split('.').filter_map(|s| s.parse::<u32>().ok());
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

pub fn is_newer(remote: &str, current: &str) -> bool {
    parse_version(remote) > parse_version(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_works() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("v1.1.0", "1.0.5"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("v1.0.0", "1.0.0"));
    }
}
