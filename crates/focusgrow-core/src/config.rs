use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Active visual backdrop mode for the timer gauge/card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AmbientMode {
    #[default]
    Default,
    YtMusic,
    Custom,
}

/// How the custom GIF or vinyl artwork is masked inside the timer card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GifDisplayMode {
    #[default]
    Circle,
    Full,
}

/// Which edge of the card the external Peeking Vinyl disc slides out from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VinylSide {
    #[default]
    Left,
    Right,
}

/// Floating Picture-in-Picture display format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PipStyle {
    #[default]
    Card,
    Capsule,
}

/// Metadata for a recently uploaded custom GIF stored locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentGif {
    pub id: String,
    pub name: String,
    pub file_name: String,
}

/// Individual track toggle & volume for the procedural soundscape mixer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AmbientTrackConfig {
    pub active: bool,
    pub volume: u8,
}

impl Default for AmbientTrackConfig {
    fn default() -> Self {
        Self { active: false, volume: 50 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomAmbientPreset {
    pub id: String,
    pub name: String,
    pub tracks: Vec<(String, bool, u8)>,
}

/// Procedural soundscape mixer configuration matching WebView2's AmbientAudioEngine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AmbientConfig {
    pub enabled: bool,
    pub auto_play: bool,
    pub master_volume: u8,
    pub active_preset: String,
    pub typing: AmbientTrackConfig,
    pub fire: AmbientTrackConfig,
    pub cricket: AmbientTrackConfig,
    pub rain_drizzle: AmbientTrackConfig,
    pub rain_heavy: AmbientTrackConfig,
    pub night: AmbientTrackConfig,
    pub clock: AmbientTrackConfig,
    pub cafe: AmbientTrackConfig,
    pub cafe_jp: AmbientTrackConfig,
    pub ocean: AmbientTrackConfig,
    pub alpha: AmbientTrackConfig,
    #[serde(default)]
    pub custom_presets: Vec<CustomAmbientPreset>,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_play: true,
            master_volume: 80,
            active_preset: "night_coder".to_string(),
            typing: AmbientTrackConfig { active: true, volume: 65 },
            fire: AmbientTrackConfig { active: false, volume: 60 },
            cricket: AmbientTrackConfig { active: true, volume: 50 },
            rain_drizzle: AmbientTrackConfig { active: false, volume: 55 },
            rain_heavy: AmbientTrackConfig { active: false, volume: 50 },
            night: AmbientTrackConfig { active: true, volume: 35 },
            clock: AmbientTrackConfig { active: false, volume: 50 },
            cafe: AmbientTrackConfig { active: false, volume: 50 },
            cafe_jp: AmbientTrackConfig { active: false, volume: 50 },
            ocean: AmbientTrackConfig { active: false, volume: 45 },
            alpha: AmbientTrackConfig { active: false, volume: 30 },
            custom_presets: vec![
                CustomAmbientPreset {
                    id: "custom_1".to_string(),
                    name: "Custom 1".to_string(),
                    tracks: vec![
                        ("typing".into(), true, 60),
                        ("fire".into(), false, 50),
                        ("cricket".into(), false, 40),
                        ("rain_drizzle".into(), true, 55),
                        ("rain_heavy".into(), false, 40),
                        ("night".into(), false, 40),
                        ("clock".into(), false, 40),
                        ("cafe".into(), false, 40),
                        ("cafe_jp".into(), true, 65),
                        ("ocean".into(), false, 40),
                        ("alpha".into(), true, 35),
                    ],
                },
                CustomAmbientPreset {
                    id: "custom_2".to_string(),
                    name: "Custom 2".to_string(),
                    tracks: vec![
                        ("typing".into(), false, 50),
                        ("fire".into(), true, 70),
                        ("cricket".into(), true, 50),
                        ("rain_drizzle".into(), false, 40),
                        ("rain_heavy".into(), false, 40),
                        ("night".into(), true, 40),
                        ("clock".into(), true, 55),
                        ("cafe".into(), true, 50),
                        ("cafe_jp".into(), false, 40),
                        ("ocean".into(), false, 40),
                        ("alpha".into(), false, 30),
                    ],
                },
            ],
        }
    }
}

