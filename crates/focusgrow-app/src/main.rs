#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_guard;
mod file_dialog;
mod health_hud;
mod http_bridge;
mod overlay;
mod peeking_vinyl;
mod qr;
mod tray;
mod ui;
mod ytmpx;

use anyhow::Result;
use focusgrow_audio::{next_sound_style, AudioEngine, AMBIENT_PRESETS, AMBIENT_TRACKS};
use app_guard::{
    close_window_gracefully, enumerate_running_apps, extract_app_icon_frame, inspect_window,
    system_idle_seconds, ForegroundAppInfo, ForegroundHook, WM_FOREGROUND_APP,
};
use focusgrow_core::gif_store::{
    import_gif_into_store, remove_recent_gif, resolve_active_gif_path, select_recent_gif,
};
use focusgrow_core::{
    auto_migrate_webview2, AmbientMode, AppConfig, DailyStats, FocusTimer, GifDisplayMode,
    GuardState, NamedPrayer, NeutralOverstayEvent, NeutralOverstayTracker, PersistedSession,
    PipStyle, PrayerManager, SessionPhase, VinylSide,
};
use focusgrow_gfx::{
    decode_image_bytes, decode_image_file_progressive, extract_album_accent, upload_frame_bitmap,
    DCompSurface, DecodedAnimation, DecodedFrame, GpuAnimation,
};
use http_bridge::{BridgeCommand, HttpBridgeServer, WM_BRIDGE_EVENT};
use health_hud::HealthHudWindow;
use overlay::{OverlayAction, OverlayMode, OverlayWindow, WM_OVERLAY_EVENT};
use peeking_vinyl::{PeekingVinylWindow, WM_VINYL_COMMAND};
use qr::{detect_lan_ipv4, encode_qr_v3_m, QrMatrix};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tray::{
    TrayIcon, IDM_TRAY_EXIT, IDM_TRAY_PAUSE, IDM_TRAY_RESTORE, IDM_TRAY_STOP,
    IDM_TRAY_TOGGLE_PIP, WM_TRAYICON,
};
use ui::{
    blocked_apps_capacity, build_share_report_text, compute_layout, export_share_card_png,
    period_app_breakdown, render_ui, AppFilter, AppSort, CityEntry, ControlId, GuardTab,
    LayoutQuery, NumericEditField, RenderView, RunningAppView, SettingsTab, StatsAppRow,
    StatsPeriod, ACCENT_PRESETS, CITY_PRESETS, COMMON_SITE_PRESETS, TIMER_THEMES,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, RECT, WPARAM,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap1;
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, InvalidateRect, ScreenToClient,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateDIBSection, DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    FindWindowW, GetClientRect, GetForegroundWindow, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, KillTimer, LoadCursorW, LoadImageW, PostMessageW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, CREATESTRUCTW, GWLP_USERDATA,
    GWL_EXSTYLE, HICON, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, ICONINFO, ICON_BIG, ICON_SMALL,
    IDC_ARROW, IMAGE_ICON, LR_DEFAULTSIZE, MINMAXINFO, MSG, SC_MOVE, SM_CXSCREEN,
    SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_CLOSE,
    WM_CREATE, WM_DESTROY, WM_DPICHANGED, WM_ENDSESSION, WM_ERASEBKGND, WM_EXITSIZEMOVE,
    WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST,
    WM_PAINT, WM_QUERYENDSESSION, WM_SETICON, WM_SIZE, WM_SYSCOMMAND, WM_TIMER,
    WM_WINDOWPOSCHANGED, WNDCLASSEXW, WS_EX_APPWINDOW, WS_EX_NOREDIRECTIONBITMAP,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use ytmpx::{MediaCommand, PlaybackState, TrackInfo, YtmpxBridge, YtmpxUpdate, WM_YTMPX_EVENT};

const APP_ICON_PNG: &[u8] = include_bytes!("../../../ui/icon.png");

const WM_GIF_LOADED: u32 = WM_APP + 15;
const WM_PRAYER_LOC_EVENT: u32 = WM_APP + 16;

enum PrayerLocMsg {
    GpsResult(std::result::Result<(f64, f64, i32, String), String>),
    CitySearchResult(u64, std::result::Result<Vec<CityEntry>, String>),
}

enum GifLoadMsg {
    FirstFrame(u64, PathBuf, DecodedAnimation),
    TailFrames(u64, PathBuf, Result<DecodedAnimation>),
}

const TIMER_SEC_ID: usize = 1;
const TIMER_GIF_ID: usize = 2;
const TIMER_VINYL_ID: usize = 3;
const TIMER_IDLE_HEARTBEAT_ID: usize = 4;
const TIMER_DEFERRED_INIT_ID: usize = 5;

#[derive(Debug, Clone, Copy)]
enum DragSlider {
    Opacity,
    VinylSpeed,
    VinylSize,
    AmbientMaster,
    AmbientTrack(usize),
}

struct AudioSlot(Option<AudioEngine>);

impl AudioSlot {
    fn ensure(&mut self, cfg: &focusgrow_core::AmbientConfig) -> &mut AudioEngine {
        self.0.get_or_insert_with(|| AudioEngine::start(cfg))
    }
    fn is_playing(&self) -> bool {
        self.0.as_ref().is_some_and(|a| a.is_playing())
    }
    fn is_master_muted(&self) -> bool {
        self.0.as_ref().is_some_and(|a| a.is_master_muted())
    }
    fn set_playing(&mut self, playing: bool, cfg: &focusgrow_core::AmbientConfig) {
        self.ensure(cfg).set_playing(playing, cfg);
    }
    fn toggle_playing(&mut self, cfg: &focusgrow_core::AmbientConfig) {
        self.ensure(cfg).toggle_playing(cfg);
    }
    fn set_master_muted(&mut self, muted: bool, cfg: &focusgrow_core::AmbientConfig) {
        self.ensure(cfg).set_master_muted(muted, cfg);
    }
    fn sync(&mut self, cfg: &focusgrow_core::AmbientConfig) {
        self.ensure(cfg).sync(cfg);
    }
    fn play_notification(&mut self, sound_id: &str, custom_path: Option<&str>) {
        self.0
            .get_or_insert_with(|| AudioEngine::start(&Default::default()))
            .play_notification(sound_id, custom_path);
    }
    fn play_countdown_pip(&mut self, secs_left: u8) {
        self.0
            .get_or_insert_with(|| AudioEngine::start(&Default::default()))
            .play_countdown_pip(secs_left);
    }
}

struct AppState {
    hwnd: HWND,
    surface: DCompSurface,
    config: AppConfig,
    timer: FocusTimer,
    stats: DailyStats,
    guard: GuardState,
    prayer_mgr: PrayerManager,
    prayer_date: String,
    prayer_times: Vec<NamedPrayer>,
    next_prayer_label: String,
    next_prayer_name: String,
    next_prayer_secs: u32,
    prayer_break_remaining: u32,
    active_prayer_break_name: String,
    last_prayer_break_key: String,
    last_prayer_dismissed_key: String,
    last_prayer_adv_key: String,
    warned_5m_phase: Option<(SessionPhase, u32)>,
    warned_1m_phase: Option<(SessionPhase, u32)>,
    prev_phase: SessionPhase,
    health_finish_until: Option<Instant>,
    health_hud: Option<Box<HealthHudWindow>>,
    overlay: Option<OverlayWindow>,
    http_bridge: Option<HttpBridgeServer>,
    #[allow(dead_code)]
    foreground_hook: Option<ForegroundHook>,
    active_app: Option<ForegroundAppInfo>,
    active_domain: String,
    block_countdown: Option<(isize, u8)>,
    blocked_app_active: Option<String>,
    dismissed_blocked_hwnd: Option<isize>,
    neutral_overstay: NeutralOverstayTracker,
    is_auto_paused: bool,
    gpu_anim: Option<GpuAnimation>,
    frame_idx: usize,
    cover_frame: Option<DecodedFrame>,
    cover_bitmap: Option<ID2D1Bitmap1>,
    cached_dynamic_accent: Option<String>,
    track: Option<TrackInfo>,
    playback: PlaybackState,
    ytmpx_connected: bool,
    ytmpx: Option<YtmpxBridge>,
    peeking_vinyl: Option<Box<PeekingVinylWindow>>,
    guard_tab: GuardTab,
    app_filter: AppFilter,
    app_sort: AppSort,
    search_query: String,
    search_focused: bool,
    running_apps: Vec<ForegroundAppInfo>,
    app_icons: HashMap<String, ID2D1Bitmap1>,
    app_icons_by_exe: HashMap<String, ID2D1Bitmap1>,
    app_icon_logo: Option<ID2D1Bitmap1>,
    tray: Option<TrayIcon>,
    deferred_done: bool,
    force_quit: bool,
    app_scroll: usize,
    settings_tab: Option<SettingsTab>,
    settings_scroll: f32,
    show_ambient_mixer: bool,
    show_stats_modal: bool,
    show_share_modal: bool,
    share_yesterday: bool,
    show_goal_modal: bool,
    show_prayer_modal: bool,
    prayer_search_query: String,
    prayer_search_focused: bool,
    site_input_query: String,
    site_input_focused: bool,
    site_scroll: usize,
    prayer_online_results: Option<Vec<CityEntry>>,
    prayer_search_gen: u64,
    prayer_loc_busy: bool,
    prayer_loc_status: Option<String>,
    prayer_loc_tx: mpsc::Sender<PrayerLocMsg>,
    prayer_loc_rx: mpsc::Receiver<PrayerLocMsg>,
    stats_period: StatsPeriod,
    heatmap_offset_months: usize,
    session_save_tick: u8,
    prayer_view_idx: Option<usize>,
    show_pip_gif_drawer: bool,
    hovered: Option<ControlId>,
    cursor_dip: Option<(f32, f32)>,
    dragging_slider: Option<DragSlider>,
    tracking_mouse_leave: bool,
    is_maximized: bool,
    in_card_vinyl_angle: f32,
    plant_sway_phase: f32,
    wave_phase: f32,
    marquee_offset: f32,
    completed_spin_until: Option<Instant>,
    guard_refresh_spin_until: Option<Instant>,
    last_vinyl_tick: Instant,
    audio: AudioSlot,
    status_hint: Option<String>,
    num_edit_field: Option<NumericEditField>,
    num_edit_buf: String,
    num_edit_fresh: bool,
    in_file_dialog: bool,
    last_pip_toggle: Instant,
    normal_rect_dip: (i32, i32, i32, i32),
    is_closing: bool,
    sec_timer_active: bool,
    vinyl_timer_active: bool,
    gif_timer_delay_ms: Option<u32>,
    loaded_gif_path: Option<PathBuf>,
    pending_gif_path: Option<PathBuf>,
    gif_load_gen: u64,
    gif_tx: mpsc::Sender<GifLoadMsg>,
    gif_rx: mpsc::Receiver<GifLoadMsg>,
    last_guard_tick: Instant,
    lan_ip: String,
    qr_matrix: Option<QrMatrix>,
}

impl AppState {
    fn dpi_scale(&self) -> f32 {
        self.surface.dpi.max(96.0) / 96.0
    }

    fn px_to_dip(&self, px: i32) -> f32 {
        px as f32 / self.dpi_scale()
    }

    fn dip_to_px(&self, dip: i32) -> i32 {
        (dip as f32 * self.dpi_scale()).round() as i32
    }

    fn effective_accent(&self) -> &str {
        if self.timer.phase() == SessionPhase::Resting {
            return "#F59E0B";
        }
        if self.config.dynamic_album_enabled && self.track.is_some() {
            if let Some(ref hex) = self.cached_dynamic_accent {
                return hex.as_str();
            }
        }
        self.config.accent_color.as_str()
    }

    fn recompute_dynamic_accent(&mut self) {
        self.cached_dynamic_accent = self
            .cover_frame
            .as_ref()
            .map(|frame| extract_album_accent(frame, self.config.use_complementary_color));
    }

    fn ends_at_label(&self) -> String {
        let st = unsafe { GetLocalTime() };
        compute_end_time_hhmm(
            st.wHour as u32,
            st.wMinute as u32,
            st.wSecond as u32,
            self.timer.total_remaining_sec(),
        )
    }

    fn load_active_gif(&mut self) {
        let gifs_dir = AppConfig::gifs_dir();
        let Some(path) = resolve_active_gif_path(&self.config, &gifs_dir) else {
            self.gif_load_gen = self.gif_load_gen.wrapping_add(1);
            self.pending_gif_path = None;
            self.loaded_gif_path = None;
            self.gpu_anim = None;
            self.frame_idx = 0;
            self.sync_gif_timer();
            return;
        };

        if self.config.ambient_mode != AmbientMode::Custom {
            self.sync_gif_timer();
            return;
        }

        self.load_gif_from_path(&path);
    }

    fn load_gif_from_path(&mut self, path: &Path) {
        if self.pending_gif_path.as_deref() == Some(path)
            || (self.loaded_gif_path.as_deref() == Some(path) && self.gpu_anim.is_some())
        {
            self.sync_gif_timer();
            return;
        }

        self.gif_load_gen = self.gif_load_gen.wrapping_add(1);
        let gen = self.gif_load_gen;
        let path_owned = path.to_path_buf();
        self.pending_gif_path = Some(path_owned.clone());
        let tx = self.gif_tx.clone();
        let hwnd_raw = self.hwnd.0 as isize;
        self.status_hint = Some("Loading GIF...".to_string());

        std::thread::spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            let notify_main = || {
                if hwnd_raw != 0 {
                    unsafe {
                        let _ = PostMessageW(
                            Some(HWND(hwnd_raw as *mut _)),
                            WM_GIF_LOADED,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            };
            let first_path = path_owned.clone();
            let first_tx = tx.clone();
            let res = decode_image_file_progressive(
                &path_owned,
                Some(|first_frame: DecodedAnimation| {
                    let _ = first_tx.send(GifLoadMsg::FirstFrame(
                        gen,
                        first_path.clone(),
                        first_frame,
                    ));
                    notify_main();
                }),
            );
            unsafe {
                CoUninitialize();
            }
            let _ = tx.send(GifLoadMsg::TailFrames(gen, path_owned, res));
            notify_main();
        });
    }

    fn handle_gif_loaded(&mut self) {
        while let Ok(msg) = self.gif_rx.try_recv() {
            match msg {
                GifLoadMsg::FirstFrame(gen, path, first) => {
                    if gen != self.gif_load_gen {
                        continue;
                    }
                    if let Ok(anim) = GpuAnimation::from_decoded(&self.surface.d2d_context, &first) {
                        self.loaded_gif_path = Some(path);
                        self.gpu_anim = Some(anim);
                        self.frame_idx = 0;
                        self.status_hint = None;
                        self.sync_gif_timer();
                        self.request_redraw();
                    }
                }
                GifLoadMsg::TailFrames(gen, path, res) => {
                    if gen != self.gif_load_gen {
                        continue;
                    }
                    self.pending_gif_path = None;
                    let already_previewed = self.loaded_gif_path.as_ref() == Some(&path)
                        && self.gpu_anim.is_some();
                    match res {
                        Ok(tail) => {
                            if already_previewed {
                                if !tail.frames.is_empty() {
                                    if let Some(ref mut anim) = self.gpu_anim {
                                        if let Err(err) = anim.append_decoded(&self.surface.d2d_context, &tail) {
                                            self.status_hint = Some(format!("GIF error: {err}"));
                                        }
                                    }
                                }
                            } else {
                                match GpuAnimation::from_decoded(&self.surface.d2d_context, &tail) {
                                    Ok(anim) => {
                                        self.loaded_gif_path = Some(path);
                                        self.gpu_anim = Some(anim);
                                        self.frame_idx = 0;
                                        self.status_hint = None;
                                    }
                                    Err(err) => {
                                        self.loaded_gif_path = None;
                                        self.gpu_anim = None;
                                        self.frame_idx = 0;
                                        self.status_hint = Some(format!("GIF error: {err}"));
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            if !already_previewed {
                                self.loaded_gif_path = None;
                                self.gpu_anim = None;
                                self.frame_idx = 0;
                                self.status_hint = Some(format!("GIF error: {err}"));
                            }
                        }
                    }
                    self.sync_gif_timer();
                    self.request_redraw();
                }
            }
        }
    }

    fn sync_sec_timer(&mut self) {
        let should_run = self.timer.is_running()
            || self.block_countdown.is_some()
            || self.prayer_break_remaining > 0;
        if should_run == self.sec_timer_active {
            return;
        }
        unsafe {
            if should_run {
                SetTimer(Some(self.hwnd), TIMER_SEC_ID, 250, None);
            } else {
                let _ = KillTimer(Some(self.hwnd), TIMER_SEC_ID);
            }
        }
        self.sec_timer_active = should_run;
    }

    fn refresh_prayer_schedule(&mut self) {
        let (today, year, month, day, cur_hour) = local_date_and_hour();
        if self.prayer_date != today || self.prayer_times.is_empty() {
            self.prayer_times = self.prayer_mgr.list(year, month, day);
            self.prayer_date = today.clone();
        }
        if !self.config.prayer_enabled {
            self.next_prayer_label = String::new();
            self.next_prayer_name.clear();
            self.next_prayer_secs = u32::MAX;
            return;
        }
        let (name, _h, secs_left) = self.prayer_mgr.next_prayer(year, month, day, cur_hour);
        self.next_prayer_name = name.to_string();
        self.next_prayer_secs = secs_left;
        let mins_left = (secs_left + 59) / 60;
        let hhmm = self
            .prayer_times
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.formatted.as_str())
            .unwrap_or("");
        if mins_left >= 60 {
            self.next_prayer_label = format!("{name} {hhmm} ({}h {}m)", mins_left / 60, mins_left % 60);
        } else {
            self.next_prayer_label = format!("{name} {hhmm} ({mins_left}m)");
        }
    }

    fn rebuild_prayer_mgr(&mut self) {
        self.prayer_mgr = PrayerManager::with_offset(
            self.config.prayer_lat,
            self.config.prayer_lng,
            self.config.prayer_tz,
            self.config.prayer_offset_mins,
        );
        self.prayer_date.clear();
        self.refresh_prayer_schedule();
    }

    fn filtered_prayer_cities(&self) -> Vec<CityEntry> {
        if let Some(ref online) = self.prayer_online_results {
            return online.iter().take(6).cloned().collect();
        }
        let q = self.prayer_search_query.trim().to_ascii_lowercase();
        let mut out: Vec<CityEntry> = Vec::with_capacity(6);
        for &(name, lat, lng, tz) in CITY_PRESETS {
            if q.is_empty() || name.to_ascii_lowercase().contains(&q) {
                out.push(CityEntry {
                    name: name.to_string(),
                    lat,
                    lng,
                    tz,
                });
                if out.len() >= 6 {
                    break;
                }
            }
        }
        out
    }

    fn apply_prayer_location(&mut self, lat: f64, lng: f64, tz: i32, label: &str) {
        self.config.prayer_lat = lat;
        self.config.prayer_lng = lng;
        self.config.prayer_tz = tz;
        let _ = self.config.save();
        self.rebuild_prayer_mgr();
        self.prayer_loc_status = Some(format!("Selected: {label} (UTC{tz:+})"));
        self.status_hint = Some(format!("Prayer location: {label} (UTC{tz:+})"));
        self.sync_overlays();
        self.sync_sec_timer();
        self.request_redraw();
    }

    fn post_prayer_loc_msg(hwnd_raw: isize, tx: &mpsc::Sender<PrayerLocMsg>, msg: PrayerLocMsg) {
        let _ = tx.send(msg);
        if hwnd_raw != 0 {
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(hwnd_raw as *mut _)),
                    WM_PRAYER_LOC_EVENT,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }

    fn start_auto_gps_lookup(&mut self) {
        if self.prayer_loc_busy {
            return;
        }
        self.prayer_loc_busy = true;
        self.prayer_loc_status = Some("Detecting Windows GPS / IP location...".to_string());
        self.request_redraw();

        let tx = self.prayer_loc_tx.clone();
        let hwnd_raw = self.hwnd.0 as isize;
        std::thread::spawn(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let script = concat!(
                "try { Add-Type -AssemblyName System.Device -ErrorAction Stop; ",
                "$w = New-Object System.Device.Location.GeoCoordinateWatcher('Default'); ",
                "$null = $w.TryStart($false, [TimeSpan]::FromMilliseconds(3500)); ",
                "$deadline = (Get-Date).AddMilliseconds(3500); ",
                "while ($w.Position.Location.IsUnknown -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 150 }; ",
                "$loc = $w.Position.Location; $w.Stop(); ",
                "if (-not $loc.IsUnknown) { ",
                "$off = [int][Math]::Round([TimeZoneInfo]::Local.GetUtcOffset([DateTime]::Now).TotalHours); ",
                "Write-Output ('OK|' + $loc.Latitude.ToString([Globalization.CultureInfo]::InvariantCulture) + '|' + $loc.Longitude.ToString([Globalization.CultureInfo]::InvariantCulture) + '|' + $off + '|Windows GPS'); ",
                "exit 0 } } catch {}; ",
                "try { $r = Invoke-RestMethod -Uri 'http://ip-api.com/json/?fields=status,city,regionName,lat,lon,offset' -TimeoutSec 6; ",
                "if ($r.status -eq 'success') { ",
                "$tz = [int][Math]::Round($r.offset / 3600.0); ",
                "$lbl = (@($r.city, $r.regionName) | Where-Object { $_ }) -join ', '; if (-not $lbl) { $lbl = 'IP Geolocation' }; ",
                "Write-Output ('OK|' + ([double]$r.lat).ToString([Globalization.CultureInfo]::InvariantCulture) + '|' + ([double]$r.lon).ToString([Globalization.CultureInfo]::InvariantCulture) + '|' + $tz + '|' + ($lbl -replace '\\|',' ')); ",
                "exit 0 } } catch {}; ",
                "Write-Output 'ERR|GPS/Network unavailable. Pick a city below or click Search online.'"
            );
            let msg = match std::process::Command::new("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let line = stdout.lines().rev().find(|l| l.starts_with("OK|") || l.starts_with("ERR|"));
                    match line {
                        Some(l) if l.starts_with("OK|") => {
                            let parts: Vec<&str> = l.splitn(5, '|').collect();
                            if parts.len() == 5 {
                                if let (Ok(lat), Ok(lng), Ok(tz)) = (
                                    parts[1].trim().parse::<f64>(),
                                    parts[2].trim().parse::<f64>(),
                                    parts[3].trim().parse::<i32>(),
                                ) {
                                    PrayerLocMsg::GpsResult(Ok((
                                        lat,
                                        lng,
                                        tz.clamp(-12, 14),
                                        parts[4].trim().to_string(),
                                    )))
                                } else {
                                    PrayerLocMsg::GpsResult(Err("Invalid coordinate response".into()))
                                }
                            } else {
                                PrayerLocMsg::GpsResult(Err("Unexpected GPS output".into()))
                            }
                        }
                        Some(l) if l.starts_with("ERR|") => {
                            PrayerLocMsg::GpsResult(Err(l.trim_start_matches("ERR|").to_string()))
                        }
                        _ => PrayerLocMsg::GpsResult(Err("GPS unavailable; pick or search a city".into())),
                    }
                }
                Err(e) => PrayerLocMsg::GpsResult(Err(format!("GPS launch failed: {e}"))),
            };
            Self::post_prayer_loc_msg(hwnd_raw, &tx, msg);
        });
    }

    fn start_online_city_search(&mut self) {
        let query = self.prayer_search_query.trim().to_string();
        if query.len() < 2 {
            self.prayer_online_results = None;
            self.prayer_loc_status = Some("Type at least 2 characters to search online.".to_string());
            self.request_redraw();
            return;
        }
        self.prayer_search_gen = self.prayer_search_gen.wrapping_add(1);
        let gen = self.prayer_search_gen;
        self.prayer_loc_busy = true;
        self.prayer_loc_status = Some(format!("Searching \"{query}\" online..."));
        self.request_redraw();

        let encoded: String = query
            .bytes()
            .flat_map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    vec![b as char]
                }
                b' ' => vec!['+'],
                _ => format!("%{b:02X}").chars().collect(),
            })
            .collect();
        let url = format!(
            "https://nominatim.openstreetmap.org/search?format=json&limit=6&q={encoded}"
        );
        let tx = self.prayer_loc_tx.clone();
        let hwnd_raw = self.hwnd.0 as isize;
        std::thread::spawn(move || {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let script = format!(
                "try {{ $res = Invoke-RestMethod -Uri '{}' -Headers @{{'User-Agent'='FocusGrow/1.0'}} -TimeoutSec 7; ",
                url
            ) + "foreach ($item in $res) { "
                + "$n = ($item.display_name -replace '[\\r\\n\\|]+', ' '); "
                + "if ($n.Length -gt 64) { $n = $n.Substring(0, 61) + '...' }; "
                + "Write-Output ($n + '|' + $item.lat + '|' + $item.lon) "
                + "} } catch { Write-Output 'ERR|Online search failed (offline?)' }";
            let msg = match std::process::Command::new("powershell")
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let mut cities = Vec::new();
                    let mut err_msg = None;
                    for line in stdout.lines() {
                        if let Some(e) = line.strip_prefix("ERR|") {
                            err_msg = Some(e.to_string());
                            break;
                        }
                        let parts: Vec<&str> = line.splitn(3, '|').collect();
                        if parts.len() == 3 {
                            if let (Ok(lat), Ok(lng)) =
                                (parts[1].trim().parse::<f64>(), parts[2].trim().parse::<f64>())
                            {
                                cities.push(CityEntry {
                                    name: parts[0].trim().to_string(),
                                    lat,
                                    lng,
                                    tz: infer_timezone_from_coords(lat, lng),
                                });
                            }
                        }
                    }
                    if let Some(e) = err_msg {
                        PrayerLocMsg::CitySearchResult(gen, Err(e))
                    } else {
                        PrayerLocMsg::CitySearchResult(gen, Ok(cities))
                    }
                }
                Err(e) => PrayerLocMsg::CitySearchResult(gen, Err(format!("Search failed: {e}"))),
            };
            Self::post_prayer_loc_msg(hwnd_raw, &tx, msg);
        });
    }

    fn handle_prayer_loc_events(&mut self) {
        let mut redraw = false;
        while let Ok(msg) = self.prayer_loc_rx.try_recv() {
            match msg {
                PrayerLocMsg::GpsResult(res) => {
                    self.prayer_loc_busy = false;
                    match res {
                        Ok((lat, lng, tz, label)) => {
                            self.apply_prayer_location(lat, lng, tz, &label);
                        }
                        Err(err) => {
                            self.prayer_loc_status = Some(err);
                        }
                    }
                    redraw = true;
                }
                PrayerLocMsg::CitySearchResult(gen, res) => {
                    if gen != self.prayer_search_gen {
                        continue;
                    }
                    self.prayer_loc_busy = false;
                    match res {
                        Ok(list) if !list.is_empty() => {
                            let cnt = list.len();
                            self.prayer_online_results = Some(list);
                            self.prayer_loc_status =
                                Some(format!("Found {cnt} online match(es). Click one to apply."));
                        }
                        Ok(_) => {
                            self.prayer_online_results = None;
                            self.prayer_loc_status =
                                Some("No online city results found. Try another name.".to_string());
                        }
                        Err(err) => {
                            self.prayer_loc_status = Some(err);
                        }
                    }
                    redraw = true;
                }
            }
        }
        if redraw {
            self.request_redraw();
        }
    }

    fn sync_overlays(&mut self) {
        let now = Instant::now();
        let phase = self.timer.phase();
        let running = self.timer.is_running();

        // Detect end of a Posture Break (Resting -> Focusing) to show celebration pill.
        if self.prev_phase == SessionPhase::Resting
            && phase == SessionPhase::Focusing
            && self.config.is_health_mode
        {
            if let Some(ref mut hud) = self.health_hud {
                hud.show_health_finished();
                self.health_finish_until = Some(now + Duration::from_secs(5));
            }
        }
        self.prev_phase = phase;

        // Wall-clock 15-minute prayer break enforcement: cannot be bypassed by closing & reopening.
        if self.config.prayer_enabled && self.config.prayer_break_enabled {
            let (today, year, month, day, cur_hour) = local_date_and_hour();
            if let Some((pname, rem_secs)) =
                self.prayer_mgr.active_prayer_window(year, month, day, cur_hour, 900)
            {
                let key = format!("{today}:{pname}");
                if self.last_prayer_dismissed_key == key {
                    self.prayer_break_remaining = 0;
                    self.active_prayer_break_name.clear();
                } else {
                    self.active_prayer_break_name = pname.to_string();
                    self.prayer_break_remaining = rem_secs;
                    if self.last_prayer_break_key != key {
                        self.last_prayer_break_key = key;
                        if self.config.notifications_enabled {
                            self.audio.play_notification(
                                &self.config.warning_sound,
                                self.config.custom_warning_audio_path.as_deref(),
                            );
                        }
                    }
                }
            } else {
                self.prayer_break_remaining = 0;
                self.active_prayer_break_name.clear();
            }
        } else {
            self.prayer_break_remaining = 0;
            self.active_prayer_break_name.clear();
        }

        // 1. Dynamic Island Health HUD: ONLY in Posture Guard (`is_health_mode == true`)!
        // Productive Focus uses fullscreen block overlays rather than floating capsules.
        let hud_finished_active = self.health_finish_until.is_some_and(|until| now < until);
        if !hud_finished_active {
            self.health_finish_until = None;
        }
        if let Some(ref mut hud) = self.health_hud {
            if self.config.is_health_mode && self.prayer_break_remaining > 0 {
                self.health_finish_until = None;
                let label = if !self.active_prayer_break_name.is_empty() {
                    self.active_prayer_break_name.as_str()
                } else if !self.next_prayer_name.is_empty() {
                    self.next_prayer_name.as_str()
                } else {
                    "Prayer"
                };
                hud.show_prayer_countdown(label, self.prayer_break_remaining);
            } else if phase == SessionPhase::Resting && self.config.is_health_mode && running {
                self.health_finish_until = None;
                hud.show_health_countdown(self.timer.remaining_sec());
            } else if self.config.is_health_mode
                && self.config.prayer_enabled
                && self.prayer_break_remaining == 0
                && self.next_prayer_secs > 0
                && self.next_prayer_secs <= self.config.prayer_advance_mins * 60
                && !self.next_prayer_name.is_empty()
            {
                self.health_finish_until = None;
                hud.show_prayer_countdown(&self.next_prayer_name, self.next_prayer_secs);
            } else if !hud_finished_active && hud.is_visible() {
                hud.hide();
            }
        }

        // 2. Fullscreen Overlay priority: PrayerBreak (in Productive Focus) > AutoCloseWarning > FocusBlock > RestBreak
        if let Some(ref mut ov) = self.overlay {
            if self.prayer_break_remaining > 0 && !self.config.is_health_mode {
                let label = if !self.active_prayer_break_name.is_empty() {
                    self.active_prayer_break_name.as_str()
                } else if !self.next_prayer_name.is_empty() {
                    self.next_prayer_name.as_str()
                } else {
                    "Prayer"
                };
                ov.show(OverlayMode::PrayerBreak, self.prayer_break_remaining, label, 0);
            } else if let Some((_, rem)) = self.block_countdown {
                let app_label = self
                    .active_app
                    .as_ref()
                    .map(|a| a.exe_name.as_str())
                    .unwrap_or("Blocked App");
                ov.show(OverlayMode::AutoCloseWarning, rem as u32, app_label, 0);
            } else if let Some(ref app_label) = self.blocked_app_active {
                ov.show(OverlayMode::FocusBlock, self.timer.remaining_sec(), app_label, 0);
            } else if self.neutral_overstay.should_show_overlay(
                phase == SessionPhase::Focusing && running && !self.config.is_health_mode,
                &self.config,
                self.active_app
                    .as_ref()
                    .map(|a| a.exe_name.as_str())
                    .unwrap_or(""),
            ) {
                let fallback_exe = self
                    .active_app
                    .as_ref()
                    .map(|a| a.exe_name.as_str())
                    .unwrap_or("");
                let app_label = self
                    .neutral_overstay
                    .softblock_app
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(fallback_exe);
                ov.show(
                    OverlayMode::NeutralOverstayBlock,
                    self.timer.remaining_sec(),
                    app_label,
                    0,
                );
            } else if phase == SessionPhase::Resting && running && !self.config.is_health_mode {
                ov.show(OverlayMode::RestBreak, self.timer.remaining_sec(), "Rest", 0);
            } else if ov.is_visible() {
                ov.hide();
            }
        }
    }

    fn handle_overlay_events(&mut self) {
        let actions = if let Some(ref ov) = self.overlay {
            ov.drain_actions()
        } else {
            return;
        };
        let mut changed = false;
        for act in actions {
            match act {
                OverlayAction::Dismissed => {
                    if self.prayer_break_remaining > 0 {
                        self.prayer_break_remaining = 0;
                        self.last_prayer_dismissed_key = self.last_prayer_break_key.clone();
                    }
                    if self.blocked_app_active.take().is_some() {
                        self.dismissed_blocked_hwnd = self.active_app.as_ref().map(|a| a.hwnd_raw);
                    }
                    changed = true;
                }
                OverlayAction::GrantPass { domain, minutes } => {
                    if self.guard.grant_temporary_pass(&domain, minutes) {
                        self.status_hint = Some(format!("Granted {minutes}m pass for {domain}"));
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.sync_overlays();
            self.sync_bridge_snapshot();
            self.request_redraw();
        }
    }

    fn sync_bridge_snapshot(&self) {
        let Some(ref bridge) = self.http_bridge else {
            return;
        };
        let (today, year, month, day, cur_hour) = local_date_and_hour();
        let state_json = self.guard.build_state_json(
            &self.config,
            &self.timer,
            &self.stats,
            &today,
            year,
            month,
            day,
            cur_hour,
            self.is_auto_paused,
        );
        let restricted_json =
            serde_json::to_string(&self.config.restricted_sites).unwrap_or_else(|_| "[]".into());
        let is_focusing_unpaused =
            self.timer.phase() == SessionPhase::Focusing && self.timer.is_running();
        let restricted_domains: Vec<String> = self
            .guard
            .restricted_sites
            .iter()
            .map(|s| s.domain.to_lowercase())
            .collect();
        let active_pass_domains: Vec<(String, u32)> = self
            .guard
            .restricted_sites
            .iter()
            .filter(|s| s.pass_remaining_sec > 0)
            .map(|s| (s.domain.to_lowercase(), s.pass_remaining_sec))
            .collect();
        bridge.update_snapshot(
            restricted_json,
            state_json,
            is_focusing_unpaused,
            restricted_domains,
            active_pass_domains,
        );
    }

    fn evaluate_foreground_guard(&mut self) {
        let should_guard = self.timer.phase() == SessionPhase::Focusing
            && self.timer.is_running()
            && !self.config.is_health_mode;
        if !should_guard {
            self.block_countdown = None;
            self.blocked_app_active = None;
            self.dismissed_blocked_hwnd = None;
            return;
        }
        let Some(ref info) = self.active_app else {
            self.block_countdown = None;
            self.blocked_app_active = None;
            self.dismissed_blocked_hwnd = None;
            return;
        };
        if self
            .guard
            .is_app_blacklisted(&info.exe_name, &info.title, &self.timer)
        {
            if self.config.auto_close_blocked_apps {
                self.blocked_app_active = None;
                let same_win = self
                    .block_countdown
                    .is_some_and(|(hwnd_raw, _)| hwnd_raw == info.hwnd_raw);
                if !same_win {
                    self.block_countdown = Some((info.hwnd_raw, 5));
                }
            } else {
                self.block_countdown = None;
                if self.dismissed_blocked_hwnd == Some(info.hwnd_raw) {
                    self.blocked_app_active = None;
                } else {
                    self.blocked_app_active = Some(info.exe_name.clone());
                }
            }
        } else {
            self.block_countdown = None;
            self.blocked_app_active = None;
            if info.hwnd_raw != self.hwnd.0 as isize {
                self.dismissed_blocked_hwnd = None;
            }
        }
        self.sync_sec_timer();
    }

    fn handle_foreground_event(&mut self) {
        let fg = unsafe { GetForegroundWindow() };
        if self.overlay.as_ref().is_some_and(|ov| ov.is_overlay_hwnd(fg)) {
            return;
        }
        if let Some(info) = inspect_window(fg) {
            let exe_low = info.exe_name.to_ascii_lowercase();
            if !exe_low.is_empty() && !self.app_icons_by_exe.contains_key(&exe_low) {
                if let Some(frame) = extract_app_icon_frame(info.hwnd_raw, &info.exe_path) {
                    if let Ok(bmp) = upload_frame_bitmap(&self.surface.d2d_context, &frame) {
                        self.app_icons_by_exe.insert(exe_low, bmp);
                    }
                }
            }
            self.guard.active_exe = info.exe_name.clone();
            self.guard.active_title = info.title.clone();
            self.active_app = Some(info);
        }
        self.evaluate_foreground_guard();
        self.sync_overlays();
        self.sync_bridge_snapshot();
        if self.guard_tab == GuardTab::BlockedApps {
            self.refresh_running_apps();
        }
        if self.guard_tab == GuardTab::BlockedApps
            || self.guard_tab == GuardTab::PrayerTimes
            || self.block_countdown.is_some()
        {
            self.request_redraw();
        }
    }

    fn handle_bridge_events(&mut self) {
        let cmds = if let Some(ref bridge) = self.http_bridge {
            bridge.drain_commands()
        } else {
            return;
        };
        let mut changed = false;
        for cmd in cmds {
            match cmd {
                BridgeCommand::GrantPass { domain, minutes } => {
                    if self.guard.grant_temporary_pass(&domain, minutes) {
                        self.status_hint = Some(format!("Granted {minutes}m pass for {domain}"));
                        changed = true;
                    }
                }
                BridgeCommand::ActiveTab { url, domain, title } => {
                    self.guard.active_url = url;
                    if !title.is_empty() {
                        self.guard.active_title = title;
                    }
                    if !domain.is_empty() && self.active_domain != domain {
                        self.guard.active_domain = domain.clone();
                        self.active_domain = domain;
                        if let Some(ref mut ov) = self.overlay {
                            ov.clear_dismissed_domain();
                        }
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.sync_overlays();
            self.sync_bridge_snapshot();
            self.request_redraw();
        }
    }

    fn on_idle_heartbeat(&mut self) {
        let (today, ..) = local_date_and_hour();
        let mut changed = self.stats.rollover_if_needed(&today);
        if changed {
            let _ = self.stats.save();
        }
        let prev_label = self.next_prayer_label.clone();
        self.refresh_prayer_schedule();
        if self.next_prayer_label != prev_label {
            changed = true;
        }
        if self.prayer_break_remaining > 0 {
            self.prayer_break_remaining = self.prayer_break_remaining.saturating_sub(1);
            changed = true;
        }
        if self.guard_tab == GuardTab::PrayerTimes && !self.config.pip_mode {
            changed = true;
        }
        self.sync_overlays();
        if !self.timer.is_running() {
            self.sync_bridge_snapshot();
        }
        if changed {
            self.request_redraw();
        }
    }

    fn persist_session_now(&self) {
        if let Some(s) = self.timer.export_session(self.config.is_health_mode) {
            let _ = s.save();
        } else {
            PersistedSession::clear();
        }
    }

    fn notify_windows(&mut self, title: &str, body: &str) {
        if self.config.notifications_enabled {
            if let Some(ref mut tray) = self.tray {
                tray.show_notification(title, body);
            }
        }
    }

    fn perform_deferred_init(&mut self) {
        if self.deferred_done {
            return;
        }
        self.deferred_done = true;

        let (today, y, m, d, _) = local_date_and_hour();
        let yesterday = prev_date_str(y, m, d);
        if !self.config.webview2_migrated {
            if auto_migrate_webview2(&today, &yesterday, &mut self.config, &mut self.stats).is_ok()
            {
                self.stats
                    .recalculate_streaks(&today, self.config.daily_goal_minutes);
                let _ = self.config.save();
                let _ = self.stats.save();
                self.guard.sync_from_config(&self.config);
                if let Some(ref mut ov) = self.overlay {
                    ov.set_block_all_monitors(self.config.block_all_monitors);
                }
                self.prayer_mgr = PrayerManager::with_offset(
                    self.config.prayer_lat,
                    self.config.prayer_lng,
                    self.config.prayer_tz,
                    self.config.prayer_offset_mins,
                );
                if self.timer.phase() == SessionPhase::Idle {
                    self.sync_schedule_to_timer();
                }
            }
        } else if self.stats.history.is_empty()
            || (self.stats.health_history.is_empty() && self.stats.work_windows.is_empty())
        {
            let mut scratch_cfg = self.config.clone();
            if auto_migrate_webview2(&today, &yesterday, &mut scratch_cfg, &mut self.stats).is_ok()
            {
                if self.config.app_categories.is_empty() && !scratch_cfg.app_categories.is_empty() {
                    self.config.app_categories = scratch_cfg.app_categories;
                    let _ = self.config.save();
                }
                self.stats
                    .recalculate_streaks(&today, self.config.daily_goal_minutes);
                let _ = self.stats.save();
            }
        }

        let _ = self.audio.ensure(&self.config.ambient);
        if self.timer.is_running()
            && self.config.ambient.enabled
            && self.config.ambient.auto_play
            && !self.audio.is_playing()
        {
            self.audio.set_playing(true, &self.config.ambient);
        }

        self.refresh_prayer_schedule();
        let bridge_port = self
            .http_bridge
            .as_ref()
            .map(|b| b.bound_port)
            .filter(|&p| p > 0)
            .unwrap_or(8766);
        self.lan_ip = detect_lan_ipv4();
        self.qr_matrix = encode_qr_v3_m(&format!("http://{}:{bridge_port}", self.lan_ip));
        self.refresh_running_apps();
        self.load_active_gif();
        self.timer.refresh_tick_anchor(Instant::now());
        self.sync_overlays();
        self.sync_vinyl_overlay();
        self.sync_sec_timer();
        self.sync_vinyl_timer();
        self.sync_bridge_snapshot();
        self.request_redraw();
    }

    fn on_second_tick(&mut self) {
        if let Some(ref mut ov) = self.overlay {
            if ov.is_visible() && !self.config.block_all_monitors {
                ov.tick_follow_active_monitor();
            }
        }
        let now = Instant::now();
        let st = unsafe { GetLocalTime() };
        let today = format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay);
        let cur_hour_idx = st.wHour as usize;
        let rolled = self.stats.rollover_if_needed(&today);
        if rolled {
            let _ = self.stats.save();
        }

        // Fallback OS foreground polling so elevated launchers/games missed by SetWinEventHook
        // are always detected within 1s.
        let fg = unsafe { GetForegroundWindow() };
        if !fg.0.is_null()
            && !self.overlay.as_ref().is_some_and(|ov| ov.is_overlay_hwnd(fg))
            && self.active_app.as_ref().map(|a| a.hwnd_raw) != Some(fg.0 as isize)
        {
            self.handle_foreground_event();
        }

        // Idle auto-pause guard when focusing actively
        if self.config.auto_pause_enabled
            && self.timer.phase() == SessionPhase::Focusing
            && self.timer.is_running()
            && system_idle_seconds() >= self.config.auto_pause_sec
        {
            self.timer.toggle_pause(now);
            self.is_auto_paused = true;
            self.block_countdown = None;
            self.persist_session_now();
            self.sync_overlays();
            self.sync_sec_timer();
            self.sync_bridge_snapshot();
            self.request_redraw();
            return;
        }

        let was_running = self.timer.is_running();
        let res = self.timer.tick(now);
        if let Some(completed_phase) = res.phase_completed {
            self.warned_5m_phase = None;
            self.warned_1m_phase = None;
            if completed_phase == SessionPhase::Resting && self.config.is_health_mode {
                self.stats
                    .record_health_stand(&today, self.config.daily_goal_minutes);
                let _ = self.stats.save();
            }
            if self.config.notifications_enabled {
                if completed_phase == SessionPhase::Focusing {
                    self.audio.play_notification(
                        &self.config.focus_sound,
                        self.config.custom_focus_audio_path.as_deref(),
                    );
                    if self.timer.phase() == SessionPhase::Resting {
                        if self.config.is_health_mode {
                            self.notify_windows(
                                "Time to Stand & Stretch!",
                                "Session period completed! Stand up, stretch, and hydrate.",
                            );
                        } else {
                            self.notify_windows(
                                "Time to Rest",
                                "Focus period done! Step away from your computer for a break.",
                            );
                        }
                    } else if self.timer.phase() == SessionPhase::Idle {
                        self.notify_windows(
                            "Session Completed",
                            "Great job! Focus session completed successfully.",
                        );
                    }
                } else if completed_phase == SessionPhase::Resting {
                    self.audio.play_notification(
                        &self.config.break_sound,
                        self.config.custom_break_audio_path.as_deref(),
                    );
                    if self.timer.phase() == SessionPhase::Focusing {
                        self.notify_windows(
                            if self.config.is_health_mode {
                                "Posture Guard Active"
                            } else {
                                "Focus Mode Active"
                            },
                            "Break finished! Ready for your next focus period.",
                        );
                    }
                }
            }
            if self.timer.phase() == SessionPhase::Idle && self.audio.is_playing() {
                self.audio.set_playing(false, &self.config.ambient);
            }
            self.persist_session_now();
        } else if res.changed {
            if was_running && self.timer.is_running() {
                let phase_key = (self.timer.phase(), self.timer.current_period());
                let rem = self.timer.remaining_sec();
                let phase_total = self.timer.total_phase_sec();
                if phase_total > 300 && rem == 300 && self.warned_5m_phase != Some(phase_key) {
                    self.warned_5m_phase = Some(phase_key);
                    let (title, body) = match self.timer.phase() {
                        SessionPhase::Resting => (
                            "Break Ending in 5 Minutes",
                            "5 minutes left before the next focus period begins.",
                        ),
                        _ => (
                            "Break in 5 Minutes",
                            "5 minutes left in this focus period. Prepare for your break.",
                        ),
                    };
                    self.notify_windows(title, body);
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.warning_sound,
                            self.config.custom_warning_audio_path.as_deref(),
                        );
                    }
                }
                let warn_sec = if phase_total <= 60 { 30 } else { 60 };
                if rem == warn_sec && self.warned_1m_phase != Some(phase_key) {
                    self.warned_1m_phase = Some(phase_key);
                    let label = if warn_sec == 30 { "30 seconds" } else { "1 minute" };
                    let (title, body) = match self.timer.phase() {
                        SessionPhase::Resting => (
                            "Break Warning",
                            format!("{label} left of break! Prepare to resume."),
                        ),
                        _ => (
                            if self.config.is_health_mode {
                                "Session Break Warning"
                            } else {
                                "Period Warning"
                            },
                            format!("{label} left of focus period!"),
                        ),
                    };
                    self.notify_windows(title, &body);
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.warning_sound,
                            self.config.custom_warning_audio_path.as_deref(),
                        );
                    }
                }
            }
            if self.config.countdown_sound_enabled
                && was_running
                && self.timer.is_running()
                && (1..=self.config.countdown_duration_secs as u32)
                    .contains(&self.timer.remaining_sec())
            {
                self.audio
                    .play_countdown_pip(self.timer.remaining_sec() as u8);
            }
        }

        let sec_step = if self.timer.is_running() {
            if res.changed || res.phase_completed.is_some() {
                self.last_guard_tick = now;
                true
            } else {
                false
            }
        } else if self.last_guard_tick.elapsed() >= Duration::from_secs(1) {
            self.last_guard_tick = now;
            true
        } else {
            false
        };

        let mut redraw = res.changed || res.phase_completed.is_some() || rolled;
        if sec_step {
            if self.timer.phase() == SessionPhase::Focusing && self.timer.is_running() {
                let active_exe = self
                    .active_app
                    .as_ref()
                    .map(|a| a.exe_name.as_str())
                    .unwrap_or("");
                if self.stats.record_focus_second_full(
                    &today,
                    cur_hour_idx,
                    st.wMinute as usize,
                    active_exe,
                    self.config.daily_goal_minutes,
                ) {
                    let _ = self.stats.save();
                }
            }

            let in_productive_focus = self.timer.phase() == SessionPhase::Focusing
                && self.timer.is_running()
                && !self.config.is_health_mode;
            let active_exe = self
                .active_app
                .as_ref()
                .map(|a| a.exe_name.as_str())
                .unwrap_or("");
            match self
                .neutral_overstay
                .tick_one_second(in_productive_focus, &self.config, active_exe)
            {
                NeutralOverstayEvent::CheckIn1x { app, mins } => {
                    self.stats.record_ignored_nudge(&today);
                    let _ = self.stats.save();
                    self.notify_windows(
                        "Neutral App Check-in",
                        &format!(
                            "You've been on {app} for {mins}m during Focus mode. Ready to return to work?"
                        ),
                    );
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.warning_sound,
                            self.config.custom_warning_audio_path.as_deref(),
                        );
                    }
                    redraw = true;
                }
                NeutralOverstayEvent::SoftBlock2x { app, mins } => {
                    self.stats.record_ignored_nudge(&today);
                    let _ = self.stats.save();
                    self.notify_windows(
                        "Time to Refocus",
                        &format!(
                            "{mins}m on {app} during Focus mode. Returning to your workspace!"
                        ),
                    );
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.warning_sound,
                            self.config.custom_warning_audio_path.as_deref(),
                        );
                    }
                    redraw = true;
                }
                NeutralOverstayEvent::Recovered => {
                    self.status_hint = Some("Focus restored! 5m sustained work completed.".to_string());
                    self.notify_windows(
                        "Focus Restored",
                        "Great job! 5 minutes of sustained focus completed. Neutral app access restored.",
                    );
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.focus_sound,
                            self.config.custom_focus_audio_path.as_deref(),
                        );
                    }
                    redraw = true;
                }
                NeutralOverstayEvent::None => {}
            }

            if self.timer.phase() != SessionPhase::Idle {
                self.session_save_tick = (self.session_save_tick + 1) % 5;
                if self.session_save_tick == 0 {
                    self.persist_session_now();
                }
            }

            self.guard.tick_passes_one_second();

            // Blocked app 5s countdown enforcement (terminates launchers/games; falls back to FocusBlock if protected)
            if let Some((hwnd_raw, rem)) = self.block_countdown {
                if self.timer.phase() != SessionPhase::Focusing
                    || !self.timer.is_running()
                    || !self.config.auto_close_blocked_apps
                {
                    self.block_countdown = None;
                    redraw = true;
                } else if rem <= 1 {
                    let closed = close_window_gracefully(hwnd_raw);
                    let name = self
                        .active_app
                        .as_ref()
                        .map(|a| a.exe_name.clone())
                        .unwrap_or_else(|| "distracting app".to_string());
                    self.block_countdown = None;
                    if closed {
                        self.status_hint = Some(format!("Closed blocked app: {name}"));
                    } else {
                        self.blocked_app_active = Some(name.clone());
                        self.status_hint = Some(format!("Blocking overlay active for {name}"));
                    }
                    redraw = true;
                } else {
                    self.block_countdown = Some((hwnd_raw, rem - 1));
                    redraw = true;
                }
            }

            if self.prayer_break_remaining > 0 {
                self.prayer_break_remaining = self.prayer_break_remaining.saturating_sub(1);
                redraw = true;
            }
            if self.guard_tab == GuardTab::PrayerTimes && !self.config.pip_mode {
                redraw = true;
            }
            self.refresh_prayer_schedule();
            if self.config.prayer_enabled
                && self.prayer_break_remaining == 0
                && self.next_prayer_secs > 0
                && self.next_prayer_secs <= self.config.prayer_advance_mins * 60
                && !self.next_prayer_name.is_empty()
            {
                let adv_key = format!("adv:{}:{}", self.prayer_date, self.next_prayer_name);
                if self.last_prayer_adv_key != adv_key {
                    self.last_prayer_adv_key = adv_key;
                    let pname = self.next_prayer_name.clone();
                    let mins = ((self.next_prayer_secs + 59) / 60).max(1);
                    self.notify_windows(
                        "Pengingat Sholat",
                        &format!("Waktu {pname} sekitar {mins} menit lagi."),
                    );
                    if self.config.notifications_enabled {
                        self.audio.play_notification(
                            &self.config.warning_sound,
                            self.config.custom_warning_audio_path.as_deref(),
                        );
                    }
                }
            }
        }
        if sec_step || redraw {
            self.sync_overlays();
            self.sync_sec_timer();
            self.sync_bridge_snapshot();
        }
        if redraw {
            self.request_redraw();
        }
    }

    fn save_resized_bounds(&mut self, update_size: bool) {
        if unsafe { IsIconic(self.hwnd).as_bool() } {
            return;
        }
        let mut rc = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut rc) }.is_err() {
            return;
        }
        if rc.left <= -10_000 || rc.top <= -10_000 {
            return;
        }
        let scale = self.dpi_scale();
        let x = (rc.left as f32 / scale).round() as i32;
        let y = (rc.top as f32 / scale).round() as i32;
        let w = ((rc.right - rc.left) as f32 / scale).round() as i32;
        let h = ((rc.bottom - rc.top) as f32 / scale).round() as i32;
        if w <= 0 || h <= 0 {
            return;
        }
        if self.config.pip_mode {
            self.config.pip_x = Some(x);
            self.config.pip_y = Some(y);
            if update_size {
                if self.config.pip_style == PipStyle::Capsule {
                    self.config.pip_capsule_width = w.clamp(195, 520);
                    self.config.pip_capsule_height = h.max(68);
                } else {
                    self.config.pip_width = w.max(180);
                    self.config.pip_height = h.max(220);
                }
            }
        } else if !self.is_maximized {
            self.normal_rect_dip = (x, y, w, h);
            if update_size {
                self.config.dashboard_width = w;
                self.config.dashboard_height = h;
            }
        }
        self.config.sanitize();
        let _ = self.config.save();
    }

    fn sync_gif_timer(&mut self) {
        let visible =
            unsafe { IsWindowVisible(self.hwnd).as_bool() && !IsIconic(self.hwnd).as_bool() };
        let desired_delay = if visible && self.config.ambient_mode == AmbientMode::Custom {
            self.gpu_anim
                .as_ref()
                .filter(|a| a.is_animated())
                .map(|a| a.frames[self.frame_idx % a.frames.len()].delay_ms.clamp(16, 5000))
        } else {
            None
        };
        if desired_delay == self.gif_timer_delay_ms {
            return;
        }
        unsafe {
            match desired_delay {
                Some(delay) => {
                    SetTimer(Some(self.hwnd), TIMER_GIF_ID, delay, None);
                }
                None => {
                    let _ = KillTimer(Some(self.hwnd), TIMER_GIF_ID);
                }
            }
        }
        self.gif_timer_delay_ms = desired_delay;
    }

    fn in_card_vinyl_animating(&self) -> bool {
        self.playback == PlaybackState::Playing
            && self.track.is_some()
            && (self.config.ambient_mode == AmbientMode::YtMusic
                || (self.config.pip_mode && self.config.pip_style == PipStyle::Capsule && self.config.pip_capsule_album_bg && self.config.pip_capsule_album_spin)
                || (!self.config.pip_mode && self.settings_tab == Some(SettingsTab::MobileSync)))
    }

    fn sync_vinyl_overlay(&mut self) {
        let enabled = self.config.pip_peeking_vinyl_enabled && self.config.pip_mode && self.config.pip_style == PipStyle::Card;
        let has_track = self.track.is_some();
        let is_playing = self.playback == PlaybackState::Playing;
        let accent = self.effective_accent().to_string();
        if let Some(ref mut vinyl) = self.peeking_vinyl {
            vinyl.update_settings(
                enabled,
                has_track,
                is_playing,
                self.config.pip_peeking_vinyl_side,
                self.config.vinyl_size_px,
                self.config.vinyl_speed_sec,
                self.config.show_vinyl_spindle,
                &accent,
            );
        }
        self.sync_vinyl_timer();
    }

    fn wave_or_hourglass_animating(&self) -> bool {
        self.timer.is_running()
            && (self.config.timer_theme == "wave" || self.config.timer_theme == "hourglass")
    }

    fn marquee_animating(&self) -> bool {
        self.track.is_some()
            && self.settings_tab.is_none()
            && (self.config.pip_mode || self.timer.phase() != SessionPhase::Idle)
    }

    fn icon_spin_animating(&self) -> bool {
        let now = Instant::now();
        self.completed_spin_until.is_some_and(|u| now < u)
            || self.guard_refresh_spin_until.is_some_and(|u| now < u)
    }

    fn spin_deg_until(until: Option<Instant>, now: Instant) -> f32 {
        const SPIN_MS: f32 = 450.0;
        let Some(end) = until else {
            return 0.0;
        };
        if now >= end {
            return 0.0;
        }
        let rem_ms = end.duration_since(now).as_secs_f32() * 1000.0;
        let t = (1.0 - rem_ms / SPIN_MS).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        eased * 360.0
    }

    fn sync_vinyl_timer(&mut self) {
        let peeking_busy = self
            .peeking_vinyl
            .as_ref()
            .is_some_and(|v| v.is_animating());
        let main_visible =
            unsafe { IsWindowVisible(self.hwnd).as_bool() && !IsIconic(self.hwnd).as_bool() };
        let main_busy = main_visible
            && (self.in_card_vinyl_animating()
                || (self.config.ambient_mode == AmbientMode::Default && self.timer.is_running())
                || self.wave_or_hourglass_animating()
                || self.marquee_animating()
                || self.icon_spin_animating());
        let active = peeking_busy || main_busy;
        if active == self.vinyl_timer_active {
            return;
        }
        unsafe {
            if active {
                SetTimer(Some(self.hwnd), TIMER_VINYL_ID, 20, None);
            } else {
                let _ = KillTimer(Some(self.hwnd), TIMER_VINYL_ID);
            }
        }
        self.vinyl_timer_active = active;
    }

    fn sync_schedule_to_timer(&mut self) {
        self.timer.set_schedule(
            self.config.total_focus_minutes,
            self.config.focus_minutes,
            self.config.break_minutes,
            self.config.skip_breaks,
        );
    }

    fn is_exe_in_blacklist(&self, exe: &str) -> bool {
        self.config
            .blacklist_apps
            .iter()
            .any(|b| b.eq_ignore_ascii_case(exe))
    }

    fn refresh_running_apps(&mut self) {
        let apps = enumerate_running_apps();
        for app in &apps {
            let key = if app.exe_path.is_empty() {
                app.exe_name.to_ascii_lowercase()
            } else {
                app.exe_path.to_ascii_lowercase()
            };
            let exe_low = app.exe_name.to_ascii_lowercase();
            let need_key = !self.app_icons.contains_key(&key);
            let need_exe = !exe_low.is_empty() && !self.app_icons_by_exe.contains_key(&exe_low);
            if need_key || need_exe {
                if let Some(frame) = extract_app_icon_frame(app.hwnd_raw, &app.exe_path) {
                    if need_key {
                        if let Ok(bmp) = upload_frame_bitmap(&self.surface.d2d_context, &frame) {
                            self.app_icons.insert(key, bmp);
                        }
                    }
                    if need_exe {
                        if let Ok(bmp) = upload_frame_bitmap(&self.surface.d2d_context, &frame) {
                            self.app_icons_by_exe.insert(exe_low, bmp);
                        }
                    }
                }
            }
        }
        self.running_apps = apps;
        let max_scroll = self.filtered_apps().len().saturating_sub(4);
        if self.app_scroll > max_scroll {
            self.app_scroll = max_scroll;
        }
    }

    fn filtered_apps(&self) -> Vec<(&str, &str, bool, Option<&ID2D1Bitmap1>)> {
        let query = self.search_query.trim().to_ascii_lowercase();
        let mut items: Vec<(&str, &str, bool, Option<&ID2D1Bitmap1>)> = Vec::new();
        for app in &self.running_apps {
            let blocked = self.is_exe_in_blacklist(&app.exe_name);
            let include = match self.app_filter {
                AppFilter::All => true,
                AppFilter::Blocked => blocked,
                AppFilter::Accessible => !blocked,
            };
            let matches_query = query.is_empty()
                || app.exe_name.to_ascii_lowercase().contains(&query)
                || app.title.to_ascii_lowercase().contains(&query);
            if include && matches_query {
                let bmp = if !app.exe_path.is_empty() {
                    self.app_icons.get(&app.exe_path.to_ascii_lowercase())
                } else {
                    self.app_icons.get(&app.exe_name.to_ascii_lowercase())
                };
                items.push((&app.exe_name, &app.title, blocked, bmp));
            }
        }
        if self.app_filter != AppFilter::Accessible {
            for b in &self.config.blacklist_apps {
                let already = self
                    .running_apps
                    .iter()
                    .any(|a| a.exe_name.eq_ignore_ascii_case(b));
                let matches_query =
                    query.is_empty() || b.to_ascii_lowercase().contains(&query);
                if !already && matches_query {
                    items.push((b.as_str(), "Not currently running", true, None));
                }
            }
        }
        match self.app_sort {
            AppSort::LatestOpen => {}
            AppSort::NameAsc => items.sort_by(|a, b| {
                a.0.to_ascii_lowercase()
                    .cmp(&b.0.to_ascii_lowercase())
                    .then_with(|| a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()))
            }),
            AppSort::BlockedFirst => items.sort_by(|a, b| {
                b.2.cmp(&a.2)
                    .then_with(|| a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()))
            }),
        }
        items
    }
}

