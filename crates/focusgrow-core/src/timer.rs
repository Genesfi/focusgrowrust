use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::AppConfig;

/// Current phase of the Pomodoro focus session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Idle,
    Focusing,
    Resting,
}

/// Snapshot of an ongoing timer session persisted across close / shutdown / sleep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub phase: String,
    pub remaining_sec: u32,
    pub total_phase_sec: u32,
    pub current_period: u32,
    pub total_periods: u32,
    pub total_focus_minutes: u32,
    pub chunk_minutes: u32,
    pub break_minutes: u32,
    pub skip_breaks: bool,
    pub is_health_mode: bool,
    pub was_paused: bool,
    pub saved_unix_secs: u64,
}

impl PersistedSession {
    pub fn session_path() -> PathBuf {
        AppConfig::data_dir().join("active_session.json")
    }

    pub fn unix_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn load() -> Option<Self> {
        let raw = fs::read_to_string(Self::session_path()).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize active_session failed")?;
        fs::write(path, json).context("write active_session.json failed")
    }

    pub fn clear() {
        let _ = fs::remove_file(Self::session_path());
    }
}

/// Output of a timer tick evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickResult {
    pub changed: bool,
    pub phase_completed: Option<SessionPhase>,
}

/// Multi-period monotonic Pomodoro timer matching FocusEngine semantics.
#[derive(Debug, Clone)]
pub struct FocusTimer {
    total_focus_minutes: u32,
    chunk_minutes: u32,
    break_minutes: u32,
    skip_breaks: bool,
    phase: SessionPhase,
    is_paused: bool,
    current_period: u32,
    total_periods: u32,
    total_phase_sec: u32,
    remaining_sec: u32,
    last_tick: Option<Instant>,
    accum: Duration,
}

impl FocusTimer {
    pub fn new(
        total_focus_minutes: u32,
        chunk_minutes: u32,
        break_minutes: u32,
        skip_breaks: bool,
    ) -> Self {
        let mut t = Self {
            total_focus_minutes: 60,
            chunk_minutes: 25,
            break_minutes: 5,
            skip_breaks: false,
            phase: SessionPhase::Idle,
            is_paused: false,
            current_period: 1,
            total_periods: 1,
            total_phase_sec: 25 * 60,
            remaining_sec: 25 * 60,
            last_tick: None,
            accum: Duration::ZERO,
        };
        t.set_schedule(total_focus_minutes, chunk_minutes, break_minutes, skip_breaks);
        t
    }

    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn is_running(&self) -> bool {
        self.phase != SessionPhase::Idle && !self.is_paused
    }

    pub fn remaining_sec(&self) -> u32 {
        self.remaining_sec
    }

    pub fn total_phase_sec(&self) -> u32 {
        self.total_phase_sec.max(1)
    }

    pub fn current_period(&self) -> u32 {
        self.current_period
    }

    pub fn total_periods(&self) -> u32 {
        self.total_periods
    }

    pub fn planned_periods(&self) -> u32 {
        if self.skip_breaks || self.total_focus_minutes <= self.chunk_minutes {
            1
        } else {
            self.total_focus_minutes.div_ceil(self.chunk_minutes).max(1)
        }
    }

    pub fn planned_break_count(&self) -> u32 {
        self.planned_periods().saturating_sub(1)
    }

    pub fn period_duration_sec(&self, period: u32) -> u32 {
        if self.skip_breaks || self.total_focus_minutes <= self.chunk_minutes {
            return self.total_focus_minutes * 60;
        }
        let total_periods = self.planned_periods();
        if period < total_periods {
            self.chunk_minutes * 60
        } else {
            let used = (total_periods.saturating_sub(1)) * self.chunk_minutes;
            self.total_focus_minutes.saturating_sub(used).max(1) * 60
        }
    }

