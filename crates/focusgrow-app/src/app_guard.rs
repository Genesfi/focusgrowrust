use focusgrow_core::AppConfig;
use focusgrow_gfx::{decode_hicon_to_frame, DecodedFrame};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::core::{BOOL, HSTRING, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE, WPARAM};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Shell::{
    ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
    SHGFI_USEFILEATTRIBUTES,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, EnumWindows, GetClassLongPtrW, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SendMessageTimeoutW,
    EVENT_SYSTEM_FOREGROUND, GCLP_HICON, GCLP_HICONSM, GWL_EXSTYLE, HICON, ICON_BIG, ICON_SMALL,
    SMTO_ABORTIFHUNG, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_CLOSE, WM_GETICON,
    WS_EX_TOOLWINDOW,
};

/// Posted to `main_hwnd` whenever the OS foreground window changes (`lparam` = foreground HWND).
pub const WM_FOREGROUND_APP: u32 = WM_APP + 12;

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

#[derive(Debug, Clone, Default)]
pub struct ForegroundAppInfo {
    pub hwnd_raw: isize,
    #[allow(dead_code)]
    pub pid: u32,
    pub exe_name: String,
    pub exe_path: String,
    pub title: String,
}

/// Zero-polling OS foreground hook using `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`.
pub struct ForegroundHook {
    hook: HWINEVENTHOOK,
}

impl ForegroundHook {
    pub fn install(notify_hwnd: HWND) -> Option<Self> {
        TARGET_HWND.store(notify_hwnd.0 as isize, Ordering::SeqCst);
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.is_invalid() {
            None
        } else {
            Some(Self { hook })
        }
    }
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        if !self.hook.is_invalid() {
            unsafe {
                let _ = UnhookWinEvent(self.hook);
            }
        }
        TARGET_HWND.store(0, Ordering::SeqCst);
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    let target = TARGET_HWND.load(Ordering::Relaxed);
    if target != 0 && !hwnd.0.is_null() {
        let _ = PostMessageW(
            Some(HWND(target as *mut _)),
            WM_FOREGROUND_APP,
            WPARAM(0),
            LPARAM(hwnd.0 as isize),
        );
    }
}

/// Reads the executable name (`discord.exe`, `chrome.exe`, etc.) and window title for `hwnd`.
pub fn inspect_window(hwnd: HWND) -> Option<ForegroundAppInfo> {
    if hwnd.0.is_null() {
        return None;
    }

    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return None;
    }

    let mut title_buf = [0u16; 256];
    let title_len = unsafe { GetWindowTextW(hwnd, &mut title_buf) }.max(0) as usize;
    let title = String::from_utf16_lossy(&title_buf[..title_len]).trim().to_string();

    let (exe_name, exe_path) = query_process_exe(pid).unwrap_or_default();
    if exe_name.is_empty() && title.is_empty() {
        return None;
    }

    Some(ForegroundAppInfo {
        hwnd_raw: hwnd.0 as isize,
        pid,
        exe_name,
        exe_path,
        title,
    })
}

fn query_process_exe(pid: u32) -> Option<(String, String)> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if !ok || len == 0 {
            return None;
        }
        let full = String::from_utf16_lossy(&buf[..len as usize]);
        let name = full
            .rsplit(|c| c == '\\' || c == '/')
            .next()
            .unwrap_or("")
            .to_string();
        (!name.is_empty()).then_some((name, full))
    }
}

/// Enumerates visible top-level application windows (matching C++ `AppDetector::GetRunningApps`).
pub fn enumerate_running_apps() -> Vec<ForegroundAppInfo> {
    struct EnumCtx {
        list: Vec<ForegroundAppInfo>,
        seen: HashSet<(String, String)>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }
        if GetWindowTextLengthW(hwnd) <= 0 {
            return TRUE;
        }
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return TRUE;
        }
        let Some(info) = inspect_window(hwnd) else {
            return TRUE;
        };
        let own_pid = std::process::id();
        if info.title.is_empty()
            || info.pid == own_pid
            || info.title == "Program Manager"
            || info.exe_name.is_empty()
            || info.exe_name.eq_ignore_ascii_case("focusgrow-app.exe")
            || info.exe_name.eq_ignore_ascii_case("focusgrow_app.exe")
            || info.exe_name.eq_ignore_ascii_case("focusgrow.exe")
            || info.exe_name.eq_ignore_ascii_case("msedgewebview2.exe")
        {
            return TRUE;
        }

        let ctx = &mut *(lparam.0 as *mut EnumCtx);
        let key = (info.exe_name.to_ascii_lowercase(), info.title.clone());
        if ctx.seen.insert(key) {
            ctx.list.push(info);
        }
        TRUE
    }

    let mut ctx = EnumCtx {
        list: Vec::new(),
        seen: HashSet::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut EnumCtx as isize));
    }
    ctx.list
}

