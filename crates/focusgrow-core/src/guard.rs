use serde::Serialize;

use crate::prayer::PrayerManager;
use crate::stats::DailyStats;
use crate::{AppConfig, FocusTimer, SessionPhase};

const SYSTEM_WHITELIST: &[&str] = &[
    "focusgrow.exe",
    "focusgrow-app.exe",
    "focusgrow_app.exe",
    "explorer.exe",
    "msedgewebview2.exe",
    "photos.exe",
    "microsoft.photos.exe",
    "openwith.exe",
    "pickerhost.exe",
    "snippingtool.exe",
    "screenclippinghost.exe",
    "screensketch.exe",
    "applicationframehost.exe",
    "taskmgr.exe",
    "dwm.exe",
    "searchhost.exe",
    "shellexperiencehost.exe",
    "systemsettings.exe",
    "textinputhost.exe",
];

const BROWSER_WHITELIST: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "firefox.exe",
    "opera.exe",
    "vivaldi.exe",
    "arc.exe",
    "whale.exe",
];

#[derive(Debug, Clone, Serialize)]
pub struct RestrictedSiteState {
    pub domain: String,
    pub max_passes_per_session: u32,
    pub used_passes_this_session: u32,
    pub pass_remaining_sec: u32,
}

#[derive(Debug, Clone)]
pub struct GuardState {
    pub restricted_sites: Vec<RestrictedSiteState>,
    pub blacklist_apps: Vec<String>,
    pub active_domain: String,
    pub active_url: String,
    pub active_exe: String,
    pub active_title: String,
}

impl GuardState {
    pub fn from_config(config: &AppConfig) -> Self {
        let restricted_sites = config
            .restricted_sites
            .iter()
            .map(|d| RestrictedSiteState {
                domain: d.to_ascii_lowercase(),
                max_passes_per_session: 2,
                used_passes_this_session: 0,
                pass_remaining_sec: 0,
            })
            .collect();

        Self {
            restricted_sites,
            blacklist_apps: config
                .blacklist_apps
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect(),
            active_domain: String::new(),
            active_url: String::new(),
            active_exe: String::new(),
            active_title: String::new(),
        }
    }

    pub fn sync_from_config(&mut self, config: &AppConfig) {
        self.blacklist_apps = config
            .blacklist_apps
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        self.restricted_sites = config
            .restricted_sites
            .iter()
            .map(|d| {
                let low = d.to_ascii_lowercase();
                if let Some(existing) = self.restricted_sites.iter().find(|s| s.domain == low) {
                    existing.clone()
                } else {
                    RestrictedSiteState {
                        domain: low,
                        max_passes_per_session: 2,
                        used_passes_this_session: 0,
                        pass_remaining_sec: 0,
                    }
                }
            })
            .collect();
    }

    pub fn reset_session_passes(&mut self) {
        for site in &mut self.restricted_sites {
            site.used_passes_this_session = 0;
            site.pass_remaining_sec = 0;
        }
    }

    pub fn tick_passes_one_second(&mut self) {
        for site in &mut self.restricted_sites {
            if site.pass_remaining_sec > 0 {
                site.pass_remaining_sec -= 1;
            }
        }
    }

    pub fn grant_temporary_pass(&mut self, domain: &str, minutes: u32) -> bool {
        let low = domain.trim().to_ascii_lowercase();
        if low.is_empty() {
            return false;
        }
        let secs = minutes.clamp(1, 30) * 60;
        for site in &mut self.restricted_sites {
            if low == site.domain || low.ends_with(&format!(".{}", site.domain)) || low.contains(&site.domain) {
                if site.used_passes_this_session >= site.max_passes_per_session {
                    return false;
                }
                site.used_passes_this_session += 1;
                site.pass_remaining_sec = secs;
                return true;
            }
        }
        false
    }

    pub fn is_domain_blocked(&self, domain_or_title: &str, timer: &FocusTimer) -> Option<&RestrictedSiteState> {
        if domain_or_title.is_empty() || timer.phase() != SessionPhase::Focusing || timer.is_paused() {
            return None;
        }
        let low = domain_or_title.trim().to_ascii_lowercase();
        for site in &self.restricted_sites {
            let matches = low == site.domain
                || low.ends_with(&format!(".{}", site.domain))
                || low.contains(&site.domain);
            if matches && site.pass_remaining_sec == 0 {
                return Some(site);
            }
        }
        None
    }