    /// Total seconds remaining across the entire scheduled session (focus + breaks).
    pub fn total_remaining_sec(&self) -> u32 {
        if self.phase == SessionPhase::Idle {
            let focus_total = self.total_focus_minutes * 60;
            let breaks_total = self.planned_break_count() * self.break_minutes * 60;
            return focus_total + breaks_total;
        }

        let mut total = self.remaining_sec;
        match self.phase {
            SessionPhase::Focusing => {
                if self.current_period < self.total_periods && !self.skip_breaks {
                    total += self.break_minutes * 60;
                }
                for p in (self.current_period + 1)..=self.total_periods {
                    total += self.period_duration_sec(p);
                    if p < self.total_periods && !self.skip_breaks {
                        total += self.break_minutes * 60;
                    }
                }
            }
            SessionPhase::Resting => {
                for p in (self.current_period + 1)..=self.total_periods {
                    total += self.period_duration_sec(p);
                    if p < self.total_periods && !self.skip_breaks {
                        total += self.break_minutes * 60;
                    }
                }
            }
            SessionPhase::Idle => {}
        }
        total
    }

    pub fn formatted_total_duration(total_sec: u32) -> String {
        let mins = total_sec.div_ceil(60);
        let h = mins / 60;
        let m = mins % 60;
        if h > 0 {
            format!("{h}h {m}m")
        } else {
            format!("{m}m")
        }
    }

    pub fn period_header_label(&self) -> String {
        match self.phase {
            SessionPhase::Idle => "Get ready to focus".to_string(),
            SessionPhase::Focusing => {
                format!("Focus period ({} of {})", self.current_period, self.total_periods)
            }
            SessionPhase::Resting => {
                format!("Break time ({} of {})", self.current_period, self.total_periods)
            }
        }
    }

    /// Fraction elapsed in the current phase (`0.0` at start -> `1.0` when done).
    pub fn elapsed_fraction(&self) -> f32 {
        if self.phase == SessionPhase::Idle {
            return 0.0;
        }
        let total = self.total_phase_sec.max(1) as f32;
        let rem = self.remaining_sec.min(self.total_phase_sec) as f32;
        ((total - rem) / total).clamp(0.0, 1.0)
    }

    /// Fraction remaining in the current phase (`1.0` at start -> `0.0` when done).
    pub fn remaining_fraction(&self) -> f32 {
        if self.phase == SessionPhase::Idle {
            return 1.0;
        }
        let total = self.total_phase_sec.max(1) as f32;
        (self.remaining_sec as f32 / total).clamp(0.0, 1.0)
    }

    /// Sub-second interpolated remaining fraction (`1.0` -> `0.0`) for smooth 60fps liquid/hourglass fills.
    pub fn smooth_remaining_fraction(&self, now: Instant) -> f32 {
        if self.phase == SessionPhase::Idle {
            return 1.0;
        }
        let sub_sec = if self.is_running() {
            let extra = self
                .last_tick
                .map(|t| now.saturating_duration_since(t))
                .unwrap_or(Duration::ZERO);
            (self.accum + extra).as_secs_f32().clamp(0.0, 1.0)
        } else {
            self.accum.as_secs_f32().clamp(0.0, 1.0)
        };
        let total = self.total_phase_sec.max(1) as f32;
        let exact_rem = (self.remaining_sec as f32 - sub_sec).max(0.0);
        (exact_rem / total).clamp(0.0, 1.0)
    }

    pub fn formatted_time(&self) -> String {
        let mins = self.remaining_sec / 60;
        let secs = self.remaining_sec % 60;
        format!("{mins:02}:{secs:02}")
    }

    pub fn set_schedule(
        &mut self,
        total_focus_minutes: u32,
        chunk_minutes: u32,
        break_minutes: u32,
        skip_breaks: bool,
    ) {
        self.total_focus_minutes = total_focus_minutes.clamp(1, 720);
        self.chunk_minutes = chunk_minutes.clamp(1, 180);
        self.break_minutes = break_minutes.clamp(1, 60);
        self.skip_breaks = skip_breaks;
        if self.phase == SessionPhase::Idle {
            self.total_periods = self.planned_periods();
            self.current_period = 1;
            self.total_phase_sec = self.period_duration_sec(1);
            self.remaining_sec = self.total_phase_sec;
        }
    }