impl AmbientConfig {
    pub fn track(&self, id: &str) -> AmbientTrackConfig {
        match id {
            "typing" => self.typing,
            "fire" => self.fire,
            "cricket" => self.cricket,
            "rain_drizzle" => self.rain_drizzle,
            "rain_heavy" => self.rain_heavy,
            "night" => self.night,
            "clock" => self.clock,
            "cafe" => self.cafe,
            "cafe_jp" => self.cafe_jp,
            "ocean" => self.ocean,
            "alpha" => self.alpha,
            _ => AmbientTrackConfig::default(),
        }
    }

    pub fn track_mut(&mut self, id: &str) -> Option<&mut AmbientTrackConfig> {
        match id {
            "typing" => Some(&mut self.typing),
            "fire" => Some(&mut self.fire),
            "cricket" => Some(&mut self.cricket),
            "rain_drizzle" => Some(&mut self.rain_drizzle),
            "rain_heavy" => Some(&mut self.rain_heavy),
            "night" => Some(&mut self.night),
            "clock" => Some(&mut self.clock),
            "cafe" => Some(&mut self.cafe),
            "cafe_jp" => Some(&mut self.cafe_jp),
            "ocean" => Some(&mut self.ocean),
            "alpha" => Some(&mut self.alpha),
            _ => None,
        }
    }

    pub fn apply_preset(&mut self, preset_id: &str) -> bool {
        let rows: [(&str, bool, u8); 11] = match preset_id {
            "night_coder" => [
                ("typing", true, 65), ("fire", false, 55), ("cricket", true, 50),
                ("rain_drizzle", false, 45), ("rain_heavy", false, 45), ("night", true, 35),
                ("clock", false, 40), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", false, 45), ("alpha", false, 30),
            ],
            "campfire_cozy" => [
                ("typing", false, 50), ("fire", true, 70), ("cricket", true, 45),
                ("rain_drizzle", false, 40), ("rain_heavy", false, 40), ("night", true, 30),
                ("clock", false, 35), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", false, 45), ("alpha", false, 30),
            ],
            "rainy_cafe" => [
                ("typing", false, 60), ("fire", false, 40), ("cricket", false, 40),
                ("rain_drizzle", true, 65), ("rain_heavy", false, 40), ("night", false, 40),
                ("clock", false, 40), ("cafe", true, 65), ("cafe_jp", false, 40), ("ocean", false, 45), ("alpha", false, 30),
            ],
            "heavy_downpour" => [
                ("typing", false, 50), ("fire", false, 40), ("cricket", false, 40),
                ("rain_drizzle", false, 40), ("rain_heavy", true, 75), ("night", true, 40),
                ("clock", false, 30), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", false, 40), ("alpha", false, 30),
            ],
            "office_flow" => [
                ("typing", true, 75), ("fire", false, 40), ("cricket", false, 40),
                ("rain_drizzle", false, 40), ("rain_heavy", false, 40), ("night", false, 40),
                ("clock", true, 45), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", false, 45), ("alpha", true, 35),
            ],
            "zen_mind" => [
                ("typing", false, 50), ("fire", false, 40), ("cricket", false, 40),
                ("rain_drizzle", true, 35), ("rain_heavy", false, 40), ("night", false, 40),
                ("clock", false, 30), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", true, 40), ("alpha", true, 55),
            ],
            "ocean_night" => [
                ("typing", false, 50), ("fire", false, 40), ("cricket", true, 35),
                ("rain_drizzle", false, 40), ("rain_heavy", false, 40), ("night", true, 45),
                ("clock", false, 30), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", true, 65), ("alpha", false, 30),
            ],
            "focus_clock" => [
                ("typing", false, 50), ("fire", false, 40), ("cricket", false, 40),
                ("rain_drizzle", false, 40), ("rain_heavy", false, 40), ("night", false, 40),
                ("clock", true, 60), ("cafe", false, 40), ("cafe_jp", false, 40), ("ocean", false, 40), ("alpha", true, 30),
            ],
            _ => {
                if let Some(cp) = self.custom_presets.iter().find(|p| p.id == preset_id) {
                    let tracks = cp.tracks.clone();
                    self.active_preset = preset_id.to_string();
                    for (id, active, volume) in tracks {
                        if let Some(t) = self.track_mut(&id) {
                            t.active = active;
                            t.volume = volume;
                        }
                    }
                    return true;
                }
                return false;
            }
        };
        self.active_preset = preset_id.to_string();
        for (id, active, volume) in rows {
            if let Some(t) = self.track_mut(id) {
                t.active = active;
                t.volume = volume;
            }
        }
        true
    }

