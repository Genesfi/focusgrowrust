use serde::Serialize;

/// Daily Islamic prayer schedule computed with astronomical angles
/// (Fajr 20°, Isha 18°, Ashar Syafi'i, +2 min ihtiyat safety buffer).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PrayerTimes {
    pub subuh: f64,
    pub dhuha: f64,
    pub dzuhur: f64,
    pub ashar: f64,
    pub maghrib: f64,
    pub isya: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedPrayer {
    pub name: &'static str,
    pub hour: f64,
    pub formatted: String,
}

#[derive(Debug, Clone)]
pub struct PrayerManager {
    pub lat: f64,
    pub lng: f64,
    pub timezone: i32,
    pub offset_mins: i32,
}

impl Default for PrayerManager {
    fn default() -> Self {
        Self {
            lat: -2.7903,
            lng: 115.2914,
            timezone: 8,
            offset_mins: 0,
        }
    }
}

impl PrayerManager {
    pub fn new(lat: f64, lng: f64, timezone: i32) -> Self {
        Self {
            lat,
            lng,
            timezone,
            offset_mins: 0,
        }
    }

    pub fn with_offset(lat: f64, lng: f64, timezone: i32, offset_mins: i32) -> Self {
        Self {
            lat,
            lng,
            timezone,
            offset_mins: offset_mins.clamp(-30, 30),
        }
    }

    pub fn format_hhmm(hour: f64) -> String {
        let normalized = fix_hour(hour);
        let total_mins = (normalized * 60.0).round() as u32;
        let h = (total_mins / 60) % 24;
        let m = total_mins % 60;
        format!("{:02}:{:02}", h, m)
    }