    pub fn start(&mut self, now: Instant) {
        self.phase = SessionPhase::Focusing;
        self.is_paused = false;
        self.total_periods = self.planned_periods();
        self.current_period = 1;
        self.total_phase_sec = self.period_duration_sec(1);
        self.remaining_sec = self.total_phase_sec;
        self.last_tick = Some(now);
        self.accum = Duration::ZERO;
    }

    pub fn toggle_pause(&mut self, now: Instant) {
        if self.phase == SessionPhase::Idle {
            self.start(now);
            return;
        }
        self.is_paused = !self.is_paused;
        if self.is_paused {
            self.last_tick = None;
        } else {
            self.last_tick = Some(now);
        }
    }

    pub fn stop(&mut self) {
        self.phase = SessionPhase::Idle;
        self.is_paused = false;
        self.total_periods = self.planned_periods();
        self.current_period = 1;
        self.total_phase_sec = self.period_duration_sec(1);
        self.remaining_sec = self.total_phase_sec;
        self.last_tick = None;
        self.accum = Duration::ZERO;
    }

    pub fn skip_phase(&mut self, now: Instant) {
        match self.phase {
            SessionPhase::Idle => self.start(now),
            SessionPhase::Focusing => self.advance_after_focus(now),
            SessionPhase::Resting => self.advance_after_rest(now),
        }
    }

    fn advance_after_focus(&mut self, now: Instant) {
        if self.current_period >= self.total_periods {
            self.stop();
        } else if self.skip_breaks {
            self.current_period += 1;
            self.enter_focus_period(now);
        } else {
            self.enter_rest(now);
        }
    }

    fn advance_after_rest(&mut self, now: Instant) {
        if self.current_period < self.total_periods {
            self.current_period += 1;
            self.enter_focus_period(now);
        } else {
            self.stop();
        }
    }

    fn enter_focus_period(&mut self, now: Instant) {
        self.phase = SessionPhase::Focusing;
        self.is_paused = false;
        self.total_phase_sec = self.period_duration_sec(self.current_period);
        self.remaining_sec = self.total_phase_sec;
        self.last_tick = Some(now);
        self.accum = Duration::ZERO;
    }

    fn enter_rest(&mut self, now: Instant) {
        self.phase = SessionPhase::Resting;
        self.is_paused = false;
        self.total_phase_sec = self.break_minutes * 60;
        self.remaining_sec = self.total_phase_sec;
        self.last_tick = Some(now);
        self.accum = Duration::ZERO;
    }

    /// Re-anchors the monotonic tick baseline without altering `remaining_sec` (e.g. after deferred startup).
    pub fn refresh_tick_anchor(&mut self, now: Instant) {
        if self.is_running() {
            self.last_tick = Some(now);
            self.accum = Duration::ZERO;
        }
    }

    /// Advances the timer using monotonic elapsed time.
    pub fn tick(&mut self, now: Instant) -> TickResult {
        if !self.is_running() {
            return TickResult {
                changed: false,
                phase_completed: None,
            };
        }

        let Some(prev) = self.last_tick else {
            self.last_tick = Some(now);
            return TickResult {
                changed: false,
                phase_completed: None,
            };
        };

        let delta = now.saturating_duration_since(prev);
        self.last_tick = Some(now);

        // Sleep / suspend gap protection: avoid draining time while PC slept,
        // and continue running automatically upon wake-up.
        if delta > Duration::from_secs(5) {
            self.accum = Duration::ZERO;
            self.last_tick = Some(now);
            return TickResult {
                changed: true,
                phase_completed: None,
            };
        }

        self.accum += delta;
        let secs_elapsed = self.accum.as_secs() as u32;
        if secs_elapsed == 0 {
            return TickResult {
                changed: false,
                phase_completed: None,
            };
        }
        self.accum -= Duration::from_secs(secs_elapsed as u64);

        if secs_elapsed < self.remaining_sec {
            self.remaining_sec -= secs_elapsed;
            return TickResult {
                changed: true,
                phase_completed: None,
            };
        }

        let completed = self.phase;
        match completed {
            SessionPhase::Focusing => self.advance_after_focus(now),
            SessionPhase::Resting => self.advance_after_rest(now),
            SessionPhase::Idle => {}
        }

        TickResult {
            changed: true,
            phase_completed: Some(completed),
        }
    }

