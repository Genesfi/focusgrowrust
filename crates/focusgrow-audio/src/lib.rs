pub mod synth;
pub mod wasapi;

use focusgrow_core::AmbientConfig;
use std::path::Path;
use std::sync::{Arc, Mutex};
use synth::SynthBank;
pub use synth::{AlertKind, AMBIENT_PRESETS, AMBIENT_TRACKS};
pub use wasapi::stop_custom_audio;
use wasapi::{play_custom_audio_file, sync_synth, trigger_synth_alert, SharedAudioState, WasapiWorker};

pub const SOUND_STYLES: [&str; 6] = ["default", "chime", "reminder", "alarm", "custom", "none"];

pub fn sound_style_label(style: &str) -> &'static str {
    match style {
        "chime" => "Zen Chime",
        "reminder" => "Soft Reminder",
        "alarm" => "Alarm Bell",
        "custom" => "Custom Audio",
        "none" => "Silent",
        _ => "Default Bell",
    }
}

pub fn next_sound_style(current: &str) -> String {
    let idx = SOUND_STYLES.iter().position(|s| *s == current).unwrap_or(0);
    SOUND_STYLES[(idx + 1) % SOUND_STYLES.len()].to_string()
}

/// High-level WASAPI Procedural Soundscape & Notification Engine.
pub struct AudioEngine {
    shared: Arc<Mutex<SharedAudioState>>,
    _worker: WasapiWorker,
    playing: bool,
    master_muted: bool,
}

impl AudioEngine {
    pub fn start(initial_cfg: &AmbientConfig) -> Self {
        let shared = Arc::new(Mutex::new(SharedAudioState {
            synth: SynthBank::new(initial_cfg),
        }));
        let worker = WasapiWorker::spawn(Arc::clone(&shared));
        Self {
            shared,
            _worker: worker,
            playing: false,
            master_muted: false,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn is_master_muted(&self) -> bool {
        self.master_muted
    }

    pub fn set_playing(&mut self, playing: bool, cfg: &AmbientConfig) {
        self.playing = playing;
        sync_synth(&self.shared, cfg, self.playing, self.master_muted);
    }

    pub fn toggle_playing(&mut self, cfg: &AmbientConfig) {
        if self.master_muted && !self.playing {
            self.master_muted = false;
        }
        self.playing = !self.playing;
        sync_synth(&self.shared, cfg, self.playing, self.master_muted);
    }

    pub fn set_master_muted(&mut self, muted: bool, cfg: &AmbientConfig) {
        self.master_muted = muted;
        sync_synth(&self.shared, cfg, self.playing, self.master_muted);
    }

    pub fn sync(&self, cfg: &AmbientConfig) {
        sync_synth(&self.shared, cfg, self.playing, self.master_muted);
    }

    pub fn play_notification(&self, style: &str, custom_path: Option<&str>) {
        if style == "none" {
            return;
        }
        if style == "custom" {
            if let Some(p) = custom_path {
                let path = Path::new(p);
                if path.exists() {
                    play_custom_audio_file(path);
                    return;
                }
            }
        }
        if let Some(kind) = AlertKind::from_name(style) {
            trigger_synth_alert(&self.shared, kind);
        } else {
            trigger_synth_alert(&self.shared, AlertKind::Default);
        }
    }

    pub fn play_countdown_pip(&self, step: u8) {
        trigger_synth_alert(&self.shared, AlertKind::CountdownPip(step));
    }
}