    pub fn save_current_as_custom(&mut self, slot: usize) -> bool {
        let current_tracks = vec![
            ("typing".to_string(), self.typing.active, self.typing.volume),
            ("fire".to_string(), self.fire.active, self.fire.volume),
            ("cricket".to_string(), self.cricket.active, self.cricket.volume),
            ("rain_drizzle".to_string(), self.rain_drizzle.active, self.rain_drizzle.volume),
            ("rain_heavy".to_string(), self.rain_heavy.active, self.rain_heavy.volume),
            ("night".to_string(), self.night.active, self.night.volume),
            ("clock".to_string(), self.clock.active, self.clock.volume),
            ("cafe".to_string(), self.cafe.active, self.cafe.volume),
            ("cafe_jp".to_string(), self.cafe_jp.active, self.cafe_jp.volume),
            ("ocean".to_string(), self.ocean.active, self.ocean.volume),
            ("alpha".to_string(), self.alpha.active, self.alpha.volume),
        ];

        while self.custom_presets.len() <= slot {
            let next_idx = self.custom_presets.len() + 1;
            self.custom_presets.push(CustomAmbientPreset {
                id: format!("custom_{}", next_idx),
                name: format!("Custom {}", next_idx),
                tracks: current_tracks.clone(),
            });
        }

        self.custom_presets[slot].tracks = current_tracks;
        self.active_preset = self.custom_presets[slot].id.clone();
        true
    }

    pub fn sanitize(&mut self) {
        self.master_volume = self.master_volume.min(100);
        for id in ["typing", "fire", "cricket", "rain_drizzle", "rain_heavy", "night", "clock", "cafe", "cafe_jp", "ocean", "alpha"] {
            if let Some(t) = self.track_mut(id) {
                t.volume = t.volume.min(100);
            }
        }
        for cp in &mut self.custom_presets {
            for (_, _, vol) in &mut cp.tracks {
                *vol = (*vol).min(100);
            }
        }
    }
}

