use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{AmbientMode, AppConfig, DailyStats, GifDisplayMode, VinylSide};

const SSTABLE_MAGIC: [u8; 8] = [0x57, 0xfb, 0x80, 0x8b, 0x24, 0x75, 0x47, 0xdb];
const STORAGE_KEY_NEEDLE: &[u8] = b"focusgrow_user_data_v1";

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub source_dir: PathBuf,
    pub streak_days: u32,
    pub today_minutes: u32,
    pub yesterday_minutes: u32,
    pub history_days: usize,
}

/// Discovers candidate Chromium WebView2 LocalStorage LevelDB directories used by the C++/WebView2 build.
pub fn find_webview2_leveldb_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let push_if_exists = |base: &Path, out: &mut Vec<PathBuf>| {
        let sub = base
            .join("FocusGrow.exe.WebView2")
            .join("EBWebView")
            .join("Default")
            .join("Local Storage")
            .join("leveldb");
        if sub.is_dir() && !out.contains(&sub) {
            out.push(sub);
        }
        let bin_sub = base
            .join("bin")
            .join("FocusGrow.exe.WebView2")
            .join("EBWebView")
            .join("Default")
            .join("Local Storage")
            .join("leveldb");
        if bin_sub.is_dir() && !out.contains(&bin_sub) {
            out.push(bin_sub);
        }
    };

    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        for _ in 0..6 {
            if let Some(dir) = cur {
                push_if_exists(dir, &mut candidates);
                cur = dir.parent();
            } else {
                break;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd.as_path());
        for _ in 0..4 {
            if let Some(dir) = cur {
                push_if_exists(dir, &mut candidates);
                cur = dir.parent();
            } else {
                break;
            }
        }
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let dir = PathBuf::from(local_app_data)
            .join("FocusGrow")
            .join("EBWebView")
            .join("Default")
            .join("Local Storage")
            .join("leveldb");
        if dir.is_dir() && !candidates.contains(&dir) {
            candidates.push(dir);
        }
    }
    candidates
}

/// Automatically locates the WebView2 database and merges its history, streak, and settings into `config` & `stats`.
pub fn auto_migrate_webview2(
    today_date: &str,
    yesterday_date: &str,
    config: &mut AppConfig,
    stats: &mut DailyStats,
) -> Result<MigrationReport> {
    let dirs = find_webview2_leveldb_dirs();
    let dir = dirs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("WebView2 user data folder (FocusGrow.exe.WebView2) not found"))?;
    migrate_from_leveldb_dir(&dir, today_date, yesterday_date, config, stats)
}

/// Reads `focusgrow_user_data_v1` directly from a Chromium LevelDB folder (`*.ldb` + `*.log`).
pub fn migrate_from_leveldb_dir(
    leveldb_dir: &Path,
    today_date: &str,
    yesterday_date: &str,
    config: &mut AppConfig,
    stats: &mut DailyStats,
) -> Result<MigrationReport> {
    let json_text = extract_latest_user_data_json(leveldb_dir)?;
    let root: Value =
        serde_json::from_str(&json_text).context("Failed to parse WebView2 userData JSON")?;
    apply_webview2_json(&root, today_date, yesterday_date, config, stats);
    config.webview2_migrated = true;
    Ok(MigrationReport {
        source_dir: leveldb_dir.to_path_buf(),
        streak_days: stats.streak_days,
        today_minutes: stats.completed_minutes_today,
        yesterday_minutes: stats.yesterday_minutes,
        history_days: stats.history.len(),
    })
}

