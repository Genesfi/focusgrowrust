use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use focusgrow_gfx::{decode_image_bytes, DecodedFrame};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Media::MediaPlaybackType;
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::ytmpx::{MediaCommand, PlaybackState, TrackInfo, YtmpxUpdate, WM_YTMPX_EVENT};

pub struct SystemMediaBridge {
    stop_flag: Arc<AtomicBool>,
    cmd_tx: mpsc::Sender<MediaCommand>,
    _thread: Option<JoinHandle<()>>,
}

impl SystemMediaBridge {
    pub fn start(notify_hwnd: HWND, updates: Arc<Mutex<std::collections::VecDeque<YtmpxUpdate>>>) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let (cmd_tx, cmd_rx) = mpsc::channel::<MediaCommand>();
        let hwnd_raw = notify_hwnd.0 as isize;

        let stop_clone = Arc::clone(&stop_flag);
        let thread = thread::spawn(move || {
            run_smtc_loop(hwnd_raw, stop_clone, updates, cmd_rx);
        });

        Self {
            stop_flag,
            cmd_tx,
            _thread: Some(thread),
        }
    }

    pub fn send_command(&self, cmd: MediaCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

impl Drop for SystemMediaBridge {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

fn run_smtc_loop(
    hwnd_raw: isize,
    stop: Arc<AtomicBool>,
    updates: Arc<Mutex<std::collections::VecDeque<YtmpxUpdate>>>,
    cmd_rx: mpsc::Receiver<MediaCommand>,
) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Ok(op) => match op.join() {
            Ok(m) => Some(m),
            Err(_) => None,
        },
        Err(_) => None,
    };

    let Some(manager) = manager else {
        unsafe { CoUninitialize() };
        return;
    };

    let mut last_title = String::new();
    let mut last_artist = String::new();
    let mut last_status = PlaybackState::Stopped;

    while !stop.load(Ordering::Relaxed) {
        // Handle incoming transport commands
        if let Ok(cmd) = cmd_rx.try_recv() {
            if let Ok(session) = manager.GetCurrentSession() {
                match cmd {
                    MediaCommand::PlayPause => {
                        let _ = session.TryTogglePlayPauseAsync();
                    }
                    MediaCommand::Next => {
                        let _ = session.TrySkipNextAsync();
                    }
                    MediaCommand::Prev => {
                        let _ = session.TrySkipPreviousAsync();
                    }
                    _ => {}
                }
            }
        }

        let mut active_track_found = false;

        let mut active_session = manager.GetCurrentSession().ok();
        if let Some(ref s) = active_session {
            let is_playing = s.GetPlaybackInfo().ok()
                .and_then(|i| i.PlaybackStatus().ok())
                .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                .unwrap_or(false);
            if !is_playing {
                if let Ok(sessions) = manager.GetSessions() {
                    for other in sessions {
                        let other_playing = other.GetPlaybackInfo().ok()
                            .and_then(|i| i.PlaybackStatus().ok())
                            .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                            .unwrap_or(false);
                        if other_playing {
                            active_session = Some(other);
                            break;
                        }
                    }
                }
            }
        } else if let Ok(sessions) = manager.GetSessions() {
            for other in sessions {
                let other_playing = other.GetPlaybackInfo().ok()
                    .and_then(|i| i.PlaybackStatus().ok())
                    .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                    .unwrap_or(false);
                if other_playing {
                    active_session = Some(other);
                    break;
                }
            }
        }

        // Query active session
        if let Some(session) = active_session {
            let app_id = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
            let app_id_lower = app_id.to_lowercase();

            // Check playback status
            let mut current_playback = PlaybackState::Stopped;
            if let Ok(info) = session.GetPlaybackInfo() {
                if let Ok(status) = info.PlaybackStatus() {
                    current_playback = match status {
                        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => PlaybackState::Playing,
                        GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => PlaybackState::Paused,
                        _ => PlaybackState::Stopped,
                    };
                }
            }

            if current_playback != PlaybackState::Stopped {
                // Check media properties (title, artist, thumbnail)
                if let Ok(props_op) = session.TryGetMediaPropertiesAsync() {
                    if let Ok(props) = props_op.join() {
                        let title = props.Title().map(|s| s.to_string()).unwrap_or_default().trim().to_string();
                        let artist = props.Artist().map(|s| s.to_string()).unwrap_or_default().trim().to_string();
                        let album_title = props.AlbumTitle().map(|s| s.to_string()).unwrap_or_default().trim().to_string();

                        let playback_type = props.PlaybackType().ok().and_then(|pt| pt.Value().ok());
                        let is_video = playback_type == Some(MediaPlaybackType::Video);

                        let is_spotify_desktop = app_id_lower.contains("spotify");
                        let is_ytm_desktop = app_id_lower.contains("youtube music") || app_id_lower.contains("ytm");
                        let is_browser = app_id_lower.contains("edge")
                            || app_id_lower.contains("chrome")
                            || app_id_lower.contains("brave")
                            || app_id_lower.contains("firefox")
                            || app_id_lower.contains("opera")
                            || app_id_lower.contains("vivaldi")
                            || app_id_lower.contains("browser");

                        let title_lower = title.to_lowercase();
                        let artist_lower = artist.to_lowercase();

                        // Strict filter: Ignore regular YouTube videos/podcasts or video streams
                        let is_regular_youtube = is_video
                            || title_lower.contains(" - youtube")
                            || title_lower.ends_with("youtube")
                            || artist_lower == "youtube"
                            || artist_lower.contains("youtube")
                            || (title_lower.contains("youtube") && !title_lower.contains("youtube music"));

                        // Browser media is accepted as Spotify/Web music if not a video/regular YouTube
                        let is_spotify_web = is_browser
                            && !is_regular_youtube
                            && (playback_type == Some(MediaPlaybackType::Music) || !artist.is_empty() || !album_title.is_empty());

                        if !title.is_empty() && !is_regular_youtube && (is_spotify_desktop || is_spotify_web || is_ytm_desktop) {
                                active_track_found = true;
                                let track_changed = title != last_title || artist != last_artist;
                                let playback_changed = current_playback != last_status;

                                if track_changed || playback_changed {
                                    last_title = title.clone();
                                    last_artist = artist.clone();
                                    last_status = current_playback;

                                    // Try to read thumbnail image bytes
                                    let mut frame: Option<DecodedFrame> = None;
                                    if let Ok(thumb_ref) = props.Thumbnail() {
                                        if let Ok(open_op) = thumb_ref.OpenReadAsync() {
                                            if let Ok(stream) = open_op.join() {
                                                if let Ok(size) = stream.Size() {
                                                    if size > 0 && size < 10 * 1024 * 1024 {
                                                        if let Ok(reader) = DataReader::CreateDataReader(&stream) {
                                                            if reader.LoadAsync(size as u32).map(|op| op.join().is_ok()).unwrap_or(false) {
                                                                let mut bytes = vec![0u8; size as usize];
                                                                if reader.ReadBytes(&mut bytes).is_ok() {
                                                                    if let Ok(anim) = decode_image_bytes(&bytes) {
                                                                        frame = anim.frames.into_iter().next();
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    let source = if is_spotify_desktop {
                                        "spotify-desktop".to_string()
                                    } else if is_spotify_web {
                                        "spotify-web".to_string()
                                    } else {
                                        "ytm-desktop".to_string()
                                    };

                                    let art_url = format!("smtc://{}/{}", source, title);
                                    let track = TrackInfo {
                                        title,
                                        artist,
                                        art_url: art_url.clone(),
                                        source,
                                        exe_name: app_id,
                                    };

                                    if let Ok(mut q) = updates.lock() {
                                        q.push_back(YtmpxUpdate::TrackChanged {
                                            track: Some(track.clone()),
                                            playback: current_playback,
                                            from_smtc: true,
                                        });
                                        if let Some(f) = frame {
                                            q.push_back(YtmpxUpdate::CoverLoaded {
                                                art_url: track.art_url,
                                                frame: Some(f),
                                            });
                                        }
                                    }
                                    unsafe {
                                        let _ = PostMessageW(Some(HWND(hwnd_raw as *mut _)), WM_YTMPX_EVENT, WPARAM(0), LPARAM(0));
                                    }
                                }
                            }
                        }
                    }
                }
            }

        // If no active SMTC track (stopped, closed, or app exited):
        if !active_track_found {
            if !last_title.is_empty() || last_status != PlaybackState::Stopped {
                last_title.clear();
                last_artist.clear();
                last_status = PlaybackState::Stopped;

                if let Ok(mut q) = updates.lock() {
                    q.push_back(YtmpxUpdate::TrackChanged {
                        track: None,
                        playback: PlaybackState::Stopped,
                        from_smtc: true,
                    });
                }
                unsafe {
                    let _ = PostMessageW(Some(HWND(hwnd_raw as *mut _)), WM_YTMPX_EVENT, WPARAM(0), LPARAM(0));
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }

    unsafe {
        CoUninitialize();
    }
}