/// Persistent user preferences for FocusGrow Native.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub total_focus_minutes: u32,
    pub focus_minutes: u32,
    pub break_minutes: u32,
    pub skip_breaks: bool,
    pub is_health_mode: bool,
    pub ambient_mode: AmbientMode,
    pub gif_display_mode: GifDisplayMode,
    pub gif_opacity: u8,
    pub custom_gif_id: Option<String>,
    pub custom_gif_name: String,
    pub recent_gifs: Vec<RecentGif>,
    pub pip_mode: bool,
    pub pip_style: PipStyle,
    pub pip_width: i32,
    pub pip_height: i32,
    pub pip_capsule_width: i32,
    pub pip_capsule_height: i32,
    pub pip_capsule_album_bg: bool,
    pub pip_capsule_album_spin: bool,
    pub pip_capsule_album_opacity: u8,
    pub pip_x: Option<i32>,
    pub pip_y: Option<i32>,
    pub hide_taskbar_in_pip: bool,
    pub accent_color: String,
    pub dynamic_album_enabled: bool,
    pub use_complementary_color: bool,
    pub pip_peeking_vinyl_enabled: bool,
    pub pip_peeking_vinyl_side: VinylSide,
    pub vinyl_speed_sec: f32,
    pub vinyl_size_px: i32,
    pub show_vinyl_spindle: bool,
    pub dashboard_width: i32,
    pub dashboard_height: i32,
    pub daily_goal_minutes: u32,
    pub auto_pause_enabled: bool,
    pub auto_pause_sec: u32,
    pub auto_close_blocked_apps: bool,
    pub prayer_enabled: bool,
    pub prayer_break_enabled: bool,
    pub prayer_advance_mins: u32,
    pub prayer_lat: f64,
    pub prayer_lng: f64,
    pub prayer_tz: i32,
    pub prayer_offset_mins: i32,
    pub blacklist_apps: Vec<String>,
    pub restricted_sites: Vec<String>,
    pub timer_theme: String,
    pub auto_rotate_hourglass: bool,
    pub stealth_active: bool,
    pub stealth_hover_peek: bool,
    pub neutral_overstay_enabled: bool,
    pub neutral_overstay_mins: u32,
    pub neutral_softblock_enabled: bool,
    pub block_all_monitors: bool,
    pub auto_pip: bool,
    pub auto_pip_on_blur: bool,
    pub notifications_enabled: bool,
    pub countdown_sound_enabled: bool,
    pub countdown_duration_secs: u8,
    pub countdown_sound_style: String,
    pub focus_sound: String,
    pub break_sound: String,
    pub warning_sound: String,
    pub custom_focus_audio_path: Option<String>,
    pub custom_break_audio_path: Option<String>,
    pub custom_warning_audio_path: Option<String>,
    #[serde(default)]
    pub custom_focus_audio_name: Option<String>,
    #[serde(default)]
    pub custom_break_audio_name: Option<String>,
    #[serde(default)]
    pub custom_warning_audio_name: Option<String>,
    pub ambient: AmbientConfig,
    pub minimize_to_tray: bool,
    pub app_language: String,
    pub completed_time_format: String,
    pub timer_subtitle_mode: String,
    pub app_categories: BTreeMap<String, String>,
    pub last_settings_tab: String,
    pub webview2_migrated: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            total_focus_minutes: 60,
            focus_minutes: 25,
            break_minutes: 5,
            skip_breaks: false,
            is_health_mode: false,
            ambient_mode: AmbientMode::Default,
            gif_display_mode: GifDisplayMode::Circle,
            gif_opacity: 78,
            custom_gif_id: None,
            custom_gif_name: String::new(),
            recent_gifs: Vec::new(),
            pip_mode: false,
            pip_style: PipStyle::Card,
            pip_width: 195,
            pip_height: 280,
            pip_capsule_width: 220,
            pip_capsule_height: 68,
            pip_capsule_album_bg: true,
            pip_capsule_album_spin: true,
            pip_capsule_album_opacity: 35,
            pip_x: None,
            pip_y: None,
            hide_taskbar_in_pip: false,
            accent_color: "#F59E0B".to_string(),
            dynamic_album_enabled: true,
            use_complementary_color: false,
            pip_peeking_vinyl_enabled: true,
            pip_peeking_vinyl_side: VinylSide::Left,
            vinyl_speed_sec: 6.0,
            vinyl_size_px: 340,
            show_vinyl_spindle: true,
            dashboard_width: 960,
            dashboard_height: 660,
            daily_goal_minutes: 240,
            auto_pause_enabled: true,
            auto_pause_sec: 15,
            auto_close_blocked_apps: false,
            prayer_enabled: true,
            prayer_break_enabled: true,
            prayer_advance_mins: 5,
            prayer_lat: -2.7903,
            prayer_lng: 115.2914,
            prayer_tz: 8,
            prayer_offset_mins: 0,
            blacklist_apps: vec![
                "facebook.exe".to_string(),
                "tiktok.exe".to_string(),
                "instagram.exe".to_string(),
            ],
            restricted_sites: vec![
                "facebook.com".to_string(),
                "fb.com".to_string(),
                "youtube.com".to_string(),
                "instagram.com".to_string(),
                "tiktok.com".to_string(),
                "twitter.com".to_string(),
                "x.com".to_string(),
                "reddit.com".to_string(),
            ],
            timer_theme: "classic".to_string(),
            auto_rotate_hourglass: false,
            stealth_active: false,
            stealth_hover_peek: true,
            neutral_overstay_enabled: true,
            neutral_overstay_mins: 15,
            neutral_softblock_enabled: true,
            block_all_monitors: false,
            auto_pip: false,
            auto_pip_on_blur: false,
            notifications_enabled: true,
            countdown_sound_enabled: true,
            countdown_duration_secs: 5,
            countdown_sound_style: "zen".to_string(),
            focus_sound: "default".to_string(),
            break_sound: "chime".to_string(),
            warning_sound: "reminder".to_string(),
            custom_focus_audio_path: None,
            custom_break_audio_path: None,
            custom_warning_audio_path: None,
            custom_focus_audio_name: None,
            custom_break_audio_name: None,
            custom_warning_audio_name: None,
            ambient: AmbientConfig::default(),
            minimize_to_tray: true,
            app_language: "en".to_string(),
            completed_time_format: "smart".to_string(),
            timer_subtitle_mode: "countdown".to_string(),
            app_categories: BTreeMap::new(),
            last_settings_tab: "appearance".to_string(),
            webview2_migrated: false,
        }
    }
}