fn sanitize_domain(raw: &str) -> String {
    let mut s = raw.trim().to_lowercase();
    if let Some(pos) = s.find("://") {
        s = s[pos + 3..].to_string();
    }
    if let Some(stripped) = s.strip_prefix("www.") {
        s = stripped.to_string();
    }
    if let Some(pos) = s.find('/') {
        s = s[..pos].to_string();
    }
    if let Some(pos) = s.find(':') {
        s = s[..pos].to_string();
    }
    s.trim().to_string()
}

impl AppState {
    fn site_entries(&self) -> Vec<String> {
        let mut entries: Vec<String> = COMMON_SITE_PRESETS.iter().map(|s| (*s).to_string()).collect();
        for s in &self.config.restricted_sites {
            if !entries.iter().any(|e| e.eq_ignore_ascii_case(s)) {
                entries.push(s.clone());
            }
        }
        entries
    }

    fn add_restricted_site(&mut self, raw: &str) {
        let domain = sanitize_domain(raw);
        if domain.is_empty() || !domain.contains('.') {
            self.status_hint = Some("Please enter a valid domain (e.g. netflix.com)".to_string());
            return;
        }
        if self.config.restricted_sites.iter().any(|d| d.eq_ignore_ascii_case(&domain)) {
            self.status_hint = Some(format!("{domain} is already in restricted sites"));
            return;
        }
        self.config.restricted_sites.push(domain.clone());
        self.guard.sync_from_config(&self.config);
        let _ = self.config.save();
        self.sync_bridge_snapshot();
        self.status_hint = Some(format!("Restricted {domain} (synced to extension)"));
        self.site_input_query.clear();
        self.site_input_focused = false;
        self.request_redraw();
    }