/// Extracts a clean premultiplied BGRA8 icon frame directly from `exe_path` or `hwnd`.
pub fn extract_app_icon_frame(hwnd_raw: isize, exe_path: &str) -> Option<DecodedFrame> {
    let mut owned_icon: Option<HICON> = None;
    let mut borrowed_icon: Option<HICON> = None;

    unsafe {
        if !exe_path.is_empty() {
            let wide = HSTRING::from(exe_path);
            let mut large = HICON::default();
            let mut small = HICON::default();
            let count = ExtractIconExW(&wide, 0, Some(&mut large), Some(&mut small), 1);
            if count > 0 {
                if !large.0.is_null() {
                    owned_icon = Some(large);
                    if !small.0.is_null() && small != large {
                        let _ = DestroyIcon(small);
                    }
                } else if !small.0.is_null() {
                    owned_icon = Some(small);
                }
            }

            if owned_icon.is_none() {
                let mut shfi = SHFILEINFOW::default();
                let res = SHGetFileInfoW(
                    &wide,
                    windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                    Some(&mut shfi),
                    std::mem::size_of::<SHFILEINFOW>() as u32,
                    SHGFI_ICON | SHGFI_LARGEICON,
                );
                if res != 0 && !shfi.hIcon.0.is_null() {
                    owned_icon = Some(shfi.hIcon);
                } else {
                    let mut shfi_attr = SHFILEINFOW::default();
                    let res_attr = SHGetFileInfoW(
                        &wide,
                        windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                        Some(&mut shfi_attr),
                        std::mem::size_of::<SHFILEINFOW>() as u32,
                        SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
                    );
                    if res_attr != 0 && !shfi_attr.hIcon.0.is_null() {
                        owned_icon = Some(shfi_attr.hIcon);
                    }
                }
            }
        }

        if owned_icon.is_none() && hwnd_raw != 0 {
            let hwnd = HWND(hwnd_raw as *mut _);
            let mut res_ptr = 0usize;
            if SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(ICON_BIG as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                40,
                Some(&mut res_ptr),
            )
            .0 != 0
                && res_ptr != 0
            {
                borrowed_icon = Some(HICON(res_ptr as *mut _));
            } else if SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(ICON_SMALL as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                40,
                Some(&mut res_ptr),
            )
            .0 != 0
                && res_ptr != 0
            {
                borrowed_icon = Some(HICON(res_ptr as *mut _));
            } else {
                let c_big = GetClassLongPtrW(hwnd, GCLP_HICON) as isize;
                if c_big != 0 {
                    borrowed_icon = Some(HICON(c_big as *mut _));
                } else {
                    let c_sm = GetClassLongPtrW(hwnd, GCLP_HICONSM) as isize;
                    if c_sm != 0 {
                        borrowed_icon = Some(HICON(c_sm as *mut _));
                    }
                }
            }
        }
    }

    let hicon = owned_icon.or(borrowed_icon)?;
    let decoded = decode_hicon_to_frame(hicon.0 as isize).ok();
    if let Some(ico) = owned_icon {
        unsafe {
            let _ = DestroyIcon(ico);
        }
    }
    decoded
}

/// Returns how many whole seconds have elapsed since the last keyboard/mouse input.
pub fn system_idle_seconds() -> u32 {
    let mut lii = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    unsafe {
        if !GetLastInputInfo(&mut lii).as_bool() {
            return 0;
        }
        let now_ms = GetTickCount();
        now_ms.wrapping_sub(lii.dwTime) / 1000
    }
}

/// Requests a distracting window to close and forcefully terminates the process if needed
/// (crucial for game launchers and DirectX games that ignore or minimize on `WM_CLOSE`).
pub fn close_window_gracefully(hwnd_raw: isize) -> bool {
    if hwnd_raw == 0 {
        return false;
    }
    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let wm_ok = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_ok();

        if pid > 0 && pid != GetCurrentProcessId() {
            if let Ok(h_proc) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let term_ok = TerminateProcess(h_proc, 0).is_ok();
                let _ = CloseHandle(h_proc);
                if term_ok {
                    return true;
                }
            }
        }
        wm_ok
    }
}

/// Saves a decoded BGRA8 icon frame to disk cache: `%LOCALAPPDATA%\FocusGrow\icon_cache\<exe_low>.bin`.
pub fn save_icon_cache(exe_low: &str, frame: &DecodedFrame) {
    if exe_low.is_empty() || frame.width == 0 || frame.height == 0 || frame.pixels_pbgra.is_empty() {
        return;
    }
    let dir = AppConfig::icon_cache_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{exe_low}.bin"));
    let mut data = Vec::with_capacity(8 + frame.pixels_pbgra.len());
    data.extend_from_slice(&frame.width.to_le_bytes());
    data.extend_from_slice(&frame.height.to_le_bytes());
    data.extend_from_slice(&frame.pixels_pbgra);
    let _ = fs::write(path, data);
}