    pub fn is_exe_blacklisted(&self, exe_name: &str, timer: &FocusTimer) -> bool {
        self.is_app_blacklisted(exe_name, "", timer)
    }

    pub fn is_app_blacklisted(&self, exe_name: &str, window_title: &str, timer: &FocusTimer) -> bool {
        if (exe_name.is_empty() && window_title.is_empty())
            || timer.phase() != SessionPhase::Focusing
            || timer.is_paused()
        {
            return false;
        }
        let low_exe = exe_name.trim().to_ascii_lowercase();
        let low_title = window_title.trim().to_ascii_lowercase();
        if low_exe == "focusgrow.exe" || low_exe == "focusgrow-app.exe" {
            return false;
        }
        if SYSTEM_WHITELIST.iter().any(|w| *w == low_exe)
            || BROWSER_WHITELIST.iter().any(|b| *b == low_exe)
        {
            return false;
        }
        self.blacklist_apps.iter().any(|b| {
            let item = b.trim();
            !item.is_empty() && (low_exe == item || (!low_title.is_empty() && low_title.contains(item)))
        })
    }

    /// Builds a JSON payload compatible with FocusGrow's HTTP `GET /state` contract on `:8766`.
    pub fn build_state_json(
        &self,
        config: &AppConfig,
        timer: &FocusTimer,
        stats: &DailyStats,
        today_date: &str,
        year: i32,
        month: i32,
        day: i32,
        cur_hour: f64,
        is_auto_paused: bool,
    ) -> String {
        let pm = PrayerManager::new(config.prayer_lat, config.prayer_lng, config.prayer_tz);
        let times = pm.calculate(year, month, day);
        let (next_name, next_hr, sec_to_next) = pm.next_prayer(year, month, day, cur_hour);

        let state_str = match timer.phase() {
            SessionPhase::Idle => "idle",
            SessionPhase::Focusing => "focusing",
            SessionPhase::Resting => "resting",
        };

        let active_passes: Vec<serde_json::Value> = self
            .restricted_sites
            .iter()
            .map(|s| {
                let passes_left = s
                    .max_passes_per_session
                    .saturating_sub(s.used_passes_this_session);
                serde_json::json!({
                    "domain": s.domain,
                    "remainingSec": s.pass_remaining_sec,
                    "passesLeft": passes_left,
                })
            })
            .collect();

        let restricted: Vec<&str> = self
            .restricted_sites
            .iter()
            .map(|s| s.domain.as_str())
            .collect();

        serde_json::json!({
            "state": state_str,
            "date": today_date,
            "remainingSec": timer.remaining_sec(),
            "maxPeriodSec": timer.total_phase_sec(),
            "formattedTime": timer.formatted_time(),
            "isPaused": timer.is_paused(),
            "isAutoPaused": is_auto_paused,
            "currentPeriod": timer.current_period(),
            "totalPeriods": timer.total_periods(),
            "completedMinutes": stats.completed_minutes_today,
            "dailyGoalHours": config.daily_goal_minutes / 60,
            "yesterdayHours": ((stats.yesterday_minutes as f64 / 60.0) * 10.0).round() / 10.0,
            "streakDays": stats.streak_days,
            "isHealthMode": config.is_health_mode,
            "skipBreaks": config.skip_breaks,
            "activeExe": self.active_exe,
            "activeDomain": self.active_domain,
            "prayer": {
                "enabled": config.prayer_enabled,
                "nextName": next_name,
                "nextTime": PrayerManager::format_hhmm(next_hr),
                "secToNext": sec_to_next,
                "allTimes": [
                    { "name": "Subuh", "time": PrayerManager::format_hhmm(times.subuh) },
                    { "name": "Dhuha", "time": PrayerManager::format_hhmm(times.dhuha) },
                    { "name": "Dzuhur", "time": PrayerManager::format_hhmm(times.dzuhur) },
                    { "name": "Ashar", "time": PrayerManager::format_hhmm(times.ashar) },
                    { "name": "Maghrib", "time": PrayerManager::format_hhmm(times.maghrib) },
                    { "name": "Isya", "time": PrayerManager::format_hhmm(times.isya) },
                ],
            },
            "activePasses": active_passes,
            "restrictedSites": restricted,
        })
        .to_string()
    }
}