pub fn default_app_category(exe_name: &str) -> &'static str {
    match exe_name.trim().to_ascii_lowercase().as_str() {
        "afterfx.exe"
        | "premiere.exe"
        | "photoshop.exe"
        | "illustrator.exe"
        | "blender.exe"
        | "figma.exe"
        | "canva.exe"
        | "audition.exe"
        | "obs64.exe"
        | "studio64.exe"
        | "code.exe"
        | "devenv.exe"
        | "rider64.exe"
        | "idea64.exe"
        | "pycharm64.exe"
        | "clion64.exe"
        | "webstorm64.exe"
        | "unity.exe"
        | "unrealeditor.exe"
        | "sublime_text.exe"
        | "cursor.exe"
        | "zed.exe"
        | "resolve.exe"
        | "antigravity ide.exe"
        | "antigravity.exe"
        | "git-bash.exe"
        | "windowsterminal.exe"
        | "powershell.exe"
        | "cmd.exe"
        | "word.exe"
        | "excel.exe"
        | "powerpnt.exe"
        | "notion.exe"
        | "obsidian.exe"
        | "focusgrow.exe"
        | "focusgrow-app.exe" => "productive",
        "steam.exe"
        | "epicgameslauncher.exe"
        | "discord.exe"
        | "spotify.exe"
        | "telegram.exe"
        | "whatsapp.exe"
        | "tiktok.exe"
        | "launcher_main.exe"
        | "client-win64-shipping.exe" => "distracting",
        _ => "neutral",
    }
}