pub fn apply_webview2_json(
    root: &Value,
    today_date: &str,
    yesterday_date: &str,
    config: &mut AppConfig,
    stats: &mut DailyStats,
) {
    // 1. Merge History (`completedMinutesByDate`)
    if let Some(hist) = root.get("completedMinutesByDate").and_then(|v| v.as_object()) {
        for (date_key, val) in hist {
            if let Some(mins) = val.as_u64() {
                let entry = stats.history.entry(date_key.clone()).or_insert(0);
                *entry = (*entry).max(mins as u32);
            }
        }
    }

    // Also include `lastDateStr` + `completedMinutesToday`
    let wv_last_date = root
        .get("lastDateStr")
        .and_then(|v| v.as_str())
        .unwrap_or(today_date);
    let wv_today_mins = root
        .get("completedMinutesToday")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if !wv_last_date.is_empty() && wv_today_mins > 0 {
        let entry = stats.history.entry(wv_last_date.to_string()).or_insert(0);
        *entry = (*entry).max(wv_today_mins);
    }

    if wv_last_date == today_date {
        stats.completed_minutes_today = stats.completed_minutes_today.max(wv_today_mins);
    } else if let Some(&hist_today) = stats.history.get(today_date) {
        stats.completed_minutes_today = stats.completed_minutes_today.max(hist_today);
    }

    // Yesterday minutes: prefer exact date in history, else WebView2 `yesterdayHours`
    if let Some(&y_mins) = stats.history.get(yesterday_date) {
        stats.yesterday_minutes = stats.yesterday_minutes.max(y_mins);
    } else if let Some(yh) = value_as_f64(root.get("yesterdayHours")) {
        let mins = (yh * 60.0).round().max(0.0) as u32;
        stats.yesterday_minutes = stats.yesterday_minutes.max(mins);
    }

    let wv_streak = root
        .get("streakDays")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let wv_health_goal = root
        .get("healthDailyGoalBreaks")
        .and_then(|v| v.as_u64())
        .unwrap_or(4) as u32;
    let wv_health_today = root
        .get("healthCompletedBreaksToday")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let wv_health_streak = root
        .get("healthStreakDays")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    stats.health_daily_goal_breaks = wv_health_goal.clamp(1, 16);
    if wv_last_date == today_date {
        stats.health_completed_breaks_today = stats.health_completed_breaks_today.max(wv_health_today);
    }
    if let Some(hstats) = root.get("healthStatsByDate").and_then(|v| v.as_object()) {
        for (date_key, val) in hstats {
            let b = val.get("breaks").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            if b > 0 {
                let e = stats.health_history.entry(date_key.clone()).or_insert(0);
                *e = (*e).max(b);
            }
        }
    }
    if let Some(hmap) = root.get("hourlyFocusStats").and_then(|v| v.as_object()) {
        for (date_key, val) in hmap {
            let entry = stats.hourly_history.entry(date_key.clone()).or_insert([0u32; 24]);
            if let Some(arr) = val.as_array() {
                for (i, item) in arr.iter().take(24).enumerate() {
                    let secs = (item.as_u64().unwrap_or(0) as u32).saturating_mul(60);
                    entry[i] = entry[i].max(secs);
                }
            } else if let Some(obj) = val.as_object() {
                for (hk, hv) in obj {
                    if let Ok(h) = hk.parse::<usize>() {
                        if h < 24 {
                            let secs = (hv.as_u64().unwrap_or(0) as u32).saturating_mul(60);
                            entry[h] = entry[h].max(secs);
                        }
                    }
                }
            }
        }
    }
    if let Some(amap) = root.get("appStatsByDate").and_then(|v| v.as_object()) {
        for (date_key, val) in amap {
            if let Some(obj) = val.as_object() {
                let day_e = stats.app_history.entry(date_key.clone()).or_default();
                for (exe, sec_val) in obj {
                    let secs = sec_val.as_u64().unwrap_or(0) as u32;
                    let ent = day_e.entry(exe.clone()).or_insert(0);
                    *ent = (*ent).max(secs);
                }
            }
        }
    }
    if let Some(wmap) = root.get("workTimeRangesByDate").and_then(|v| v.as_object()) {
        for (date_key, val) in wmap {
            let first = val
                .get("firstStart")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let last = val
                .get("lastActive")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !first.is_empty() || !last.is_empty() {
                stats.work_windows.entry(date_key.clone()).or_insert((first, last));
            }
        }
    }
    if let Some(nmap) = root.get("ignoredNudgeStatsByDate").and_then(|v| v.as_object()) {
        for (date_key, val) in nmap {
            if let Some(c) = val.as_u64() {
                let ent = stats.ignored_nudges.entry(date_key.clone()).or_insert(0);
                *ent = (*ent).max(c as u32);
            }
        }
    }
    if let Some(cats) = root.get("appCategories").and_then(|v| v.as_object()) {
        for (exe_k, cat_v) in cats {
            if let Some(cat) = cat_v.as_str() {
                let low_k = exe_k.trim().to_ascii_lowercase();
                if !low_k.is_empty() && matches!(cat, "productive" | "neutral" | "distracting") {
                    config
                        .app_categories
                        .entry(low_k)
                        .or_insert_with(|| cat.to_string());
                }
            }
        }
    }

    stats.streak_days = stats.streak_days.max(wv_streak);
    stats.health_streak_days = stats.health_streak_days.max(wv_health_streak);
    if !today_date.is_empty() {
        stats.last_date = today_date.to_string();
    }

    // 2. Migrate Config & Preferences
    if let Some(gh) = value_as_f64(root.get("dailyGoalHours")) {
        config.daily_goal_minutes = (gh * 60.0).round().clamp(15.0, 720.0) as u32;
    }
    if let Some(v) = root.get("selectedMins").and_then(|v| v.as_u64()) {
        config.total_focus_minutes = (v as u32).clamp(1, 720);
    }
    if let Some(v) = root.get("selectedPeriodMins").and_then(|v| v.as_u64()) {
        config.focus_minutes = (v as u32).clamp(1, 180);
    }
    if let Some(v) = root.get("selectedBreakMins").and_then(|v| v.as_u64()) {
        config.break_minutes = (v as u32).clamp(1, 60);
    }
    if let Some(v) = root.get("skipBreaks").and_then(|v| v.as_bool()) {
        config.skip_breaks = v;
    }
    if let Some(v) = root.get("sessionMode").and_then(|v| v.as_str()) {
        config.is_health_mode = v == "health";
    }
    if let Some(v) = root.get("timerTheme").and_then(|v| v.as_str()) {
        config.timer_theme = v.to_string();
    }
    if let Some(v) = root.get("ambientMode").and_then(|v| v.as_str()) {
        config.ambient_mode = match v {
            "ytmusic" => AmbientMode::YtMusic,
            "custom" => AmbientMode::Custom,
            _ => AmbientMode::Default,
        };
    }
    if let Some(v) = root.get("gifDisplayMode").and_then(|v| v.as_str()) {
        config.gif_display_mode = if v == "full" {
            GifDisplayMode::Full
        } else {
            GifDisplayMode::Circle
        };
    }
    if let Some(v) = root.get("gifOpacity").and_then(|v| v.as_u64()) {
        config.gif_opacity = (v as u8).clamp(10, 100);
    }
    if let Some(v) = root.get("accentColor").and_then(|v| v.as_str()) {
        if v.starts_with('#') && v.len() >= 7 {
            config.accent_color = v.to_string();
        }
    }
    if let Some(v) = root.get("dynamicAlbumEnabled").and_then(|v| v.as_bool()) {
        config.dynamic_album_enabled = v;
    } else if let Some(v) = root.get("accentMode").and_then(|v| v.as_str()) {
        config.dynamic_album_enabled = v == "ytmusic_dynamic";
    }
    if let Some(v) = root.get("useComplementaryColor").and_then(|v| v.as_bool()) {
        config.use_complementary_color = v;
    }
    if let Some(v) = root.get("pipPeekingVinylEnabled").and_then(|v| v.as_bool()) {
        config.pip_peeking_vinyl_enabled = v;
    }
    if let Some(v) = root.get("pipPeekingVinylSide").and_then(|v| v.as_str()) {
        config.pip_peeking_vinyl_side = if v == "right" {
            VinylSide::Right
        } else {
            VinylSide::Left
        };
    }
    if let Some(v) = value_as_f64(root.get("vinylSpeed")) {
        config.vinyl_speed_sec = (v as f32).clamp(1.0, 30.0);
    }
    if let Some(v) = root.get("vinylSize").and_then(|v| v.as_i64()) {
        config.vinyl_size_px = (v as i32).clamp(120, 850);
    }
    if let Some(v) = root.get("showVinylSpindle").and_then(|v| v.as_bool()) {
        config.show_vinyl_spindle = v;
    }
    if let Some(v) = root.get("autoPipOnStart").and_then(|v| v.as_bool()) {
        config.auto_pip = v;
    }
    if let Some(v) = root.get("autoPipOnBlur").and_then(|v| v.as_bool()) {
        config.auto_pip_on_blur = v;
    }
    if let Some(v) = root.get("hideTaskbarInPip").and_then(|v| v.as_bool()) {
        config.hide_taskbar_in_pip = v;
    }
    if let Some(v) = root.get("pipWidth").and_then(|v| v.as_i64()) {
        config.pip_width = (v as i32).clamp(150, 440);
    }
    if let Some(v) = root.get("pipHeight").and_then(|v| v.as_i64()) {
        config.pip_height = (v as i32).clamp(210, 550);
    }
    if let Some(v) = root.get("notificationsEnabled").and_then(|v| v.as_bool()) {
        config.notifications_enabled = v;
    }
    if let Some(v) = root.get("countdownSoundEnabled").and_then(|v| v.as_bool()) {
        config.countdown_sound_enabled = v;
    }
    if let Some(v) = root.get("countdownDurationSec").and_then(|v| v.as_u64()) {
        config.countdown_duration_secs = v as u8;
    }
    if let Some(v) = root.get("countdownSoundStyle").and_then(|v| v.as_str()) {
        config.countdown_sound_style = v.to_string();
    }
    if let Some(v) = root.get("focusNotificationSound").and_then(|v| v.as_str()) {
        config.focus_sound = v.to_string();
    }
    if let Some(v) = root.get("breakNotificationSound").and_then(|v| v.as_str()) {
        config.break_sound = v.to_string();
    }
    if let Some(v) = root.get("warningNotificationSound").and_then(|v| v.as_str()) {
        config.warning_sound = v.to_string();
    }
    if let Some(v) = root.get("completedTimeFormat").and_then(|v| v.as_str()) {
        config.completed_time_format = v.to_string();
    }
    if let Some(v) = root.get("timerSubtitleMode").and_then(|v| v.as_str()) {
        config.timer_subtitle_mode = v.to_string();
    }
    if let Some(v) = root.get("prayerEnabled").and_then(|v| v.as_bool()) {
        config.prayer_enabled = v;
    }
    if let Some(v) = root.get("prayerBreakEnabled").and_then(|v| v.as_bool()) {
        config.prayer_break_enabled = v;
    }
    if let Some(v) = root.get("prayerAdvance").and_then(|v| v.as_u64()) {
        config.prayer_advance_mins = (v as u32).clamp(1, 30);
    }
    if let Some(v) = value_as_f64(root.get("prayerLat")) {
        config.prayer_lat = v;
    }
    if let Some(v) = value_as_f64(root.get("prayerLng")) {
        config.prayer_lng = v;
    }
    if let Some(v) = root.get("prayerTz").and_then(|v| v.as_i64()) {
        config.prayer_tz = (v as i32).clamp(-12, 14);
    }
    if let Some(v) = root.get("autoPauseEnabled").and_then(|v| v.as_bool()) {
        config.auto_pause_enabled = v;
    }
    if let Some(v) = root.get("autoPauseSec").and_then(|v| v.as_u64()) {
        config.auto_pause_sec = (v as u32).clamp(5, 600);
    }
    if let Some(v) = root.get("autoCloseBlockedApps").and_then(|v| v.as_bool()) {
        config.auto_close_blocked_apps = v;
    }
    if let Some(v) = root.get("blockAllMonitors").and_then(|v| v.as_bool()) {
        config.block_all_monitors = v;
    }
    if let Some(v) = root.get("minimizeToTrayOnClose").and_then(|v| v.as_bool()) {
        config.minimize_to_tray = v;
    }
    if let Some(arr) = root.get("blockedApps").and_then(|v| v.as_array()) {
        let mut merged = config.blacklist_apps.clone();
        for item in arr {
            if let Some(s) = item.as_str() {
                if !merged.iter().any(|e| e.eq_ignore_ascii_case(s)) {
                    merged.push(s.to_string());
                }
            }
        }
        config.blacklist_apps = merged;
    }
    if let Some(arr) = root.get("restrictedSites").and_then(|v| v.as_array()) {
        let mut merged = config.restricted_sites.clone();
        for item in arr {
            if let Some(s) = item.as_str() {
                if !merged.iter().any(|e| e.eq_ignore_ascii_case(s)) {
                    merged.push(s.to_string());
                }
            }
        }
        config.restricted_sites = merged;
    }
    config.sanitize();
}

