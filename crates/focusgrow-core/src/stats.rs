use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::AppConfig;

fn default_health_goal_breaks() -> u32 {
    4
}

/// Persistent daily focus analytics, posture shield & streak state (`%LocalAppData%\FocusGrow\stats.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyStats {
    pub last_date: String,
    pub completed_minutes_today: u32,
    pub yesterday_minutes: u32,
    pub streak_days: u32,
    pub history: BTreeMap<String, u32>,
    #[serde(default = "default_health_goal_breaks")]
    pub health_daily_goal_breaks: u32,
    pub health_completed_breaks_today: u32,
    pub health_streak_days: u32,
    pub health_history: BTreeMap<String, u32>,
    pub hourly_history: BTreeMap<String, [u32; 24]>,
    pub hourly_app_history: BTreeMap<String, BTreeMap<usize, BTreeMap<String, u32>>>,
    pub app_history: BTreeMap<String, BTreeMap<String, u32>>,
    pub work_windows: BTreeMap<String, (String, String)>,
    pub ignored_nudges: BTreeMap<String, u32>,
    #[serde(skip)]
    sec_accum: u32,
}

impl Default for DailyStats {
    fn default() -> Self {
        Self {
            last_date: String::new(),
            completed_minutes_today: 0,
            yesterday_minutes: 0,
            streak_days: 0,
            history: BTreeMap::new(),
            health_daily_goal_breaks: 4,
            health_completed_breaks_today: 0,
            health_streak_days: 0,
            health_history: BTreeMap::new(),
            hourly_history: BTreeMap::new(),
            hourly_app_history: BTreeMap::new(),
            app_history: BTreeMap::new(),
            work_windows: BTreeMap::new(),
            ignored_nudges: BTreeMap::new(),
            sec_accum: 0,
        }
    }
}

impl DailyStats {
    pub fn stats_path() -> PathBuf {
        AppConfig::data_dir().join("stats.json")
    }

    pub fn load(today_date: &str) -> Self {
        let mut s = Self::load_from_path(&Self::stats_path()).unwrap_or_default();
        if s.health_daily_goal_breaks == 0 {
            s.health_daily_goal_breaks = 4;
        }
        s.rollover_if_needed(today_date);
        s
    }