impl AppConfig {
    pub fn app_category(&self, exe_name: &str) -> &'static str {
        let key = exe_name.trim().to_ascii_lowercase();
        if let Some(val) = self.app_categories.get(&key) {
            match val.as_str() {
                "productive" => return "productive",
                "distracting" => return "distracting",
                "neutral" => return "neutral",
                _ => {}
            }
        }
        default_app_category(&key)
    }

    pub fn cycle_app_category(&mut self, exe_name: &str) -> &'static str {
        let key = exe_name.trim().to_ascii_lowercase();
        if key.is_empty() {
            return "neutral";
        }
        let next = match self.app_category(&key) {
            "productive" => "neutral",
            "neutral" => "distracting",
            _ => "productive",
        };
        self.app_categories.insert(key, next.to_string());
        next
    }

    /// Clamps numeric preferences to safe UI/rendering bounds.
    pub fn sanitize(&mut self) {
        self.total_focus_minutes = self.total_focus_minutes.clamp(1, 720);
        self.focus_minutes = self.focus_minutes.clamp(1, 180);
        self.break_minutes = self.break_minutes.clamp(1, 60);
        self.gif_opacity = self.gif_opacity.clamp(10, 100);
        self.pip_width = self.pip_width.clamp(150, 440);
        self.pip_height = self.pip_height.clamp(210, 550);
        self.pip_capsule_width = self.pip_capsule_width.clamp(195, 520);
        self.pip_capsule_height = self.pip_capsule_height.clamp(68, 120);
        self.pip_capsule_album_opacity = self.pip_capsule_album_opacity.clamp(10, 100);
        self.dashboard_width = self.dashboard_width.clamp(760, 1920);
        self.dashboard_height = self.dashboard_height.clamp(520, 1200);
        self.daily_goal_minutes = self.daily_goal_minutes.clamp(15, 720);
        self.auto_pause_sec = self.auto_pause_sec.clamp(5, 600);
        self.neutral_overstay_mins = self.neutral_overstay_mins.clamp(2, 120);
        self.prayer_advance_mins = self.prayer_advance_mins.clamp(1, 30);
        self.prayer_tz = self.prayer_tz.clamp(-12, 14);
        self.vinyl_speed_sec = self.vinyl_speed_sec.clamp(1.0, 30.0);
        self.vinyl_size_px = self.vinyl_size_px.clamp(120, 850);
        self.countdown_duration_secs = match self.countdown_duration_secs {
            3 | 5 | 10 => self.countdown_duration_secs,
            _ => 5,
        };
        if self.countdown_sound_style != "digital" {
            self.countdown_sound_style = "zen".to_string();
        }
        if !matches!(self.completed_time_format.as_str(), "smart" | "decimal" | "minutes") {
            self.completed_time_format = "smart".to_string();
        }
        if !matches!(self.timer_subtitle_mode.as_str(), "countdown" | "finishtime" | "totalrem") {
            self.timer_subtitle_mode = "countdown".to_string();
        }
        self.ambient.sanitize();
        if self.recent_gifs.len() > 5 {
            self.recent_gifs.truncate(5);
        }
    }

    /// Resolves the root application data folder under `%LocalAppData%\FocusGrow`.
    pub fn data_dir() -> PathBuf {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            if !local_app_data.trim().is_empty() {
                return PathBuf::from(local_app_data).join("FocusGrow");
            }
        }
        PathBuf::from("data")
    }

    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.json")
    }

    pub fn gifs_dir() -> PathBuf {
        Self::data_dir().join("gifs")
    }

    pub fn sounds_dir() -> PathBuf {
        Self::data_dir().join("sounds")
    }

    /// Loads config from the default path, returning defaults if absent or unreadable.
    pub fn load() -> Self {
        Self::load_from(&Self::config_path())
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        let mut cfg: Self = serde_json::from_str(&raw).unwrap_or_default();
        if (cfg.pip_width == 280 && cfg.pip_height == 400)
            || (cfg.pip_width == 440 && cfg.pip_height == 550)
        {
            cfg.pip_width = 195;
            cfg.pip_height = 280;
        }
        if cfg.pip_capsule_width < 195 {
            cfg.pip_capsule_width = 220;
        }
        if cfg.pip_capsule_height < 68 {
            cfg.pip_capsule_height = 68;
        }
        cfg.sanitize();
        cfg
    }

    /// Saves config to the default path.
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::config_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config dir {}", parent.display()))?;
        }
        let mut copy = self.clone();
        copy.sanitize();
        let json = serde_json::to_string_pretty(&copy)?;
        fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_values() {
        let mut cfg = AppConfig {
            total_focus_minutes: 0,
            focus_minutes: 0,
            break_minutes: 999,
            gif_opacity: 2,
            pip_width: 90,
            pip_height: 100,
            vinyl_speed_sec: 0.1,
            vinyl_size_px: 2000,
            ..Default::default()
        };
        cfg.sanitize();
        assert_eq!(cfg.total_focus_minutes, 1);
        assert_eq!(cfg.focus_minutes, 1);
        assert_eq!(cfg.break_minutes, 60);
        assert_eq!(cfg.gif_opacity, 10);
        assert_eq!(cfg.pip_width, 150);
        assert_eq!(cfg.pip_height, 210);
        assert_eq!(cfg.vinyl_speed_sec, 1.0);
        assert_eq!(cfg.vinyl_size_px, 850);
    }

    #[test]
    fn roundtrip_save_and_load() {
        let temp_dir = std::env::temp_dir().join("focusgrow_core_cfg_test");
        let _ = fs::remove_dir_all(&temp_dir);
        let cfg_path = temp_dir.join("config.json");

        let cfg = AppConfig {
            total_focus_minutes: 360,
            focus_minutes: 30,
            ambient_mode: AmbientMode::Custom,
            gif_display_mode: GifDisplayMode::Full,
            gif_opacity: 64,
            pip_peeking_vinyl_side: VinylSide::Right,
            custom_focus_audio_name: Some("bell_alarm.wav".to_string()),
            ..Default::default()
        };
        cfg.save_to(&cfg_path).expect("save config");

        let loaded = AppConfig::load_from(&cfg_path);
        assert_eq!(loaded.total_focus_minutes, 360);
        assert_eq!(loaded.focus_minutes, 30);
        assert_eq!(loaded.ambient_mode, AmbientMode::Custom);
        assert_eq!(loaded.gif_display_mode, GifDisplayMode::Full);
        assert_eq!(loaded.gif_opacity, 64);
        assert_eq!(loaded.pip_peeking_vinyl_side, VinylSide::Right);
        assert_eq!(loaded.custom_focus_audio_name.as_deref(), Some("bell_alarm.wav"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn custom_presets_roundtrip_save_and_apply() {
        let mut cfg = AmbientConfig::default();
        cfg.typing.volume = 92;
        cfg.typing.active = true;
        cfg.cafe_jp.volume = 88;
        cfg.cafe_jp.active = true;
        cfg.fire.active = false;

        assert!(cfg.save_current_as_custom(0));
        assert_eq!(cfg.active_preset, "custom_1");

        // Switch to built-in preset
        assert!(cfg.apply_preset("night_coder"));
        assert_eq!(cfg.active_preset, "night_coder");
        assert_ne!(cfg.typing.volume, 92);

        // Restore custom 1
        assert!(cfg.apply_preset("custom_1"));
        assert_eq!(cfg.active_preset, "custom_1");
        assert_eq!(cfg.typing.volume, 92);
        assert!(cfg.typing.active);
        assert_eq!(cfg.cafe_jp.volume, 88);
        assert!(cfg.cafe_jp.active);
        assert!(!cfg.fire.active);
    }
}