fn value_as_f64(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
}

fn extract_latest_user_data_json(leveldb_dir: &Path) -> Result<String> {
    let mut best: Option<(u64, u8, Vec<u8>)> = None;
    let mut files: Vec<PathBuf> = fs::read_dir(leveldb_dir)
        .with_context(|| format!("Cannot read LevelDB dir {}", leveldb_dir.display()))?
        .filter_map(|e| e.ok().map(|ent| ent.path()))
        .collect();
    files.sort();

    for path in &files {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("ldb") {
            if let Ok(bytes) = fs::read(path) {
                scan_sstable(&bytes, &mut best);
            }
        } else if ext.eq_ignore_ascii_case("log") {
            if let Ok(bytes) = fs::read(path) {
                scan_wal_log(&bytes, &mut best);
            }
        }
    }

    let (_, ktype, raw_val) = best
        .filter(|(_, ktype, val)| *ktype == 1 && !val.is_empty())
        .ok_or_else(|| anyhow!("No focusgrow_user_data_v1 record found in {}", leveldb_dir.display()))?;

    let _ = ktype;
    decode_chromium_ls_value(&raw_val)
}

fn decode_chromium_ls_value(val: &[u8]) -> Result<String> {
    if val.is_empty() {
        return Err(anyhow!("Empty LocalStorage value"));
    }
    let encoding = val[0];
    let body = &val[1..];
    if encoding == 0 {
        let mut units = Vec::with_capacity(body.len() / 2);
        for chunk in body.chunks_exact(2) {
            units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        Ok(String::from_utf16_lossy(&units))
    } else {
        Ok(String::from_utf8_lossy(body).into_owned())
    }
}

fn consider_entry(user_key: &[u8], seq: u64, ktype: u8, val: &[u8], best: &mut Option<(u64, u8, Vec<u8>)>) {
    if !user_key.windows(STORAGE_KEY_NEEDLE.len()).any(|w| w == STORAGE_KEY_NEEDLE) {
        return;
    }
    let replace = match best {
        None => true,
        Some((best_seq, _, _)) => seq >= *best_seq,
    };
    if replace {
        *best = Some((seq, ktype, val.to_vec()));
    }
}

fn read_varint(buf: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut res = 0u64;
    let mut shift = 0u32;
    while pos < buf.len() && shift < 64 {
        let b = buf[pos];
        pos += 1;
        res |= ((b & 0x7f) as u64) << shift;
        if (b & 0x80) == 0 {
            return Some((res, pos));
        }
        shift += 7;
    }
    None
}

fn snappy_decompress(src: &[u8]) -> Option<Vec<u8>> {
    let (ulen, mut pos) = read_varint(src, 0)?;
    let mut out = Vec::with_capacity((ulen as usize).min(4 * 1024 * 1024));
    while pos < src.len() {
        let tag = src[pos];
        pos += 1;
        match tag & 0x03 {
            0 => {
                let l = (tag >> 2) as usize;
                let lit_len = if l < 60 {
                    l + 1
                } else {
                    let extra = l - 59;
                    if pos + extra > src.len() {
                        return None;
                    }
                    let mut n = 0usize;
                    for i in 0..extra {
                        n |= (src[pos + i] as usize) << (8 * i);
                    }
                    pos += extra;
                    n + 1
                };
                if pos + lit_len > src.len() {
                    return None;
                }
                out.extend_from_slice(&src[pos..pos + lit_len]);
                pos += lit_len;
            }
            1 => {
                if pos >= src.len() {
                    return None;
                }
                let length = (((tag >> 2) & 0x07) as usize) + 4;
                let offset = (((tag & 0xe0) as usize) << 3) | (src[pos] as usize);
                pos += 1;
                copy_from_history(&mut out, offset, length)?;
            }
            2 => {
                if pos + 2 > src.len() {
                    return None;
                }
                let length = ((tag >> 2) as usize) + 1;
                let offset = u16::from_le_bytes([src[pos], src[pos + 1]]) as usize;
                pos += 2;
                copy_from_history(&mut out, offset, length)?;
            }
            _ => {
                if pos + 4 > src.len() {
                    return None;
                }
                let length = ((tag >> 2) as usize) + 1;
                let offset = u32::from_le_bytes([src[pos], src[pos + 1], src[pos + 2], src[pos + 3]]) as usize;
                pos += 4;
                copy_from_history(&mut out, offset, length)?;
            }
        }
    }
    Some(out)
}

fn copy_from_history(out: &mut Vec<u8>, offset: usize, length: usize) -> Option<()> {
    if offset == 0 || offset > out.len() {
        return None;
    }
    for _ in 0..length {
        let b = out[out.len() - offset];
        out.push(b);
    }
    Some(())
}

fn decode_sstable_block(data: &[u8], off: usize, sz: usize) -> Option<Vec<u8>> {
    if off.checked_add(sz)? >= data.len() {
        return None;
    }
    let comp_type = data[off + sz];
    let raw = &data[off..off + sz];
    if comp_type == 1 {
        snappy_decompress(raw)
    } else {
        Some(raw.to_vec())
    }
}

fn parse_block_entries(block: &[u8]) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    if block.len() < 4 {
        return None;
    }
    let num_restarts = u32::from_le_bytes(block[block.len() - 4..].try_into().ok()?) as usize;
    let footer_bytes = 4 + num_restarts.checked_mul(4)?;
    if footer_bytes > block.len() {
        return None;
    }
    let limit = block.len() - footer_bytes;
    let mut pos = 0usize;
    let mut key = Vec::new();
    let mut out = Vec::new();
    while pos < limit {
        let (shared, p1) = read_varint(block, pos)?;
        let (non_shared, p2) = read_varint(block, p1)?;
        let (val_len, p3) = read_varint(block, p2)?;
        let shared = shared as usize;
        let non_shared = non_shared as usize;
        let val_len = val_len as usize;
        if shared > key.len() || p3 + non_shared + val_len > limit {
            return None;
        }
        key.truncate(shared);
        key.extend_from_slice(&block[p3..p3 + non_shared]);
        let val_start = p3 + non_shared;
        let val = block[val_start..val_start + val_len].to_vec();
        pos = val_start + val_len;
        out.push((key.clone(), val));
    }
    Some(out)
}