    pub fn load_from_path(path: &Path) -> Option<Self> {
        let raw = fs::read_to_string(path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_path(&Self::stats_path())
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize DailyStats failed")?;
        fs::write(path, json).context("write stats.json failed")
    }

    pub fn is_shielded_today(&self) -> bool {
        self.health_completed_breaks_today >= self.health_daily_goal_breaks.max(1)
    }

    pub fn is_goal_met_with_tolerance(mins: u32, daily_goal_minutes: u32) -> bool {
        if mins == 0 {
            return false;
        }
        let target_mins = daily_goal_minutes.max(1);
        if mins >= target_mins {
            return true;
        }
        let mins_tenths = ((mins as f64 / 60.0) * 10.0).round();
        let target_tenths = ((target_mins as f64 / 60.0) * 10.0).round();
        mins_tenths >= target_tenths
    }

    pub fn day_minutes(&self, date: &str, today_date: &str) -> u32 {
        if date == today_date {
            return self
                .completed_minutes_today
                .max(*self.history.get(date).unwrap_or(&0));
        }
        if let Some(&m) = self.history.get(date) {
            if m > 0 {
                return m;
            }
        }
        if let Some(apps) = self.app_history.get(date) {
            let secs: u32 = apps.values().copied().sum();
            return secs / 60;
        }
        0
    }

    pub fn day_health_breaks(&self, date: &str, today_date: &str) -> u32 {
        if date == today_date {
            return self
                .health_completed_breaks_today
                .max(*self.health_history.get(date).unwrap_or(&0));
        }
        *self.health_history.get(date).unwrap_or(&0)
    }

    /// Recomputes both Focus Streak and Posture Streak matching WebView2's Shield/Freeze logic.
    pub fn recalculate_streaks(&mut self, today_date: &str, daily_goal_minutes: u32) {
        if today_date.is_empty() {
            return;
        }
        let goal_mins = daily_goal_minutes.max(15);
        let goal_breaks = self.health_daily_goal_breaks.max(1);

        let mut focus_streak = 0u32;
        if Self::is_goal_met_with_tolerance(self.day_minutes(today_date, today_date), goal_mins) {
            focus_streak += 1;
        }
        let mut cursor = prev_iso_date(today_date);
        for _ in 0..365 {
            let mins = self.day_minutes(&cursor, today_date);
            let breaks = self.day_health_breaks(&cursor, today_date);
            if Self::is_goal_met_with_tolerance(mins, goal_mins) {
                focus_streak += 1;
            } else if breaks >= goal_breaks {
                // Posture stand goal shields work focus streak without breaking it.
            } else {
                break;
            }
            cursor = prev_iso_date(&cursor);
        }
        if !self.history.is_empty() {
            self.streak_days = focus_streak;
        }

        let mut health_streak = 0u32;
        if self.day_health_breaks(today_date, today_date) >= goal_breaks {
            health_streak += 1;
        }
        let mut h_cursor = prev_iso_date(today_date);
        for _ in 0..365 {
            let breaks = self.day_health_breaks(&h_cursor, today_date);
            let mins = self.day_minutes(&h_cursor, today_date);
            if breaks >= goal_breaks {
                health_streak += 1;
            } else if Self::is_goal_met_with_tolerance(mins, goal_mins) {
                // Work goal shields posture streak from breaking.
            } else {
                break;
            }
            h_cursor = prev_iso_date(&h_cursor);
        }
        if !self.health_history.is_empty() || !self.history.is_empty() {
            self.health_streak_days = health_streak;
        }
    }

    /// Returns ranked `(exe_name, secs)` focused during a specific `hour` (`0..23`) across `[end_date - days + 1 ..= end_date]`.
    pub fn period_hourly_apps(&self, end_date: &str, days: usize, hour: usize) -> Vec<(String, u32)> {
        let mut totals: BTreeMap<String, u32> = BTreeMap::new();
        let mut cur = end_date.to_string();
        let h = hour.min(23);
        for _ in 0..days.max(1) {
            if let Some(hours_map) = self.hourly_app_history.get(&cur) {
                if let Some(apps_map) = hours_map.get(&h) {
                    for (exe, &secs) in apps_map {
                        if secs > 0 {
                            *totals.entry(exe.clone()).or_insert(0) += secs;
                        }
                    }
                }
            }
            cur = prev_iso_date(&cur);
        }
        let mut list: Vec<(String, u32)> = totals.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }

    /// Handles midnight date rollover cleanly without breaking an ongoing focus session.
    pub fn rollover_if_needed(&mut self, today_date: &str) -> bool {
        if self.last_date.is_empty() {
            self.last_date = today_date.to_string();
            return false;
        }
        if self.last_date == today_date {
            return false;
        }

        self.yesterday_minutes = self.completed_minutes_today;
        if self.completed_minutes_today > 0 {
            self.history
                .insert(self.last_date.clone(), self.completed_minutes_today);
        }
        if self.health_completed_breaks_today > 0 {
            self.health_history
                .insert(self.last_date.clone(), self.health_completed_breaks_today);
        }
        self.completed_minutes_today = 0;
        self.health_completed_breaks_today = 0;
        self.last_date = today_date.to_string();
        true
    }

    /// Backward-compatible wrapper used by existing unit tests.
    pub fn record_focus_second(&mut self, today_date: &str) -> bool {
        self.record_focus_second_with_context(today_date, 0, "", 60)
    }

    /// Accumulates focused seconds, per-hour distribution, app usage, work window, and streaks.
    pub fn record_focus_second_full(
        &mut self,
        today_date: &str,
        hour: usize,
        minute: usize,
        exe_name: &str,
        daily_goal_minutes: u32,
    ) -> bool {
        let hh12 = match hour % 12 {
            0 => 12,
            v => v,
        };
        let ampm = if hour >= 12 { "PM" } else { "AM" };
        let stamp = format!("{:02}:{:02} {}", hh12, minute.min(59), ampm);
        let entry = self
            .work_windows
            .entry(today_date.to_string())
            .or_insert_with(|| (stamp.clone(), stamp.clone()));
        if entry.0.is_empty() {
            entry.0 = stamp.clone();
        }
        entry.1 = stamp;
        self.record_focus_second_with_context(today_date, hour, exe_name, daily_goal_minutes)
    }

    /// Accumulates focused seconds, per-hour distribution, app usage breakdown, and streaks.
    pub fn record_focus_second_with_context(
        &mut self,
        today_date: &str,
        hour: usize,
        exe_name: &str,
        daily_goal_minutes: u32,
    ) -> bool {
        let rolled = self.rollover_if_needed(today_date);
        let h_bucket = self
            .hourly_history
            .entry(today_date.to_string())
            .or_insert([0u32; 24]);
        h_bucket[hour.min(23)] = h_bucket[hour.min(23)].saturating_add(1);

        let clean_exe = exe_name.trim();
        if !clean_exe.is_empty() {
            let h_idx = hour.min(23);
            let day_apps = self.app_history.entry(today_date.to_string()).or_default();
            let secs = day_apps.entry(clean_exe.to_string()).or_insert(0);
            *secs = secs.saturating_add(1);

            let hour_apps = self
                .hourly_app_history
                .entry(today_date.to_string())
                .or_default()
                .entry(h_idx)
                .or_default();
            let h_secs = hour_apps.entry(clean_exe.to_string()).or_insert(0);
            *h_secs = h_secs.saturating_add(1);
        }

        self.sec_accum += 1;
        if self.sec_accum < 60 {
            return rolled;
        }
        self.sec_accum = 0;
        self.completed_minutes_today += 1;
        self.history
            .insert(today_date.to_string(), self.completed_minutes_today);
        if self.history.len() == 1 && !Self::is_goal_met_with_tolerance(self.completed_minutes_today, daily_goal_minutes) {
            self.streak_days = self.streak_days.max(1);
        } else {
            self.recalculate_streaks(today_date, daily_goal_minutes);
        }
        true
    }

    /// Records a completed Posture Guard stand break and updates posture + shielded work streak.
    pub fn record_health_stand(&mut self, today_date: &str, daily_goal_minutes: u32) {
        self.rollover_if_needed(today_date);
        self.health_completed_breaks_today = self.health_completed_breaks_today.saturating_add(1);
        self.health_history
            .insert(today_date.to_string(), self.health_completed_breaks_today);
        self.recalculate_streaks(today_date, daily_goal_minutes);
    }

    /// Increments today's ignored-nudge / neutral-overstay counter.
    pub fn record_ignored_nudge(&mut self, today_date: &str) {
        self.rollover_if_needed(today_date);
        let cnt = self
            .ignored_nudges
            .entry(today_date.to_string())
            .or_insert(0);
        *cnt = cnt.saturating_add(1);
    }
}

/// Returns `"YYYY-MM-DD"` for the calendar day immediately preceding `date`.
pub fn prev_iso_date(date: &str) -> String {
    let mut parts = date.split('-');
    let y: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(2026);
    let m: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    if d > 1 {
        return format!("{:04}-{:02}-{:02}", y, m, d - 1);
    }
    let (py, pm) = if m > 1 { (y, m - 1) } else { (y - 1, 12) };
    let leap = (py % 4 == 0 && py % 100 != 0) || (py % 400 == 0);
    let dim = match pm {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    format!("{:04}-{:02}-{:02}", py, pm, dim)
}

/// Short weekday label (`"Mon".."Sun"`) via Sakamoto's algorithm for `"YYYY-MM-DD"`.
pub fn weekday_index(date: &str) -> usize {
    let mut parts = date.split('-');
    let mut y: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(2026);
    let m: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if m < 3 {
        y -= 1;
    }
    let idx = (y + y / 4 - y / 100 + y / 400 + T[(m.clamp(1, 12) - 1) as usize] + d) % 7;
    idx.rem_euclid(7) as usize
}

pub fn weekday_short(date: &str) -> &'static str {
    match weekday_index(date) {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        _ => "Sat",
    }
}

pub fn next_iso_date(date: &str) -> String {
    let mut parts = date.split('-');
    let y: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(2026);
    let m: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let max_d = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => if leap { 29 } else { 28 },
        _ => 30,
    };
    if d < max_d {
        format!("{:04}-{:02}-{:02}", y, m, d + 1)
    } else if m < 12 {
        format!("{:04}-{:02}-01", y, m + 1)
    } else {
        format!("{:04}-01-01", y + 1)
    }
}