/// Outcome of advancing neutral-app overstay tracking by one second during active focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeutralOverstayEvent {
    None,
    CheckIn1x { app: String, mins: u32 },
    SoftBlock2x { app: String, mins: u32 },
}

/// Tracks continuous neutral-app usage during active Productive Focus sessions only.
/// Fires a 1x check-in reminder at `neutral_overstay_mins` and escalates to a 2x
/// notification + optional soft-block at `2 * neutral_overstay_mins`.
#[derive(Debug, Default, Clone)]
pub struct NeutralOverstayTracker {
    continuous_neutral_sec: u32,
    continuous_productive_sec: u32,
    warned_1x: bool,
    triggered_2x: bool,
    pub softblock_active: bool,
    pub softblock_app: Option<String>,
}

impl NeutralOverstayTracker {
    pub fn reset(&mut self) {
        self.continuous_neutral_sec = 0;
        self.continuous_productive_sec = 0;
        self.warned_1x = false;
        self.triggered_2x = false;
        self.softblock_active = false;
        self.softblock_app = None;
    }

    /// Dismisses an active soft-block overlay (`[Esc]`) and grants a fresh clean interval.
    pub fn dismiss_softblock(&mut self) {
        self.softblock_active = false;
        self.softblock_app = None;
        self.continuous_neutral_sec = 0;
        self.warned_1x = false;
        self.triggered_2x = false;
    }

    pub fn should_show_overlay(
        &self,
        in_productive_focus: bool,
        config: &AppConfig,
        active_exe: &str,
    ) -> bool {
        self.softblock_active
            && in_productive_focus
            && config.neutral_overstay_enabled
            && config.neutral_softblock_enabled
            && config.app_category(active_exe) != "productive"
    }