fn scan_sstable(data: &[u8], best: &mut Option<(u64, u8, Vec<u8>)>) {
    if data.len() < 48 || data[data.len() - 8..] != SSTABLE_MAGIC {
        return;
    }
    let footer = &data[data.len() - 48..];
    let Some((_, p1)) = read_varint(footer, 0) else { return; };
    let Some((_, p2)) = read_varint(footer, p1) else { return; };
    let Some((idx_off, p3)) = read_varint(footer, p2) else { return; };
    let Some((idx_sz, _)) = read_varint(footer, p3) else { return; };
    let Some(idx_block) = decode_sstable_block(data, idx_off as usize, idx_sz as usize) else {
        return;
    };
    let Some(idx_entries) = parse_block_entries(&idx_block) else {
        return;
    };
    for (_, handle) in idx_entries {
        let Some((boff, hp)) = read_varint(&handle, 0) else { continue; };
        let Some((bsz, _)) = read_varint(&handle, hp) else { continue; };
        let Some(dblock) = decode_sstable_block(data, boff as usize, bsz as usize) else { continue; };
        let Some(entries) = parse_block_entries(&dblock) else { continue; };
        for (ikey, val) in entries {
            if ikey.len() < 8 {
                continue;
            }
            let user_key = &ikey[..ikey.len() - 8];
            let seq_and_type = u64::from_le_bytes(ikey[ikey.len() - 8..].try_into().unwrap());
            let ktype = (seq_and_type & 0xff) as u8;
            let seq = seq_and_type >> 8;
            consider_entry(user_key, seq, ktype, &val, best);
        }
    }
}