    pub fn calculate(&self, year: i32, month: i32, day: i32) -> PrayerTimes {
        let (mut y, mut m) = (year as f64, month as f64);
        if m <= 2.0 {
            y -= 1.0;
            m += 12.0;
        }
        let a = (y / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();
        // Evaluate solar orbital elements near local midday rather than 00:00 UT.
        let jd = (365.25 * (y + 4716.0)).floor()
            + (30.6001 * (m + 1.0)).floor()
            + day as f64
            + b
            - 1524.5
            + (0.5 - self.timezone as f64 / 24.0);

        let d = jd - 2451545.0;
        let g = fix_angle(357.529 + 0.98560028 * d);
        let q = fix_angle(280.459 + 0.98564736 * d);
        let l = fix_angle(q + 1.915 * g.to_radians().sin() + 0.020 * (2.0 * g).to_radians().sin());

        let e = 23.439 - 0.00000036 * d;
        let ra = fix_hour(
            e.to_radians()
                .cos()
                .mul_add(l.to_radians().sin(), 0.0)
                .atan2(l.to_radians().cos())
                .to_degrees()
                / 15.0,
        );
        let declination = (e.to_radians().sin() * l.to_radians().sin())
            .asin()
            .to_degrees();
        let eq_time = q / 15.0 - ra;
        let noon = 12.0 + self.timezone as f64 - self.lng / 15.0 - eq_time;

        let calc_base = |angle: f64, before_noon: bool| -> f64 {
            let lat_rad = self.lat.to_radians();
            let dec_rad = declination.to_radians();
            let h_rad = angle.to_radians();
            let cos_h = (-h_rad.sin() - lat_rad.sin() * dec_rad.sin())
                / (lat_rad.cos() * dec_rad.cos());
            if !(-1.0..=1.0).contains(&cos_h) {
                return noon;
            }
            let delta = cos_h.acos().to_degrees() / 15.0;
            if before_noon {
                noon - delta
            } else {
                noon + delta
            }
        };

        // Round fractional seconds upward with ihtiyat safety buffer + apply user offset.
        let ceil_with_offset = |raw_hr: f64| -> f64 {
            let shifted_mins = raw_hr * 60.0 + self.offset_mins as f64;
            let whole_mins = (shifted_mins - 1e-6).ceil();
            whole_mins / 60.0
        };

        let dzuhur = ceil_with_offset(noon + (2.0 / 60.0));
        let subuh = ceil_with_offset(calc_base(20.0, true) + (2.0 / 60.0));
        let isya = ceil_with_offset(calc_base(18.0, false) + (2.0 / 60.0));
        let maghrib = ceil_with_offset(calc_base(0.833, false) + (2.0 / 60.0));

        let lat_rad = self.lat.to_radians();
        let dec_rad = declination.to_radians();
        let acot = (1.0 / (1.0 + (lat_rad - dec_rad).abs().tan())).atan();
        let h_ashar = acot.to_degrees();
        let cos_h_ashar = (h_ashar.to_radians().sin() - lat_rad.sin() * dec_rad.sin())
            / (lat_rad.cos() * dec_rad.cos());
        let ashar = if (-1.0..=1.0).contains(&cos_h_ashar) {
            ceil_with_offset(noon + cos_h_ashar.acos().to_degrees() / 15.0 + (2.0 / 60.0))
        } else {
            ceil_with_offset(noon + 3.25)
        };

        let sunrise = calc_base(0.833, true);
        let dhuha = ceil_with_offset(sunrise + (20.0 / 60.0));

        PrayerTimes {
            subuh,
            dhuha,
            dzuhur,
            ashar,
            maghrib,
            isya,
        }
    }

    pub fn list(&self, year: i32, month: i32, day: i32) -> Vec<NamedPrayer> {
        let t = self.calculate(year, month, day);
        vec![
            NamedPrayer { name: "Subuh", hour: t.subuh, formatted: Self::format_hhmm(t.subuh) },
            NamedPrayer { name: "Dhuha", hour: t.dhuha, formatted: Self::format_hhmm(t.dhuha) },
            NamedPrayer { name: "Dzuhur", hour: t.dzuhur, formatted: Self::format_hhmm(t.dzuhur) },
            NamedPrayer { name: "Ashar", hour: t.ashar, formatted: Self::format_hhmm(t.ashar) },
            NamedPrayer { name: "Maghrib", hour: t.maghrib, formatted: Self::format_hhmm(t.maghrib) },
            NamedPrayer { name: "Isya", hour: t.isya, formatted: Self::format_hhmm(t.isya) },
        ]
    }

    /// Returns `(next_name, next_hour, sec_until_next)`.
    pub fn next_prayer(&self, year: i32, month: i32, day: i32, cur_hour: f64) -> (&'static str, f64, u32) {
        let t = self.calculate(year, month, day);
        let wajib = [
            ("Subuh", t.subuh),
            ("Dzuhur", t.dzuhur),
            ("Ashar", t.ashar),
            ("Maghrib", t.maghrib),
            ("Isya", t.isya),
        ];
        for (name, hr) in wajib {
            if hr > cur_hour {
                let sec = ((hr - cur_hour) * 3600.0).round().max(0.0) as u32;
                return (name, hr, sec);
            }
        }
        // After Isya -> tomorrow Subuh
        let hr = t.subuh + 24.0;
        let sec = ((hr - cur_hour) * 3600.0).round().max(0.0) as u32;
        ("Subuh", t.subuh, sec)
    }

    /// Returns `Some((prayer_name, remaining_secs))` if local wall-clock time falls inside `[prayer_time, prayer_time + duration_sec)`.
    /// Prevents bypassing the 15-minute prayer break by closing & reopening the app during or just before prayer time.
    pub fn active_prayer_window(
        &self,
        year: i32,
        month: i32,
        day: i32,
        cur_hour: f64,
        duration_sec: u32,
    ) -> Option<(&'static str, u32)> {
        let t = self.calculate(year, month, day);
        let cur_sec = cur_hour * 3600.0;
        let wajib = [
            ("Subuh", t.subuh),
            ("Dzuhur", t.dzuhur),
            ("Ashar", t.ashar),
            ("Maghrib", t.maghrib),
            ("Isya", t.isya),
        ];
        for (name, hr) in wajib {
            let start_sec = hr * 3600.0;
            if cur_sec >= start_sec {
                let elapsed = cur_sec - start_sec;
                if elapsed < duration_sec as f64 {
                    let rem = duration_sec.saturating_sub(elapsed.floor() as u32).max(1);
                    return Some((name, rem));
                }
            }
        }
        None
    }
}

fn fix_angle(a: f64) -> f64 {
    let r = a % 360.0;
    if r < 0.0 { r + 360.0 } else { r }
}

fn fix_hour(h: f64) -> f64 {
    let r = h % 24.0;
    if r < 0.0 { r + 24.0 } else { r }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prayer_times_follow_chronological_order_in_wita() {
        let mgr = PrayerManager::default();
        let t = mgr.calculate(2026, 11, 12);
        assert!(t.subuh > 3.5 && t.subuh < 6.0);
        assert!(t.subuh < t.dhuha);
        assert!(t.dhuha < t.dzuhur);
        assert!(t.dzuhur < t.ashar);
        assert!(t.ashar < t.maghrib);
        assert!(t.maghrib < t.isya);
        assert_eq!(PrayerManager::format_hhmm(12.5), "12:30");
    }

    #[test]
    fn next_prayer_picks_dzuhur_in_morning() {
        let mgr = PrayerManager::default();
        let (name, _, sec) = mgr.next_prayer(2026, 11, 12, 9.0);
        assert_eq!(name, "Dzuhur");
        assert!(sec > 2 * 3600);
    }

    #[test]
    fn active_prayer_window_detects_reopen_mid_break() {
        let mgr = PrayerManager::default();
        let t = mgr.calculate(2026, 11, 12);
        // Reopening app 5 minutes (300s) after Maghrib started must still enforce remaining 10m (600s)
        let active = mgr.active_prayer_window(2026, 11, 12, t.maghrib + (300.0 / 3600.0), 900);
        assert!(matches!(active, Some(("Maghrib", 599..=601))));
        // After 16 minutes, window has elapsed
        assert_eq!(
            mgr.active_prayer_window(2026, 11, 12, t.maghrib + (960.0 / 3600.0), 900),
            None
        );
    }
}