pub fn month_short(date: &str) -> &'static str {
    let m: u32 = date
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    }
}

pub fn date_month_year(date: &str) -> String {
    let y = date.split('-').next().unwrap_or("2026");
    format!("{} {}", month_short(date), y)
}

pub fn format_full_date_label(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        let y = parts[0];
        let d = parts[2].trim_start_matches('0');
        let day_str = if d.is_empty() { "1" } else { d };
        return format!("{}, {} {} {}", weekday_short(date), day_str, month_short(date), y);
    }
    date.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_minutes_and_rolls_over_midnight() {
        let mut stats = DailyStats::default();
        assert!(!stats.rollover_if_needed("2026-11-12"));

        for _ in 0..59 {
            assert!(!stats.record_focus_second("2026-11-12"));
        }
        assert!(stats.record_focus_second("2026-11-12"));
        assert_eq!(stats.completed_minutes_today, 1);

        assert!(stats.rollover_if_needed("2026-11-13"));
        assert_eq!(stats.completed_minutes_today, 0);
        assert_eq!(stats.yesterday_minutes, 1);
        assert_eq!(stats.history.get("2026-11-12"), Some(&1));
    }

    #[test]
    fn streak_shield_preserves_streak_when_posture_goal_met() {
        let mut stats = DailyStats::default();
        stats.health_daily_goal_breaks = 4;
        stats.history.insert("2026-11-09".into(), 240);
        stats.history.insert("2026-11-10".into(), 240);
        // 2026-11-11 missed focus, but hit 4 posture stands -> Shield active
        stats.health_history.insert("2026-11-11".into(), 4);
        stats.completed_minutes_today = 240;
        stats.history.insert("2026-11-12".into(), 240);
        stats.recalculate_streaks("2026-11-12", 240);
        assert_eq!(stats.streak_days, 3);
    }

    #[test]
    fn tracks_hourly_app_usage_and_ranks_per_hour() {
        let mut stats = DailyStats::default();
        for _ in 0..90 {
            stats.record_focus_second_with_context("2026-11-12", 14, "Code.exe", 240);
        }
        for _ in 0..30 {
            stats.record_focus_second_with_context("2026-11-12", 14, "msedge.exe", 240);
        }
        let ranked = stats.period_hourly_apps("2026-11-12", 1, 14);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0], ("Code.exe".to_string(), 90));
        assert_eq!(ranked[1], ("msedge.exe".to_string(), 30));
        assert_eq!(format_full_date_label("2026-11-12"), "Thu, 12 Nov 2026");
    }
}