    fn layout_query(&self, w: f32, h: f32) -> LayoutQuery<'_> {
        let cap = blocked_apps_capacity(h);
        let filtered_len = self.filtered_apps().len();
        let start = self.app_scroll.min(filtered_len.saturating_sub(cap));
        let visible_app_rows = filtered_len.saturating_sub(start).min(cap);
        let (today, ..) = local_date_and_hour();
        let stats_app_rows = if self.show_stats_modal {
            period_app_breakdown(&self.stats, &today, self.stats_period)
                .len()
                .min(12)
        } else {
            0
        };
        let prayer_city_rows = if self.show_prayer_modal {
            self.filtered_prayer_cities().len()
        } else {
            0
        };
        LayoutQuery {
            w,
            h,
            config: &self.config,
            guard_tab: self.guard_tab,
            visible_app_rows,
            site_rows: self.site_entries().len(),
            site_scroll: self.site_scroll,
            settings_tab: self.settings_tab,
            settings_scroll: self.settings_scroll,
            show_ambient_mixer: self.show_ambient_mixer,
            show_stats_modal: self.show_stats_modal,
            show_share_modal: self.show_share_modal,
            show_goal_modal: self.show_goal_modal,
            show_prayer_modal: self.show_prayer_modal,
            prayer_city_rows,
            stats_app_rows,
            show_pip_drawer: self.show_pip_gif_drawer,
            has_track: self.track.is_some(),
            phase: self.timer.phase(),
        }
    }

    fn request_redraw(&self) {
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    fn render_now(&self) {
        let ends_at = self.ends_at_label();
        let (local_clock_text, local_date_text) = local_clock_and_date();
        let st = unsafe { GetLocalTime() };
        let today_date = format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay);
        let cur_hour = st.wHour as usize;
        let active_exe = self
            .active_app
            .as_ref()
            .map(|a| a.exe_name.as_str())
            .unwrap_or("");
        let active_pass_count = self
            .guard
            .restricted_sites
            .iter()
            .filter(|s| s.pass_remaining_sec > 0)
            .count();
        let bridge_port = self.http_bridge.as_ref().map(|b| b.bound_port).unwrap_or(0);

        let (_, h_dip) = self.surface.dip_size();
        let cap = blocked_apps_capacity(h_dip);
        let all_apps = self.filtered_apps();
        let total_matching_apps = all_apps.len();
        let start = self.app_scroll.min(total_matching_apps.saturating_sub(cap));
        let visible_apps: Vec<RunningAppView<'_>> = all_apps
            .into_iter()
            .skip(start)
            .take(cap)
            .map(|(exe_name, title, blocked, icon)| RunningAppView {
                exe_name,
                title,
                blocked,
                icon,
            })
            .collect();

        let stats_raw = if self.show_stats_modal {
            period_app_breakdown(&self.stats, &today_date, self.stats_period)
        } else {
            Vec::new()
        };
        let stats_apps = self.build_stats_apps(&stats_raw);

        self.with_render_view(&visible_apps, start, total_matching_apps, &stats_apps, &today_date, cur_hour, &ends_at, &local_clock_text, &local_date_text, active_exe, active_pass_count, bridge_port, |view| {
            if let Err(err) = self.surface.render(|dc, factory, dwrite, w, h| {
                render_ui(dc, factory, dwrite, w, h, view)?;
                Ok(())
            }) {
                eprintln!("[FocusGrow] render error: {err:?}");
            }
        });
    }

    fn build_stats_apps<'a>(&'a self, stats_raw: &'a [(String, u32)]) -> Vec<StatsAppRow<'a>> {
        stats_raw
            .iter()
            .take(12)
            .map(|(exe, secs)| {
                let low = exe.to_ascii_lowercase();
                let icon = if low == "focusgrow.exe" || low == "focusgrow-app.exe" {
                    self.app_icon_logo.as_ref()
                } else {
                    self.app_icons_by_exe.get(&low)
                };
                StatsAppRow {
                    exe_name: exe.as_str(),
                    secs: *secs,
                    category: self.config.app_category(exe),
                    is_blocked: self.is_exe_in_blacklist(exe),
                    icon,
                }
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn with_render_view<R, F: FnOnce(&RenderView<'_>) -> R>(
        &self,
        visible_apps: &[RunningAppView<'_>],
        app_scroll: usize,
        total_matching_apps: usize,
        stats_apps: &[StatsAppRow<'_>],
        today_date: &str,
        cur_hour: usize,
        ends_at: &str,
        local_clock_text: &str,
        local_date_text: &str,
        active_exe: &str,
        active_pass_count: usize,
        bridge_port: u16,
        f: F,
    ) -> R {
        let prayer_cities = if self.show_prayer_modal {
            self.filtered_prayer_cities()
        } else {
            Vec::new()
        };
        let spin_now = Instant::now();
        let completed_spin_deg = Self::spin_deg_until(self.completed_spin_until, spin_now);
        let guard_refresh_spin_deg = Self::spin_deg_until(self.guard_refresh_spin_until, spin_now);
        let view = RenderView {
            config: &self.config,
            timer: &self.timer,
            app_icon_logo: self.app_icon_logo.as_ref(),
            gif_anim: self.gpu_anim.as_ref(),
            gif_frame_index: self.frame_idx,
            plant_sway_phase: self.plant_sway_phase,
            wave_phase: self.wave_phase,
            marquee_offset: self.marquee_offset,
            ambient_playing: self.audio.is_playing(),
            ambient_muted: self.audio.is_master_muted(),
            cover_bitmap: self.cover_bitmap.as_ref(),
            in_card_vinyl_angle: self.in_card_vinyl_angle,
            track: self.track.as_ref(),
            playback: self.playback,
            ytmpx_connected: self.ytmpx_connected,
            effective_accent: self.effective_accent(),
            ends_at_text: ends_at,
            guard_tab: self.guard_tab,
            app_filter: self.app_filter,
            app_sort: self.app_sort,
            search_query: &self.search_query,
            search_focused: self.search_focused,
            running_apps: visible_apps,
            app_scroll,
            total_matching_apps,
            settings_tab: self.settings_tab,
            settings_scroll: self.settings_scroll,
            show_ambient_mixer: self.show_ambient_mixer,
            show_pip_gif_drawer: self.show_pip_gif_drawer,
            hovered: self.hovered,
            active_num_edit: self
                .num_edit_field
                .map(|fld| (fld, self.num_edit_buf.as_str())),
            status_hint: self.status_hint.as_deref(),
            daily_completed_mins: self.stats.completed_minutes_today,
            yesterday_mins: self.stats.yesterday_minutes,
            streak_days: self.stats.streak_days,
            stats: &self.stats,
            today_date,
            cur_hour,
            show_stats_modal: self.show_stats_modal,
            show_share_modal: self.show_share_modal,
            share_yesterday: self.share_yesterday,
            show_goal_modal: self.show_goal_modal,
            show_prayer_modal: self.show_prayer_modal,
            prayer_search_query: &self.prayer_search_query,
            prayer_search_focused: self.prayer_search_focused,
            prayer_city_results: &prayer_cities,
            prayer_loc_busy: self.prayer_loc_busy,
            prayer_loc_status: self.prayer_loc_status.as_deref(),
            stats_period: self.stats_period,
            heatmap_offset_months: self.heatmap_offset_months,
            stats_apps,
            app_icons_map: &self.app_icons_by_exe,
            cursor_dip: self.cursor_dip,
            next_prayer_label: &self.next_prayer_label,
            local_clock_text,
            local_date_text,
            prayer_view_idx: self.prayer_view_idx,
            prayer_times: &self.prayer_times,
            bridge_port,
            lan_ip: &self.lan_ip,
            qr_matrix: self.qr_matrix.as_ref(),
            active_exe,
            active_domain: &self.active_domain,
            active_pass_count,
            site_input_focused: self.site_input_focused,
            site_input_query: &self.site_input_query,
            site_scroll: self.site_scroll,
            block_countdown_sec: self.block_countdown.map(|(_, s)| s),
            is_auto_paused: self.is_auto_paused,
            completed_spin_deg,
            guard_refresh_spin_deg,
        };
        f(&view)
    }

    fn run_with_minimal_view<R, F: FnOnce(&RenderView<'_>) -> R>(&self, f: F) -> R {
        let st = unsafe { GetLocalTime() };
        let today_date = format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay);
        let cur_hour = st.wHour as usize;
        let (local_clock_text, local_date_text) = local_clock_and_date();
        let ends_at = self.ends_at_label();
        let bridge_port = self.http_bridge.as_ref().map(|b| b.bound_port).unwrap_or(0);
        self.with_render_view(
            &[],
            0,
            0,
            &[],
            &today_date,
            cur_hour,
            &ends_at,
            &local_clock_text,
            &local_date_text,
            "",
            0,
            bridge_port,
            f,
        )
    }

    fn handle_ytmpx_events(&mut self) {
        let updates = if let Some(ref bridge) = self.ytmpx {
            bridge.drain_updates()
        } else {
            return;
        };

        let mut need_redraw = false;
        let mut need_vinyl_sync = false;

        for upd in updates {
            match upd {
                YtmpxUpdate::ConnectionChanged(conn) => {
                    self.ytmpx_connected = conn;
                    need_redraw = true;
                }
                YtmpxUpdate::TrackChanged { track, playback } => {
                    let prev_art = self.track.as_ref().map(|t| t.art_url.clone()).unwrap_or_default();
                    let prev_title = self.track.as_ref().map(|t| t.title.clone()).unwrap_or_default();
                    let next_art = track.as_ref().map(|t| t.art_url.clone()).unwrap_or_default();
                    let next_title = track.as_ref().map(|t| t.title.clone()).unwrap_or_default();
                    if prev_title != next_title {
                        self.marquee_offset = 0.0;
                    }
                    self.track = track;
                    self.playback = playback;
                    if self.track.is_none() {
                        self.cover_frame = None;
                        self.cover_bitmap = None;
                        self.cached_dynamic_accent = None;
                        if let Some(ref mut v) = self.peeking_vinyl {
                            v.set_cover_frame(None, false);
                        }
                    } else if next_art.is_empty() && !prev_art.is_empty() {
                        self.cover_frame = None;
                        self.cover_bitmap = None;
                        self.cached_dynamic_accent = None;
                    }
                    need_redraw = true;
                    need_vinyl_sync = true;
                }
                YtmpxUpdate::PlaybackChanged(playback) => {
                    self.playback = playback;
                    need_redraw = true;
                    need_vinyl_sync = true;
                }
                YtmpxUpdate::CoverLoaded { art_url, frame } => {
                    let matches_current = self
                        .track
                        .as_ref()
                        .is_some_and(|t| t.art_url == art_url);
                    if !matches_current {
                        continue;
                    }
                    self.cover_frame = frame.clone();
                    self.cover_bitmap = frame
                        .as_ref()
                        .and_then(|f| upload_frame_bitmap(&self.surface.d2d_context, f).ok());
                    self.recompute_dynamic_accent();
                    let was_pip = self.config.pip_mode && self.config.pip_peeking_vinyl_enabled;
                    if let Some(ref mut vinyl) = self.peeking_vinyl {
                        vinyl.set_cover_frame(frame, was_pip);
                    }
                    need_redraw = true;
                    need_vinyl_sync = true;
                }
            }
        }

        if need_vinyl_sync {
            self.last_vinyl_tick = Instant::now();
            self.sync_vinyl_overlay();
        }
        if need_redraw {
            self.request_redraw();
        }
    }

    fn commit_numeric_edit(&mut self) {
        let Some(field) = self.num_edit_field.take() else {
            return;
        };
        let buf = std::mem::take(&mut self.num_edit_buf);
        self.num_edit_fresh = false;
        if let Ok(val) = buf.trim().parse::<u32>() {
            match field {
                NumericEditField::TotalMins => {
                    self.config.total_focus_minutes = val.clamp(1, 720);
                }
                NumericEditField::PeriodMins => {
                    self.config.focus_minutes = val.clamp(1, 180);
                }
                NumericEditField::BreakMins => {
                    self.config.break_minutes = val.clamp(1, 60);
                }
            }
            self.config.sanitize();
            self.sync_schedule_to_timer();
            let _ = self.config.save();
        }
        self.request_redraw();
    }

    fn on_window_blur(&mut self) {
        if self.config.pip_mode
            || !self.config.auto_pip_on_blur
            || self.is_closing
            || self.in_file_dialog
            || self.timer.phase() != SessionPhase::Focusing
            || self.last_pip_toggle.elapsed() < Duration::from_millis(350)
        {
            return;
        }
        if unsafe { IsIconic(self.hwnd).as_bool() || !IsWindowVisible(self.hwnd).as_bool() } {
            return;
        }
        self.settings_tab = None;
        self.show_ambient_mixer = false;
        self.show_prayer_modal = false;
        self.prayer_search_focused = false;
        self.sync_vinyl_timer();
        self.toggle_pip();
    }

    fn toggle_pip(&mut self) {
        self.last_pip_toggle = Instant::now();
        let mut rc = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut rc);
        }
        let scale = self.dpi_scale();

        if !self.config.pip_mode {
            self.normal_rect_dip = (
                (rc.left as f32 / scale).round() as i32,
                (rc.top as f32 / scale).round() as i32,
                ((rc.right - rc.left) as f32 / scale).round() as i32,
                ((rc.bottom - rc.top) as f32 / scale).round() as i32,
            );
            self.config.pip_mode = true;
            self.is_maximized = false;
            self.show_pip_gif_drawer = false;

            let target_w = if self.config.pip_style == PipStyle::Capsule {
                self.dip_to_px(self.config.pip_capsule_width.clamp(195, 520))
            } else {
                self.dip_to_px(self.config.pip_width.max(180))
            };
            let target_h = if self.config.pip_style == PipStyle::Capsule {
                self.dip_to_px(self.config.pip_capsule_height.max(68))
            } else {
                self.dip_to_px(self.config.pip_height.max(220))
            };
            let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            let (target_x, target_y) = clamp_window_pos_to_virtual_screen(
                self.config
                    .pip_x
                    .map(|x| self.dip_to_px(x))
                    .unwrap_or_else(|| (screen_w - target_w - 36).max(0)),
                self.config
                    .pip_y
                    .map(|y| self.dip_to_px(y))
                    .unwrap_or_else(|| (screen_h - target_h - 72).max(0)),
                target_w,
                target_h,
            );

            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    target_x,
                    target_y,
                    target_w,
                    target_h,
                    SWP_NOACTIVATE,
                );
            }
            self.apply_window_chrome();
        } else {
            self.config.pip_x = Some((rc.left as f32 / scale).round() as i32);
            self.config.pip_y = Some((rc.top as f32 / scale).round() as i32);
            if self.config.pip_style == PipStyle::Card {
                self.config.pip_width = (((rc.right - rc.left) as f32 / scale).round() as i32).max(180);
                self.config.pip_height = (((rc.bottom - rc.top) as f32 / scale).round() as i32).max(220);
            } else {
                self.config.pip_capsule_width = (((rc.right - rc.left) as f32 / scale).round() as i32).clamp(195, 520);
                self.config.pip_capsule_height = (((rc.bottom - rc.top) as f32 / scale).round() as i32).max(68);
            }
            self.config.pip_mode = false;
            self.show_pip_gif_drawer = false;

            let (nx, ny, nw, nh) = self.normal_rect_dip;
            self.apply_window_chrome();
            unsafe {
                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_NOTOPMOST),
                    self.dip_to_px(nx),
                    self.dip_to_px(ny),
                    self.dip_to_px(nw),
                    self.dip_to_px(nh),
                    SWP_NOACTIVATE,
                );
            }
        }

        let _ = self.config.save();
        self.sync_vinyl_overlay();
        self.request_redraw();
    }

    fn toggle_pip_style(&mut self) {
        if !self.config.pip_mode {
            return;
        }
        self.config.pip_style = match self.config.pip_style {
            PipStyle::Card => PipStyle::Capsule,
            PipStyle::Capsule => PipStyle::Card,
        };
        let _ = self.config.save();
        self.apply_pip_geometry();
        self.sync_vinyl_overlay();
        self.request_redraw();
    }

    fn apply_pip_geometry(&mut self) {
        if !self.config.pip_mode {
            return;
        }
        let target_w = if self.config.pip_style == PipStyle::Capsule {
            self.dip_to_px(self.config.pip_capsule_width.clamp(195, 520))
        } else {
            self.dip_to_px(self.config.pip_width.max(180))
        };
        let target_h = if self.config.pip_style == PipStyle::Capsule {
            self.dip_to_px(self.config.pip_capsule_height.max(68))
        } else {
            self.dip_to_px(self.config.pip_height.max(220))
        };
        let mut rc = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut rc);
        }
        let (clamped_x, clamped_y) =
            clamp_window_pos_to_virtual_screen(rc.left, rc.top, target_w, target_h);
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                clamped_x,
                clamped_y,
                target_w,
                target_h,
                SWP_NOACTIVATE,
            );
        }
        let _ = self.surface.resize(target_w as u32, target_h as u32, self.surface.dpi);
    }

    fn apply_window_chrome(&self) {
        unsafe {
            let mut ex = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32;
            if self.config.pip_mode && self.config.hide_taskbar_in_pip {
                ex |= WS_EX_TOOLWINDOW.0;
                ex &= !WS_EX_APPWINDOW.0;
            } else {
                ex |= WS_EX_APPWINDOW.0;
                ex &= !WS_EX_TOOLWINDOW.0;
            }
            if self.config.pip_mode {
                ex |= WS_EX_TOPMOST.0;
            } else {
                ex &= !WS_EX_TOPMOST.0;
            }
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex as isize);
            let _ = SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
            );
        }
    }

    fn update_slider_from_x(&mut self, x_dip: f32) {
        let Some(which) = self.dragging_slider else {
            return;
        };
        let (w_dip, h_dip) = self.surface.dip_size();
        let layout = compute_layout(&self.layout_query(w_dip, h_dip));
        match which {
            DragSlider::Opacity => {
                if let Some(slider) = layout.opacity_slider_rect {
                    let norm = ((x_dip - slider.x) / slider.w.max(1.0)).clamp(0.0, 1.0);
                    let next = (15.0 + norm * 85.0).round() as u8;
                    if next != self.config.gif_opacity {
                        self.config.gif_opacity = next;
                        self.render_now();
                    }
                }
            }
            DragSlider::VinylSpeed => {
                if let Some(slider) = layout.hit_rects.iter().rev().find(|h| h.id == ControlId::VinylSpeedSlider).map(|h| h.rect) {
                    let norm = ((x_dip - slider.x) / slider.w.max(1.0)).clamp(0.0, 1.0);
                    let next = (1.0 + norm * 29.0).round();
                    if (next - self.config.vinyl_speed_sec).abs() > 0.1 {
                        self.config.vinyl_speed_sec = next;
                        self.sync_vinyl_overlay();
                        self.render_now();
                    }
                }
            }
            DragSlider::VinylSize => {
                if let Some(slider) = layout.hit_rects.iter().rev().find(|h| h.id == ControlId::VinylSizeSlider).map(|h| h.rect) {
                    let norm = ((x_dip - slider.x) / slider.w.max(1.0)).clamp(0.0, 1.0);
                    let steps = ((norm * (850.0 - 120.0)) / 10.0).round() * 10.0;
                    let next = (120 + steps as i32).clamp(120, 850);
                    if next != self.config.vinyl_size_px {
                        self.config.vinyl_size_px = next;
                        self.sync_vinyl_overlay();
                        self.render_now();
                    }
                }
            }
            DragSlider::AmbientMaster => {
                if let Some(slider) = layout.hit_rects.iter().rev().find(|h| h.id == ControlId::AmbientMasterSlider).map(|h| h.rect) {
                    let norm = ((x_dip - slider.x) / slider.w.max(1.0)).clamp(0.0, 1.0);
                    let next = (norm * 100.0).round() as u8;
                    if next != self.config.ambient.master_volume {
                        self.config.ambient.master_volume = next;
                        self.audio.sync(&self.config.ambient);
                        self.render_now();
                    }
                }
            }
            DragSlider::AmbientTrack(idx) => {
                if let Some(slider) = layout.hit_rects.iter().rev().find(|h| h.id == ControlId::AmbientTrackSlider(idx)).map(|h| h.rect) {
                    if let Some(meta) = AMBIENT_TRACKS.get(idx) {
                        let norm = ((x_dip - slider.x) / slider.w.max(1.0)).clamp(0.0, 1.0);
                        let next = (norm * 100.0).round() as u8;
                        if let Some(tr) = self.config.ambient.track_mut(meta.id) {
                            if next != tr.volume {
                                tr.volume = next;
                                let slot = if self.config.ambient.active_preset == "custom_2" { 1 } else { 0 };
                                self.config.ambient.save_current_as_custom(slot);
                                self.audio.sync(&self.config.ambient);
                                self.render_now();
                            }
                        }
                    }
                }
            }
        }
    }

    fn import_custom_audio(&mut self, slot: &str) {
        self.in_file_dialog = true;
        let picked = file_dialog::pick_audio_file(self.hwnd);
        self.in_file_dialog = false;
        if let Ok(Some(src_path)) = picked {
            // Stop any currently playing audio so file handles are released
            focusgrow_audio::stop_custom_audio();

            let orig_name = src_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("custom_audio.wav")
                .to_string();
            let sounds_dir = AppConfig::sounds_dir();
            if std::fs::create_dir_all(&sounds_dir).is_ok() {
                let ext = src_path.extension().and_then(|e| e.to_str()).unwrap_or("wav");
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let dest = sounds_dir.join(format!("{slot}_{stamp}.{ext}"));
                match std::fs::copy(&src_path, &dest) {
                    Ok(_) => {
                        let dest_str = dest.to_string_lossy().to_string();
                        let prev_path = match slot {
                            "focus" => self.config.custom_focus_audio_path.take(),
                            "break" => self.config.custom_break_audio_path.take(),
                            _ => self.config.custom_warning_audio_path.take(),
                        };
                        if let Some(prev) = prev_path {
                            let _ = std::fs::remove_file(prev);
                        }

                        match slot {
                            "focus" => {
                                self.config.custom_focus_audio_path = Some(dest_str);
                                self.config.custom_focus_audio_name = Some(orig_name.clone());
                                self.config.focus_sound = "custom".to_string();
                            }
                            "break" => {
                                self.config.custom_break_audio_path = Some(dest_str);
                                self.config.custom_break_audio_name = Some(orig_name.clone());
                                self.config.break_sound = "custom".to_string();
                            }
                            _ => {
                                self.config.custom_warning_audio_path = Some(dest_str);
                                self.config.custom_warning_audio_name = Some(orig_name.clone());
                                self.config.warning_sound = "custom".to_string();
                            }
                        }
                        let _ = self.config.save();
                        self.status_hint = Some(format!("Loaded custom audio: {orig_name}"));
                    }
                    Err(err) => {
                        self.status_hint = Some(format!("Audio import failed: {err}"));
                    }
                }
            }
            self.request_redraw();
        }
    }

    fn handle_click(&mut self, id: ControlId, x_dip: f32) {
        let now = Instant::now();
        if id != ControlId::BtnAppSearchBox && self.search_focused {
            self.search_focused = false;
        }
        if id != ControlId::BtnSiteSearchBox && id != ControlId::BtnSiteAdd && id != ControlId::BtnSiteClear && self.site_input_focused {
            self.site_input_focused = false;
        }
        let target_num = match id {
            ControlId::InputTotalMins => Some(NumericEditField::TotalMins),
            ControlId::InputPeriodMins => Some(NumericEditField::PeriodMins),
            ControlId::InputBreakMins => Some(NumericEditField::BreakMins),
            _ => None,
        };
        if self.num_edit_field != target_num {
            self.commit_numeric_edit();
        }
        match id {
            ControlId::DragBar => unsafe {
                let _ = ReleaseCapture();
                let _ = PostMessageW(
                    Some(self.hwnd),
                    WM_SYSCOMMAND,
                    WPARAM((SC_MOVE | 0x0002) as usize),
                    LPARAM(0),
                );
            },
            ControlId::BtnMinimize => unsafe {
                let _ = ShowWindow(self.hwnd, SW_MINIMIZE);
            },
            ControlId::BtnMaximizeRestore => {
                if !self.config.pip_mode {
                    unsafe {
                        let cmd = if self.is_maximized { SW_RESTORE } else { SW_MAXIMIZE };
                        let _ = ShowWindow(self.hwnd, cmd);
                    }
                    self.is_maximized = !self.is_maximized;
                }
            }
            ControlId::BtnClose => {
                self.persist_session_now();
                if self.config.minimize_to_tray {
                    let accent = self.effective_accent().to_string();
                    unsafe {
                        let _ = ShowWindow(self.hwnd, SW_HIDE);
                    }
                    if let Some(ref mut v) = self.peeking_vinyl {
                        v.update_settings(
                            false,
                            false,
                            false,
                            self.config.pip_peeking_vinyl_side,
                            self.config.vinyl_size_px,
                            self.config.vinyl_speed_sec,
                            self.config.show_vinyl_spindle,
                            &accent,
                        );
                    }
                } else {
                    self.is_closing = true;
                    self.force_quit = true;
                    unsafe {
                        let _ = DestroyWindow(self.hwnd);
                    }
                }
            }
            ControlId::BtnTogglePip => self.toggle_pip(),
            ControlId::BtnTogglePipStyle => self.toggle_pip_style(),
            ControlId::BtnPipStyleCard => {
                if self.config.pip_style != PipStyle::Card {
                    self.config.pip_style = PipStyle::Card;
                    let _ = self.config.save();
                    if self.config.pip_mode {
                        self.apply_pip_geometry();
                        self.sync_vinyl_overlay();
                    }
                    self.request_redraw();
                }
            }
            ControlId::BtnPipStyleCapsule => {
                if self.config.pip_style != PipStyle::Capsule {
                    self.config.pip_style = PipStyle::Capsule;
                    let _ = self.config.save();
                    if self.config.pip_mode {
                        self.apply_pip_geometry();
                        self.sync_vinyl_overlay();
                    }
                    self.request_redraw();
                }
            }
            ControlId::BtnToggleCapsuleAlbumBg => {
                self.config.pip_capsule_album_bg = !self.config.pip_capsule_album_bg;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleCapsuleAlbumSpin => {
                self.config.pip_capsule_album_spin = !self.config.pip_capsule_album_spin;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnCapsuleAlbumOpacity => {
                self.config.pip_capsule_album_opacity = match self.config.pip_capsule_album_opacity {
                    0..=25 => 35,
                    26..=40 => 50,
                    41..=65 => 75,
                    66..=85 => 100,
                    _ => 20,
                };
                self.config.sanitize();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnTopToolbarPip => {
                self.config.auto_pip_on_blur = !self.config.auto_pip_on_blur;
                self.status_hint = None;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::InputTotalMins => {
                self.num_edit_field = Some(NumericEditField::TotalMins);
                self.num_edit_buf = self.config.total_focus_minutes.to_string();
                self.num_edit_fresh = true;
                self.request_redraw();
            }
            ControlId::InputPeriodMins => {
                self.num_edit_field = Some(NumericEditField::PeriodMins);
                self.num_edit_buf = self.config.focus_minutes.to_string();
                self.num_edit_fresh = true;
                self.request_redraw();
            }
            ControlId::InputBreakMins => {
                self.num_edit_field = Some(NumericEditField::BreakMins);
                self.num_edit_buf = self.config.break_minutes.to_string();
                self.num_edit_fresh = true;
                self.request_redraw();
            }
            ControlId::BtnTopToolbarSound => {
                self.show_ambient_mixer = !self.show_ambient_mixer;
                if self.show_ambient_mixer {
                    self.settings_tab = None;
                    self.show_stats_modal = false;
                    self.show_share_modal = false;
                    self.show_goal_modal = false;
                    self.show_prayer_modal = false;
                    self.prayer_search_focused = false;
                    self.settings_scroll = 0.0;
                }
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnOpenStats => {
                self.show_stats_modal = !self.show_stats_modal;
                self.show_share_modal = false;
                if self.show_stats_modal {
                    self.settings_tab = None;
                    self.show_ambient_mixer = false;
                    self.show_goal_modal = false;
                    self.show_prayer_modal = false;
                    self.prayer_search_focused = false;
                    self.settings_scroll = 0.0;
                }
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnOpenShareCard => {
                self.share_yesterday = self.stats_period == StatsPeriod::Yesterday;
                self.show_share_modal = true;
                self.request_redraw();
            }
            ControlId::BtnShareDayToday => {
                self.share_yesterday = false;
                self.request_redraw();
            }
            ControlId::BtnShareDayYesterday => {
                self.share_yesterday = true;
                self.request_redraw();
            }
            ControlId::BtnShareCopyText => {
                let txt = self.run_with_minimal_view(|v| build_share_report_text(v, self.share_yesterday));
                if copy_text_to_clipboard(&txt) {
                    self.status_hint = Some("Copied Share Report text to clipboard!".to_string());
                } else {
                    self.status_hint = Some("Failed to copy report text".to_string());
                }
                self.request_redraw();
            }
            ControlId::BtnShareCopyImage => {
                let tmp_png = std::env::temp_dir().join("focusgrow_share_card.png");
                let res = self.run_with_minimal_view(|v| {
                    export_share_card_png(
                        &self.surface.d2d_context,
                        &self.surface.d2d_factory,
                        &self.surface.dwrite_factory,
                        v,
                        &tmp_png,
                    )
                });
                match res {
                    Ok(()) if copy_image_file_to_clipboard(&tmp_png) => {
                        self.status_hint = Some("Share Card image copied to clipboard!".to_string());
                    }
                    Ok(()) => {
                        self.status_hint = Some("Failed copying image to clipboard".to_string());
                    }
                    Err(err) => {
                        self.status_hint = Some(format!("Share Card error: {err}"));
                    }
                }
                self.request_redraw();
            }
            ControlId::BtnShareSavePng => {
                let (today, ..) = local_date_and_hour();
                let target_date = if self.share_yesterday {
                    focusgrow_core::stats::prev_iso_date(&today)
                } else {
                    today
                };
                let default_name = format!("FocusGrow_Report_{target_date}.png");
                self.in_file_dialog = true;
                let picked = file_dialog::save_png_file(self.hwnd, &default_name);
                self.in_file_dialog = false;
                if let Ok(Some(out_path)) = picked {
                    let res = self.run_with_minimal_view(|v| {
                        export_share_card_png(
                            &self.surface.d2d_context,
                            &self.surface.d2d_factory,
                            &self.surface.dwrite_factory,
                            v,
                            &out_path,
                        )
                    });
                    match res {
                        Ok(()) => {
                            self.status_hint = Some(format!("Saved PNG: {}", out_path.display()));
                        }
                        Err(err) => {
                            self.status_hint = Some(format!("Save PNG error: {err}"));
                        }
                    }
                    self.request_redraw();
                }
            }
            ControlId::BtnEditGoal => {
                self.show_goal_modal = !self.show_goal_modal;
                if self.show_goal_modal {
                    self.settings_tab = None;
                    self.show_ambient_mixer = false;
                    self.show_stats_modal = false;
                    self.show_share_modal = false;
                    self.show_prayer_modal = false;
                    self.prayer_search_focused = false;
                }
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnGoalHours(hrs) => {
                self.config.daily_goal_minutes = (hrs * 60).clamp(60, 720);
                let (today, ..) = local_date_and_hour();
                self.stats
                    .recalculate_streaks(&today, self.config.daily_goal_minutes);
                let _ = self.config.save();
                let _ = self.stats.save();
                self.request_redraw();
            }
            ControlId::BtnHealthGoalBreaks(breaks) => {
                self.stats.health_daily_goal_breaks = breaks.clamp(1, 20);
                let (today, ..) = local_date_and_hour();
                self.stats
                    .recalculate_streaks(&today, self.config.daily_goal_minutes);
                let _ = self.stats.save();
                self.request_redraw();
            }
            ControlId::BtnStatsPeriod(p) => {
                self.stats_period = p;
                self.settings_scroll = 0.0;
                self.request_redraw();
            }
            ControlId::BtnHeatmapPrev => {
                self.heatmap_offset_months = (self.heatmap_offset_months + 1).min(12);
                self.request_redraw();
            }
            ControlId::BtnHeatmapNext => {
                self.heatmap_offset_months = self.heatmap_offset_months.saturating_sub(1);
                self.request_redraw();
            }
            ControlId::BtnBlockAppFromStats(idx) => {
                let (today, ..) = local_date_and_hour();
                let apps = period_app_breakdown(&self.stats, &today, self.stats_period);
                if let Some((exe_owned, _)) = apps.get(idx) {
                    if let Some(pos) = self
                        .config
                        .blacklist_apps
                        .iter()
                        .position(|b| b.eq_ignore_ascii_case(exe_owned))
                    {
                        self.config.blacklist_apps.remove(pos);
                        self.status_hint = Some(format!("Unblocked {exe_owned}"));
                    } else if !exe_owned.eq_ignore_ascii_case("focusgrow-app.exe") {
                        self.config.blacklist_apps.push(exe_owned.clone());
                        self.status_hint = Some(format!("Blocked {exe_owned}"));
                    }
                    self.guard.sync_from_config(&self.config);
                    let _ = self.config.save();
                    self.evaluate_foreground_guard();
                    self.sync_bridge_snapshot();
                    self.request_redraw();
                }
            }
            ControlId::BtnCycleAppCategory(idx) => {
                let (today, ..) = local_date_and_hour();
                let apps = period_app_breakdown(&self.stats, &today, self.stats_period);
                if let Some((exe_owned, _)) = apps.get(idx) {
                    let next = self.config.cycle_app_category(exe_owned);
                    let _ = self.config.save();
                    self.status_hint = Some(format!("{exe_owned} set to {}", next.to_uppercase()));
                    self.request_redraw();
                }
            }
            ControlId::BtnModeFocus => {
                self.config.is_health_mode = false;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnModeHealth => {
                self.config.is_health_mode = true;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnTotalUp => {
                let step = self.config.focus_minutes.max(1);
                let cur = self.config.total_focus_minutes;
                let next = if cur < step {
                    let delta = if cur < 5 { 1 } else { 5 };
                    (cur + delta).min(step)
                } else {
                    let candidate = (cur / step) * step + step;
                    candidate.min(720)
                };
                self.config.total_focus_minutes = next.clamp(1, 720);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnTotalDown => {
                let step = self.config.focus_minutes.max(1);
                let cur = self.config.total_focus_minutes;
                let next = if cur <= step {
                    if cur <= 5 {
                        cur.saturating_sub(1)
                    } else {
                        cur.saturating_sub(5)
                    }
                } else {
                    let mut prev = ((cur + step - 1) / step) * step - step;
                    if prev >= cur {
                        prev = prev.saturating_sub(step);
                    }
                    prev.max(step)
                };
                self.config.total_focus_minutes = next.clamp(1, 720);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnChunkUp => {
                let cur = self.config.focus_minutes;
                let next = if cur < 5 { cur + 1 } else { cur + 5 };
                self.config.focus_minutes = next.clamp(1, 180);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnChunkDown => {
                let cur = self.config.focus_minutes;
                let next = if cur <= 5 { cur.saturating_sub(1) } else { cur.saturating_sub(5) };
                self.config.focus_minutes = next.clamp(1, 180);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnBreakUp => {
                let cur = self.config.break_minutes;
                let next = if cur < 5 { cur + 1 } else { cur + 5 };
                self.config.break_minutes = next.clamp(1, 60);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnBreakDown => {
                let cur = self.config.break_minutes;
                let next = if cur <= 5 { cur.saturating_sub(1) } else { cur.saturating_sub(5) };
                self.config.break_minutes = next.clamp(1, 60);
                self.config.sanitize();
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleSkipBreaks => {
                self.config.skip_breaks = !self.config.skip_breaks;
                self.sync_schedule_to_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnPlayPause => {
                let starting_idle = self.timer.phase() == SessionPhase::Idle;
                self.is_auto_paused = false;
                self.status_hint = None;
                self.timer.toggle_pause(now);
                if starting_idle {
                    self.warned_5m_phase = None;
                    self.warned_1m_phase = None;
                    if self.config.is_health_mode {
                        self.notify_windows(
                            "Posture Guard Started",
                            "Session started! Relax your shoulders and maintain good posture.",
                        );
                    } else {
                        self.notify_windows(
                            "Focus Mode Started",
                            &format!(
                                "Session started! Period 1 of {}. Stay focused!",
                                self.timer.total_periods().max(1)
                            ),
                        );
                    }
                }
                if starting_idle && self.config.auto_pip && !self.config.pip_mode {
                    self.toggle_pip();
                }
                if starting_idle
                    && self.timer.is_running()
                    && self.config.ambient.enabled
                    && self.config.ambient.auto_play
                    && !self.audio.is_playing()
                {
                    self.audio.set_playing(true, &self.config.ambient);
                }
                self.evaluate_foreground_guard();
                self.persist_session_now();
                self.sync_overlays();
                self.sync_vinyl_timer();
                self.sync_sec_timer();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnReset => {
                self.is_auto_paused = false;
                self.block_countdown = None;
                self.timer.stop();
                PersistedSession::clear();
                self.guard.reset_session_passes();
                if self.audio.is_playing() {
                    self.audio.set_playing(false, &self.config.ambient);
                }
                self.sync_overlays();
                self.sync_vinyl_timer();
                self.sync_sec_timer();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnSkip => {
                self.is_auto_paused = false;
                self.block_countdown = None;
                self.timer.skip_phase(now);
                self.evaluate_foreground_guard();
                self.persist_session_now();
                self.sync_overlays();
                self.sync_sec_timer();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnTopToolbarGif => {
                if self.settings_tab == Some(SettingsTab::Appearance) && self.settings_scroll > 240.0 {
                    self.settings_tab = None;
                } else {
                    self.settings_tab = Some(SettingsTab::Appearance);
                    self.settings_scroll = 385.0;
                }
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnTopToolbarAppearance => {
                self.show_ambient_mixer = false;
                self.show_stats_modal = false;
                self.show_share_modal = false;
                self.show_goal_modal = false;
                self.show_prayer_modal = false;
                self.prayer_search_focused = false;
                if self.settings_tab.is_some() {
                    self.settings_tab = None;
                } else {
                    let tab = SettingsTab::from_key(&self.config.last_settings_tab);
                    self.settings_tab = Some(tab);
                    self.settings_scroll = 0.0;
                }
                self.last_vinyl_tick = Instant::now();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnTopToolbarMusic => {
                self.show_ambient_mixer = false;
                self.show_stats_modal = false;
                self.show_share_modal = false;
                self.show_goal_modal = false;
                self.show_prayer_modal = false;
                self.prayer_search_focused = false;
                if self.settings_tab == Some(SettingsTab::MobileSync) {
                    self.settings_tab = None;
                } else {
                    self.settings_tab = Some(SettingsTab::MobileSync);
                    self.config.last_settings_tab = SettingsTab::MobileSync.key().to_string();
                    let _ = self.config.save();
                    self.settings_scroll = 0.0;
                }
                self.last_vinyl_tick = Instant::now();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnSettingsTab(tab) => {
                self.show_ambient_mixer = false;
                self.show_stats_modal = false;
                self.show_share_modal = false;
                self.show_goal_modal = false;
                self.show_prayer_modal = false;
                self.prayer_search_focused = false;
                self.settings_tab = Some(tab);
                self.config.last_settings_tab = tab.key().to_string();
                let _ = self.config.save();
                self.settings_scroll = 0.0;
                self.last_vinyl_tick = Instant::now();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnCloseModal | ControlId::ModalBackdrop => {
                if self.show_share_modal {
                    self.show_share_modal = false;
                } else if self.show_prayer_modal {
                    self.show_prayer_modal = false;
                    self.prayer_search_focused = false;
                } else {
                    self.settings_tab = None;
                    self.show_ambient_mixer = false;
                    self.show_stats_modal = false;
                    self.show_goal_modal = false;
                }
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::ModalSurface => {}
            ControlId::BtnGuardTabApps => {
                self.guard_tab = GuardTab::BlockedApps;
                self.app_scroll = 0;
                self.site_input_focused = false;
                self.refresh_running_apps();
                self.request_redraw();
            }
            ControlId::BtnGuardTabSites => {
                self.guard_tab = GuardTab::RestrictedSites;
                self.site_scroll = 0;
                self.site_input_focused = false;
                self.request_redraw();
            }
            ControlId::BtnGuardTabPrayer => {
                self.guard_tab = GuardTab::PrayerTimes;
                self.site_input_focused = false;
                self.refresh_prayer_schedule();
                self.request_redraw();
            }
            ControlId::BtnGuardRefreshApps => {
                self.guard_refresh_spin_until = Some(Instant::now() + Duration::from_millis(450));
                self.sync_vinyl_timer();
                self.refresh_running_apps();
                self.request_redraw();
            }
            ControlId::BtnFilterAll => {
                self.app_filter = AppFilter::All;
                self.app_scroll = 0;
                self.request_redraw();
            }
            ControlId::BtnFilterBlocked => {
                self.app_filter = AppFilter::Blocked;
                self.app_scroll = 0;
                self.request_redraw();
            }
            ControlId::BtnFilterAccessible => {
                self.app_filter = AppFilter::Accessible;
                self.app_scroll = 0;
                self.request_redraw();
            }
            ControlId::BtnAppSearchBox => {
                self.search_focused = true;
                self.request_redraw();
            }
            ControlId::BtnAppSearchClear => {
                self.search_query.clear();
                self.app_scroll = 0;
                self.request_redraw();
            }
            ControlId::BtnAppSort => {
                self.app_sort = self.app_sort.next();
                self.app_scroll = 0;
                self.request_redraw();
            }
            ControlId::BtnRingTheme(idx) => {
                if let Some((key, _)) = TIMER_THEMES.get(idx) {
                    self.config.timer_theme = (*key).to_string();
                    let _ = self.config.save();
                    self.sync_vinyl_timer();
                    self.request_redraw();
                }
            }
            ControlId::BtnToggleHourglassRotate => {
                self.config.auto_rotate_hourglass = !self.config.auto_rotate_hourglass;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleStealthPeek => {
                self.config.stealth_hover_peek = !self.config.stealth_hover_peek;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnAmbientDefault => {
                self.config.ambient_mode = AmbientMode::Default;
                let _ = self.config.save();
                self.load_active_gif();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnAmbientYtMusic => {
                self.config.ambient_mode = AmbientMode::YtMusic;
                let _ = self.config.save();
                self.load_active_gif();
                self.last_vinyl_tick = Instant::now();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnAmbientCustom => {
                self.config.ambient_mode = AmbientMode::Custom;
                let _ = self.config.save();
                self.load_active_gif();
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnUploadGif => {
                self.in_file_dialog = true;
                let picked = file_dialog::pick_image_file(self.hwnd);
                self.in_file_dialog = false;
                if let Ok(Some(src_path)) = picked {
                    let gifs_dir = AppConfig::gifs_dir();
                    match import_gif_into_store(&src_path, &gifs_dir, &mut self.config) {
                        Ok(dest_path) => {
                            let _ = self.config.save();
                            self.load_gif_from_path(&dest_path);
                        }
                        Err(err) => {
                            self.status_hint = Some(format!("Import error: {err}"));
                        }
                    }
                    self.request_redraw();
                }
            }
            ControlId::BtnGifCircle => {
                self.config.gif_display_mode = GifDisplayMode::Circle;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnGifFull => {
                self.config.gif_display_mode = GifDisplayMode::Full;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::OpacitySlider => {
                self.dragging_slider = Some(DragSlider::Opacity);
                unsafe {
                    SetCapture(self.hwnd);
                }
                self.update_slider_from_x(x_dip);
            }
            ControlId::RecentGifSelect(idx) => {
                if let Some(item) = self.config.recent_gifs.get(idx).cloned() {
                    let gifs_dir = AppConfig::gifs_dir();
                    if select_recent_gif(&item.id, &gifs_dir, &mut self.config).is_some() {
                        let _ = self.config.save();
                        self.load_active_gif();
                        self.request_redraw();
                    }
                }
            }
            ControlId::RecentGifDelete(idx) => {
                if let Some(item) = self.config.recent_gifs.get(idx).cloned() {
                    let gifs_dir = AppConfig::gifs_dir();
                    remove_recent_gif(&item.id, &gifs_dir, &mut self.config);
                    let _ = self.config.save();
                    self.load_active_gif();
                    self.request_redraw();
                }
            }
            ControlId::BtnClearRecentGifs => {
                let gifs_dir = AppConfig::gifs_dir();
                for item in self.config.recent_gifs.clone() {
                    remove_recent_gif(&item.id, &gifs_dir, &mut self.config);
                }
                let _ = self.config.save();
                self.load_active_gif();
                self.request_redraw();
            }
            ControlId::VinylSpeedSlider => {
                self.dragging_slider = Some(DragSlider::VinylSpeed);
                unsafe {
                    SetCapture(self.hwnd);
                }
                self.update_slider_from_x(x_dip);
            }
            ControlId::VinylSizeSlider => {
                self.dragging_slider = Some(DragSlider::VinylSize);
                unsafe {
                    SetCapture(self.hwnd);
                }
                self.update_slider_from_x(x_dip);
            }
            ControlId::BtnVinylSpeedPreset(idx) => {
                if let Some(speed) = [3.0f32, 6.0, 10.0, 16.0].get(idx) {
                    self.config.vinyl_speed_sec = *speed;
                    let _ = self.config.save();
                    self.sync_vinyl_overlay();
                    self.request_redraw();
                }
            }
            ControlId::BtnVinylSizePreset(idx) => {
                if let Some(sz) = [150i32, 340, 680].get(idx) {
                    self.config.vinyl_size_px = *sz;
                    let _ = self.config.save();
                    self.sync_vinyl_overlay();
                    self.request_redraw();
                }
            }
            ControlId::AccentPreset(idx) => {
                if let Some((_, hex)) = ACCENT_PRESETS.get(idx) {
                    self.config.accent_color = (*hex).to_string();
                    let _ = self.config.save();
                    self.sync_vinyl_overlay();
                    self.request_redraw();
                }
            }
            ControlId::BtnToggleDynamicAlbum => {
                self.config.dynamic_album_enabled = !self.config.dynamic_album_enabled;
                self.recompute_dynamic_accent();
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnToggleStealthTimer => {
                self.config.stealth_active = !self.config.stealth_active;
                let _ = self.config.save();
                self.status_hint = Some(if self.config.stealth_active {
                    "Stealth Mode ON (hover timer to peek)".to_string()
                } else {
                    "Stealth Mode OFF".to_string()
                });
                self.request_redraw();
            }
            ControlId::BtnToggleComplementary => {
                self.config.use_complementary_color = !self.config.use_complementary_color;
                self.recompute_dynamic_accent();
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnTogglePeekingVinyl => {
                self.config.pip_peeking_vinyl_enabled = !self.config.pip_peeking_vinyl_enabled;
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnToggleVinylSpindle => {
                self.config.show_vinyl_spindle = !self.config.show_vinyl_spindle;
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnToggleHideTaskbarInPip => {
                self.config.hide_taskbar_in_pip = !self.config.hide_taskbar_in_pip;
                let _ = self.config.save();
                self.apply_window_chrome();
                self.request_redraw();
            }
            ControlId::BtnVinylSideLeft => {
                self.config.pip_peeking_vinyl_side = VinylSide::Left;
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnVinylSideRight => {
                self.config.pip_peeking_vinyl_side = VinylSide::Right;
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnVinylSpeedDown => {
                self.config.vinyl_speed_sec = (self.config.vinyl_speed_sec - 1.0).clamp(1.0, 30.0);
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnVinylSpeedUp => {
                self.config.vinyl_speed_sec = (self.config.vinyl_speed_sec + 1.0).clamp(1.0, 30.0);
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnVinylSizeDown => {
                self.config.vinyl_size_px = (self.config.vinyl_size_px - 20).clamp(120, 850);
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnVinylSizeUp => {
                self.config.vinyl_size_px = (self.config.vinyl_size_px + 20).clamp(120, 850);
                let _ = self.config.save();
                self.sync_vinyl_overlay();
                self.request_redraw();
            }
            ControlId::BtnMediaPrev => self.send_media(MediaCommand::Prev),
            ControlId::BtnMediaPlayPause => self.send_media(MediaCommand::PlayPause),
            ControlId::BtnMediaNext => self.send_media(MediaCommand::Next),
            ControlId::BtnMediaToggleWindow => self.send_media(MediaCommand::ToggleMusicWindow),
            ControlId::BtnCopyMobileUrl => {
                let port = self
                    .http_bridge
                    .as_ref()
                    .map(|b| b.bound_port)
                    .filter(|&p| p > 0)
                    .unwrap_or(8766);
                let ip = if self.lan_ip.is_empty() {
                    "127.0.0.1"
                } else {
                    self.lan_ip.as_str()
                };
                let url = format!("http://{ip}:{port}");
                if copy_text_to_clipboard(&url) {
                    self.status_hint = Some(format!("Copied URL: {url}"));
                } else {
                    self.status_hint = Some(format!("Mobile URL: {url}"));
                }
                self.request_redraw();
            }
            ControlId::BtnOpenMobileUrl => {
                let port = self
                    .http_bridge
                    .as_ref()
                    .map(|b| b.bound_port)
                    .filter(|&p| p > 0)
                    .unwrap_or(8766);
                let ip = if self.lan_ip.is_empty() {
                    "127.0.0.1"
                } else {
                    self.lan_ip.as_str()
                };
                let url = format!("http://{ip}:{port}");
                let _ = std::process::Command::new("explorer.exe").arg(&url).spawn();
                self.status_hint = Some(format!("Opened {url}"));
                self.request_redraw();
            }
            ControlId::BtnTogglePrayerEnabled => {
                self.config.prayer_enabled = !self.config.prayer_enabled;
                self.refresh_prayer_schedule();
                let _ = self.config.save();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnToggleAutoPause => {
                self.config.auto_pause_enabled = !self.config.auto_pause_enabled;
                if !self.config.auto_pause_enabled {
                    self.is_auto_paused = false;
                }
                let _ = self.config.save();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnToggleAutoClose => {
                self.config.auto_close_blocked_apps = !self.config.auto_close_blocked_apps;
                self.evaluate_foreground_guard();
                let _ = self.config.save();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnAutoPauseThreshold => {
                self.config.auto_pause_sec = match self.config.auto_pause_sec {
                    0..=20 => 30,
                    21..=45 => 60,
                    46..=90 => 120,
                    _ => 15,
                };
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleNeutralOverstay => {
                self.config.neutral_overstay_enabled = !self.config.neutral_overstay_enabled;
                if !self.config.neutral_overstay_enabled {
                    self.neutral_overstay.reset();
                    self.sync_overlays();
                }
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnNeutralThreshold => {
                self.config.neutral_overstay_mins = match self.config.neutral_overstay_mins {
                    0..=2 => 5,
                    3..=7 => 10,
                    8..=12 => 15,
                    13..=22 => 30,
                    23..=38 => 45,
                    39..=52 => 60,
                    _ => 2,
                };
                self.config.sanitize();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleNeutralSoftblock => {
                self.config.neutral_softblock_enabled = !self.config.neutral_softblock_enabled;
                if !self.config.neutral_softblock_enabled {
                    self.neutral_overstay.softblock_active = false;
                    self.neutral_overstay.softblock_app = None;
                    self.sync_overlays();
                }
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleBlockAllMonitors => {
                self.config.block_all_monitors = !self.config.block_all_monitors;
                if let Some(ref mut ov) = self.overlay {
                    ov.set_block_all_monitors(self.config.block_all_monitors);
                }
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleAutoPip => {
                self.config.auto_pip = !self.config.auto_pip;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleAutoPipBlur => {
                self.config.auto_pip_on_blur = !self.config.auto_pip_on_blur;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleNotifications => {
                self.config.notifications_enabled = !self.config.notifications_enabled;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleCountdownSound => {
                self.config.countdown_sound_enabled = !self.config.countdown_sound_enabled;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnCycleCountdownSecs => {
                self.config.countdown_duration_secs = match self.config.countdown_duration_secs {
                    3 => 5,
                    5 => 10,
                    _ => 3,
                };
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnCycleCountdownStyle => {
                self.config.countdown_sound_style = if self.config.countdown_sound_style == "digital" {
                    "zen".to_string()
                } else {
                    "digital".to_string()
                };
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnCycleTimerSubtitle => {
                self.config.timer_subtitle_mode = match self.config.timer_subtitle_mode.as_str() {
                    "countdown" => "finishtime".to_string(),
                    "finishtime" => "totalrem".to_string(),
                    _ => "countdown".to_string(),
                };
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleCompletedFormat => {
                self.config.completed_time_format = match self.config.completed_time_format.as_str() {
                    "smart" => "decimal".to_string(),
                    "decimal" => "minutes".to_string(),
                    _ => "smart".to_string(),
                };
                self.completed_spin_until = Some(Instant::now() + Duration::from_millis(450));
                self.sync_vinyl_timer();
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnPrayerPrev => {
                let len = self.prayer_times.len();
                if len > 0 {
                    let cur = self.prayer_view_idx.unwrap_or_else(|| {
                        self.prayer_times
                            .iter()
                            .position(|p| self.next_prayer_label.starts_with(p.name))
                            .unwrap_or(0)
                    });
                    self.prayer_view_idx = Some(if cur == 0 { len - 1 } else { cur - 1 });
                    self.request_redraw();
                }
            }
            ControlId::BtnPrayerNext => {
                let len = self.prayer_times.len();
                if len > 0 {
                    let cur = self.prayer_view_idx.unwrap_or_else(|| {
                        self.prayer_times
                            .iter()
                            .position(|p| self.next_prayer_label.starts_with(p.name))
                            .unwrap_or(0)
                    });
                    self.prayer_view_idx = Some((cur + 1) % len);
                    self.request_redraw();
                }
            }
            ControlId::BtnTogglePrayerBreak => {
                self.config.prayer_break_enabled = !self.config.prayer_break_enabled;
                let _ = self.config.save();
                self.sync_bridge_snapshot();
                self.request_redraw();
            }
            ControlId::BtnPrayerAdvanceDown => {
                self.config.prayer_advance_mins = self.config.prayer_advance_mins.saturating_sub(1).max(1);
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnPrayerAdvanceUp => {
                self.config.prayer_advance_mins = (self.config.prayer_advance_mins + 1).min(30);
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnPrayerOffsetDown => {
                self.config.prayer_offset_mins = (self.config.prayer_offset_mins - 1).clamp(-15, 15);
                let _ = self.config.save();
                self.rebuild_prayer_mgr();
                self.sync_overlays();
                self.sync_sec_timer();
                self.request_redraw();
            }
            ControlId::BtnPrayerOffsetUp => {
                self.config.prayer_offset_mins = (self.config.prayer_offset_mins + 1).clamp(-15, 15);
                let _ = self.config.save();
                self.rebuild_prayer_mgr();
                self.sync_overlays();
                self.sync_sec_timer();
                self.request_redraw();
            }
            ControlId::BtnPrayerLocationCycle => {
                self.settings_tab = None;
                self.show_ambient_mixer = false;
                self.show_stats_modal = false;
                self.show_share_modal = false;
                self.show_goal_modal = false;
                self.show_prayer_modal = true;
                self.prayer_search_focused = true;
                self.prayer_online_results = None;
                self.prayer_loc_status = None;
                self.sync_vinyl_timer();
                self.request_redraw();
            }
            ControlId::BtnPrayerAutoGps => {
                self.start_auto_gps_lookup();
            }
            ControlId::BtnPrayerSearchBox => {
                self.prayer_search_focused = true;
                self.request_redraw();
            }
            ControlId::BtnPrayerSearchClear => {
                self.prayer_search_query.clear();
                self.prayer_online_results = None;
                self.prayer_loc_status = None;
                self.prayer_search_focused = true;
                self.request_redraw();
            }
            ControlId::BtnPrayerSearchOnline => {
                self.start_online_city_search();
            }
            ControlId::BtnPrayerCityPick(idx) => {
                let cities = self.filtered_prayer_cities();
                if let Some(c) = cities.get(idx) {
                    self.apply_prayer_location(c.lat, c.lng, c.tz, &c.name);
                    self.show_prayer_modal = false;
                    self.prayer_search_focused = false;
                    self.sync_vinyl_timer();
                    self.request_redraw();
                }
            }
            ControlId::BtnFocusSoundCycle => {
                self.config.focus_sound = next_sound_style(&self.config.focus_sound);
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnFocusSoundTest => {
                self.audio.play_notification(
                    &self.config.focus_sound,
                    self.config.custom_focus_audio_path.as_deref(),
                );
            }
            ControlId::BtnFocusSoundUpload => self.import_custom_audio("focus"),
            ControlId::BtnBreakSoundCycle => {
                self.config.break_sound = next_sound_style(&self.config.break_sound);
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnBreakSoundTest => {
                self.audio.play_notification(
                    &self.config.break_sound,
                    self.config.custom_break_audio_path.as_deref(),
                );
            }
            ControlId::BtnBreakSoundUpload => self.import_custom_audio("break"),
            ControlId::BtnWarningSoundCycle => {
                self.config.warning_sound = next_sound_style(&self.config.warning_sound);
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnWarningSoundTest => {
                self.audio.play_notification(
                    &self.config.warning_sound,
                    self.config.custom_warning_audio_path.as_deref(),
                );
            }
            ControlId::BtnWarningSoundUpload => self.import_custom_audio("warning"),
            ControlId::BtnAmbientMasterToggle => {
                self.audio.toggle_playing(&self.config.ambient);
                self.request_redraw();
            }
            ControlId::BtnAmbientMuteAll => {
                let next_mute = !self.audio.is_master_muted();
                self.audio.set_master_muted(next_mute, &self.config.ambient);
                self.request_redraw();
            }
            ControlId::BtnAmbientAutoPlay => {
                self.config.ambient.auto_play = !self.config.ambient.auto_play;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::AmbientMasterSlider => {
                self.dragging_slider = Some(DragSlider::AmbientMaster);
                unsafe {
                    SetCapture(self.hwnd);
                }
                self.update_slider_from_x(x_dip);
            }
            ControlId::BtnAmbientPreset(idx) => {
                if let Some(preset) = AMBIENT_PRESETS.get(idx) {
                    self.config.ambient.apply_preset(preset.id);
                    if !self.audio.is_playing() {
                        self.audio.set_playing(true, &self.config.ambient);
                    } else {
                        self.audio.sync(&self.config.ambient);
                    }
                    let _ = self.config.save();
                    self.request_redraw();
                }
            }
            ControlId::BtnCustomPreset(slot) => {
                let preset_id = format!("custom_{}", slot + 1);
                if self.config.ambient.apply_preset(&preset_id) {
                    if !self.audio.is_playing() {
                        self.audio.set_playing(true, &self.config.ambient);
                    } else {
                        self.audio.sync(&self.config.ambient);
                    }
                    let _ = self.config.save();
                    self.request_redraw();
                }
            }
            ControlId::BtnAmbientTrackToggle(idx) => {
                if let Some(meta) = AMBIENT_TRACKS.get(idx) {
                    if let Some(tr) = self.config.ambient.track_mut(meta.id) {
                        tr.active = !tr.active;
                    }
                    let slot = if self.config.ambient.active_preset == "custom_2" { 1 } else { 0 };
                    self.config.ambient.save_current_as_custom(slot);
                    if !self.audio.is_playing() {
                        self.audio.set_playing(true, &self.config.ambient);
                    } else {
                        self.audio.sync(&self.config.ambient);
                    }
                    let _ = self.config.save();
                    self.request_redraw();
                }
            }
            ControlId::AmbientTrackSlider(idx) => {
                self.dragging_slider = Some(DragSlider::AmbientTrack(idx));
                unsafe {
                    SetCapture(self.hwnd);
                }
                self.update_slider_from_x(x_dip);
            }
            ControlId::BtnToggleMinimizeTray => {
                self.config.minimize_to_tray = !self.config.minimize_to_tray;
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnToggleLanguage => {
                self.config.app_language = if self.config.app_language.eq_ignore_ascii_case("id") {
                    "en".to_string()
                } else {
                    "id".to_string()
                };
                let _ = self.config.save();
                self.request_redraw();
            }
            ControlId::BtnSysResetToday => {
                self.stats.completed_minutes_today = 0;
                self.stats.health_completed_breaks_today = 0;
                let (today, ..) = local_date_and_hour();
                self.stats
                    .recalculate_streaks(&today, self.config.daily_goal_minutes);
                let _ = self.stats.save();
                self.status_hint = Some("Today's focus progress reset".to_string());
                self.request_redraw();
            }
            ControlId::BtnSysClearApps => {
                self.config.blacklist_apps.clear();
                self.guard.sync_from_config(&self.config);
                let _ = self.config.save();
                self.evaluate_foreground_guard();
                self.sync_bridge_snapshot();
                self.status_hint = Some("Cleared blocked apps list".to_string());
                self.request_redraw();
            }
            ControlId::BtnSysClearGifs => {
                let gifs_dir = AppConfig::gifs_dir();
                for item in self.config.recent_gifs.clone() {
                    remove_recent_gif(&item.id, &gifs_dir, &mut self.config);
                }
                let _ = self.config.save();
                self.load_active_gif();
                self.status_hint = Some("Cleaned GIF storage cache".to_string());
                self.request_redraw();
            }
            ControlId::BtnSysMigrateWebView2 => {
                let (today, y, m, d, _) = local_date_and_hour();
                let yesterday = prev_date_str(y, m, d);
                if auto_migrate_webview2(&today, &yesterday, &mut self.config, &mut self.stats).is_ok() {
                    let _ = self.config.save();
                    let _ = self.stats.save();
                    self.guard.sync_from_config(&self.config);
                    self.prayer_mgr = PrayerManager::new(
                        self.config.prayer_lat,
                        self.config.prayer_lng,
                        self.config.prayer_tz,
                    );
                    self.sync_schedule_to_timer();
                    self.load_active_gif();
                    self.refresh_prayer_schedule();
                    self.sync_bridge_snapshot();
                    self.sync_vinyl_overlay();
                    self.settings_tab = None;
                    self.request_redraw();
                }
            }
            ControlId::GuardAppToggle(vis_idx) => {
                let (_, h_dip) = self.surface.dip_size();
                let cap = blocked_apps_capacity(h_dip);
                let all_apps = self.filtered_apps();
                let start = self.app_scroll.min(all_apps.len().saturating_sub(cap));
                if let Some((exe, _, _, _)) = all_apps.get(start + vis_idx) {
                    let exe_owned = (*exe).to_string();
                    if let Some(pos) = self
                        .config
                        .blacklist_apps
                        .iter()
                        .position(|b| b.eq_ignore_ascii_case(&exe_owned))
                    {
                        self.config.blacklist_apps.remove(pos);
                        self.status_hint = Some(format!("Unblocked {exe_owned}"));
                    } else if !exe_owned.eq_ignore_ascii_case("focusgrow-app.exe") {
                        self.config.blacklist_apps.push(exe_owned.clone());
                        self.status_hint = Some(format!("Blocked {exe_owned}"));
                    }
                    self.guard.sync_from_config(&self.config);
                    let _ = self.config.save();
                    self.evaluate_foreground_guard();
                    self.sync_bridge_snapshot();
                    self.request_redraw();
                }
            }
            ControlId::BtnSiteSearchBox => {
                self.site_input_focused = true;
                self.request_redraw();
            }
            ControlId::BtnSiteClear => {
                self.site_input_query.clear();
                self.request_redraw();
            }
            ControlId::BtnSiteAdd => {
                let q = self.site_input_query.clone();
                self.add_restricted_site(&q);
            }
            ControlId::SiteCustomDelete(idx) => {
                let entries = self.site_entries();
                if let Some(domain) = entries.get(idx) {
                    let d_clean = domain.clone();
                    self.config.restricted_sites.retain(|s| !s.eq_ignore_ascii_case(&d_clean));
                    self.guard.sync_from_config(&self.config);
                    let _ = self.config.save();
                    self.sync_bridge_snapshot();
                    self.status_hint = Some(format!("Removed {d_clean}"));
                    self.request_redraw();
                }
            }
            ControlId::SitePresetToggle(idx) => {
                let entries = self.site_entries();
                if let Some(domain) = entries.get(idx) {
                    if let Some(pos) = self
                        .config
                        .restricted_sites
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(domain))
                    {
                        self.config.restricted_sites.remove(pos);
                        self.status_hint = Some(format!("Allowed {domain}"));
                    } else {
                        self.config.restricted_sites.push(domain.clone());
                        self.status_hint = Some(format!("Restricted {domain}"));
                    }
                    self.guard.sync_from_config(&self.config);
                    let _ = self.config.save();
                    self.sync_bridge_snapshot();
                    self.request_redraw();
                }
            }
            ControlId::BtnGuardClearPasses => {
                self.guard.reset_session_passes();
                self.sync_bridge_snapshot();
                self.status_hint = Some("Cleared active site passes".to_string());
                self.request_redraw();
            }
        }
    }

    fn send_media(&self, cmd: MediaCommand) {
        if let Some(ref bridge) = self.ytmpx {
            bridge.send_command(cmd);
        }
    }
}

fn local_clock_and_date() -> (String, String) {
    let st = unsafe { GetLocalTime() };
    let (h12, ampm) = match st.wHour {
        0 => (12, "AM"),
        1..=11 => (st.wHour, "AM"),
        12 => (12, "PM"),
        _ => (st.wHour - 12, "PM"),
    };
    let clock = format!("{}:{:02}:{:02} {}", h12, st.wMinute, st.wSecond, ampm);
    let dow = match st.wDayOfWeek {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        _ => "Saturday",
    };
    let month = match st.wMonth {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        _ => "December",
    };
    let date = format!("{}, {} {} {}", dow, st.wDay, month, st.wYear);
    (clock, date)
}

fn prev_date_str(year: i32, month: i32, day: i32) -> String {
    if day > 1 {
        return format!("{:04}-{:02}-{:02}", year, month, day - 1);
    }
    let (py, pm) = if month > 1 {
        (year, month - 1)
    } else {
        (year - 1, 12)
    };
    let leap = (py % 4 == 0 && py % 100 != 0) || (py % 400 == 0);
    let dim = match pm {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => if leap { 29 } else { 28 },
        _ => 30,
    };
    format!("{:04}-{:02}-{:02}", py, pm, dim)
}

fn local_date_and_hour() -> (String, i32, i32, i32, f64) {
    let st = unsafe { GetLocalTime() };
    let today = format!("{:04}-{:02}-{:02}", st.wYear, st.wMonth, st.wDay);
    let cur_hour =
        st.wHour as f64 + st.wMinute as f64 / 60.0 + st.wSecond as f64 / 3600.0;
    (
        today,
        st.wYear as i32,
        st.wMonth as i32,
        st.wDay as i32,
        cur_hour,
    )
}

fn infer_timezone_from_coords(lat: f64, lng: f64) -> i32 {
    if (95.0..=141.0).contains(&lng) && (-11.5..=6.5).contains(&lat) {
        if lng < 109.0 {
            7
        } else if lng < 126.0 {
            8
        } else {
            9
        }
    } else {
        ((lng / 15.0).round() as i32).clamp(-12, 14)
    }
}

fn compute_end_time_hhmm(hour: u32, minute: u32, second: u32, add_sec: u32) -> String {
    let base_sec = hour * 3600 + minute * 60 + second;
    let total_min = ((base_sec + add_sec) / 60) % (24 * 60);
    format!("{:02}:{:02}", total_min / 60, total_min % 60)
}

fn clamp_window_pos_to_virtual_screen(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);
        let max_x = (vx + vw - w).max(vx);
        let max_y = (vy + vh - h).max(vy);
        (x.clamp(vx, max_x), y.clamp(vy, max_y))
    }
}

fn main() -> Result<()> {
    let mutex_handle: HANDLE = unsafe {
        CreateMutexW(
            None,
            false,
            w!("Global\\FocusGrow_SingleInstance_Mutex_Genesfi"),
        )?
    };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            if let Ok(existing) = FindWindowW(w!("FocusGrowNativeWindow"), None) {
                if !existing.0.is_null() {
                    if IsIconic(existing).as_bool() {
                        let _ = ShowWindow(existing, SW_RESTORE);
                    } else {
                        let _ = ShowWindow(existing, SW_SHOW);
                    }
                    let _ = SetForegroundWindow(existing);
                }
            }
            let _ = CloseHandle(mutex_handle);
        }
        return Ok(());
    }

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }

    let result = run_app();

    unsafe {
        CoUninitialize();
        let _ = CloseHandle(mutex_handle);
    }
    if result.is_ok() {
        std::process::exit(0);
    }
    result
}

fn create_hicon_from_png(png_bytes: &[u8]) -> Option<HICON> {
    let anim = decode_image_bytes(png_bytes).ok()?;
    let frame = anim.frames.first()?;
    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 || frame.pixels_pbgra.len() < (w * h * 4) as usize {
        return None;
    }
    unsafe {
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down BGRA
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hdc = CreateCompatibleDC(None);
        let hbm_color = CreateDIBSection(Some(hdc), &bi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let _ = DeleteDC(hdc);
        if !bits.is_null() {
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
            dst.copy_from_slice(&frame.pixels_pbgra[..(w * h * 4) as usize]);
        }
        let mask_len = (((w + 15) / 16) * 2 * h) as usize;
        let mask_bits = vec![0u8; mask_len];
        let hbm_mask = CreateBitmap(w, h, 1, 1, Some(mask_bits.as_ptr() as *const _));
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(hbm_color.into());
        if !hbm_mask.0.is_null() {
            let _ = DeleteObject(hbm_mask.into());
        }
        icon
    }
}

fn run_app() -> Result<()> {
    let mut config = AppConfig::load();
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    let class_name = w!("FocusGrowNativeWindow");
    let app_hicon = unsafe {
        LoadImageW(
            Some(hinstance),
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE,
        )
        .ok()
        .map(|h| HICON(h.0))
        .or_else(|| create_hicon_from_png(APP_ICON_PNG))
    };

    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(wnd_proc),
        hInstance: hinstance,
        hIcon: app_hicon.unwrap_or_default(),
        hIconSm: app_hicon.unwrap_or_default(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        lpszClassName: class_name,
        ..Default::default()
    };
    unsafe {
        RegisterClassExW(&wc);
    }

    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let dash_w = config.dashboard_width.clamp(760, 1800);
    let dash_h = config.dashboard_height.clamp(520, 1200);
    let (init_w_dip, init_h_dip) = if config.pip_mode {
        if config.pip_style == PipStyle::Capsule {
            (config.pip_capsule_width.clamp(195, 520), config.pip_capsule_height.max(68))
        } else {
            (config.pip_width.max(180), config.pip_height.max(220))
        }
    } else {
        (dash_w, dash_h)
    };

    let ex_style = WS_EX_NOREDIRECTIONBITMAP
        | if config.pip_mode {
            WS_EX_TOPMOST
        } else {
            WS_EX_APPWINDOW
        };

    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class_name,
            w!("FocusGrow"),
            WS_POPUP,
            0,
            0,
            init_w_dip,
            init_h_dip,
            None,
            None,
            Some(hinstance),
            None,
        )
    }?;

    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32;
    let scale = dpi / 96.0;
    let px_w = (init_w_dip as f32 * scale).round() as i32;
    let px_h = (init_h_dip as f32 * scale).round() as i32;
    let (px_x, px_y) = if config.pip_mode {
        let raw_x = config
            .pip_x
            .map(|v| (v as f32 * scale).round() as i32)
            .unwrap_or_else(|| (screen_w - px_w - 36).max(0));
        let raw_y = config
            .pip_y
            .map(|v| (v as f32 * scale).round() as i32)
            .unwrap_or_else(|| (screen_h - px_h - 72).max(0));
        clamp_window_pos_to_virtual_screen(raw_x, raw_y, px_w, px_h)
    } else {
        clamp_window_pos_to_virtual_screen(
            ((screen_w - px_w) / 2).max(0),
            ((screen_h - px_h) / 2).max(0),
            px_w,
            px_h,
        )
    };

    let mut client = RECT::default();
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            px_x,
            px_y,
            px_w,
            px_h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = GetClientRect(hwnd, &mut client);
    }

    let surface = DCompSurface::new(
        hwnd,
        (client.right - client.left).max(1) as u32,
        (client.bottom - client.top).max(1) as u32,
        dpi,
    )?;

    let app_icon_logo = decode_image_bytes(APP_ICON_PNG)
        .ok()
        .and_then(|anim| {
            anim.frames
                .first()
                .and_then(|f| upload_frame_bitmap(&surface.d2d_context, f).ok())
        });

    let (today, ..) = local_date_and_hour();
    let stats = DailyStats::load(&today);
    let mut timer = FocusTimer::new(
        config.total_focus_minutes,
        config.focus_minutes,
        config.break_minutes,
        config.skip_breaks,
    );
    if let Some(saved) = PersistedSession::load() {
        let age_secs = PersistedSession::unix_now().saturating_sub(saved.saved_unix_secs);
        if saved.phase != "idle" && saved.remaining_sec > 0 && age_secs < 12 * 3600 {
            config.is_health_mode = saved.is_health_mode;
            timer.restore_session(&saved, Instant::now());
        } else {
            PersistedSession::clear();
        }
    }

    let tray = app_hicon.map(|ico| TrayIcon::create(hwnd, ico));
    let peeking_vinyl = PeekingVinylWindow::create(hwnd).ok();
    let health_hud = HealthHudWindow::create().ok();
    let overlay = OverlayWindow::new(hwnd, config.block_all_monitors).ok();
    let ytmpx = Some(YtmpxBridge::start(hwnd));
    let guard = GuardState::from_config(&config);
    let prayer_mgr = PrayerManager::with_offset(
        config.prayer_lat,
        config.prayer_lng,
        config.prayer_tz,
        config.prayer_offset_mins,
    );
    let http_bridge = Some(HttpBridgeServer::start(hwnd));
    let foreground_hook = ForegroundHook::install(hwnd);
    let lan_ip = detect_lan_ipv4();
    let bridge_port_init = http_bridge
        .as_ref()
        .map(|b| b.bound_port)
        .filter(|&p| p > 0)
        .unwrap_or(8766);
    let qr_matrix = encode_qr_v3_m(&format!("http://{lan_ip}:{bridge_port_init}"));
    let (gif_tx, gif_rx) = mpsc::channel();
    let (prayer_loc_tx, prayer_loc_rx) = mpsc::channel();

    let mut state = Box::new(AppState {
        hwnd,
        surface,
        config,
        timer,
        stats,
        guard,
        prayer_mgr,
        prayer_date: String::new(),
        prayer_times: Vec::new(),
        next_prayer_label: String::new(),
        next_prayer_name: String::new(),
        next_prayer_secs: u32::MAX,
        prayer_break_remaining: 0,
        active_prayer_break_name: String::new(),
        last_prayer_break_key: String::new(),
        last_prayer_dismissed_key: String::new(),
        last_prayer_adv_key: String::new(),
        warned_5m_phase: None,
        warned_1m_phase: None,
        prev_phase: SessionPhase::Idle,
        health_finish_until: None,
        health_hud,
        overlay,
        http_bridge,
        foreground_hook,
        active_app: None,
        active_domain: String::new(),
        block_countdown: None,
        blocked_app_active: None,
        dismissed_blocked_hwnd: None,
        neutral_overstay: NeutralOverstayTracker::default(),
        is_auto_paused: false,
        gpu_anim: None,
        frame_idx: 0,
        cover_frame: None,
        cover_bitmap: None,
        cached_dynamic_accent: None,
        track: None,
        playback: PlaybackState::Stopped,
        ytmpx_connected: false,
        ytmpx,
        peeking_vinyl,
        guard_tab: GuardTab::BlockedApps,
        app_filter: AppFilter::All,
        app_sort: AppSort::LatestOpen,
        search_query: String::new(),
        search_focused: false,
        running_apps: Vec::new(),
        app_icons: HashMap::new(),
        app_icons_by_exe: HashMap::new(),
        app_icon_logo,
        tray,
        deferred_done: false,
        force_quit: false,
        app_scroll: 0,
        settings_tab: None,
        settings_scroll: 0.0,
        show_ambient_mixer: false,
        show_stats_modal: false,
        show_share_modal: false,
        share_yesterday: false,
        show_goal_modal: false,
        show_prayer_modal: false,
        prayer_search_query: String::new(),
        prayer_search_focused: false,
        site_input_query: String::new(),
        site_input_focused: false,
        site_scroll: 0,
        prayer_online_results: None,
        prayer_search_gen: 0,
        prayer_loc_busy: false,
        prayer_loc_status: None,
        prayer_loc_tx,
        prayer_loc_rx,
        stats_period: StatsPeriod::Today,
        heatmap_offset_months: 0,
        session_save_tick: 0,
        prayer_view_idx: None,
        show_pip_gif_drawer: false,
        hovered: None,
        cursor_dip: None,
        dragging_slider: None,
        tracking_mouse_leave: false,
        is_maximized: false,
        in_card_vinyl_angle: 0.0,
        plant_sway_phase: 0.0,
        wave_phase: 0.0,
        marquee_offset: 0.0,
        completed_spin_until: None,
        guard_refresh_spin_until: None,
        last_vinyl_tick: Instant::now(),
        audio: AudioSlot(None),
        status_hint: None,
        num_edit_field: None,
        num_edit_buf: String::new(),
        num_edit_fresh: false,
        in_file_dialog: false,
        last_pip_toggle: Instant::now() - Duration::from_secs(5),
        normal_rect_dip: (
            ((screen_w - dash_w) / 2).max(0),
            ((screen_h - dash_h) / 2).max(0),
            dash_w,
            dash_h,
        ),
        is_closing: false,
        sec_timer_active: false,
        vinyl_timer_active: false,
        gif_timer_delay_ms: None,
        loaded_gif_path: None,
        pending_gif_path: None,
        gif_load_gen: 0,
        gif_tx,
        gif_rx,
        last_guard_tick: Instant::now(),
        lan_ip,
        qr_matrix,
    });

    state.refresh_prayer_schedule();
    state.load_active_gif();
    state.apply_window_chrome();
    state.render_now();

    let pip_mode_init = state.config.pip_mode;
    let state_ptr = Box::into_raw(state);
    unsafe {
        if let Some(icon) = app_hicon {
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(icon.0 as isize)));
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(icon.0 as isize)));
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        SetTimer(Some(hwnd), TIMER_DEFERRED_INIT_ID, 15, None);
        SetTimer(Some(hwnd), TIMER_IDLE_HEARTBEAT_ID, 10_000, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let z_insert = if pip_mode_init { HWND_TOPMOST } else { HWND_TOP };
        let _ = SetWindowPos(hwnd, Some(z_insert), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        let _ = SetForegroundWindow(hwnd);
    }

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let _cs = lparam.0 as *const CREATESTRUCTW;
        return LRESULT(0);
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let state = &mut *ptr;

    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let _ = windows::Win32::Graphics::Gdi::ValidateRect(Some(hwnd), None);
            state.render_now();
            LRESULT(0)
        }
        WM_SIZE => {
            let w_px = (lparam.0 & 0xFFFF) as u32;
            let h_px = ((lparam.0 >> 16) & 0xFFFF) as u32;
            let dpi = GetDpiForWindow(hwnd).max(96) as f32;
            if w_px > 0 && h_px > 0 {
                let _ = state.surface.resize(w_px, h_px, dpi);
                state.render_now();
            }
            if let Some(ref mut vinyl) = state.peeking_vinyl {
                vinyl.sync_bounds_and_zorder(false);
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED => {
            if let Some(ref mut vinyl) = state.peeking_vinyl {
                vinyl.sync_bounds_and_zorder(false);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_DPICHANGED => {
            let new_dpi = (wparam.0 & 0xFFFF) as f32;
            let suggested = &*(lparam.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                suggested.left,
                suggested.top,
                suggested.right - suggested.left,
                suggested.bottom - suggested.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = state.surface.resize(
                (suggested.right - suggested.left).max(1) as u32,
                (suggested.bottom - suggested.top).max(1) as u32,
                new_dpi,
            );
            state.render_now();
            if let Some(ref mut vinyl) = state.peeking_vinyl {
                vinyl.sync_bounds_and_zorder(false);
            }
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let mmi = &mut *(lparam.0 as *mut MINMAXINFO);
            if state.config.pip_mode {
                if state.config.pip_style == PipStyle::Capsule {
                    mmi.ptMinTrackSize.x = state.dip_to_px(195);
                    mmi.ptMinTrackSize.y = state.dip_to_px(68);
                    mmi.ptMaxTrackSize.x = state.dip_to_px(520);
                    mmi.ptMaxTrackSize.y = state.dip_to_px(68);
                } else {
                    mmi.ptMinTrackSize.x = state.dip_to_px(150);
                    mmi.ptMinTrackSize.y = state.dip_to_px(210);
                    mmi.ptMaxTrackSize.x = state.dip_to_px(440);
                    mmi.ptMaxTrackSize.y = state.dip_to_px(550);
                }
            } else {
                mmi.ptMinTrackSize.x = state.dip_to_px(760);
                mmi.ptMinTrackSize.y = state.dip_to_px(520);
            }
            LRESULT(0)
        }
        WM_NCHITTEST => {
            let sx = (lparam.0 & 0xFFFF) as i16 as i32;
            let sy = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            let mut rc = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rc);

            // In Capsule mode: allow horizontal resizing from borders, native dragging from body, and clicking buttons
            if state.config.pip_mode && state.config.pip_style == PipStyle::Capsule {
                let border = state.dip_to_px(7).max(5);
                if sx < rc.left + border {
                    return LRESULT(HTLEFT as isize);
                }
                if sx >= rc.right - border {
                    return LRESULT(HTRIGHT as isize);
                }

                let x_dip = state.px_to_dip(sx - rc.left);
                let y_dip = state.px_to_dip(sy - rc.top);
                let (w_dip, h_dip) = state.surface.dip_size();
                let layout = compute_layout(&state.layout_query(w_dip, h_dip));
                if let Some(hit) = layout.hit_test(x_dip, y_dip) {
                    if hit != ControlId::DragBar {
                        return LRESULT(HTCLIENT as isize);
                    }
                }
                return LRESULT(HTCAPTION as isize);
            }

            let border = state.dip_to_px(9).max(7);

            let left = sx < rc.left + border;
            let right = sx >= rc.right - border;
            let top = sy < rc.top + border;
            let bottom = sy >= rc.bottom - border;

            let hit = match (left, top, right, bottom) {
                (true, true, _, _) => HTTOPLEFT,
                (_, true, true, _) => HTTOPRIGHT,
                (true, _, _, true) => HTBOTTOMLEFT,
                (_, _, true, true) => HTBOTTOMRIGHT,
                (true, _, _, _) => HTLEFT,
                (_, _, true, _) => HTRIGHT,
                (_, true, _, _) => HTTOP,
                (_, _, _, true) => HTBOTTOM,
                _ => HTCLIENT,
            };
            LRESULT(hit as isize)
        }
        WM_EXITSIZEMOVE => {
            state.save_resized_bounds(true);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if !state.tracking_mouse_leave {
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                let _ = TrackMouseEvent(&mut tme);
                state.tracking_mouse_leave = true;
            }
            let x = state.px_to_dip((lparam.0 & 0xFFFF) as i16 as i32);
            let y = state.px_to_dip(((lparam.0 >> 16) & 0xFFFF) as i16 as i32);

            if state.dragging_slider.is_some() {
                state.update_slider_from_x(x);
                return LRESULT(0);
            }

            let (w_dip, h_dip) = state.surface.dip_size();
            let layout = compute_layout(&state.layout_query(w_dip, h_dip));
            let next_hover = layout.hit_test(x, y);
            let prev_cursor = state.cursor_dip;
            state.cursor_dip = Some((x, y));
            let track_chart_hover = state.show_stats_modal && !state.show_share_modal && prev_cursor != state.cursor_dip;
            if next_hover != state.hovered || track_chart_hover {
                state.hovered = next_hover;
                state.request_redraw();
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            state.tracking_mouse_leave = false;
            let had_cursor = state.cursor_dip.take().is_some();
            if state.hovered.is_some() || (had_cursor && state.show_stats_modal && !state.show_share_modal) {
                state.hovered = None;
                state.request_redraw();
            }
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let active_state = (wparam.0 & 0xFFFF) as u32;
            let minimized = ((wparam.0 >> 16) & 0xFFFF) != 0;
            if active_state == WA_INACTIVE && !minimized {
                state.on_window_blur();
                state.handle_foreground_event();
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            let x = state.px_to_dip((lparam.0 & 0xFFFF) as i16 as i32);
            let y = state.px_to_dip(((lparam.0 >> 16) & 0xFFFF) as i16 as i32);
            let (w_dip, h_dip) = state.surface.dip_size();
            let layout = compute_layout(&state.layout_query(w_dip, h_dip));

            if let Some(hit_id) = layout.hit_test(x, y) {
                state.handle_click(hit_id, x);
            } else {
                state.commit_numeric_edit();
                if state.search_focused || state.prayer_search_focused {
                    state.search_focused = false;
                    state.prayer_search_focused = false;
                    state.request_redraw();
                }
                let _ = ReleaseCapture();
                let _ = PostMessageW(
                    Some(hwnd),
                    WM_SYSCOMMAND,
                    WPARAM((SC_MOVE | 0x0002) as usize),
                    LPARAM(0),
                );
            }
            LRESULT(0)
        }
        WM_CHAR => {
            let code = wparam.0 as u32;
            if let Some(c) = char::from_u32(code) {
                if state.num_edit_field.is_some() {
                    if code == 0x1B {
                        state.num_edit_field = None;
                        state.num_edit_buf.clear();
                        state.num_edit_fresh = false;
                        state.request_redraw();
                    } else if code == 0x0D || code == 0x09 {
                        state.commit_numeric_edit();
                    } else if code == 0x08 {
                        if state.num_edit_fresh {
                            state.num_edit_buf.clear();
                            state.num_edit_fresh = false;
                        } else {
                            state.num_edit_buf.pop();
                        }
                        state.request_redraw();
                    } else if c.is_ascii_digit() {
                        if state.num_edit_fresh {
                            state.num_edit_buf.clear();
                            state.num_edit_fresh = false;
                        }
                        if state.num_edit_buf.len() < 3 {
                            state.num_edit_buf.push(c);
                        }
                        state.request_redraw();
                    }
                    return LRESULT(0);
                }
                if code == 0x1B {
                    if state.show_share_modal {
                        state.show_share_modal = false;
                        state.request_redraw();
                    } else if state.show_prayer_modal {
                        state.show_prayer_modal = false;
                        state.prayer_search_focused = false;
                        state.request_redraw();
                    } else if state.settings_tab.is_some()
                        || state.show_ambient_mixer
                        || state.show_stats_modal
                        || state.show_goal_modal
                    {
                        state.settings_tab = None;
                        state.show_ambient_mixer = false;
                        state.show_stats_modal = false;
                        state.show_goal_modal = false;
                        state.sync_vinyl_timer();
                        state.request_redraw();
                    } else if state.search_focused || !state.search_query.is_empty() {
                        state.search_query.clear();
                        state.search_focused = false;
                        state.app_scroll = 0;
                        state.request_redraw();
                    } else if state.site_input_focused || !state.site_input_query.is_empty() {
                        state.site_input_query.clear();
                        state.site_input_focused = false;
                        state.request_redraw();
                    }
                } else if state.show_prayer_modal && state.prayer_search_focused {
                    if code == 0x0D {
                        state.start_online_city_search();
                    } else if code == 0x08 {
                        state.prayer_search_query.pop();
                        state.prayer_online_results = None;
                        state.prayer_loc_status = None;
                        state.request_redraw();
                    } else if !c.is_control() && state.prayer_search_query.chars().count() < 48 {
                        state.prayer_search_query.push(c);
                        state.prayer_online_results = None;
                        state.prayer_loc_status = None;
                        state.request_redraw();
                    }
                } else if !state.config.pip_mode
                    && state.settings_tab.is_none()
                    && !state.show_stats_modal
                    && !state.show_goal_modal
                    && !state.show_prayer_modal
                    && state.guard_tab == GuardTab::BlockedApps
                    && state.search_focused
                {
                    if code == 0x08 {
                        state.search_query.pop();
                        state.app_scroll = 0;
                        state.request_redraw();
                    } else if !c.is_control() && state.search_query.chars().count() < 48 {
                        state.search_query.push(c);
                        state.app_scroll = 0;
                        state.request_redraw();
                    }
                } else if !state.config.pip_mode
                    && state.settings_tab.is_none()
                    && !state.show_stats_modal
                    && !state.show_goal_modal
                    && !state.show_prayer_modal
                    && state.guard_tab == GuardTab::RestrictedSites
                    && state.site_input_focused
                {
                    if code == 0x0D {
                        let q = state.site_input_query.clone();
                        state.add_restricted_site(&q);
                    } else if code == 0x08 {
                        state.site_input_query.pop();
                        state.request_redraw();
                    } else if !c.is_control() && state.site_input_query.chars().count() < 64 {
                        state.site_input_query.push(c);
                        state.request_redraw();
                    }
                }
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if !state.config.pip_mode {
                let mut pt = POINT {
                    x: (lparam.0 & 0xFFFF) as i16 as i32,
                    y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
                };
                let _ = ScreenToClient(hwnd, &mut pt);
                let x = state.px_to_dip(pt.x);
                let y = state.px_to_dip(pt.y);
                let (w_dip, h_dip) = state.surface.dip_size();
                let layout = compute_layout(&state.layout_query(w_dip, h_dip));
                let delta = ((wparam.0 >> 16) & 0xFFFF) as u16 as i16;
                if state.settings_tab.is_some() || state.show_ambient_mixer || state.show_stats_modal {
                    if layout.modal_body_rect.is_some_and(|r| r.contains(x, y))
                        && layout.settings_max_scroll > 0.0
                    {
                        let next = (state.settings_scroll - (delta as f32 / 120.0) * 44.0)
                            .clamp(0.0, layout.settings_max_scroll);
                        if (next - state.settings_scroll).abs() > 0.1 {
                            state.settings_scroll = next;
                            state.request_redraw();
                        }
                    }
                } else if state.guard_tab == GuardTab::BlockedApps
                    && layout.guard_list_rect.is_some_and(|r| r.contains(x, y))
                {
                    let cap = blocked_apps_capacity(h_dip);
                    let max_scroll = state.filtered_apps().len().saturating_sub(cap);
                    let prev = state.app_scroll;
                    if delta < 0 {
                        state.app_scroll = (state.app_scroll + 1).min(max_scroll);
                    } else if delta > 0 {
                        state.app_scroll = state.app_scroll.saturating_sub(1);
                    }
                    if state.app_scroll != prev {
                        state.request_redraw();
                    }
                } else if state.guard_tab == GuardTab::RestrictedSites {
                    let total = state.site_entries().len();
                    let max_scroll = (total / 2).saturating_sub(3);
                    let prev = state.site_scroll;
                    if delta < 0 {
                        state.site_scroll = (state.site_scroll + 1).min(max_scroll);
                    } else if delta > 0 {
                        state.site_scroll = state.site_scroll.saturating_sub(1);
                    }
                    if state.site_scroll != prev {
                        state.request_redraw();
                    }
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            if state.dragging_slider.is_some() {
                state.dragging_slider = None;
                let _ = ReleaseCapture();
                let _ = state.config.save();
            }
            LRESULT(0)
        }
        WM_YTMPX_EVENT => {
            state.handle_ytmpx_events();
            LRESULT(0)
        }
        WM_FOREGROUND_APP => {
            state.handle_foreground_event();
            LRESULT(0)
        }
        WM_BRIDGE_EVENT => {
            state.handle_bridge_events();
            LRESULT(0)
        }
        WM_OVERLAY_EVENT => {
            state.handle_overlay_events();
            LRESULT(0)
        }
        WM_GIF_LOADED => {
            state.handle_gif_loaded();
            LRESULT(0)
        }
        WM_PRAYER_LOC_EVENT => {
            state.handle_prayer_loc_events();
            LRESULT(0)
        }
        WM_VINYL_COMMAND => {
            let cmd = match wparam.0 {
                0 => Some(MediaCommand::Prev),
                1 => Some(MediaCommand::PlayPause),
                2 => Some(MediaCommand::Next),
                3 => Some(MediaCommand::ToggleMusicWindow),
                4 => Some(MediaCommand::VolumeUp),
                5 => Some(MediaCommand::VolumeDown),
                _ => None,
            };
            if let Some(c) = cmd {
                state.send_media(c);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_DEFERRED_INIT_ID {
                let _ = KillTimer(Some(hwnd), TIMER_DEFERRED_INIT_ID);
                state.perform_deferred_init();
            } else if wparam.0 == TIMER_SEC_ID {
                state.on_second_tick();
            } else if wparam.0 == TIMER_IDLE_HEARTBEAT_ID {
                state.on_idle_heartbeat();
            } else if wparam.0 == TIMER_GIF_ID {
                let can_draw = IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool();
                if can_draw {
                    if let Some(anim) = &state.gpu_anim {
                        if anim.is_animated() {
                            state.frame_idx = (state.frame_idx + 1) % anim.frames.len();
                            state.request_redraw();
                        }
                    }
                }
                state.sync_gif_timer();
            } else if wparam.0 == TIMER_VINYL_ID {
                let now = Instant::now();
                let dt = now
                    .duration_since(state.last_vinyl_tick)
                    .as_secs_f32()
                    .clamp(0.0, 0.1);
                state.last_vinyl_tick = now;

                if let Some(ref mut vinyl) = state.peeking_vinyl {
                    vinyl.tick_and_render();
                }
                let mut need_render = false;
                if state.in_card_vinyl_animating() {
                    let deg_per_sec = 360.0 / state.config.vinyl_speed_sec.clamp(2.0, 24.0);
                    state.in_card_vinyl_angle =
                        (state.in_card_vinyl_angle + deg_per_sec * dt) % 360.0;
                    need_render = true;
                }
                if state.config.ambient_mode == AmbientMode::Default && state.timer.is_running() {
                    state.plant_sway_phase =
                        (state.plant_sway_phase + 2.2 * dt) % std::f32::consts::TAU;
                    need_render = true;
                }
                if state.wave_or_hourglass_animating() {
                    state.wave_phase = if state.wave_phase > 100_000.0 {
                        0.0
                    } else {
                        state.wave_phase + dt
                    };
                    need_render = true;
                }
                if state.marquee_animating() {
                    state.marquee_offset = (state.marquee_offset + 34.0 * dt) % 20000.0;
                    need_render = true;
                }
                if state.completed_spin_until.is_some() {
                    if state.completed_spin_until.is_some_and(|u| now >= u) {
                        state.completed_spin_until = None;
                    }
                    need_render = true;
                }
                if state.guard_refresh_spin_until.is_some() {
                    if state.guard_refresh_spin_until.is_some_and(|u| now >= u) {
                        state.guard_refresh_spin_until = None;
                    }
                    need_render = true;
                }
                if need_render && IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() {
                    state.render_now();
                }
                state.sync_vinyl_timer();
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let ev = (lparam.0 & 0xFFFF) as u32;
            if ev == 0x0202 || ev == 0x0203 {
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                } else {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
                let _ = SetForegroundWindow(hwnd);
                state.sync_vinyl_overlay();
                state.sync_gif_timer();
            } else if ev == 0x0205 {
                let is_pip = state.config.pip_mode;
                let is_active = state.timer.phase() != SessionPhase::Idle;
                let is_paused = state.timer.is_paused();
                let cmd = state
                    .tray
                    .as_ref()
                    .map(|t| t.show_context_menu(hwnd, is_pip, is_active, is_paused))
                    .unwrap_or(0);
                match cmd {
                    IDM_TRAY_RESTORE => {
                        if IsIconic(hwnd).as_bool() {
                            let _ = ShowWindow(hwnd, SW_RESTORE);
                        } else {
                            let _ = ShowWindow(hwnd, SW_SHOW);
                        }
                        let _ = SetForegroundWindow(hwnd);
                        state.sync_vinyl_overlay();
                        state.sync_gif_timer();
                    }
                    IDM_TRAY_TOGGLE_PIP => {
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        state.toggle_pip();
                        state.sync_gif_timer();
                        let _ = SetForegroundWindow(hwnd);
                    }
                    IDM_TRAY_PAUSE => {
                        state.handle_click(ControlId::BtnPlayPause, 0.0);
                    }
                    IDM_TRAY_STOP => {
                        state.handle_click(ControlId::BtnReset, 0.0);
                    }
                    IDM_TRAY_EXIT => {
                        state.is_closing = true;
                        state.force_quit = true;
                        state.persist_session_now();
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                state.is_closing = true;
                state.persist_session_now();
                state.save_resized_bounds(false);
                let _ = state.stats.save();
                let _ = state.config.save();
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            state.persist_session_now();
            if state.config.minimize_to_tray && !state.force_quit {
                let accent = state.effective_accent().to_string();
                let _ = ShowWindow(hwnd, SW_HIDE);
                if let Some(ref mut v) = state.peeking_vinyl {
                    v.update_settings(
                        false,
                        false,
                        false,
                        state.config.pip_peeking_vinyl_side,
                        state.config.vinyl_size_px,
                        state.config.vinyl_speed_sec,
                        state.config.show_vinyl_spindle,
                        &accent,
                    );
                }
            } else {
                state.is_closing = true;
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            state.is_closing = true;
            state.tray.take();
            state.persist_session_now();
            state.save_resized_bounds(false);
            let _ = state.stats.save();
            let _ = KillTimer(Some(hwnd), TIMER_SEC_ID);
            let _ = KillTimer(Some(hwnd), TIMER_GIF_ID);
            let _ = KillTimer(Some(hwnd), TIMER_VINYL_ID);
            let _ = KillTimer(Some(hwnd), TIMER_IDLE_HEARTBEAT_ID);
            let _ = KillTimer(Some(hwnd), TIMER_DEFERRED_INIT_ID);
            if let Some(ref vinyl) = state.peeking_vinyl {
                vinyl.destroy();
            }
            if let Some(ref hud) = state.health_hud {
                hud.destroy();
            }
            if let Some(ref mut ov) = state.overlay {
                ov.destroy();
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(ptr));
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => {
            let _ = POINT::default();
            let _ = ScreenToClient;
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

fn copy_text_to_clipboard(text: &str) -> bool {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("clip.exe")
        .stdin(Stdio::piped())
        .creation_flags(0x0800_0000)
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

fn copy_image_file_to_clipboard(path: &Path) -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let escaped = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \
         $img = [System.Drawing.Image]::FromFile('{escaped}'); \
         [System.Windows.Forms.Clipboard]::SetImage($img); \
         $img.Dispose()"
    );
    Command::new("powershell.exe")
        .args(["-NoProfile", "-Sta", "-Command", &script])
        .creation_flags(0x0800_0000)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::compute_end_time_hhmm;

    #[test]
    fn computes_wraparound_end_time_correctly() {
        assert_eq!(compute_end_time_hhmm(14, 10, 0, 6 * 3600 + 55 * 60), "21:05");
        assert_eq!(compute_end_time_hhmm(23, 30, 0, 45 * 60), "00:15");
    }
}
