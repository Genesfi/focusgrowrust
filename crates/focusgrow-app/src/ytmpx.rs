use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use focusgrow_gfx::{decode_image_bytes, DecodedFrame};
use serde::Deserialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{accept, client, Error as WsError, Message, WebSocket};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinInet::{
    InternetCloseHandle, InternetOpenUrlW, InternetOpenW, InternetReadFile,
    INTERNET_FLAG_IGNORE_CERT_CN_INVALID, INTERNET_FLAG_IGNORE_CERT_DATE_INVALID,
    INTERNET_FLAG_NO_CACHE_WRITE, INTERNET_FLAG_RELOAD, INTERNET_FLAG_SECURE,
    INTERNET_OPEN_TYPE_PRECONFIG,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN, VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

/// Posted to the main HWND when YTMPX state or decoded cover art updates arrive.
pub const WM_YTMPX_EVENT: u32 = WM_APP + 10;

const YTMPX_WS_URL: &str = "ws://127.0.0.1:8765";
const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const MAX_COVER_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    Prev,
    PlayPause,
    Next,
    ToggleMusicWindow,
    VolumeUp,
    VolumeDown,
}

impl MediaCommand {
    pub fn as_ws_command(self) -> &'static str {
        match self {
            Self::Prev => "prev",
            Self::PlayPause => "playPause",
            Self::Next => "next",
            Self::ToggleMusicWindow => "toggleMusicWindow",
            Self::VolumeUp => "volumeUp",
            Self::VolumeDown => "volumeDown",
        }
    }

    fn fallback_vk(self) -> Option<VIRTUAL_KEY> {
        match self {
            Self::Prev => Some(VK_MEDIA_PREV_TRACK),
            Self::PlayPause => Some(VK_MEDIA_PLAY_PAUSE),
            Self::Next => Some(VK_MEDIA_NEXT_TRACK),
            Self::VolumeUp => Some(VK_VOLUME_UP),
            Self::VolumeDown => Some(VK_VOLUME_DOWN),
            Self::ToggleMusicWindow => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub art_url: String,
    pub source: String,
    pub exe_name: String,
}

pub fn clean_sponsor_text(text: &str) -> String {
    let lower = text.to_lowercase();
    let ad_words = ["bersponsor", "sponsored", "promoted", "iklan", "ad"];

    for word in &ad_words {
        if lower.trim() == *word {
            return String::new();
        }
    }

    let mut result = text.to_string();
    for word in &ad_words {
        let w_len = word.len();
        if result.to_lowercase().starts_with(word) {
            let rest = &result[w_len..];
            result = rest
                .trim_start_matches(|c: char| c == ' ' || c == '•' || c == '·' || c == '-' || c == '—' || c == ':' || c == '|')
                .trim()
                .to_string();
        }
        if result.to_lowercase().ends_with(word) {
            let cut = result.len() - w_len;
            let rest = &result[..cut];
            result = rest
                .trim_end_matches(|c: char| c == ' ' || c == '•' || c == '·' || c == '-' || c == '—' || c == ':' || c == '|')
                .trim()
                .to_string();
        }
    }
    result
}

impl TrackInfo {
    pub fn ticker_text(&self) -> String {
        let clean_title = clean_sponsor_text(&self.title);
        let clean_artist = clean_sponsor_text(&self.artist);
        if clean_title.is_empty() {
            return clean_artist;
        }
        if clean_artist.is_empty() {
            clean_title
        } else {
            format!("{} — {}", clean_title, clean_artist)
        }
    }
}

#[derive(Clone)]
pub enum YtmpxUpdate {
    ConnectionChanged(bool),
    TrackChanged {
        track: Option<TrackInfo>,
        playback: PlaybackState,
        from_smtc: bool,
    },
    PlaybackChanged {
        playback: PlaybackState,
        from_smtc: bool,
    },
    CoverLoaded {
        art_url: String,
        frame: Option<DecodedFrame>,
    },
}

/// Manages WebSocket sync to YTMPX (`ws://127.0.0.1:8765`) and async album cover fetching.
pub struct YtmpxBridge {
    stop_flag: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    #[allow(dead_code)]
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cmd_tx: mpsc::Sender<MediaCommand>,
    _ws_thread: Option<JoinHandle<()>>,
    _cover_thread: Option<JoinHandle<()>>,
}

impl YtmpxBridge {
    pub fn start(notify_hwnd: HWND, updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();
        let (cover_tx, cover_rx) = mpsc::channel::<String>();

        let hwnd_raw = notify_hwnd.0 as isize;

        let ws_stop = Arc::clone(&stop_flag);
        let ws_connected = Arc::clone(&connected);
        let ws_updates = Arc::clone(&updates);
        let ws_thread = thread::spawn(move || {
            run_ws_loop(hwnd_raw, ws_stop, ws_connected, ws_updates, cmd_rx, cover_tx);
        });

        let cov_stop = Arc::clone(&stop_flag);
        let cov_updates = Arc::clone(&updates);
        let cover_thread = thread::spawn(move || {
            run_cover_loop(hwnd_raw, cov_stop, cov_updates, cover_rx);
        });

        Self {
            stop_flag,
            connected,
            updates,
            cmd_tx,
            _ws_thread: Some(ws_thread),
            _cover_thread: Some(cover_thread),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn send_command(&self, cmd: MediaCommand) {
        if self.connected.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(cmd);
        } else {
            send_media_key_fallback(cmd);
        }
    }

    #[allow(dead_code)]
    pub fn drain_updates(&self) -> Vec<YtmpxUpdate> {
        let mut guard = self.updates.lock().unwrap_or_else(|e| e.into_inner());
        guard.drain(..).collect()
    }
}

impl Drop for YtmpxBridge {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

fn push_update(hwnd_raw: isize, updates: &Arc<Mutex<VecDeque<YtmpxUpdate>>>, item: YtmpxUpdate) {
    if let Ok(mut guard) = updates.lock() {
        guard.push_back(item);
    }
    unsafe {
        let _ = PostMessageW(Some(HWND(hwnd_raw as *mut _)), WM_YTMPX_EVENT, WPARAM(0), LPARAM(0));
    }
}

fn run_ws_loop(
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    #[allow(dead_code)]
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    cover_tx: mpsc::Sender<String>,
) {
    // 1. Try to start as native embedded WebSocket server on :8765
    match TcpListener::bind("127.0.0.1:8765") {
        Ok(listener) => {
            run_server_loop(listener, hwnd_raw, stop, connected, updates, cmd_rx, cover_tx);
        }
        Err(_) => {
            // Port already bound by another process / existing server, fallback to client mode
            run_client_loop(hwnd_raw, stop, connected, updates, cmd_rx, cover_tx);
        }
    }
}

fn run_server_loop(
    listener: TcpListener,
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    #[allow(dead_code)]
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    cover_tx: mpsc::Sender<String>,
) {
    let _ = listener.set_nonblocking(true);
    let mut clients: Vec<WebSocket<TcpStream>> = Vec::new();
    let mut last_art_url = String::new();
    let mut was_connected = false;

    while !stop.load(Ordering::Relaxed) {
        // Accept any new incoming WebSocket client connections
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                if let Ok(ws) = accept(stream) {
                    let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(15)));
                    clients.push(ws);
                    if !was_connected {
                        was_connected = true;
                        connected.store(true, Ordering::Relaxed);
                        push_update(hwnd_raw, &updates, YtmpxUpdate::ConnectionChanged(true));
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }

        // Broadcast pending media commands to all connected clients
        while let Ok(cmd) = cmd_rx.try_recv() {
            let payload = format!(r#"{{"command":"{}"}}"#, cmd.as_ws_command());
            clients.retain_mut(|client| {
                client.send(Message::text(&payload)).is_ok()
            });
        }

        // Read incoming messages from all connected clients
        clients.retain_mut(|client| {
            loop {
                match client.read() {
                    Ok(msg) => {
                        if let Ok(text) = msg.to_text() {
                            if let Some(parsed) = parse_ytmpx_message(text) {
                                handle_parsed_message(
                                    hwnd_raw,
                                    parsed,
                                    &updates,
                                    &cover_tx,
                                    &mut last_art_url,
                                );
                            }
                        }
                    }
                    Err(WsError::Io(ref io_err))
                        if matches!(
                            io_err.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        return true;
                    }
                    Err(_) => {
                        // Client closed or faulted
                        return false;
                    }
                }
            }
        });

        // Update connection status
        let is_now_connected = !clients.is_empty();
        if is_now_connected != was_connected {
            was_connected = is_now_connected;
            connected.store(is_now_connected, Ordering::Relaxed);
            push_update(hwnd_raw, &updates, YtmpxUpdate::ConnectionChanged(is_now_connected));
            if !is_now_connected {
                last_art_url.clear();
                push_update(
                    hwnd_raw,
                    &updates,
                    YtmpxUpdate::TrackChanged {
                        track: None,
                        playback: PlaybackState::Stopped,
                        from_smtc: false,
                    },
                );
            }
        }

        sleep_interruptible(&stop, Duration::from_millis(35));
    }

    for mut c in clients {
        let _ = c.close(None);
    }
}

fn run_client_loop(
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    #[allow(dead_code)]
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
    cover_tx: mpsc::Sender<String>,
) {
    let mut last_art_url = String::new();

    while !stop.load(Ordering::Relaxed) {
        // Drain stale commands before connecting
        while cmd_rx.try_recv().is_ok() {}

        let tcp = match TcpStream::connect("127.0.0.1:8765") {
            Ok(s) => s,
            Err(_) => {
                if connected.swap(false, Ordering::Relaxed) {
                    push_update(hwnd_raw, &updates, YtmpxUpdate::ConnectionChanged(false));
                }
                sleep_interruptible(&stop, RECONNECT_DELAY);
                continue;
            }
        };
        let _ = tcp.set_read_timeout(Some(Duration::from_millis(120)));

        let mut ws = match client(YTMPX_WS_URL, MaybeTlsStream::Plain(tcp)) {
            Ok((sock, _)) => sock,
            Err(_) => {
                sleep_interruptible(&stop, RECONNECT_DELAY);
                continue;
            }
        };

        connected.store(true, Ordering::Relaxed);
        push_update(hwnd_raw, &updates, YtmpxUpdate::ConnectionChanged(true));

        while !stop.load(Ordering::Relaxed) {
            while let Ok(cmd) = cmd_rx.try_recv() {
                let payload = format!(r#"{{"command":"{}"}}"#, cmd.as_ws_command());
                if ws.send(Message::text(payload)).is_err() {
                    break;
                }
            }

            match ws.read() {
                Ok(msg) => {
                    if let Ok(text) = msg.to_text() {
                        if let Some(parsed) = parse_ytmpx_message(text) {
                            handle_parsed_message(
                                hwnd_raw,
                                parsed,
                                &updates,
                                &cover_tx,
                                &mut last_art_url,
                            );
                        }
                    }
                }
                Err(WsError::Io(ref io_err))
                    if matches!(
                        io_err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(_) => break,
            }
        }

        let _ = ws.close(None);
        if connected.swap(false, Ordering::Relaxed) {
            push_update(hwnd_raw, &updates, YtmpxUpdate::ConnectionChanged(false));
        }
        sleep_interruptible(&stop, RECONNECT_DELAY);
    }
}

fn handle_parsed_message(
    hwnd_raw: isize,
    parsed: ParsedWsMessage,
    updates: &Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cover_tx: &mpsc::Sender<String>,
    _last_art_url: &mut String,
) {
    match parsed {
        ParsedWsMessage::Track { track, playback } => {
            if !track.art_url.is_empty() {
                let _ = cover_tx.send(track.art_url.clone());
            }
            push_update(
                hwnd_raw,
                updates,
                YtmpxUpdate::TrackChanged {
                    track: Some(track),
                    playback,
                    from_smtc: false,
                },
            );
        }
        ParsedWsMessage::Playback(state) => {
            if state == PlaybackState::Stopped {
                push_update(
                    hwnd_raw,
                    updates,
                    YtmpxUpdate::TrackChanged {
                        track: None,
                        playback: PlaybackState::Stopped,
                        from_smtc: false,
                    },
                );
            } else {
                push_update(
                    hwnd_raw,
                    updates,
                    YtmpxUpdate::PlaybackChanged {
                        playback: state,
                        from_smtc: false,
                    },
                );
            }
        }
    }
}

fn run_cover_loop(
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    #[allow(dead_code)]
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cover_rx: mpsc::Receiver<String>,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let mut cache: std::collections::HashMap<String, Option<DecodedFrame>> = std::collections::HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        let first = match cover_rx.recv_timeout(Duration::from_millis(250)) {
            Ok(url) => url,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        // Keep only the latest queued URL so fast track skips drop stale artwork
        let mut latest = first;
        while let Ok(newer) = cover_rx.try_recv() {
            latest = newer;
        }

        if stop.load(Ordering::Relaxed) {
            break;
        }

        let frame = if let Some(cached) = cache.get(&latest) {
            cached.clone()
        } else {
            let fetched = fetch_and_decode_cover(&latest);
            if cache.len() > 30 {
                cache.clear();
            }
            cache.insert(latest.clone(), fetched.clone());
            fetched
        };

        push_update(
            hwnd_raw,
            &updates,
            YtmpxUpdate::CoverLoaded {
                art_url: latest,
                frame,
            },
        );
    }

    unsafe {
        CoUninitialize();
    }
}

fn fetch_and_decode_cover(art_url: &str) -> Option<DecodedFrame> {
    let clean_url = art_url.trim();
    if clean_url.is_empty() {
        return None;
    }

    let bytes = if let Some(rest) = clean_url.strip_prefix("data:") {
        let (_, b64) = rest.split_once("base64,")?;
        STANDARD.decode(b64.trim()).ok()?
    } else if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        // Optimize Google CDN URLs to force standard JPEG (-rj) instead of WebP (-rw),
        // allowing Windows WIC to decode images natively on all Windows systems without needing WebP codec extensions.
        let mut target_url = clean_url.to_string();
        if target_url.contains("googleusercontent.com") {
            if target_url.contains("-rw") {
                target_url = target_url.replace("-rw", "-rj");
            } else if !target_url.contains("-rj") {
                if target_url.contains('=') {
                    target_url.push_str("-rj");
                } else {
                    target_url.push_str("=w512-h512-l90-rj");
                }
            }
        }
        fetch_http_bytes(&target_url).or_else(|| fetch_http_bytes(clean_url))?
    } else if let Some(stripped) = clean_url.strip_prefix("//") {
        let full = format!("https://{stripped}");
        return fetch_and_decode_cover(&full);
    } else if let Some(path) = clean_url.strip_prefix("file:///") {
        std::fs::read(path).ok()?
    } else if let Some(path) = clean_url.strip_prefix("file://") {
        std::fs::read(path).ok()?
    } else if std::path::Path::new(clean_url).exists() {
        std::fs::read(clean_url).ok()?
    } else {
        return None;
    };

    let anim = decode_image_bytes(&bytes).ok()?;
    anim.frames.into_iter().next()
}

fn fetch_http_bytes(url: &str) -> Option<Vec<u8>> {
    // 1. Try fast in-process WinINet first
    if let Some(bytes) = fetch_wininet_bytes(url) {
        return Some(bytes);
    }

    // 2. Fallback to Windows curl.exe (handles complex SSL/proxies seamlessly)
    fetch_curl_bytes(url)
}

fn fetch_wininet_bytes(url: &str) -> Option<Vec<u8>> {
    let agent: Vec<u16> = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) FocusGrow/1.0\0".encode_utf16().collect();
    let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let h_net = InternetOpenW(
            PCWSTR(agent.as_ptr()),
            INTERNET_OPEN_TYPE_PRECONFIG.0,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if h_net.is_null() {
            return None;
        }

        let flags = if url.starts_with("https://") {
            INTERNET_FLAG_RELOAD
                | INTERNET_FLAG_NO_CACHE_WRITE
                | INTERNET_FLAG_SECURE
                | INTERNET_FLAG_IGNORE_CERT_CN_INVALID
                | INTERNET_FLAG_IGNORE_CERT_DATE_INVALID
        } else {
            INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE
        };

        let headers: Vec<u16> = "Accept: image/jpeg, image/png, image/*\r\n\0".encode_utf16().collect();
        let h_url = InternetOpenUrlW(
            h_net,
            PCWSTR(url_wide.as_ptr()),
            Some(&headers[..]),
            flags,
            None,
        );
        if h_url.is_null() {
            let _ = InternetCloseHandle(h_net);
            return None;
        }

        let mut data = Vec::with_capacity(64 * 1024);
        let mut chunk = [0u8; 8192];
        loop {
            let mut read = 0u32;
            let ok = InternetReadFile(
                h_url,
                chunk.as_mut_ptr() as *mut _,
                chunk.len() as u32,
                &mut read,
            );
            if ok.is_err() || read == 0 {
                break;
            }
            if data.len() + read as usize > MAX_COVER_BYTES {
                let _ = InternetCloseHandle(h_url);
                let _ = InternetCloseHandle(h_net);
                return None;
            }
            data.extend_from_slice(&chunk[..read as usize]);
        }

        let _ = InternetCloseHandle(h_url);
        let _ = InternetCloseHandle(h_net);
        (!data.is_empty()).then_some(data)
    }
}

fn fetch_curl_bytes(url: &str) -> Option<Vec<u8>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("curl.exe")
        .args(&[
            "-s",
            "-L",
            "--max-time",
            "4",
            "-H",
            "Accept: image/jpeg, image/png, image/*",
            "-A",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) FocusGrow/1.0",
            url,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if output.status.success() && !output.stdout.is_empty() && output.stdout.len() <= MAX_COVER_BYTES {
        Some(output.stdout)
    } else {
        None
    }
}

fn sleep_interruptible(stop: &AtomicBool, dur: Duration) {
    let steps = (dur.as_millis() / 100).max(1);
    for _ in 0..steps {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn send_media_key_fallback(cmd: MediaCommand) {
    let Some(vk) = cmd.fallback_vk() else {
        return;
    };
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParsedWsMessage {
    Track {
        track: TrackInfo,
        playback: PlaybackState,
    },
    Playback(PlaybackState),
}

#[derive(Deserialize)]
struct RawYtmpxPayload {
    command: Option<String>,
    event: Option<String>,
    source: Option<String>,
    #[serde(rename = "exeName")]
    exe_name: Option<String>,
    #[serde(rename = "isPlaying")]
    is_playing: Option<bool>,
    image: Option<String>,
    thumbnail: Option<String>,
    cover: Option<String>,
    art: Option<String>,
    #[serde(rename = "coverUrl")]
    cover_url: Option<String>,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
    #[serde(rename = "albumArtUrl")]
    album_art_url: Option<String>,
    #[serde(rename = "albumArt")]
    album_art: Option<String>,
    artwork: Option<String>,
    metadata: Option<RawMetadata>,
}

#[derive(Deserialize)]
struct RawMetadata {
    title: Option<String>,
    author: Option<String>,
    artist: Option<String>,
    image: Option<String>,
    thumbnail: Option<String>,
    cover: Option<String>,
    art: Option<String>,
    #[serde(rename = "coverUrl")]
    cover_url: Option<String>,
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
    #[serde(rename = "albumArtUrl")]
    album_art_url: Option<String>,
    #[serde(rename = "albumArt")]
    album_art: Option<String>,
    artwork: Option<String>,
    status: Option<String>,
}

pub(crate) fn parse_ytmpx_message(json_text: &str) -> Option<ParsedWsMessage> {
    let payload: RawYtmpxPayload = serde_json::from_str(json_text).ok()?;
    if payload.command.is_some() {
        return None;
    }

    let event = payload.event.as_deref().unwrap_or("");
    let state_override = match event {
        "stop" => return Some(ParsedWsMessage::Playback(PlaybackState::Stopped)),
        "pause" => Some(PlaybackState::Paused),
        "resume" => Some(PlaybackState::Playing),
        _ => None,
    };

    if let Some(ref meta) = payload.metadata {
        let title = meta.title.as_deref().unwrap_or_default().trim().to_string();
        if !title.is_empty() {
            let artist = meta
                .author
                .as_deref()
                .or(meta.artist.as_deref())
                .unwrap_or_default()
                .trim()
                .to_string();
            let art_url = meta
                .image
                .as_deref()
                .or(meta.thumbnail.as_deref())
                .or(meta.cover.as_deref())
                .or(meta.art.as_deref())
                .or(meta.cover_url.as_deref())
                .or(meta.thumbnail_url.as_deref())
                .or(meta.album_art_url.as_deref())
                .or(meta.album_art.as_deref())
                .or(meta.artwork.as_deref())
                .or(payload.image.as_deref())
                .or(payload.thumbnail.as_deref())
                .or(payload.cover.as_deref())
                .or(payload.art.as_deref())
                .or(payload.cover_url.as_deref())
                .or(payload.thumbnail_url.as_deref())
                .or(payload.album_art_url.as_deref())
                .or(payload.album_art.as_deref())
                .or(payload.artwork.as_deref())
                .unwrap_or_default()
                .trim()
                .to_string();

            let playback = state_override.unwrap_or_else(|| {
                if payload.is_playing == Some(false)
                    || meta.status.as_deref() == Some("paused")
                {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                }
            });

            let raw_src = payload.source.unwrap_or_default();
            let raw_exe = payload.exe_name.unwrap_or_default();
            let source = if raw_src.to_lowercase().contains("tauri") || raw_exe.to_lowercase().contains("tauri") {
                "ytm-tauri".to_string()
            } else if raw_src.is_empty() {
                "ytm-browser".to_string()
            } else {
                raw_src
            };

            return Some(ParsedWsMessage::Track {
                track: TrackInfo {
                    title: clean_sponsor_text(&title),
                    artist: clean_sponsor_text(&artist),
                    art_url,
                    source,
                    exe_name: raw_exe,
                },
                playback,
            });
        }
    }

    state_override.map(ParsedWsMessage::Playback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_echoed_command_messages() {
        let echo = r#"{"command":"playPause"}"#;
        assert!(parse_ytmpx_message(echo).is_none());
    }

    #[test]
    fn parses_track_and_playback_events() {
        let track_json = r#"{
            "event": "track",
            "source": "ytm-tauri",
            "metadata": {
                "title": "Nights Like This",
                "author": "The Kid LAROI",
                "image": "https://example.com/art.jpg"
            }
        }"#;
        assert_eq!(
            parse_ytmpx_message(track_json),
            Some(ParsedWsMessage::Track {
                track: TrackInfo {
                    title: "Nights Like This".to_string(),
                    artist: "The Kid LAROI".to_string(),
                    art_url: "https://example.com/art.jpg".to_string(),
                    source: "ytm-tauri".to_string(),
                    exe_name: String::new(),
                },
                playback: PlaybackState::Playing,
            })
        );

        assert_eq!(
            parse_ytmpx_message(r#"{"event":"pause"}"#),
            Some(ParsedWsMessage::Playback(PlaybackState::Paused))
        );
        assert_eq!(
            parse_ytmpx_message(r#"{"event":"resume"}"#),
            Some(ParsedWsMessage::Playback(PlaybackState::Playing))
        );
        assert_eq!(
            parse_ytmpx_message(r#"{"event":"stop"}"#),
            Some(ParsedWsMessage::Playback(PlaybackState::Stopped))
        );
    }

    #[test]
    fn formats_ticker_text_gracefully() {
        let track = TrackInfo {
            title: "Song Title".into(),
            artist: "Artist Name".into(),
            ..Default::default()
        };
        assert_eq!(track.ticker_text(), "Song Title — Artist Name");
    }
}