/// Loads a decoded BGRA8 icon frame from disk cache.
pub fn load_icon_cache(exe_low: &str) -> Option<DecodedFrame> {
    let path = AppConfig::icon_cache_dir().join(format!("{exe_low}.bin"));
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 8 {
        return None;
    }
    let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let expected = (width as usize) * (height as usize) * 4;
    if bytes.len() != 8 + expected {
        return None;
    }
    Some(DecodedFrame {
        width,
        height,
        delay_ms: 0,
        pixels_pbgra: bytes[8..].to_vec(),
    })
}

/// Loads all cached icon frames from disk on application startup.
pub fn load_all_cached_icons() -> Vec<(String, DecodedFrame)> {
    let dir = AppConfig::icon_cache_dir();
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("bin") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(frame) = load_icon_cache(stem) {
                    list.push((stem.to_ascii_lowercase(), frame));
                }
            }
        }
    }
    list
}

/// Remembers a known executable's full on-disk path (e.g. `Spotify.exe` -> `C:\Users\...\Spotify.exe`).
pub fn record_known_app_path(exe_name: &str, exe_path: &str) {
    if exe_name.is_empty() || exe_path.is_empty() {
        return;
    }
    let p = AppConfig::known_apps_path();
    let mut map: HashMap<String, String> = fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let low = exe_name.to_ascii_lowercase();
    if map.get(&low).map(|s| s.as_str()) != Some(exe_path) {
        map.insert(low, exe_path.to_string());
        if let Ok(json) = serde_json::to_string_pretty(&map) {
            let _ = fs::write(p, json);
        }
    }
}

/// Resolves an app's on-disk executable path even when the app is NOT currently running.
pub fn resolve_app_exe_path(exe_name: &str) -> Option<String> {
    let low = exe_name.to_ascii_lowercase();
    // 1. Check known_app_paths.json
    let p = AppConfig::known_apps_path();
    if let Ok(raw) = fs::read_to_string(&p) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&raw) {
            if let Some(path) = map.get(&low) {
                if Path::new(path).exists() {
                    return Some(path.clone());
                }
            }
        }
    }

    // 2. Check Windows Registry App Paths: HKCU & HKLM \SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\<exe_name>
    for hkey in [
        windows::Win32::System::Registry::HKEY_CURRENT_USER,
        windows::Win32::System::Registry::HKEY_LOCAL_MACHINE,
    ] {
        let subkey = format!("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{}", exe_name);
        let wide_sub = HSTRING::from(subkey);
        let mut key_handle = windows::Win32::System::Registry::HKEY::default();
        unsafe {
            if windows::Win32::System::Registry::RegOpenKeyExW(
                hkey,
                &wide_sub,
                None,
                windows::Win32::System::Registry::KEY_READ,
                &mut key_handle,
            )
            .is_ok()
            {
                let mut buf = [0u16; 512];
                let mut cb_size = (buf.len() * 2) as u32;
                let res = windows::Win32::System::Registry::RegQueryValueExW(
                    key_handle,
                    None,
                    None,
                    None,
                    Some(buf.as_mut_ptr() as *mut _),
                    Some(&mut cb_size),
                );
                let _ = windows::Win32::System::Registry::RegCloseKey(key_handle);
                if res.is_ok() {
                    let path_str = String::from_utf16_lossy(&buf[..(cb_size as usize / 2).saturating_sub(1)])
                        .trim_matches('"')
                        .to_string();
                    if Path::new(&path_str).exists() {
                        return Some(path_str);
                    }
                }
            }
        }
    }

    // 3. Check common user install paths (e.g. WindowsApps, AppData, Office, Program Files)
    let mut candidates = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = Path::new(&local);
        candidates.push(base.join("Microsoft").join("WindowsApps").join(exe_name));
        candidates.push(base.join("Programs").join(exe_name));
        candidates.push(base.join("Discord").join(exe_name));
        candidates.push(base.join("Spotify").join(exe_name));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let base = Path::new(&appdata);
        candidates.push(base.join("Spotify").join(exe_name));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        let base = Path::new(&pf);
        candidates.push(base.join("Microsoft Office").join("root").join("Office16").join(exe_name));
        candidates.push(base.join("Microsoft Office").join("Office16").join(exe_name));
        candidates.push(base.join(exe_name));
    }
    if let Ok(pfx86) = std::env::var("ProgramFiles(x86)") {
        let base = Path::new(&pfx86);
        candidates.push(base.join("Microsoft Office").join("root").join("Office16").join(exe_name));
        candidates.push(base.join("Microsoft Office").join("Office16").join(exe_name));
        candidates.push(base.join(exe_name));
    }

    for c in candidates {
        if c.exists() {
            return Some(c.to_string_lossy().to_string());
        }
    }
    None
}