    /// Serializes an active session for auto-resume across exit / sleep / shutdown.
    pub fn export_session(&self, is_health_mode: bool) -> Option<PersistedSession> {
        let phase_str = match self.phase {
            SessionPhase::Idle => return None,
            SessionPhase::Focusing => "focusing",
            SessionPhase::Resting => "resting",
        };
        Some(PersistedSession {
            phase: phase_str.to_string(),
            remaining_sec: self.remaining_sec,
            total_phase_sec: self.total_phase_sec.max(1),
            current_period: self.current_period.max(1),
            total_periods: self.total_periods.max(1),
            total_focus_minutes: self.total_focus_minutes,
            chunk_minutes: self.chunk_minutes,
            break_minutes: self.break_minutes,
            skip_breaks: self.skip_breaks,
            is_health_mode,
            was_paused: self.is_paused,
            saved_unix_secs: PersistedSession::unix_now(),
        })
    }

    /// Restores a previously active session from disk and resumes immediately.
    pub fn restore_session(&mut self, s: &PersistedSession, now: Instant) {
        self.total_focus_minutes = s.total_focus_minutes.clamp(1, 720);
        self.chunk_minutes = s.chunk_minutes.clamp(1, 180);
        self.break_minutes = s.break_minutes.clamp(1, 60);
        self.skip_breaks = s.skip_breaks;
        self.phase = if s.phase == "resting" {
            SessionPhase::Resting
        } else {
            SessionPhase::Focusing
        };
        self.total_periods = s.total_periods.max(1);
        self.current_period = s.current_period.clamp(1, self.total_periods);
        self.total_phase_sec = s.total_phase_sec.max(1);
        self.remaining_sec = s.remaining_sec.clamp(1, self.total_phase_sec);
        self.is_paused = false;
        self.accum = Duration::ZERO;
        self.last_tick = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_transitions_focus_to_rest_across_periods() {
        let t0 = Instant::now();
        let mut timer = FocusTimer::new(10, 5, 2, false);
        assert_eq!(timer.planned_periods(), 2);
        assert_eq!(timer.formatted_time(), "05:00");

        timer.start(t0);
        assert_eq!(timer.phase(), SessionPhase::Focusing);
        assert_eq!(timer.period_header_label(), "Focus period (1 of 2)");

        let mut t = t0;
        for _ in 0..(5 * 60 - 1) {
            t += Duration::from_secs(1);
            let res = timer.tick(t);
            assert!(res.changed);
            assert_eq!(res.phase_completed, None);
        }
        assert_eq!(timer.remaining_sec(), 1);

        t += Duration::from_secs(1);
        let res = timer.tick(t);
        assert_eq!(res.phase_completed, Some(SessionPhase::Focusing));
        assert_eq!(timer.phase(), SessionPhase::Resting);
        assert_eq!(timer.formatted_time(), "02:00");
    }

    #[test]
    fn total_remaining_matches_360_min_schedule_in_screenshot() {
        // 360 mins total, 30 min periods, 5 min breaks -> 12 periods, 11 breaks = 415 mins = 6h 55m
        let timer = FocusTimer::new(360, 30, 5, false);
        assert_eq!(timer.planned_periods(), 12);
        assert_eq!(timer.planned_break_count(), 11);
        assert_eq!(
            FocusTimer::formatted_total_duration(timer.total_remaining_sec()),
            "6h 55m"
        );
    }

    #[test]
    fn suspend_gap_auto_resumes_without_draining() {
        let t0 = Instant::now();
        let mut timer = FocusTimer::new(60, 25, 5, false);
        timer.start(t0);

        let res = timer.tick(t0 + Duration::from_secs(60));
        assert!(res.changed);
        assert!(!timer.is_paused());
        assert_eq!(timer.remaining_sec(), 25 * 60);
    }
}