fn scan_wal_log(data: &[u8], best: &mut Option<(u64, u8, Vec<u8>)>) {
    const BLOCK_SIZE: usize = 32768;
    let mut records: Vec<Vec<u8>> = Vec::new();
    let mut current = Vec::new();

    for chunk in data.chunks(BLOCK_SIZE) {
        let mut pos = 0usize;
        while pos + 7 <= chunk.len() {
            let len = u16::from_le_bytes([chunk[pos + 4], chunk[pos + 5]]) as usize;
            let rec_type = chunk[pos + 6];
            pos += 7;
            if pos + len > chunk.len() {
                break;
            }
            let payload = &chunk[pos..pos + len];
            pos += len;
            match rec_type {
                1 => {
                    records.push(payload.to_vec());
                    current.clear();
                }
                2 => {
                    current.clear();
                    current.extend_from_slice(payload);
                }
                3 => {
                    current.extend_from_slice(payload);
                }
                4 => {
                    current.extend_from_slice(payload);
                    records.push(std::mem::take(&mut current));
                }
                _ => {}
            }
        }
    }

    for rec in records {
        if rec.len() < 12 {
            continue;
        }
        let mut seq = u64::from_le_bytes(rec[0..8].try_into().unwrap());
        let count = u32::from_le_bytes(rec[8..12].try_into().unwrap());
        let mut pos = 12usize;
        for _ in 0..count {
            if pos >= rec.len() {
                break;
            }
            let ktype = rec[pos];
            pos += 1;
            let Some((klen, p1)) = read_varint(&rec, pos) else { break; };
            let klen = klen as usize;
            if p1 + klen > rec.len() {
                break;
            }
            let user_key = &rec[p1..p1 + klen];
            pos = p1 + klen;
            if ktype == 1 {
                let Some((vlen, p2)) = read_varint(&rec, pos) else { break; };
                let vlen = vlen as usize;
                if p2 + vlen > rec.len() {
                    break;
                }
                let val = &rec[p2..p2 + vlen];
                pos = p2 + vlen;
                consider_entry(user_key, seq, 1, val, best);
            } else {
                consider_entry(user_key, seq, 0, &[], best);
            }
            seq += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_webview2_json_preserving_streak_and_history() {
        let json: Value = serde_json::json!({
            "dailyGoalHours": 4,
            "completedMinutesToday": 156,
            "yesterdayHours": "4.8",
            "streakDays": 71,
            "lastDateStr": "2026-11-12",
            "timerTheme": "wave",
            "ambientMode": "ytmusic",
            "completedTimeFormat": "smart",
            "completedMinutesByDate": {
                "2026-11-10": 479,
                "2026-11-11": 291
            }
        });
        let mut cfg = AppConfig::default();
        let mut stats = DailyStats::default();
        apply_webview2_json(&json, "2026-11-12", "2026-11-11", &mut cfg, &mut stats);

        assert_eq!(stats.streak_days, 71);
        assert_eq!(stats.completed_minutes_today, 156);
        assert_eq!(stats.yesterday_minutes, 291);
        assert_eq!(stats.history.get("2026-11-10"), Some(&479));
        assert_eq!(cfg.daily_goal_minutes, 240);
        assert_eq!(cfg.timer_theme, "wave");
        assert_eq!(cfg.ambient_mode, AmbientMode::YtMusic);
        assert_eq!(cfg.completed_time_format, "smart");
    }

    #[test]
    fn reads_real_workspace_webview2_database_if_present() {
        let path = Path::new("../../bin/FocusGrow.exe.WebView2/EBWebView/Default/Local Storage/leveldb");
        if path.is_dir() {
            let mut cfg = AppConfig::default();
            let mut stats = DailyStats::default();
            let report = migrate_from_leveldb_dir(path, "2026-11-12", "2026-11-11", &mut cfg, &mut stats)
                .expect("should parse workspace WebView2 leveldb");
            assert!(report.streak_days >= 1);
            assert!(report.history_days >= 1);
        }
    }
}

