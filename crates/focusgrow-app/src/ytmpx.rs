use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use focusgrow_gfx::{decode_image_bytes, DecodedFrame};
use serde::Deserialize;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{client, Error as WsError, Message};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinInet::{
    InternetCloseHandle, InternetOpenUrlW, InternetOpenW, InternetReadFile, INTERNET_FLAG_NO_CACHE_WRITE,
    INTERNET_FLAG_RELOAD, INTERNET_OPEN_TYPE_PRECONFIG,
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

impl TrackInfo {
    pub fn ticker_text(&self) -> String {
        if self.title.is_empty() {
            return String::new();
        }
        if self.artist.is_empty() {
            self.title.clone()
        } else {
            format!("{} — {}", self.title, self.artist)
        }
    }
}

#[derive(Clone)]
pub enum YtmpxUpdate {
    ConnectionChanged(bool),
    TrackChanged {
        track: Option<TrackInfo>,
        playback: PlaybackState,
    },
    PlaybackChanged(PlaybackState),
    CoverLoaded {
        art_url: String,
        frame: Option<DecodedFrame>,
    },
}

/// Manages WebSocket sync to YTMPX (`ws://127.0.0.1:8765`) and async album cover fetching.
pub struct YtmpxBridge {
    stop_flag: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cmd_tx: mpsc::Sender<MediaCommand>,
    _ws_thread: Option<JoinHandle<()>>,
    _cover_thread: Option<JoinHandle<()>>,
}

impl YtmpxBridge {
    pub fn start(notify_hwnd: HWND) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let updates = Arc::new(Mutex::new(VecDeque::new()));
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

    pub fn send_command(&self, cmd: MediaCommand) {
        if self.connected.load(Ordering::Relaxed) {
            let _ = self.cmd_tx.send(cmd);
        } else {
            send_media_key_fallback(cmd);
        }
    }

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
    last_art_url: &mut String,
) {
    match parsed {
        ParsedWsMessage::Track { track, playback } => {
            let art_changed = !track.art_url.is_empty() && track.art_url != *last_art_url;
            if art_changed {
                *last_art_url = track.art_url.clone();
                let _ = cover_tx.send(track.art_url.clone());
            }
            push_update(
                hwnd_raw,
                updates,
                YtmpxUpdate::TrackChanged {
                    track: Some(track),
                    playback,
                },
            );
        }
        ParsedWsMessage::Playback(state) => {
            if state == PlaybackState::Stopped {
                last_art_url.clear();
                push_update(
                    hwnd_raw,
                    updates,
                    YtmpxUpdate::TrackChanged {
                        track: None,
                        playback: PlaybackState::Stopped,
                    },
                );
            } else {
                push_update(hwnd_raw, updates, YtmpxUpdate::PlaybackChanged(state));
            }
        }
    }
}

fn run_cover_loop(
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    updates: Arc<Mutex<VecDeque<YtmpxUpdate>>>,
    cover_rx: mpsc::Receiver<String>,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

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

        let frame = fetch_and_decode_cover(&latest);
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
    let bytes = if let Some(rest) = art_url.strip_prefix("data:") {
        let (_, b64) = rest.split_once("base64,")?;
        STANDARD.decode(b64.trim()).ok()?
    } else if art_url.starts_with("http://") || art_url.starts_with("https://") {
        fetch_http_bytes(art_url)?
    } else {
        return None;
    };

    let anim = decode_image_bytes(&bytes).ok()?;
    anim.frames.into_iter().next()
}

fn fetch_http_bytes(url: &str) -> Option<Vec<u8>> {
    let agent: Vec<u16> = "FocusGrow/1.0\0".encode_utf16().collect();
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

        let h_url = InternetOpenUrlW(
            h_net,
            PCWSTR(url_wide.as_ptr()),
            None,
            INTERNET_FLAG_RELOAD | INTERNET_FLAG_NO_CACHE_WRITE,
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

fn sleep_interruptible(stop: &AtomicBool, dur: Duration) {
    let steps = (dur.as_millis() / 100).max(1);
    for _ in 0..steps {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn send_media_key_fallback(cmd: MediaCommand) {
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
    metadata: Option<RawMetadata>,
}

#[derive(Deserialize)]
struct RawMetadata {
    title: Option<String>,
    author: Option<String>,
    artist: Option<String>,
    image: Option<String>,
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

    if let Some(meta) = payload.metadata {
        let title = meta.title.unwrap_or_default().trim().to_string();
        if !title.is_empty() {
            let artist = meta
                .author
                .or(meta.artist)
                .unwrap_or_default()
                .trim()
                .to_string();
            let art_url = meta.image.unwrap_or_default().trim().to_string();

            let playback = state_override.unwrap_or_else(|| {
                if payload.is_playing == Some(false)
                    || meta.status.as_deref() == Some("paused")
                {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                }
            });

            return Some(ParsedWsMessage::Track {
                track: TrackInfo {
                    title,
                    artist,
                    art_url,
                    source: payload.source.unwrap_or_default(),
                    exe_name: payload.exe_name.unwrap_or_default(),
                    
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