    pub fn tick_one_second(
        &mut self,
        in_productive_focus: bool,
        config: &AppConfig,
        active_exe: &str,
    ) -> NeutralOverstayEvent {
        if !in_productive_focus || !config.neutral_overstay_enabled {
            self.reset();
            return NeutralOverstayEvent::None;
        }

        let clean = active_exe.trim();
        let lower = clean.to_ascii_lowercase();
        if lower.is_empty()
            || lower == "focusgrow.exe"
            || lower == "focusgrow_app.exe"
            || lower == "focusgrow-app.exe"
        {
            return NeutralOverstayEvent::None;
        }

        match config.app_category(clean) {
            "productive" => {
                self.continuous_productive_sec = self.continuous_productive_sec.saturating_add(1);
                if self.continuous_productive_sec >= 60 {
                    self.reset();
                }
                NeutralOverstayEvent::None
            }
            "neutral" => {
                self.continuous_productive_sec = 0;
                if self.softblock_active {
                    return NeutralOverstayEvent::None;
                }
                self.continuous_neutral_sec = self.continuous_neutral_sec.saturating_add(1);
                let mins = config.neutral_overstay_mins.clamp(5, 120);
                let interval_sec = mins * 60;
                if !self.warned_1x && self.continuous_neutral_sec >= interval_sec {
                    self.warned_1x = true;
                    return NeutralOverstayEvent::CheckIn1x {
                        app: clean.to_string(),
                        mins,
                    };
                }
                if !self.triggered_2x && self.continuous_neutral_sec >= interval_sec.saturating_mul(2)
                {
                    self.triggered_2x = true;
                    if config.neutral_softblock_enabled {
                        self.softblock_active = true;
                        self.softblock_app = Some(clean.to_string());
                    }
                    return NeutralOverstayEvent::SoftBlock2x {
                        app: clean.to_string(),
                        mins: mins * 2,
                    };
                }
                NeutralOverstayEvent::None
            }
            _ => NeutralOverstayEvent::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn domain_blocking_respects_focus_phase_and_temporary_pass() {
        let cfg = AppConfig::default();
        let mut guard = GuardState::from_config(&cfg);
        let mut timer = FocusTimer::new(60, 25, 5, false);

        // Idle -> not blocked
        assert!(guard.is_domain_blocked("youtube.com", &timer).is_none());

        timer.start(Instant::now());
        assert!(guard.is_domain_blocked("youtube.com", &timer).is_some());

        // Grant pass #1
        assert!(guard.grant_temporary_pass("youtube.com", 5));
        assert!(guard.is_domain_blocked("youtube.com", &timer).is_none());

        // Exhaust remaining pass duration
        for _ in 0..300 {
            guard.tick_passes_one_second();
        }
        assert!(guard.is_domain_blocked("youtube.com", &timer).is_some());

        // Grant pass #2 (max)
        assert!(guard.grant_temporary_pass("youtube.com", 5));
        for _ in 0..300 {
            guard.tick_passes_one_second();
        }
        // Pass #3 should be rejected
        assert!(!guard.grant_temporary_pass("youtube.com", 5));
    }

    #[test]
    fn system_whitelist_is_never_blacklisted() {
        let mut cfg = AppConfig::default();
        cfg.blacklist_apps.push("explorer.exe".into());
        cfg.blacklist_apps.push("discord.exe".into());
        let guard = GuardState::from_config(&cfg);
        let mut timer = FocusTimer::new(60, 25, 5, false);
        timer.start(Instant::now());

        assert!(!guard.is_exe_blacklisted("explorer.exe", &timer));
        assert!(guard.is_exe_blacklisted("Discord.exe", &timer));
    }

    #[test]
    fn neutral_overstay_triggers_1x_checkin_2x_softblock_and_productive_reset() {
        let mut cfg = AppConfig::default();
        cfg.neutral_overstay_enabled = true;
        cfg.neutral_overstay_mins = 5; // 300s 1x, 600s 2x
        cfg.neutral_softblock_enabled = true;

        let mut tracker = NeutralOverstayTracker::default();

        // Not in productive focus (e.g. Posture Guard or Idle) -> never accumulates
        for _ in 0..700 {
            assert_eq!(
                tracker.tick_one_second(false, &cfg, "msedge.exe"),
                NeutralOverstayEvent::None
            );
        }
        assert!(!tracker.softblock_active);

        // 1x Check-in at second 300 (5m)
        for _ in 0..299 {
            assert_eq!(
                tracker.tick_one_second(true, &cfg, "msedge.exe"),
                NeutralOverstayEvent::None
            );
        }
        assert_eq!(
            tracker.tick_one_second(true, &cfg, "msedge.exe"),
            NeutralOverstayEvent::CheckIn1x {
                app: "msedge.exe".to_string(),
                mins: 5,
            }
        );

        // FocusGrow UI clicks shouldn't disrupt or advance streak
        assert_eq!(
            tracker.tick_one_second(true, &cfg, "FocusGrow.exe"),
            NeutralOverstayEvent::None
        );

        // 2x Soft-Block at second 600 (10m)
        for _ in 0..299 {
            assert_eq!(
                tracker.tick_one_second(true, &cfg, "msedge.exe"),
                NeutralOverstayEvent::None
            );
        }
        assert_eq!(
            tracker.tick_one_second(true, &cfg, "msedge.exe"),
            NeutralOverstayEvent::SoftBlock2x {
                app: "msedge.exe".to_string(),
                mins: 10,
            }
        );
        assert!(tracker.softblock_active);
        assert!(tracker.should_show_overlay(true, &cfg, "msedge.exe"));

        // Switching briefly (<60s) to a productive app hides overlay temporarily
        assert!(!tracker.should_show_overlay(true, &cfg, "Code.exe"));
        for _ in 0..59 {
            tracker.tick_one_second(true, &cfg, "Code.exe");
        }
        assert!(tracker.softblock_active);
        assert!(tracker.should_show_overlay(true, &cfg, "msedge.exe"));

        // Completing 60 continuous seconds on a productive app resets streak & softblock
        tracker.tick_one_second(true, &cfg, "Code.exe");
        assert!(!tracker.softblock_active);
        assert!(!tracker.should_show_overlay(true, &cfg, "msedge.exe"));

        // Dismiss clears soft-block and resets counter
        for _ in 0..600 {
            tracker.tick_one_second(true, &cfg, "msedge.exe");
        }
        assert!(tracker.softblock_active);
        tracker.dismiss_softblock();
        assert!(!tracker.softblock_active);
        assert!(!tracker.should_show_overlay(true, &cfg, "msedge.exe"));
    }
}

