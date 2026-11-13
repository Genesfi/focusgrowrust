use anyhow::{Context, Result};
use focusgrow_audio::{sound_style_label, AMBIENT_PRESETS, AMBIENT_TRACKS};
use focusgrow_core::stats::{
    date_month_year, format_full_date_label, month_short, next_iso_date, prev_iso_date,
    weekday_index, weekday_short,
};
use focusgrow_core::{
    AmbientMode, AppConfig, DailyStats, FocusTimer, GifDisplayMode, NamedPrayer, PipStyle,
    SessionPhase, VinylSide,
};
use focusgrow_gfx::dcomp::{
    create_text_format, create_text_format_single_line, hex_color, rgba,
};
use focusgrow_gfx::{draw_vinyl_disc, GpuAnimation};
use std::collections::HashMap;
use std::path::Path;
use windows::core::HSTRING;
use windows::Win32::Foundation::GENERIC_WRITE;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN, D2D1_PIXEL_FORMAT,
    D2D_RECT_F, D2D_SIZE_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Factory1, ID2D1Geometry, ID2D1PathGeometry1,
    ID2D1StrokeStyle1, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_LARGE,
    D2D1_ARC_SIZE_SMALL, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_CAP_STYLE_ROUND,
    D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE, D2D1_INTERPOLATION_MODE_LINEAR,
    D2D1_LAYER_OPTIONS1_NONE, D2D1_LAYER_PARAMETERS1, D2D1_MAP_OPTIONS_READ, D2D1_ROUNDED_RECT,
    D2D1_STROKE_STYLE_PROPERTIES1, D2D1_SWEEP_DIRECTION_CLOCKWISE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows_numerics::{Matrix3x2, Vector2};

use crate::qr::QrMatrix;
use crate::ytmpx::{PlaybackState, TrackInfo};

pub const ACCENT_PRESETS: [(&str, &str); 6] = [
    ("Blue", "#60CDFF"),
    ("Pink", "#FF2A70"),
    ("Green", "#00E676"),
    ("Purple", "#A855F7"),
    ("Orange", "#F59E0B"),
    ("Custom", "#D74246"),
];

pub const TIMER_THEMES: [(&str, &str); 6] = [
    ("classic", "Classic"),
    ("hourglass", "Hourglass"),
    ("wave", "Liquid Wave"),
    ("blocks", "Blocks"),
    ("dots", "Dots"),
    ("orbit", "Orbit"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct CityEntry {
    pub name: String,
    pub lat: f64,
    pub lng: f64,
    pub tz: i32,
}

pub const CITY_PRESETS: &[(&str, f64, f64, i32)] = &[
    ("Makassar, South Sulawesi", -5.1477, 119.4327, 8),
    ("Jakarta, DKI Jakarta", -6.2088, 106.8456, 7),
    ("Bandung, West Java", -6.9175, 107.6191, 7),
    ("Surabaya, East Java", -7.2575, 112.7521, 7),
    ("Yogyakarta, DIY", -7.7956, 110.3695, 7),
    ("Semarang, Central Java", -6.9667, 110.4167, 7),
    ("Medan, North Sumatra", 3.5952, 98.6722, 7),
    ("Banda Aceh, Aceh", 5.5483, 95.3238, 7),
    ("Palembang, South Sumatra", -2.9761, 104.7754, 7),
    ("Pekanbaru, Riau", 0.5071, 101.4478, 7),
    ("Padang, West Sumatra", -0.9471, 100.4172, 7),
    ("Bandar Lampung, Lampung", -5.4292, 105.2610, 7),
    ("Banjarmasin, South Kalimantan", -3.3194, 114.5908, 8),
    ("Banjarbaru, South Kalimantan", -3.4400, 114.8325, 8),
    ("Tapin / Rantau, South Kalimantan", -2.9333, 115.1333, 8),
    ("Padang Batung / HSS, South Kalimantan", -2.7903, 115.2914, 8),
    ("Kandangan / HSS, South Kalimantan", -2.7850, 115.2650, 8),
    ("Balikpapan, East Kalimantan", -1.2379, 116.8529, 8),
    ("Samarinda, East Kalimantan", -0.5022, 117.1536, 8),
    ("Pontianak, West Kalimantan", -0.0263, 109.3425, 7),
    ("Denpasar, Bali", -8.6705, 115.2126, 8),
    ("Mataram, Lombok NTB", -8.5833, 116.1167, 8),
    ("Kupang, NTT", -10.1772, 123.6070, 8),
    ("Manado, North Sulawesi", 1.4748, 124.8421, 8),
    ("Palu, Central Sulawesi", -0.8917, 119.8707, 8),
    ("Kendari, Southeast Sulawesi", -3.9985, 122.5129, 8),
    ("Ambon, Maluku", -3.6954, 128.1814, 9),
    ("Jayapura, Papua", -2.5916, 140.6690, 9),
    ("Singapore", 1.3521, 103.8198, 8),
    ("Kuala Lumpur, Malaysia", 3.1390, 101.6869, 8),
    ("Makkah, Saudi Arabia", 21.4225, 39.8262, 3),
    ("Madinah, Saudi Arabia", 24.4672, 39.6112, 3),
    ("Tokyo, Japan", 35.6762, 139.6503, 9),
    ("London, United Kingdom", 51.5072, -0.1276, 0),
];

fn pt(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

pub const COMMON_SITE_PRESETS: [&str; 6] = [
    "youtube.com",
    "x.com",
    "instagram.com",
    "tiktok.com",
    "reddit.com",
    "facebook.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Session,
    PipMode,
    Sound,
    MobileSync,
    System,
}

impl SettingsTab {
    pub fn key(self) -> &'static str {
        match self {
            SettingsTab::Appearance => "appearance",
            SettingsTab::Session => "session",
            SettingsTab::PipMode => "pip",
            SettingsTab::Sound => "sound",
            SettingsTab::MobileSync => "mobile",
            SettingsTab::System => "system",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "session" => SettingsTab::Session,
            "pip" => SettingsTab::PipMode,
            "sound" => SettingsTab::Sound,
            "mobile" => SettingsTab::MobileSync,
            "system" => SettingsTab::System,
            _ => SettingsTab::Appearance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardTab {
    BlockedApps,
    RestrictedSites,
    PrayerTimes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFilter {
    All,
    Blocked,
    Accessible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSort {
    LatestOpen,
    NameAsc,
    BlockedFirst,
}

impl AppSort {
    pub fn label(self) -> &'static str {
        match self {
            AppSort::LatestOpen => "Latest Open",
            AppSort::NameAsc => "Name A–Z",
            AppSort::BlockedFirst => "Blocked First",
        }
    }

    pub fn next(self) -> Self {
        match self {
            AppSort::LatestOpen => AppSort::NameAsc,
            AppSort::NameAsc => AppSort::BlockedFirst,
            AppSort::BlockedFirst => AppSort::LatestOpen,
        }
    }
}

pub struct StatsAppRow<'a> {
    pub exe_name: &'a str,
    pub secs: u32,
    pub category: &'a str,
    pub is_blocked: bool,
    pub icon: Option<&'a ID2D1Bitmap1>,
}

pub struct RunningAppView<'a> {
    pub exe_name: &'a str,
    pub title: &'a str,
    pub blocked: bool,
    pub icon: Option<&'a ID2D1Bitmap1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericEditField {
    TotalMins,
    PeriodMins,
    BreakMins,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsPeriod {
    Today,
    Yesterday,
    Week,
    Month,
    Quarter,
}

impl StatsPeriod {
    pub fn days(self) -> usize {
        match self {
            StatsPeriod::Today | StatsPeriod::Yesterday => 1,
            StatsPeriod::Week => 7,
            StatsPeriod::Month => 30,
            StatsPeriod::Quarter => 90,
        }
    }

    pub fn end_date(self, today_date: &str) -> String {
        match self {
            StatsPeriod::Yesterday => prev_iso_date(today_date),
            _ => today_date.to_string(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            StatsPeriod::Today => "Today",
            StatsPeriod::Yesterday => "Yesterday",
            StatsPeriod::Week => "This Week",
            StatsPeriod::Month => "This Month",
            StatsPeriod::Quarter => "3 Months",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlId {
    DragBar,
    BtnMinimize,
    BtnMaximizeRestore,
    BtnClose,
    BtnTogglePip,
    BtnTogglePipStyle,
    BtnPipStyleCard,
    BtnPipStyleCapsule,
    BtnToggleCapsuleAlbumBg,
    BtnToggleCapsuleAlbumSpin,
    BtnCapsuleAlbumOpacity,
    BtnTopToolbarSound,
    BtnTopToolbarMusic,
    #[allow(dead_code)]
    BtnTopToolbarGif,
    BtnTopToolbarAppearance,
    BtnTopToolbarPip,
    BtnOpenStats,
    BtnOpenShareCard,
    BtnShareDayToday,
    BtnShareDayYesterday,
    BtnShareCopyText,
    BtnShareCopyImage,
    BtnShareSavePng,
    BtnEditGoal,
    BtnGoalHours(u32),
    BtnHealthGoalBreaks(u32),
    BtnStatsPeriod(StatsPeriod),
    BtnHeatmapPrev,
    BtnHeatmapNext,
    BtnBlockAppFromStats(usize),
    BtnCycleAppCategory(usize),
    BtnModeFocus,
    BtnModeHealth,
    InputTotalMins,
    InputPeriodMins,
    InputBreakMins,
    BtnTotalUp,
    BtnTotalDown,
    BtnChunkDown,
    BtnChunkUp,
    BtnBreakDown,
    BtnBreakUp,
    BtnToggleSkipBreaks,
    BtnPlayPause,
    BtnReset,
    #[allow(dead_code)]
    BtnSkip,
    BtnGuardTabApps,
    BtnGuardTabSites,
    BtnGuardTabPrayer,
    BtnGuardRefreshApps,
    BtnAppSearchBox,
    BtnAppSearchClear,
    BtnAppSort,
    BtnFilterAll,
    BtnFilterBlocked,
    BtnFilterAccessible,
    GuardAppToggle(usize),
    SitePresetToggle(usize),
    SiteCustomDelete(usize),
    BtnSiteSearchBox,
    BtnSiteAdd,
    BtnSiteClear,
    BtnSettingsTab(SettingsTab),
    BtnCloseModal,
    ModalBackdrop,
    ModalSurface,
    BtnRingTheme(usize),
    BtnToggleHourglassRotate,
    BtnToggleStealthPeek,
    BtnToggleStealthTimer,
    BtnAmbientDefault,
    BtnAmbientYtMusic,
    BtnAmbientCustom,
    BtnUploadGif,
    BtnGifCircle,
    BtnGifFull,
    OpacitySlider,
    VinylSpeedSlider,
    VinylSizeSlider,
    BtnVinylSpeedPreset(usize),
    BtnVinylSizePreset(usize),
    RecentGifSelect(usize),
    RecentGifDelete(usize),
    BtnClearRecentGifs,
    AccentPreset(usize),
    BtnToggleDynamicAlbum,
    BtnToggleComplementary,
    BtnToggleMediaSync,
    BtnToggleSpotifySync,
    BtnTogglePeekingVinyl,
    BtnToggleVinylSpindle,
    BtnToggleHideTaskbarInPip,
    BtnVinylSideLeft,
    BtnVinylSideRight,
    #[allow(dead_code)]
    BtnVinylSpeedDown,
    #[allow(dead_code)]
    BtnVinylSpeedUp,
    #[allow(dead_code)]
    BtnVinylSizeDown,
    #[allow(dead_code)]
    BtnVinylSizeUp,
    BtnMediaPrev,
    BtnMediaPlayPause,
    BtnMediaNext,
    BtnMediaToggleWindow,
    BtnTogglePrayerEnabled,
    BtnToggleAutoPause,
    BtnAutoPauseThreshold,
    BtnToggleNeutralOverstay,
    BtnNeutralThreshold,
    BtnToggleNeutralSoftblock,
    BtnToggleAutoClose,
    BtnToggleBlockAllMonitors,
    BtnToggleAutoPip,
    BtnToggleAutoPipBlur,
    BtnToggleNotifications,
    BtnToggleCountdownSound,
    BtnCycleCountdownSecs,
    BtnCycleCountdownStyle,
    BtnCycleTimerSubtitle,
    BtnToggleCompletedFormat,
    BtnPrayerPrev,
    BtnPrayerNext,
    BtnTogglePrayerBreak,
    BtnPrayerAdvanceDown,
    BtnPrayerAdvanceUp,
    BtnPrayerOffsetDown,
    BtnPrayerOffsetUp,
    BtnPrayerLocationCycle,
    BtnPrayerAutoGps,
    BtnPrayerSearchBox,
    BtnPrayerSearchClear,
    BtnPrayerSearchOnline,
    BtnPrayerCityPick(usize),
    BtnCopyMobileUrl,
    BtnOpenMobileUrl,
    BtnFocusSoundCycle,
    BtnFocusSoundTest,
    BtnFocusSoundUpload,
    BtnBreakSoundCycle,
    BtnBreakSoundTest,
    BtnBreakSoundUpload,
    BtnWarningSoundCycle,
    BtnWarningSoundTest,
    BtnWarningSoundUpload,
    BtnAmbientMasterToggle,
    BtnAmbientMuteAll,
    BtnAmbientAutoPlay,
    AmbientMasterSlider,
    BtnAmbientPreset(usize),
    BtnCustomPreset(usize),
    BtnAmbientTrackToggle(usize),
    AmbientTrackSlider(usize),
    BtnToggleMinimizeTray,
    BtnToggleLanguage,
    BtnSysResetToday,
    BtnSysClearApps,
    BtnSysClearGifs,
    BtnCheckUpdate,
    BtnGuardClearPasses,
}

#[derive(Debug, Clone, Copy)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn to_d2d(self) -> D2D_RECT_F {
        D2D_RECT_F {
            left: self.x,
            top: self.y,
            right: self.x + self.w,
            bottom: self.y + self.h,
        }
    }
}

pub struct HitRect {
    pub id: ControlId,
    pub rect: RectF,
}

pub struct UiLayout {
    pub card_rect: RectF,
    pub card_radius: f32,
    #[allow(dead_code)]
    pub drag_rect: RectF,
    pub gauge_cx: f32,
    pub gauge_cy: f32,
    pub gauge_radius: f32,
    pub left_card_rect: Option<RectF>,
    pub guard_list_rect: Option<RectF>,
    pub modal_rect: Option<RectF>,
    pub modal_body_rect: Option<RectF>,
    pub settings_max_scroll: f32,
    pub opacity_slider_rect: Option<RectF>,
    pub hit_rects: Vec<HitRect>,
}

impl UiLayout {
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ControlId> {
        for item in self.hit_rects.iter().rev() {
            if item.rect.contains(x, y) {
                return Some(item.id);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPeriod {
    Morning,   // 05:00 - 11:59
    Afternoon, // 12:00 - 17:59
    Night,     // 18:00 - 04:59
}

pub struct RenderView<'a> {
    pub config: &'a AppConfig,
    pub timer: &'a FocusTimer,
    pub app_icon_logo: Option<&'a ID2D1Bitmap1>,
    pub gif_anim: Option<&'a GpuAnimation>,
    pub gif_frame_index: usize,
    pub plant_sway_phase: f32,
    pub wave_phase: f32,
    pub marquee_offset: f32,
    pub ambient_playing: bool,
    pub ambient_muted: bool,
    pub cover_bitmap: Option<&'a ID2D1Bitmap1>,
    pub in_card_vinyl_angle: f32,
    pub track: Option<&'a TrackInfo>,
    pub playback: PlaybackState,
    pub ytmpx_connected: bool,
    pub effective_accent: &'a str,
    pub ends_at_text: &'a str,
    pub ends_at_period: DayPeriod,
    pub guard_tab: GuardTab,
    pub app_filter: AppFilter,
    pub app_sort: AppSort,
    pub search_query: &'a str,
    pub search_focused: bool,
    pub running_apps: &'a [RunningAppView<'a>],
    pub app_scroll: usize,
    pub total_matching_apps: usize,
    pub settings_tab: Option<SettingsTab>,
    pub settings_scroll: f32,
    pub show_ambient_mixer: bool,
    pub show_pip_gif_drawer: bool,
    pub hovered: Option<ControlId>,
    pub active_num_edit: Option<(NumericEditField, &'a str)>,
    #[allow(dead_code)]
    pub status_hint: Option<&'a str>,
    pub daily_completed_mins: u32,
    pub yesterday_mins: u32,
    pub streak_days: u32,
    pub stats: &'a DailyStats,
    pub today_date: &'a str,
    pub cur_hour: usize,
    pub show_stats_modal: bool,
    pub show_share_modal: bool,
    pub share_yesterday: bool,
    pub show_goal_modal: bool,
    pub show_prayer_modal: bool,
    pub prayer_search_query: &'a str,
    pub prayer_search_focused: bool,
    pub prayer_city_results: &'a [CityEntry],
    pub prayer_loc_busy: bool,
    pub prayer_loc_status: Option<&'a str>,
    pub stats_period: StatsPeriod,
    pub heatmap_offset_months: usize,
    pub stats_apps: &'a [StatsAppRow<'a>],
    pub app_icons_map: &'a HashMap<String, ID2D1Bitmap1>,
    pub cursor_dip: Option<(f32, f32)>,
    pub next_prayer_label: &'a str,
    pub local_clock_text: &'a str,
    pub local_date_text: &'a str,
    pub prayer_view_idx: Option<usize>,
    pub prayer_times: &'a [NamedPrayer],
    pub bridge_port: u16,
    pub lan_ip: &'a str,
    pub qr_matrix: Option<&'a QrMatrix>,
    pub active_exe: &'a str,
    pub active_domain: &'a str,
    pub active_pass_count: usize,
    pub site_input_focused: bool,
    pub site_input_query: &'a str,
    pub site_scroll: usize,
    pub block_countdown_sec: Option<u8>,
    pub is_auto_paused: bool,
    pub completed_spin_deg: f32,
    pub guard_refresh_spin_deg: f32,
    pub update_status: &'a crate::updater::UpdateStatus,
}

pub struct LayoutQuery<'a> {
    pub w: f32,
    pub h: f32,
    pub config: &'a AppConfig,
    pub guard_tab: GuardTab,
    pub visible_app_rows: usize,
    pub site_rows: usize,
    pub site_scroll: usize,
    pub settings_tab: Option<SettingsTab>,
    pub settings_scroll: f32,
    pub show_ambient_mixer: bool,
    pub show_stats_modal: bool,
    pub show_share_modal: bool,
    pub show_goal_modal: bool,
    pub show_prayer_modal: bool,
    pub prayer_city_rows: usize,
    pub stats_app_rows: usize,
    pub show_pip_drawer: bool,
    pub has_track: bool,
    pub phase: SessionPhase,
}

pub fn compute_layout(q: &LayoutQuery<'_>) -> UiLayout {
    if q.config.pip_mode {
        if q.config.pip_style == PipStyle::Capsule {
            compute_pip_capsule_layout(q.w, q.h, q.config, q.has_track, q.phase)
        } else {
            compute_pip_layout(q.w, q.h, q.config, q.show_pip_drawer, q.has_track, q.phase)
        }
    } else {
        compute_dashboard_layout(q)
    }
}

fn compute_pip_capsule_layout(
    w: f32,
    _h: f32,
    _config: &AppConfig,
    has_track: bool,
    _phase: SessionPhase,
) -> UiLayout {
    let mut hits = Vec::new();
    let pad = 2.0f32;
    let capsule_h = 38.0;
    let capsule_w = (w - pad * 2.0).max(120.0);
    let capsule_x = pad;
    let capsule_y = pad;
    let capsule_rect = RectF {
        x: capsule_x,
        y: capsule_y,
        w: capsule_w,
        h: capsule_h,
    };

    let is_compact = capsule_w < 235.0;
    let btn_s = if is_compact { 16.0 } else { 18.0 };
    let btn_gap = if is_compact { 2.0 } else { 3.0 };
    let btn_margin = if is_compact { 5.0 } else { 8.0 };
    let btn_y = capsule_rect.y + (capsule_h - btn_s) * 0.5;

    let close_x = capsule_rect.x + capsule_rect.w - btn_margin - btn_s;
    let pop_x = close_x - btn_gap - btn_s;
    let style_x = pop_x - btn_gap - btn_s;
    let pp_x = style_x - btn_gap - btn_s;

    // Draggable area: the entire capsule body from left up to the action buttons
    let drag_w = (pp_x - 3.0 - capsule_rect.x).max(20.0);
    hits.push(HitRect {
        id: ControlId::DragBar,
        rect: RectF {
            x: capsule_rect.x,
            y: capsule_rect.y,
            w: drag_w,
            h: capsule_h,
        },
    });

    // Stealth Mode toggle on timer digits in PiP capsule
    let gif_s = if is_compact { 26.0 } else { 30.0 };
    let timer_x = capsule_rect.x + 3.5 + gif_s + 4.0;
    let timer_w = if is_compact { 40.0 } else { 48.0 };
    hits.push(HitRect {
        id: ControlId::BtnToggleStealthTimer,
        rect: RectF {
            x: timer_x,
            y: capsule_rect.y + 4.0,
            w: timer_w,
            h: capsule_h - 8.0,
        },
    });

    // 4 Action buttons on the right of the capsule
    hits.push(HitRect {
        id: ControlId::BtnClose,
        rect: RectF { x: close_x, y: btn_y, w: btn_s, h: btn_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTogglePip,
        rect: RectF { x: pop_x, y: btn_y, w: btn_s, h: btn_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTogglePipStyle,
        rect: RectF { x: style_x, y: btn_y, w: btn_s, h: btn_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnPlayPause,
        rect: RectF { x: pp_x, y: btn_y, w: btn_s, h: btn_s },
    });

    // Floating Glass Under-Bar for Music Controls (only visible when an active track is detected)
    if has_track {
        let under_h = 24.0;
        let under_y = capsule_rect.y + capsule_h + 3.0;
        let under_w = if is_compact {
            124.0f32.min(capsule_w - 6.0).max(60.0)
        } else {
            136.0f32.min(capsule_w - 12.0).max(60.0)
        };
        let under_x = capsule_rect.x + (capsule_w - under_w) * 0.5;
        let btn_under_y = under_y + (under_h - 20.0) * 0.5;
        let slot_w = under_w / 4.0;

        let b_prev_w = if is_compact { 24.0 } else { 26.0 };
        let b_pp_w = if is_compact { 28.0 } else { 32.0 };
        let b_next_w = if is_compact { 24.0 } else { 26.0 };
        let b_win_w = if is_compact { 24.0 } else { 28.0 };

        hits.push(HitRect {
            id: ControlId::BtnMediaPrev,
            rect: RectF { x: under_x + (slot_w - b_prev_w) * 0.5, y: btn_under_y, w: b_prev_w, h: 20.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnMediaPlayPause,
            rect: RectF { x: under_x + slot_w + (slot_w - b_pp_w) * 0.5, y: btn_under_y, w: b_pp_w, h: 20.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnMediaNext,
            rect: RectF { x: under_x + slot_w * 2.0 + (slot_w - b_next_w) * 0.5, y: btn_under_y, w: b_next_w, h: 20.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnMediaToggleWindow,
            rect: RectF { x: under_x + slot_w * 3.0 + (slot_w - b_win_w) * 0.5, y: btn_under_y, w: b_win_w, h: 20.0 },
        });
    }

    UiLayout {
        card_rect: capsule_rect,
        card_radius: capsule_h * 0.5,
        drag_rect: capsule_rect,
        gauge_cx: capsule_rect.x + 19.0,
        gauge_cy: capsule_rect.y + 19.0,
        gauge_radius: 15.0,
        left_card_rect: Some(capsule_rect),
        guard_list_rect: None,
        modal_rect: None,
        modal_body_rect: None,
        settings_max_scroll: 0.0,
        opacity_slider_rect: None,
        hit_rects: hits,
    }
}

fn compute_pip_layout(
    w: f32,
    h: f32,
    _config: &AppConfig,
    _show_pip_drawer: bool,
    has_track: bool,
    phase: SessionPhase,
) -> UiLayout {
    let mut hits = Vec::new();
    let pad = 2.0f32;
    let card_rect = RectF {
        x: pad,
        y: pad,
        w: (w - pad * 2.0).max(146.0),
        h: (h - pad * 2.0).max(206.0),
    };

    // Match WebView2 compact PiP header: title + 3 tiny icons (Style, Pop-out & Close)
    let header_h = 26.0;
    let btn_s = 18.0;
    let btn_y = card_rect.y + 5.0;
    let close_x = card_rect.x + card_rect.w - 8.0 - btn_s;
    let pip_x = close_x - 4.0 - btn_s;
    let style_x = pip_x - 4.0 - btn_s;

    let drag_rect = RectF {
        x: card_rect.x,
        y: card_rect.y,
        w: (style_x - card_rect.x - 4.0).max(40.0),
        h: header_h,
    };
    hits.push(HitRect {
        id: ControlId::DragBar,
        rect: drag_rect,
    });
    hits.push(HitRect {
        id: ControlId::BtnTogglePipStyle,
        rect: RectF { x: style_x, y: btn_y, w: btn_s, h: btn_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTogglePip,
        rect: RectF { x: pip_x, y: btn_y, w: btn_s, h: btn_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnClose,
        rect: RectF { x: close_x, y: btn_y, w: btn_s, h: btn_s },
    });

    let ticker_h = if has_track { 28.0 } else { 0.0 };
    let bottom_pad = 8.0;
    let ticker_y = card_rect.y + card_rect.h - bottom_pad - ticker_h;

    let controls_h = (card_rect.w * 0.165).clamp(28.0, 38.0);
    let remaining_text_h = 15.0;
    let controls_y = if has_track {
        ticker_y - remaining_text_h - 6.0 - controls_h
    } else {
        card_rect.y + card_rect.h - 14.0 - controls_h - remaining_text_h
    };

    let gauge_top = card_rect.y + header_h + 2.0;
    let gauge_bottom = (controls_y - 6.0).max(gauge_top + 76.0);
    let avail_h = gauge_bottom - gauge_top;
    let avail_w = card_rect.w - 22.0;
    let gauge_radius = (avail_h.min(avail_w) * 0.44).clamp(36.0, 82.0);
    let gauge_cx = card_rect.x + card_rect.w * 0.5;
    let gauge_cy = gauge_top + avail_h * 0.5;
    let stealth_r = (gauge_radius * 0.78).max(26.0);
    hits.push(HitRect {
        id: ControlId::BtnToggleStealthTimer,
        rect: RectF {
            x: gauge_cx - stealth_r,
            y: gauge_cy - stealth_r,
            w: stealth_r * 2.0,
            h: stealth_r * 2.0,
        },
    });

    let show_secondary = phase != SessionPhase::Idle;
    if show_secondary {
        let gap = (card_rect.w * 0.045).clamp(7.0, 12.0);
        let total_bw = controls_h * 2.0 + gap;
        let start_bx = gauge_cx - total_bw * 0.5;
        hits.push(HitRect {
            id: ControlId::BtnPlayPause,
            rect: RectF { x: start_bx, y: controls_y, w: controls_h, h: controls_h },
        });
        hits.push(HitRect {
            id: ControlId::BtnReset,
            rect: RectF { x: start_bx + controls_h + gap, y: controls_y, w: controls_h, h: controls_h },
        });
        hits.push(HitRect {
            id: ControlId::BtnCycleTimerSubtitle,
            rect: RectF { x: card_rect.x + 8.0, y: controls_y + controls_h + 2.0, w: card_rect.w - 16.0, h: 16.0 },
        });
    } else {
        hits.push(HitRect {
            id: ControlId::BtnPlayPause,
            rect: RectF {
                x: gauge_cx - controls_h * 0.5,
                y: controls_y,
                w: controls_h,
                h: controls_h,
            },
        });
    }

    if has_track {
        let pill_r = RectF {
            x: card_rect.x + 8.0,
            y: ticker_y,
            w: (card_rect.w - 16.0).max(110.0),
            h: ticker_h,
        };
        push_ticker_hits(pill_r, &mut hits);
    }

    UiLayout {
        card_rect,
        card_radius: 12.0,
        drag_rect,
        gauge_cx,
        gauge_cy,
        gauge_radius,
        left_card_rect: Some(card_rect),
        guard_list_rect: None,
        modal_rect: None,
        modal_body_rect: None,
        settings_max_scroll: 0.0,
        opacity_slider_rect: None,
        hit_rects: hits,
    }
}

fn compute_dashboard_layout(q: &LayoutQuery<'_>) -> UiLayout {
    let mut hits = Vec::new();
    let pad = 2.0f32;
    let card_rect = RectF {
        x: pad,
        y: pad,
        w: (q.w - pad * 2.0).max(600.0),
        h: (q.h - pad * 2.0).max(460.0),
    };

    // Custom titlebar
    let title_h = 36.0;
    let cap_w = 32.0;
    let cap_h = 26.0;
    let cap_y = card_rect.y + 5.0;
    let close_x = card_rect.x + card_rect.w - 10.0 - cap_w;
    let max_x = close_x - 4.0 - cap_w;
    let min_x = max_x - 4.0 - cap_w;
    let drag_rect = RectF {
        x: card_rect.x,
        y: card_rect.y,
        w: (min_x - card_rect.x - 6.0).max(120.0),
        h: title_h,
    };
    hits.push(HitRect { id: ControlId::DragBar, rect: drag_rect });
    hits.push(HitRect {
        id: ControlId::BtnMinimize,
        rect: RectF { x: min_x, y: cap_y, w: cap_w, h: cap_h },
    });
    hits.push(HitRect {
        id: ControlId::BtnMaximizeRestore,
        rect: RectF { x: max_x, y: cap_y, w: cap_w, h: cap_h },
    });
    hits.push(HitRect {
        id: ControlId::BtnClose,
        rect: RectF { x: close_x, y: cap_y, w: cap_w, h: cap_h },
    });

    // Two columns below titlebar (matching main_dashboard.png)
    let outer_pad = 16.0;
    let col_gap = 14.0;
    let body_x = card_rect.x + outer_pad;
    let body_y = card_rect.y + title_h + 6.0;
    let body_w = card_rect.w - outer_pad * 2.0;
    let body_h = card_rect.h - title_h - outer_pad - 6.0;

    let left_w = (body_w * 0.37).clamp(295.0, 360.0);
    let left_card = RectF { x: body_x, y: body_y, w: left_w, h: body_h };
    let right_x = left_card.x + left_card.w + col_gap;
    let right_w = (body_w - left_w - col_gap).max(320.0);

    // Left card header PiP expand icon
    hits.push(HitRect {
        id: ControlId::BtnTogglePip,
        rect: RectF { x: left_card.x + left_card.w - 40.0, y: left_card.y + 13.0, w: 26.0, h: 26.0 },
    });

    let lx = left_card.x + 16.0;
    let lw = left_card.w - 32.0;
    let mut gauge_cx = left_card.x + left_card.w * 0.5;
    let mut gauge_cy = left_card.y + left_card.h * 0.48;
    let mut gauge_radius = 86.0f32;

    if q.phase == SessionPhase::Idle {
        // Segmented toggle: Productive Focus | Posture Guard
        let mode_y = left_card.y + 48.0;
        let half = (lw - 4.0) * 0.5;
        hits.push(HitRect {
            id: ControlId::BtnModeFocus,
            rect: RectF { x: lx + 2.0, y: mode_y + 2.0, w: half, h: 30.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnModeHealth,
            rect: RectF { x: lx + half + 2.0, y: mode_y + 2.0, w: half, h: 30.0 },
        });

        // Subtitle takes ~42px, optional YT Music ticker adds 36px above step box
        let ticker_reserve = if q.has_track { 36.0 } else { 0.0 };
        if q.has_track {
            let pill_r = RectF {
                x: lx,
                y: mode_y + 84.0,
                w: lw,
                h: 28.0,
            };
            push_ticker_hits(pill_r, &mut hits);
        }

        // Big stepper box below explanatory text (and optional YT Music ticker)
        let step_box_y = mode_y + 84.0 + ticker_reserve;
        let step_box_h: f32 = if q.has_track && left_card.h < 515.0 { 78.0 } else { 88.0 };
        let input_h = (step_box_h - 22.0).max(52.0);
        hits.push(HitRect {
            id: ControlId::InputTotalMins,
            rect: RectF {
                x: lx + 14.0,
                y: step_box_y + (step_box_h - input_h) * 0.5,
                w: 98.0,
                h: input_h,
            },
        });
        let arrow_x = lx + lw - 48.0;
        let arrow_h: f32 = ((step_box_h - 24.0) * 0.5).max(26.0);
        hits.push(HitRect {
            id: ControlId::BtnTotalUp,
            rect: RectF { x: arrow_x, y: step_box_y + 10.0, w: 36.0, h: arrow_h },
        });
        hits.push(HitRect {
            id: ControlId::BtnTotalDown,
            rect: RectF { x: arrow_x, y: step_box_y + 14.0 + arrow_h, w: 36.0, h: arrow_h },
        });

        // Focus period length & Break duration rows
        let row1_y = step_box_y + step_box_h + 10.0;
        let plus_x = lx + lw - 40.0;
        let minus_x = plus_x - 104.0;
        hits.push(HitRect {
            id: ControlId::BtnChunkDown,
            rect: RectF { x: minus_x, y: row1_y + 5.0, w: 26.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::InputPeriodMins,
            rect: RectF { x: minus_x + 30.0, y: row1_y + 4.0, w: 36.0, h: 28.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnChunkUp,
            rect: RectF { x: plus_x, y: row1_y + 5.0, w: 26.0, h: 26.0 },
        });

        let row2_y = row1_y + 40.0;
        hits.push(HitRect {
            id: ControlId::BtnBreakDown,
            rect: RectF { x: minus_x, y: row2_y + 5.0, w: 26.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::InputBreakMins,
            rect: RectF { x: minus_x + 30.0, y: row2_y + 4.0, w: 36.0, h: 28.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnBreakUp,
            rect: RectF { x: plus_x, y: row2_y + 5.0, w: 26.0, h: 26.0 },
        });

        let cta_y = left_card.y + left_card.h - 56.0;
        let skip_y = cta_y - 34.0;
        hits.push(HitRect {
            id: ControlId::BtnToggleSkipBreaks,
            rect: RectF { x: lx + lw * 0.28, y: skip_y, w: lw * 0.44, h: 24.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnPlayPause,
            rect: RectF { x: lx, y: cta_y, w: lw, h: 42.0 },
        });
    } else {
        let has_ticker = q.has_track;
        let ticker_reserve = if has_ticker { 42.0 } else { 0.0 };
        gauge_cx = left_card.x + left_card.w * 0.5;
        gauge_cy = left_card.y + (left_card.h - ticker_reserve) * 0.43;
        gauge_radius = (lw * 0.35).clamp(68.0, 94.0);
        let stealth_r = (gauge_radius * 0.78).max(40.0);
        hits.push(HitRect {
            id: ControlId::BtnToggleStealthTimer,
            rect: RectF {
                x: gauge_cx - stealth_r,
                y: gauge_cy - stealth_r,
                w: stealth_r * 2.0,
                h: stealth_r * 2.0,
            },
        });

        let btn_y = left_card.y + left_card.h - 102.0 - ticker_reserve;
        let btn_s = 48.0;
        let gap = 18.0;
        let row_w = btn_s * 2.0 + gap;
        let bx = gauge_cx - row_w * 0.5;
        hits.push(HitRect {
            id: ControlId::BtnPlayPause,
            rect: RectF { x: bx, y: btn_y, w: btn_s, h: btn_s },
        });
        hits.push(HitRect {
            id: ControlId::BtnReset,
            rect: RectF { x: bx + btn_s + gap, y: btn_y, w: btn_s, h: btn_s },
        });
        hits.push(HitRect {
            id: ControlId::BtnCycleTimerSubtitle,
            rect: RectF { x: left_card.x + 16.0, y: btn_y + btn_s + 8.0, w: lw, h: 22.0 },
        });
        if has_ticker {
            let pill_r = RectF {
                x: left_card.x + 12.0,
                y: left_card.y + left_card.h - 40.0,
                w: (left_card.w - 24.0).max(140.0),
                h: 28.0,
            };
            push_ticker_hits(pill_r, &mut hits);
        }
    }

    // Right top card (Daily progress + Posture Habit bar)
    let top_h = 228.0;
    let top_card = RectF { x: right_x, y: body_y, w: right_w, h: top_h };
    let icon_y = top_card.y + 13.0;
    let icon_s = 22.0;
    let icon_gap = 4.0;
    let ic6 = top_card.x + top_card.w - 14.0 - icon_s;
    let ic5 = ic6 - icon_gap - icon_s;
    let ic4 = ic5 - icon_gap - icon_s;
    let ic3 = ic4 - icon_gap - icon_s;
    let ic2 = ic3 - icon_gap - icon_s;
    let ic1 = ic2 - icon_gap - icon_s;
    let col_w = (top_card.w - 32.0) / 3.0;
    let goal_col_x = top_card.x + 16.0 + col_w;
    hits.push(HitRect {
        id: ControlId::BtnEditGoal,
        rect: RectF { x: goal_col_x + (col_w - 104.0) * 0.5, y: top_card.y + 34.0, w: 104.0, h: 104.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnToggleCompletedFormat,
        rect: RectF {
            x: top_card.x + top_card.w * 0.25,
            y: top_card.y + 143.0,
            w: top_card.w * 0.50,
            h: 22.0,
        },
    });
    hits.push(HitRect {
        id: ControlId::BtnTopToolbarSound,
        rect: RectF { x: ic1, y: icon_y, w: icon_s, h: icon_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTopToolbarMusic,
        rect: RectF { x: ic2, y: icon_y, w: icon_s, h: icon_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTopToolbarPip,
        rect: RectF { x: ic3, y: icon_y, w: icon_s, h: icon_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnTopToolbarAppearance,
        rect: RectF { x: ic4, y: icon_y, w: icon_s, h: icon_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnOpenStats,
        rect: RectF { x: ic5, y: icon_y, w: icon_s, h: icon_s },
    });
    hits.push(HitRect {
        id: ControlId::BtnEditGoal,
        rect: RectF { x: ic6, y: icon_y, w: icon_s, h: icon_s },
    });

    // Right bottom card (Distraction Guard: Blocked Apps | Restricted Sites | Prayer Times)
    let bot_y = top_card.y + top_card.h + col_gap;
    let bot_card = RectF {
        x: right_x,
        y: bot_y,
        w: right_w,
        h: (body_y + body_h - bot_y).max(210.0),
    };

    let tab_y = bot_card.y + 14.0;
    let tab_x0 = bot_card.x + 16.0;
    let refresh_s = 26.0;
    let refresh_x = bot_card.x + bot_card.w - 16.0 - refresh_s;
    hits.push(HitRect {
        id: ControlId::BtnGuardRefreshApps,
        rect: RectF { x: refresh_x, y: tab_y + 1.0, w: refresh_s, h: refresh_s },
    });

    let tabs_avail = (refresh_x - 110.0 - tab_x0).max(240.0);
    let tab_gap = 6.0;
    let tab_w = ((tabs_avail - tab_gap * 2.0) / 3.0).clamp(78.0, 126.0);
    hits.push(HitRect {
        id: ControlId::BtnGuardTabApps,
        rect: RectF { x: tab_x0, y: tab_y, w: tab_w, h: 28.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnGuardTabSites,
        rect: RectF { x: tab_x0 + tab_w + tab_gap, y: tab_y, w: tab_w, h: 28.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnGuardTabPrayer,
        rect: RectF { x: tab_x0 + (tab_w + tab_gap) * 2.0, y: tab_y, w: tab_w, h: 28.0 },
    });

    let content_rect = RectF {
        x: bot_card.x + 16.0,
        y: tab_y + 38.0,
        w: bot_card.w - 32.0,
        h: (bot_card.y + bot_card.h - (tab_y + 38.0) - 12.0).max(120.0),
    };

    let mut guard_list_rect = None;
    match q.guard_tab {
        GuardTab::BlockedApps => {
            guard_list_rect = push_blocked_apps_hits(content_rect, q.visible_app_rows, &mut hits);
        }
        GuardTab::RestrictedSites => {
            push_restricted_sites_hits(content_rect, q.site_rows, q.site_scroll, &mut hits);
        }
        GuardTab::PrayerTimes => {
            push_prayer_tab_hits(content_rect, &mut hits);
        }
    }

    // Optional centered Settings & Preferences Modal (6-tab scrollable Fluent modal)
    let mut modal_rect = None;
    let mut modal_body_rect = None;
    let mut settings_max_scroll = 0.0f32;
    let mut opacity_slider_rect = None;
    if let Some(tab) = q.settings_tab {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 60.0).clamp(540.0, 640.0);
        let mh = (card_rect.h - 40.0).clamp(420.0, 560.0);
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });

        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        let tab_bar = RectF { x: mr.x + 18.0, y: mr.y + 58.0, w: mr.w - 36.0, h: 32.0 };
        let tabs = [
            SettingsTab::Appearance,
            SettingsTab::Session,
            SettingsTab::PipMode,
            SettingsTab::Sound,
            SettingsTab::MobileSync,
            SettingsTab::System,
        ];
        let tab_w = (tab_bar.w - 8.0) / tabs.len() as f32;
        for (i, t) in tabs.into_iter().enumerate() {
            hits.push(HitRect {
                id: ControlId::BtnSettingsTab(t),
                rect: RectF {
                    x: tab_bar.x + 4.0 + tab_w * i as f32,
                    y: tab_bar.y + 3.0,
                    w: tab_w,
                    h: 26.0,
                },
            });
        }

        let mc = RectF {
            x: mr.x + 18.0,
            y: tab_bar.y + tab_bar.h + 10.0,
            w: mr.w - 36.0,
            h: (mr.y + mr.h - (tab_bar.y + tab_bar.h + 10.0) - 14.0).max(160.0),
        };
        modal_body_rect = Some(mc);
        let (sl, max_s) = push_settings_tab_hits(mc, q.config, tab, q.settings_scroll, &mut hits);
        opacity_slider_rect = sl;
        settings_max_scroll = max_s;
    } else if q.show_ambient_mixer {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 80.0).clamp(510.0, 580.0);
        let mh = (card_rect.h - 40.0).clamp(430.0, 560.0);
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        let mc = RectF {
            x: mr.x + 18.0,
            y: mr.y + 62.0,
            w: mr.w - 36.0,
            h: (mr.y + mr.h - (mr.y + 62.0) - 14.0).max(160.0),
        };
        modal_body_rect = Some(mc);
        settings_max_scroll = push_ambient_mixer_hits(mc, q.settings_scroll, &mut hits);
    } else if q.show_goal_modal {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 120.0).clamp(420.0, 480.0);
        let mh = 340.0;
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        let hour_choices: [u32; 7] = [1, 2, 3, 4, 5, 6, 8];
        let chip_gap = 8.0;
        let chip_w = (mr.w - 40.0 - chip_gap * 6.0) / 7.0;
        for (i, h_val) in hour_choices.into_iter().enumerate() {
            hits.push(HitRect {
                id: ControlId::BtnGoalHours(h_val),
                rect: RectF {
                    x: mr.x + 20.0 + (chip_w + chip_gap) * i as f32,
                    y: mr.y + 114.0,
                    w: chip_w,
                    h: 34.0,
                },
            });
        }
        let stand_choices: [u32; 5] = [2, 3, 4, 6, 8];
        let sg = 8.0;
        let sw = (mr.w - 40.0 - sg * 4.0) / 5.0;
        for (i, b_val) in stand_choices.into_iter().enumerate() {
            hits.push(HitRect {
                id: ControlId::BtnHealthGoalBreaks(b_val),
                rect: RectF {
                    x: mr.x + 20.0 + (sw + sg) * i as f32,
                    y: mr.y + 216.0,
                    w: sw,
                    h: 34.0,
                },
            });
        }
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + 20.0, y: mr.y + mr.h - 52.0, w: mr.w - 40.0, h: 34.0 },
        });
    } else if q.show_share_modal {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 40.0).clamp(520.0, 620.0);
        let mh = (card_rect.h - 32.0).clamp(430.0, 500.0);
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnShareDayToday,
            rect: RectF { x: mr.x + mr.w - 196.0, y: mr.y + 14.0, w: 68.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnShareDayYesterday,
            rect: RectF { x: mr.x + mr.w - 124.0, y: mr.y + 14.0, w: 80.0, h: 26.0 },
        });
        let btn_y = mr.y + mr.h - 46.0;
        let gap = 10.0;
        let bw = (mr.w - 36.0 - gap * 2.0) / 3.0;
        hits.push(HitRect {
            id: ControlId::BtnShareCopyText,
            rect: RectF { x: mr.x + 18.0, y: btn_y, w: bw, h: 32.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnShareCopyImage,
            rect: RectF { x: mr.x + 18.0 + bw + gap, y: btn_y, w: bw, h: 32.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnShareSavePng,
            rect: RectF { x: mr.x + 18.0 + (bw + gap) * 2.0, y: btn_y, w: bw, h: 32.0 },
        });
    } else if q.show_stats_modal {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 30.0).clamp(660.0, 880.0);
        let mh = (card_rect.h - 26.0).clamp(480.0, 650.0);
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnOpenShareCard,
            rect: RectF { x: mr.x + mr.w - 148.0, y: mr.y + 14.0, w: 102.0, h: 26.0 },
        });
        let periods = [
            StatsPeriod::Today,
            StatsPeriod::Yesterday,
            StatsPeriod::Week,
            StatsPeriod::Month,
            StatsPeriod::Quarter,
        ];
        let tab_bar = RectF { x: mr.x + 18.0, y: mr.y + 56.0, w: mr.w - 36.0, h: 32.0 };
        let pw = (tab_bar.w - 8.0) / periods.len() as f32;
        for (i, p) in periods.into_iter().enumerate() {
            hits.push(HitRect {
                id: ControlId::BtnStatsPeriod(p),
                rect: RectF {
                    x: tab_bar.x + 4.0 + pw * i as f32,
                    y: tab_bar.y + 3.0,
                    w: pw,
                    h: 26.0,
                },
            });
        }
        let mc = RectF {
            x: mr.x + 18.0,
            y: tab_bar.y + tab_bar.h + 10.0,
            w: mr.w - 36.0,
            h: (mr.y + mr.h - (tab_bar.y + tab_bar.h + 10.0) - 14.0).max(180.0),
        };
        modal_body_rect = Some(mc);
        settings_max_scroll = push_stats_modal_hits(mc, q.settings_scroll, q.stats_app_rows, &mut hits);
    } else if q.show_prayer_modal {
        hits.push(HitRect {
            id: ControlId::ModalBackdrop,
            rect: card_rect,
        });
        let mw = (card_rect.w - 60.0).clamp(460.0, 540.0);
        let mh = (card_rect.h - 40.0).clamp(400.0, 456.0);
        let mr = RectF {
            x: card_rect.x + (card_rect.w - mw) * 0.5,
            y: card_rect.y + (card_rect.h - mh) * 0.5,
            w: mw,
            h: mh,
        };
        modal_rect = Some(mr);
        hits.push(HitRect { id: ControlId::ModalSurface, rect: mr });
        hits.push(HitRect {
            id: ControlId::BtnCloseModal,
            rect: RectF { x: mr.x + mr.w - 38.0, y: mr.y + 14.0, w: 26.0, h: 26.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnPrayerAutoGps,
            rect: RectF { x: mr.x + mr.w - 134.0, y: mr.y + 14.0, w: 88.0, h: 26.0 },
        });
        let sx = mr.x + 18.0;
        let sw = (mr.w - 36.0 - 82.0).max(160.0);
        let sy = mr.y + 56.0;
        hits.push(HitRect {
            id: ControlId::BtnPrayerSearchBox,
            rect: RectF { x: sx, y: sy, w: sw, h: 32.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnPrayerSearchClear,
            rect: RectF { x: sx + sw - 26.0, y: sy + 5.0, w: 22.0, h: 22.0 },
        });
        hits.push(HitRect {
            id: ControlId::BtnPrayerSearchOnline,
            rect: RectF { x: sx + sw + 8.0, y: sy, w: 74.0, h: 32.0 },
        });
        let list_y0 = sy + 44.0;
        for i in 0..q.prayer_city_rows.min(6) {
            hits.push(HitRect {
                id: ControlId::BtnPrayerCityPick(i),
                rect: RectF {
                    x: mr.x + 18.0,
                    y: list_y0 + i as f32 * 40.0,
                    w: mr.w - 36.0,
                    h: 35.0,
                },
            });
        }
    }

    UiLayout {
        card_rect,
        card_radius: 14.0,
        drag_rect,
        gauge_cx,
        gauge_cy,
        gauge_radius,
        left_card_rect: Some(left_card),
        guard_list_rect,
        modal_rect,
        modal_body_rect,
        settings_max_scroll,
        opacity_slider_rect,
        hit_rects: hits,
    }
}

struct AppearanceCards {
    sec1: RectF,
    sec2: RectF,
    sec3: RectF,
    total_h: f32,
}

fn appearance_cards(body: RectF, config: &AppConfig, scroll: f32) -> AppearanceCards {
    let w = (body.w - 10.0).max(200.0);
    let x = body.x;
    let y1 = body.y - scroll;
    let sec1_h = if config.timer_theme == "hourglass" { 266.0 } else { 226.0 };
    let sec1 = RectF { x, y: y1, w, h: sec1_h };

    let y2 = y1 + sec1_h + 12.0;
    let sec2_h = 296.0;
    let sec2 = RectF { x, y: y2, w, h: sec2_h };

    let y3 = y2 + sec2_h + 12.0;
    let sec3_h = match config.ambient_mode {
        AmbientMode::Default => 154.0,
        AmbientMode::YtMusic => 352.0,
        AmbientMode::Custom => {
            let rows = if config.recent_gifs.is_empty() { 1.0 } else { config.recent_gifs.len().min(5) as f32 };
            336.0 + rows * 30.0
        }
    };
    let sec3 = RectF { x, y: y3, w, h: sec3_h };

    AppearanceCards {
        sec1,
        sec2,
        sec3,
        total_h: sec1_h + 12.0 + sec2_h + 12.0 + sec3_h + 8.0,
    }
}

fn push_ticker_hits(pill: RectF, hits: &mut Vec<HitRect>) {
    let mb_size = 18.0;
    let mb_y = pill.y + (pill.h - mb_size) * 0.5;
    let b1 = pill.x + 6.0;
    let b2 = b1 + mb_size + 3.0;
    let b3 = b2 + mb_size + 3.0;
    hits.push(HitRect {
        id: ControlId::BtnMediaPrev,
        rect: RectF { x: b1, y: mb_y, w: mb_size, h: mb_size },
    });
    hits.push(HitRect {
        id: ControlId::BtnMediaPlayPause,
        rect: RectF { x: b2, y: mb_y, w: mb_size, h: mb_size },
    });
    hits.push(HitRect {
        id: ControlId::BtnMediaNext,
        rect: RectF { x: b3, y: mb_y, w: mb_size, h: mb_size },
    });
    let text_x = b3 + mb_size + 6.0;
    hits.push(HitRect {
        id: ControlId::BtnMediaToggleWindow,
        rect: RectF {
            x: text_x,
            y: pill.y,
            w: (pill.x + pill.w - 6.0 - text_x).max(20.0),
            h: pill.h,
        },
    });
}

fn push_modal_hit(body: RectF, hits: &mut Vec<HitRect>, id: ControlId, r: RectF) {
    let top = r.y.max(body.y);
    let bottom = (r.y + r.h).min(body.y + body.h);
    let left = r.x.max(body.x);
    let right = (r.x + r.w).min(body.x + body.w);
    if bottom > top + 2.0 && right > left + 2.0 {
        hits.push(HitRect {
            id,
            rect: RectF { x: left, y: top, w: right - left, h: bottom - top },
        });
    }
}

fn push_settings_tab_hits(
    body: RectF,
    config: &AppConfig,
    tab: SettingsTab,
    scroll: f32,
    hits: &mut Vec<HitRect>,
) -> (Option<RectF>, f32) {
    let inner_w = (body.w - 10.0).max(200.0);
    let mut opacity_slider_rect = None;
    let total_h: f32;

    match tab {
        SettingsTab::Appearance => {
            let cards = appearance_cards(body, config, scroll);
            total_h = cards.total_h;

            // Card 1: Timer Ring Theme (3x2) + Hourglass auto-rotate + Stealth peek
            let px = cards.sec1.x + 14.0;
            let pw = cards.sec1.w - 28.0;
            let chip_w = (pw - 16.0) / 3.0;
            let mut y = cards.sec1.y + 52.0;
            for i in 0..TIMER_THEMES.len() {
                let col = (i % 3) as f32;
                let row = (i / 3) as f32;
                push_modal_hit(
                    body,
                    hits,
                    ControlId::BtnRingTheme(i),
                    RectF { x: px + (chip_w + 8.0) * col, y: y + row * 38.0, w: chip_w, h: 32.0 },
                );
            }
            y += 82.0;
            if config.timer_theme == "hourglass" {
                push_modal_hit(body, hits, ControlId::BtnToggleHourglassRotate, RectF { x: px, y, w: pw, h: 36.0 });
                y += 40.0;
            }
            push_modal_hit(body, hits, ControlId::BtnToggleStealthTimer, RectF { x: px, y, w: pw, h: 40.0 });
            y += 42.0;
            push_modal_hit(body, hits, ControlId::BtnToggleStealthPeek, RectF { x: px, y, w: pw, h: 40.0 });

            // Card 2: Accent Color Palette (6 labeled swatches + Dynamic Sync + Complementary)
            let p2x = cards.sec2.x + 14.0;
            let p2w = cards.sec2.w - 28.0;
            let sw_w = (p2w - 30.0) / 6.0;
            let sw_y = cards.sec2.y + 54.0;
            for i in 0..ACCENT_PRESETS.len() {
                push_modal_hit(
                    body,
                    hits,
                    ControlId::AccentPreset(i),
                    RectF { x: p2x + (sw_w + 6.0) * i as f32, y: sw_y, w: sw_w, h: 32.0 },
                );
            }
            push_modal_hit(body, hits, ControlId::BtnToggleDynamicAlbum, RectF { x: p2x, y: sw_y + 44.0, w: p2w, h: 42.0 });
            push_modal_hit(body, hits, ControlId::BtnToggleComplementary, RectF { x: p2x, y: sw_y + 92.0, w: p2w, h: 42.0 });
            push_modal_hit(body, hits, ControlId::BtnToggleMediaSync, RectF { x: p2x, y: sw_y + 140.0, w: p2w, h: 42.0 });
            push_modal_hit(body, hits, ControlId::BtnToggleSpotifySync, RectF { x: p2x, y: sw_y + 188.0, w: p2w, h: 42.0 });

            // Card 3: Mode-specific Ambient Backdrop (Default Plant | YT Music Vinyl | Custom GIF)
            let p3x = cards.sec3.x + 14.0;
            let p3w = cards.sec3.w - 28.0;
            let mut y3 = cards.sec3.y + 52.0;
            let seg3 = (p3w - 12.0) / 3.0;
            push_modal_hit(body, hits, ControlId::BtnAmbientDefault, RectF { x: p3x, y: y3, w: seg3, h: 30.0 });
            push_modal_hit(body, hits, ControlId::BtnAmbientYtMusic, RectF { x: p3x + seg3 + 6.0, y: y3, w: seg3, h: 30.0 });
            push_modal_hit(body, hits, ControlId::BtnAmbientCustom, RectF { x: p3x + (seg3 + 6.0) * 2.0, y: y3, w: seg3, h: 30.0 });
            y3 += 40.0;

            match config.ambient_mode {
                AmbientMode::Default => {}
                AmbientMode::YtMusic => {
                    let mw = (p3w - 10.0) * 0.5;
                    push_modal_hit(body, hits, ControlId::BtnGifCircle, RectF { x: p3x, y: y3 + 18.0, w: mw, h: 30.0 });
                    push_modal_hit(body, hits, ControlId::BtnGifFull, RectF { x: p3x + mw + 10.0, y: y3 + 18.0, w: mw, h: 30.0 });
                    y3 += 58.0;

                    push_modal_hit(body, hits, ControlId::VinylSpeedSlider, RectF { x: p3x, y: y3 + 18.0, w: p3w, h: 16.0 });
                    y3 += 40.0;
                    let sw = (p3w - 18.0) / 4.0;
                    for i in 0..4 {
                        push_modal_hit(body, hits, ControlId::BtnVinylSpeedPreset(i), RectF { x: p3x + (sw + 6.0) * i as f32, y: y3, w: sw, h: 26.0 });
                    }
                    y3 += 36.0;

                    push_modal_hit(body, hits, ControlId::VinylSizeSlider, RectF { x: p3x, y: y3 + 18.0, w: p3w, h: 16.0 });
                    y3 += 40.0;
                    let zw = (p3w - 12.0) / 3.0;
                    for i in 0..3 {
                        push_modal_hit(body, hits, ControlId::BtnVinylSizePreset(i), RectF { x: p3x + (zw + 6.0) * i as f32, y: y3, w: zw, h: 26.0 });
                    }
                    y3 += 34.0;
                    push_modal_hit(body, hits, ControlId::BtnToggleVinylSpindle, RectF { x: p3x, y: y3, w: p3w, h: 36.0 });
                }
                AmbientMode::Custom => {
                    push_modal_hit(body, hits, ControlId::BtnUploadGif, RectF { x: p3x + p3w - 196.0, y: y3 - 2.0, w: 96.0, h: 22.0 });
                    push_modal_hit(body, hits, ControlId::BtnClearRecentGifs, RectF { x: p3x + p3w - 94.0, y: y3 - 2.0, w: 94.0, h: 22.0 });
                    y3 += 24.0;
                    if config.recent_gifs.is_empty() {
                        y3 += 30.0;
                    } else {
                        for i in 0..config.recent_gifs.len().min(5) {
                            push_modal_hit(body, hits, ControlId::RecentGifSelect(i), RectF { x: p3x, y: y3, w: p3w - 28.0, h: 26.0 });
                            push_modal_hit(body, hits, ControlId::RecentGifDelete(i), RectF { x: p3x + p3w - 24.0, y: y3 + 2.0, w: 22.0, h: 22.0 });
                            y3 += 30.0;
                        }
                    }
                    y3 += 8.0;
                    let mw = (p3w - 10.0) * 0.5;
                    push_modal_hit(body, hits, ControlId::BtnGifCircle, RectF { x: p3x, y: y3 + 18.0, w: mw, h: 30.0 });
                    push_modal_hit(body, hits, ControlId::BtnGifFull, RectF { x: p3x + mw + 10.0, y: y3 + 18.0, w: mw, h: 30.0 });
                    y3 += 56.0 + 100.0; // Space for centered GIF Preview thumbnail + name

                    let slider = RectF { x: p3x, y: y3 + 18.0, w: p3w, h: 16.0 };
                    push_modal_hit(body, hits, ControlId::OpacitySlider, slider);
                    opacity_slider_rect = Some(slider);
                }
            }
        }
        SettingsTab::Session => {
            let ids = [
                ControlId::BtnToggleAutoPause,
                ControlId::BtnAutoPauseThreshold,
                ControlId::BtnToggleNeutralOverstay,
                ControlId::BtnNeutralThreshold,
                ControlId::BtnToggleNeutralSoftblock,
                ControlId::BtnToggleAutoClose,
                ControlId::BtnToggleBlockAllMonitors,
            ];
            let mut y = body.y - scroll;
            for id in ids {
                push_modal_hit(body, hits, id, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                y += 54.0;
            }
            total_h = ids.len() as f32 * 54.0;
        }
        SettingsTab::PipMode => {
            let mut y = body.y - scroll;
            let sw_mode = (inner_w - 10.0) * 0.5;
            push_modal_hit(body, hits, ControlId::BtnPipStyleCard, RectF { x: body.x, y: y + 20.0, w: sw_mode, h: 30.0 });
            push_modal_hit(body, hits, ControlId::BtnPipStyleCapsule, RectF { x: body.x + sw_mode + 10.0, y: y + 20.0, w: sw_mode, h: 30.0 });
            y += 60.0;

            if config.pip_style == PipStyle::Capsule {
                push_modal_hit(body, hits, ControlId::BtnToggleCapsuleAlbumBg, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                y += 54.0;
                if config.pip_capsule_album_bg {
                    push_modal_hit(body, hits, ControlId::BtnToggleCapsuleAlbumSpin, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                    y += 54.0;
                    push_modal_hit(body, hits, ControlId::BtnCapsuleAlbumOpacity, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                    y += 54.0;
                }
            }

            let ids = [
                ControlId::BtnToggleAutoPip,
                ControlId::BtnToggleAutoPipBlur,
                ControlId::BtnToggleHideTaskbarInPip,
            ];
            for id in ids {
                push_modal_hit(body, hits, id, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                y += 54.0;
            }

            if config.pip_style == PipStyle::Card {
                push_modal_hit(body, hits, ControlId::BtnTogglePeekingVinyl, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                y += 54.0;
                let sw = (inner_w - 10.0) * 0.5;
                push_modal_hit(body, hits, ControlId::BtnVinylSideLeft, RectF { x: body.x, y: y + 20.0, w: sw, h: 30.0 });
                push_modal_hit(body, hits, ControlId::BtnVinylSideRight, RectF { x: body.x + sw + 10.0, y: y + 20.0, w: sw, h: 30.0 });
                y += 60.0;
            }
            total_h = y - (body.y - scroll) + 20.0;
        }
        SettingsTab::Sound => {
            let sl = sound_tab_layout(body, config, scroll);
            push_modal_hit(body, hits, ControlId::BtnToggleNotifications, sl.notif_row);
            push_modal_hit(body, hits, ControlId::BtnFocusSoundCycle, sl.alert_focus.cycle);
            push_modal_hit(body, hits, ControlId::BtnFocusSoundTest, sl.alert_focus.test);
            if let Some(uf) = sl.alert_focus.upload_btn {
                push_modal_hit(body, hits, ControlId::BtnFocusSoundUpload, uf);
            }
            push_modal_hit(body, hits, ControlId::BtnBreakSoundCycle, sl.alert_break.cycle);
            push_modal_hit(body, hits, ControlId::BtnBreakSoundTest, sl.alert_break.test);
            if let Some(ub) = sl.alert_break.upload_btn {
                push_modal_hit(body, hits, ControlId::BtnBreakSoundUpload, ub);
            }
            push_modal_hit(body, hits, ControlId::BtnWarningSoundCycle, sl.alert_warn.cycle);
            push_modal_hit(body, hits, ControlId::BtnWarningSoundTest, sl.alert_warn.test);
            if let Some(uw) = sl.alert_warn.upload_btn {
                push_modal_hit(body, hits, ControlId::BtnWarningSoundUpload, uw);
            }
            push_modal_hit(body, hits, ControlId::BtnCycleCountdownSecs, sl.countdown_secs);
            push_modal_hit(body, hits, ControlId::BtnCycleCountdownStyle, sl.countdown_style);
            push_modal_hit(body, hits, ControlId::BtnToggleCountdownSound, sl.countdown_toggle);
            total_h = sl.total_h;
        }
        SettingsTab::MobileSync => {
            let info_x = body.x + 148.0;
            let btn_y = body.y + 102.0 - scroll;
            let bs = 36.0;
            push_modal_hit(body, hits, ControlId::BtnMediaPrev, RectF { x: info_x, y: btn_y, w: bs, h: bs });
            push_modal_hit(body, hits, ControlId::BtnMediaPlayPause, RectF { x: info_x + bs + 10.0, y: btn_y, w: bs, h: bs });
            push_modal_hit(body, hits, ControlId::BtnMediaNext, RectF { x: info_x + (bs + 10.0) * 2.0, y: btn_y, w: bs, h: bs });
            push_modal_hit(body, hits, ControlId::BtnMediaToggleWindow, RectF { x: info_x + (bs + 10.0) * 3.0 + 8.0, y: btn_y + 2.0, w: 112.0, h: 32.0 });

            let qr_card_y = body.y + 162.0 - scroll;
            let right_x = body.x + 180.0;
            push_modal_hit(body, hits, ControlId::BtnCopyMobileUrl, RectF { x: right_x, y: qr_card_y + 148.0, w: 108.0, h: 30.0 });
            push_modal_hit(body, hits, ControlId::BtnOpenMobileUrl, RectF { x: right_x + 116.0, y: qr_card_y + 148.0, w: 124.0, h: 30.0 });
            total_h = 372.0;
        }
        SettingsTab::System => {
            let ids = [
                ControlId::BtnCheckUpdate,
                ControlId::BtnToggleMinimizeTray,
                ControlId::BtnToggleLanguage,
                ControlId::BtnSysResetToday,
                ControlId::BtnSysClearApps,
                ControlId::BtnSysClearGifs,
            ];
            let mut y = body.y - scroll;
            for id in ids {
                push_modal_hit(body, hits, id, RectF { x: body.x, y, w: inner_w, h: 48.0 });
                y += 54.0;
            }
            total_h = ids.len() as f32 * 54.0;
        }
    }

    (opacity_slider_rect, (total_h - body.h).max(0.0))
}

fn push_blocked_apps_hits(
    area: RectF,
    visible_app_rows: usize,
    hits: &mut Vec<HitRect>,
) -> Option<RectF> {
    let search_y = area.y + 20.0;
    hits.push(HitRect {
        id: ControlId::BtnAppSearchBox,
        rect: RectF { x: area.x, y: search_y, w: (area.w - 30.0).max(80.0), h: 28.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnAppSearchClear,
        rect: RectF { x: area.x + area.w - 26.0, y: search_y + 2.0, w: 24.0, h: 24.0 },
    });

    let filter_y = search_y + 34.0;
    hits.push(HitRect {
        id: ControlId::BtnFilterAll,
        rect: RectF { x: area.x, y: filter_y, w: 42.0, h: 24.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnFilterBlocked,
        rect: RectF { x: area.x + 48.0, y: filter_y, w: 68.0, h: 24.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnFilterAccessible,
        rect: RectF { x: area.x + 122.0, y: filter_y, w: 86.0, h: 24.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnAppSort,
        rect: RectF { x: area.x + area.w - 134.0, y: filter_y, w: 134.0, h: 24.0 },
    });

    let list_y = filter_y + 30.0;
    let list_h = (area.y + area.h - list_y).max(60.0);
    let list_rect = RectF {
        x: area.x,
        y: list_y,
        w: area.w,
        h: list_h,
    };
    let row_gap = 6.0;
    let cap = (((list_h + row_gap) / (38.0 + row_gap)).floor() as usize).clamp(3, 18);
    let row_h = ((list_h - (cap.saturating_sub(1) as f32) * row_gap) / cap as f32).clamp(38.0, 48.0);
    for i in 0..visible_app_rows {
        let ry = list_y + (row_h + row_gap) * i as f32;
        if ry + row_h > area.y + area.h + 2.0 {
            break;
        }
        hits.push(HitRect {
            id: ControlId::GuardAppToggle(i),
            rect: RectF { x: area.x, y: ry, w: area.w, h: row_h },
        });
    }
    Some(list_rect)
}

/// Computes how many Blocked Apps rows fit cleanly in the right-bottom card for a given window height (DIP).
pub fn blocked_apps_capacity(win_h: f32) -> usize {
    let card_h = (win_h - 4.0).max(460.0);
    let body_h = card_h - 36.0 - 16.0 - 6.0;
    let bot_h = (body_h - 228.0 - 14.0).max(210.0);
    let content_h = (bot_h - 14.0 - 38.0 - 12.0).max(120.0);
    let list_h = (content_h - 84.0).max(60.0);
    (((list_h + 6.0) / (38.0 + 6.0)).floor() as usize).clamp(3, 18)
}

fn push_restricted_sites_hits(area: RectF, site_rows: usize, site_scroll: usize, hits: &mut Vec<HitRect>) {
    // 1. Add domain input bar + Add button
    let bar_y = area.y + 18.0;
    let bar_h = 28.0;
    let add_btn_w = 68.0;
    let input_w = area.w - add_btn_w - 8.0;

    hits.push(HitRect {
        id: ControlId::BtnSiteSearchBox,
        rect: RectF { x: area.x, y: bar_y, w: input_w, h: bar_h },
    });
    hits.push(HitRect {
        id: ControlId::BtnSiteClear,
        rect: RectF { x: area.x + input_w - 24.0, y: bar_y + 4.0, w: 20.0, h: 20.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnSiteAdd,
        rect: RectF { x: area.x + input_w + 8.0, y: bar_y, w: add_btn_w, h: bar_h },
    });

    // 2. Grid of domains
    let grid_y = bar_y + bar_h + 8.0;
    let col_w = (area.w - 10.0) * 0.5;
    let row_h = 32.0;
    let gap = 6.0;

    let bottom_reserved = 34.0;
    let max_avail_h = area.y + area.h - bottom_reserved - grid_y;
    let visible_rows = ((max_avail_h / (row_h + gap)).floor() as usize).max(1);
    let visible_cap = visible_rows * 2;

    let start_idx = (site_scroll * 2).min(site_rows.saturating_sub(visible_cap));
    for vis_i in 0..visible_cap {
        let i = start_idx + vis_i;
        if i >= site_rows {
            break;
        }
        let col = vis_i % 2;
        let row = vis_i / 2;
        let ry = grid_y + (row_h + gap) * row as f32;
        let rx = area.x + (col_w + 10.0) * col as f32;

        hits.push(HitRect {
            id: ControlId::SitePresetToggle(i),
            rect: RectF {
                x: rx,
                y: ry,
                w: col_w,
                h: row_h,
            },
        });
        if i >= COMMON_SITE_PRESETS.len() {
            hits.push(HitRect {
                id: ControlId::SiteCustomDelete(i),
                rect: RectF {
                    x: rx + col_w - 62.0,
                    y: ry + (row_h - 20.0) * 0.5,
                    w: 20.0,
                    h: 20.0,
                },
            });
        }
    }

    let btn_y = area.y + area.h - 30.0;
    hits.push(HitRect {
        id: ControlId::BtnGuardClearPasses,
        rect: RectF { x: area.x + area.w - 148.0, y: btn_y, w: 148.0, h: 28.0 },
    });
}

fn push_prayer_tab_hits(area: RectF, hits: &mut Vec<HitRect>) {
    // Top hero clock + next prayer card
    let hero_r = RectF { x: area.x, y: area.y, w: area.w, h: 62.0 };
    hits.push(HitRect {
        id: ControlId::BtnPrayerPrev,
        rect: RectF { x: hero_r.x + hero_r.w - 134.0, y: hero_r.y + 16.0, w: 22.0, h: 30.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnPrayerNext,
        rect: RectF { x: hero_r.x + hero_r.w - 30.0, y: hero_r.y + 16.0, w: 22.0, h: 30.0 },
    });

    // Settings card: Enable Prayer Notifications, Auto Break, Notify before, Schedule Offset
    let card_y = area.y + 68.0;
    hits.push(HitRect {
        id: ControlId::BtnTogglePrayerEnabled,
        rect: RectF { x: area.x + 12.0, y: card_y + 7.0, w: area.w - 24.0, h: 24.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnTogglePrayerBreak,
        rect: RectF { x: area.x + 12.0, y: card_y + 33.0, w: area.w - 24.0, h: 24.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnPrayerAdvanceUp,
        rect: RectF { x: area.x + area.w - 38.0, y: card_y + 59.0, w: 24.0, h: 22.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnPrayerAdvanceDown,
        rect: RectF { x: area.x + area.w - 104.0, y: card_y + 59.0, w: 24.0, h: 22.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnPrayerOffsetUp,
        rect: RectF { x: area.x + area.w - 38.0, y: card_y + 85.0, w: 24.0, h: 22.0 },
    });
    hits.push(HitRect {
        id: ControlId::BtnPrayerOffsetDown,
        rect: RectF { x: area.x + area.w - 104.0, y: card_y + 85.0, w: 24.0, h: 22.0 },
    });
    let loc_y = card_y + 118.0;
    hits.push(HitRect {
        id: ControlId::BtnPrayerLocationCycle,
        rect: RectF { x: area.x + area.w - 118.0, y: loc_y + 5.0, w: 108.0, h: 20.0 },
    });
}

pub fn render_ui(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    w: f32,
    h: f32,
    view: &RenderView<'_>,
) -> Result<UiLayout> {
    let site_rows = build_site_entries(view).len();
    let stats_app_rows = if view.show_stats_modal {
        period_app_breakdown(view.stats, view.today_date, view.stats_period)
            .len()
            .min(8)
    } else {
        0
    };
    let layout = compute_layout(&LayoutQuery {
        w,
        h,
        config: view.config,
        guard_tab: view.guard_tab,
        visible_app_rows: view.running_apps.len(),
        site_rows,
        site_scroll: view.site_scroll,
        settings_tab: view.settings_tab,
        settings_scroll: view.settings_scroll,
        show_ambient_mixer: view.show_ambient_mixer,
        show_stats_modal: view.show_stats_modal,
        show_share_modal: view.show_share_modal,
        show_goal_modal: view.show_goal_modal,
        show_prayer_modal: view.show_prayer_modal,
        prayer_city_rows: view.prayer_city_results.len(),
        stats_app_rows,
        show_pip_drawer: view.show_pip_gif_drawer,
        has_track: view.track.is_some(),
        phase: view.timer.phase(),
    });

    unsafe {
        dc.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));
    }

    if view.config.pip_mode {
        if view.config.pip_style == PipStyle::Capsule {
            render_pip_capsule_ui(dc, factory, dwrite, &layout, view)?;
        } else {
            render_pip_ui(dc, factory, dwrite, &layout, view)?;
        }
    } else {
        render_dashboard_ui(dc, factory, dwrite, &layout, view)?;
    }

    Ok(layout)
}

fn render_pip_ui(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let card = layout.card_rect;
    draw_rounded_card(dc, card, layout.card_radius, rgba(24, 24, 26, 0.98), rgba(255, 255, 255, 0.12), 1.0)?;

    // Full-bleed backdrop inside PiP card (GIF or YT Music album art)
    draw_backdrop_media(dc, factory, card, layout.card_radius, layout, view, true)?;

    // Compact header matching WebView2 PiP: "Focus period (X/Y)" + 3 icons (Style, Expand, Close)
    let header_title = if view.timer.is_paused() {
        "Paused".to_string()
    } else {
        match view.timer.phase() {
            SessionPhase::Idle => "FocusGrow".to_string(),
            SessionPhase::Focusing => {
                let prefix = if view.config.is_health_mode { "Active" } else { "Focus period" };
                format!("{prefix} ({}/{})", view.timer.current_period(), view.timer.total_periods())
            }
            SessionPhase::Resting => {
                format!("Break ({}/{})", view.timer.current_period(), view.timer.total_periods())
            }
        }
    };
    let header_size = if card.w < 215.0 { 9.5 } else { 10.6 };
    draw_text_shadow_nowrap(
        dc,
        dwrite,
        &header_title,
        RectF {
            x: card.x + 10.0,
            y: card.y + 4.0,
            w: (card.w - 82.0).max(40.0),
            h: 20.0,
        },
        header_size,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_vector_pill(dc, find_hit(layout, ControlId::BtnTogglePipStyle), VectorIcon::CapsulePill, view.hovered == Some(ControlId::BtnTogglePipStyle), false, "#93C5FD")?;
    draw_vector_pill(dc, find_hit(layout, ControlId::BtnTogglePip), VectorIcon::PopOut, view.hovered == Some(ControlId::BtnTogglePip), false, view.effective_accent)?;
    draw_vector_pill(dc, find_hit(layout, ControlId::BtnClose), VectorIcon::CloseSoft, view.hovered == Some(ControlId::BtnClose), false, "#D4D4D8")?;

    // Center circular gauge
    draw_focus_ring(dc, factory, dwrite, layout, view, true)?;

    // Primary playback controls + remaining duration
    draw_pip_controls(dc, factory, dwrite, layout, view)?;

    // Bottom music ticker pill
    if let Some(track) = view.track {
        let pill = RectF {
            x: card.x + 8.0,
            y: card.y + card.h - 36.0,
            w: (card.w - 16.0).max(110.0),
            h: 28.0,
        };
        draw_music_ticker_pill(dc, factory, dwrite, pill, layout, view, track)?;
    }

    Ok(())
}

fn render_pip_capsule_ui(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let capsule = layout.card_rect;
    let radius = capsule.h * 0.5;

    // 1. Background Glass Capsule
    draw_rounded_card(
        dc,
        capsule,
        radius,
        rgba(16, 16, 20, 0.94),
        rgba(255, 255, 255, 0.14),
        1.0,
    )?;

    // 1b. Subtle pomodoro progress outline around capsule
    let progress = if view.timer.phase() == SessionPhase::Idle {
        0.0
    } else {
        (1.0 - view.timer.smooth_remaining_fraction(std::time::Instant::now())).clamp(0.0, 1.0)
    };
    if progress > 0.001 && view.timer.phase() != SessionPhase::Idle {
        let prog_brush = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, 0.60), None) }?;
        let rr = D2D1_ROUNDED_RECT {
            rect: capsule.to_d2d(),
            radiusX: radius,
            radiusY: radius,
        };
        unsafe {
            dc.DrawRoundedRectangle(&rr, &prog_brush, 1.5, None);
        }
    }

    // 1c. YouTube Music Album Art Full-Capsule Background (Rotates smoothly like vinyl)
    if view.config.pip_capsule_album_bg {
        if let Some(cover) = view.cover_bitmap {
            let opacity = (view.config.pip_capsule_album_opacity as f32 / 100.0).clamp(0.05, 1.0);
            let cx = capsule.x + capsule.w * 0.5;
            let cy = capsule.y + capsule.h * 0.5;
            let diag = (capsule.w * capsule.w + capsule.h * capsule.h).sqrt().max(capsule.w);
            let art_s = diag * 1.08;
            let art_rect = RectF {
                x: cx - art_s * 0.5,
                y: cy - art_s * 0.5,
                w: art_s,
                h: art_s,
            };

            let geo = unsafe {
                factory.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                    rect: capsule.to_d2d(),
                    radiusX: radius,
                    radiusY: radius,
                })
            };
            if let Ok(g) = geo {
                let mask: ID2D1Geometry = g.into();
                let params = D2D1_LAYER_PARAMETERS1 {
                    contentBounds: capsule.to_d2d(),
                    geometricMask: core::mem::ManuallyDrop::new(Some(mask)),
                    maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                    maskTransform: Matrix3x2::identity(),
                    opacity: 1.0,
                    opacityBrush: core::mem::ManuallyDrop::new(None),
                    layerOptions: D2D1_LAYER_OPTIONS1_NONE,
                };
                unsafe {
                    dc.PushLayer(&params, None);
                }

                if view.config.pip_capsule_album_spin {
                    let angle = if view.playback == PlaybackState::Playing {
                        view.in_card_vinyl_angle
                    } else {
                        0.0
                    };
                    let mut prev = Matrix3x2::identity();
                    unsafe {
                        dc.GetTransform(&mut prev);
                        dc.SetTransform(&(Matrix3x2::rotation_around(angle, pt(cx, cy)) * prev));
                        dc.DrawBitmap(
                            cover,
                            Some(&art_rect.to_d2d()),
                            opacity,
                            D2D1_INTERPOLATION_MODE_LINEAR,
                            None,
                            None,
                        );
                        dc.SetTransform(&prev);
                    }
                } else {
                    unsafe {
                        dc.DrawBitmap(
                            cover,
                            Some(&art_rect.to_d2d()),
                            opacity,
                            D2D1_INTERPOLATION_MODE_LINEAR,
                            None,
                            None,
                        );
                    }
                }

                // Soft dark wash so text, timer, and avatar remain crisp
                if let Ok(wash_brush) = unsafe { dc.CreateSolidColorBrush(&rgba(10, 10, 14, 0.35), None) } {
                    unsafe {
                        dc.FillRectangle(&capsule.to_d2d(), &wash_brush);
                    }
                }

                unsafe {
                    dc.PopLayer();
                    drop(core::mem::ManuallyDrop::into_inner(params.geometricMask));
                }
            }
        }
    }

    // 2. Mini Avatar GIF on the Left
    let is_compact = capsule.w < 235.0;
    let gif_s = if is_compact { 26.0 } else { 30.0 };
    let gif_r = RectF {
        x: capsule.x + 3.5,
        y: capsule.y + (capsule.h - gif_s) * 0.5,
        w: gif_s,
        h: gif_s,
    };
    let avatar_radius = gif_s * 0.5;

    let mut drew_avatar = false;
    if view.config.ambient_mode == AmbientMode::Custom {
        if let Some(anim) = view.gif_anim {
            if !anim.frames.is_empty() {
                let bmp = &anim.frames[view.gif_frame_index % anim.frames.len()].bitmap;
                draw_cover_bitmap_rounded(dc, factory, bmp, gif_r, avatar_radius, 1.0)?;
                drew_avatar = true;
            }
        }
    }

    if !drew_avatar {
        if let Some(logo) = view.app_icon_logo {
            draw_cover_bitmap_rounded(dc, factory, logo, gif_r, avatar_radius, 1.0)?;
        } else {
            draw_rounded_card(dc, gif_r, avatar_radius, hex_color(view.effective_accent, 0.85), rgba(255, 255, 255, 0.2), 1.0)?;
        }
    }

    // Avatar outline ring
    let ring_color = hex_color(view.effective_accent, 0.80);
    let ring_brush = unsafe { dc.CreateSolidColorBrush(&ring_color, None) }?;
    unsafe {
        dc.DrawEllipse(
            &D2D1_ELLIPSE {
                point: pt(gif_r.x + avatar_radius, gif_r.y + avatar_radius),
                radiusX: avatar_radius,
                radiusY: avatar_radius,
            },
            &ring_brush,
            1.2,
            None,
        );
    }

    // 3. Timer Text & Phase Dot
    let peeking_stealth = view.config.stealth_hover_peek
        && view.hovered == Some(ControlId::BtnToggleStealthTimer);
    let timer_str = if view.config.stealth_active && !peeking_stealth {
        "••:••".to_string()
    } else {
        view.timer.formatted_time()
    };
    let timer_x = gif_r.x + gif_r.w + 4.0;
    let timer_w = if is_compact { 40.0 } else { 48.0 };
    let timer_r = RectF {
        x: timer_x,
        y: capsule.y + 4.0,
        w: timer_w,
        h: capsule.h - 8.0,
    };
    let phase_color = if view.timer.is_paused() {
        rgba(234, 179, 8, 0.98)
    } else {
        match view.timer.phase() {
            SessionPhase::Focusing => rgba(52, 211, 153, 0.98),
            SessionPhase::Resting => rgba(251, 146, 60, 0.98),
            SessionPhase::Idle => rgba(156, 163, 175, 0.80),
        }
    };
    let phase_brush = unsafe { dc.CreateSolidColorBrush(&phase_color, None) }?;
    unsafe {
        dc.FillEllipse(
            &D2D1_ELLIPSE {
                point: pt(timer_x + 1.0, capsule.y + capsule.h * 0.5),
                radiusX: 2.2,
                radiusY: 2.2,
            },
            &phase_brush,
        );
    }
    let timer_font_size = if is_compact { 11.5 } else { 12.5 };
    draw_text_shadow_nowrap(
        dc,
        dwrite,
        &timer_str,
        RectF { x: timer_x + 6.0, y: timer_r.y, w: timer_r.w - 6.0, h: timer_r.h },
        timer_font_size,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    // 4. Middle Track Ticker or Phase Title
    let btn_s = if is_compact { 16.0 } else { 18.0 };
    let btn_gap = if is_compact { 2.0 } else { 3.0 };
    let btn_margin = if is_compact { 5.0 } else { 8.0 };
    let close_x = capsule.x + capsule.w - btn_margin - btn_s;
    let pop_x = close_x - btn_gap - btn_s;
    let style_x = pop_x - btn_gap - btn_s;
    let pp_x = style_x - btn_gap - btn_s;

    let ticker_x = timer_x + timer_r.w + 4.0;
    let ticker_w = (pp_x - 4.0 - ticker_x).max(0.0);
    let ticker_r = RectF {
        x: ticker_x,
        y: capsule.y + 4.0,
        w: ticker_w,
        h: capsule.h - 8.0,
    };

    if ticker_w > 12.0 {
        if let Some(track) = view.track {
            let ticker_text = format!("♪ {}", track.ticker_text());
            draw_marquee_text(
                dc,
                dwrite,
                &ticker_text,
                ticker_r,
                if is_compact { 9.5 } else { 10.0 },
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                hex_color(view.effective_accent, 0.95),
                view.marquee_offset,
            )?;
        } else {
            let phase_label = if view.timer.is_paused() {
                "Paused"
            } else {
                match view.timer.phase() {
                    SessionPhase::Idle => "FocusGrow",
                    SessionPhase::Focusing => "Focus Session",
                    SessionPhase::Resting => "Rest & Relax",
                }
            };
            draw_text_shadow_nowrap(
                dc,
                dwrite,
                phase_label,
                ticker_r,
                if is_compact { 9.5 } else { 10.0 },
                DWRITE_FONT_WEIGHT_MEDIUM,
                rgba(200, 200, 212, 0.85),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        }
    }

    // 5. Right Action Buttons on the Capsule
    let play_icon = if view.timer.is_running() {
        VectorIcon::PauseBars
    } else {
        VectorIcon::PlayTri
    };
    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnPlayPause),
        play_icon,
        view.hovered == Some(ControlId::BtnPlayPause),
        view.timer.is_running(),
        view.effective_accent,
    )?;

    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnTogglePipStyle),
        VectorIcon::CapsulePill,
        view.hovered == Some(ControlId::BtnTogglePipStyle),
        false,
        "#93C5FD",
    )?;

    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnTogglePip),
        VectorIcon::PopOut,
        view.hovered == Some(ControlId::BtnTogglePip),
        false,
        view.effective_accent,
    )?;

    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnClose),
        VectorIcon::CloseSoft,
        view.hovered == Some(ControlId::BtnClose),
        false,
        "#D4D4D8",
    )?;

    // 6. Floating Glass Under-Bar for Music Controls (only rendered when an active track is detected)
    if view.track.is_some() {
        let under_h = 24.0;
        let under_y = capsule.y + capsule.h + 3.0;
        let under_w = 136.0f32.min(capsule.w - 12.0).max(60.0);
        let under_x = capsule.x + (capsule.w - under_w) * 0.5;
        let under_rect = RectF {
            x: under_x,
            y: under_y,
            w: under_w,
            h: under_h,
        };

        draw_rounded_card(
            dc,
            under_rect,
            under_h * 0.5,
            rgba(14, 14, 18, 0.82),
            rgba(255, 255, 255, 0.12),
            0.8,
        )?;

        let btn_bg = rgba(255, 255, 255, 0.08);
        let btn_border = rgba(255, 255, 255, 0.16);
        let btn_fg = rgba(255, 255, 255, 0.92);

        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnMediaPrev),
            VectorIcon::SkipPrev,
            btn_bg,
            btn_border,
            btn_fg,
            view.hovered == Some(ControlId::BtnMediaPrev),
        )?;

        let pp_icon = if view.playback == PlaybackState::Playing {
            VectorIcon::PauseBars
        } else {
            VectorIcon::PlayTri
        };
        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnMediaPlayPause),
            pp_icon,
            hex_color(view.effective_accent, 0.85),
            rgba(0, 0, 0, 0.0),
            contrast_fg(view.effective_accent),
            view.hovered == Some(ControlId::BtnMediaPlayPause),
        )?;

        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnMediaNext),
            VectorIcon::SkipNext,
            btn_bg,
            btn_border,
            btn_fg,
            view.hovered == Some(ControlId::BtnMediaNext),
        )?;

        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnMediaToggleWindow),
            VectorIcon::Headphones,
            btn_bg,
            btn_border,
            btn_fg,
            view.hovered == Some(ControlId::BtnMediaToggleWindow),
        )?;
    }

    Ok(())
}

fn render_dashboard_ui(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let card = layout.card_rect;
    draw_rounded_card(dc, card, layout.card_radius, rgba(24, 24, 24, 0.99), rgba(255, 255, 255, 0.08), 1.0)?;

    // Titlebar real app icon + FocusGrow title
    if let Some(logo) = view.app_icon_logo {
        let icon_r = RectF { x: card.x + 14.0, y: card.y + 9.0, w: 18.0, h: 18.0 };
        unsafe {
            dc.DrawBitmap(logo, Some(&icon_r.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
        }
    } else {
        let dot_brush = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, 0.95), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(card.x + 22.0, card.y + 18.0), radiusX: 5.5, radiusY: 5.5 }, &dot_brush);
        }
    }
    draw_text_nowrap(
        dc,
        dwrite,
        "FocusGrow",
        RectF { x: card.x + 38.0, y: card.y + 7.0, w: 120.0, h: 22.0 },
        13.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(244, 244, 248, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_vector_pill(dc, find_hit(layout, ControlId::BtnMinimize), VectorIcon::Minimize, view.hovered == Some(ControlId::BtnMinimize), false, view.effective_accent)?;
    draw_vector_pill(dc, find_hit(layout, ControlId::BtnMaximizeRestore), VectorIcon::Maximize, view.hovered == Some(ControlId::BtnMaximizeRestore), false, view.effective_accent)?;
    draw_vector_pill(dc, find_hit(layout, ControlId::BtnClose), VectorIcon::Close, view.hovered == Some(ControlId::BtnClose), false, "#EF4444")?;

    // Left Card
    if let Some(left) = layout.left_card_rect {
        draw_rounded_card(dc, left, 12.0, rgba(36, 36, 38, 0.96), rgba(255, 255, 255, 0.08), 1.0)?;
        draw_backdrop_media(dc, factory, left, 12.0, layout, view, false)?;
        draw_left_focus_card(dc, factory, dwrite, left, layout, view)?;
    }

    // Right Top & Bottom Cards
    if let Some(left) = layout.left_card_rect {
        let rx = left.x + left.w + 14.0;
        let rw = (card.x + card.w - 16.0 - rx).max(320.0);
        let top_card = RectF { x: rx, y: left.y, w: rw, h: 228.0 };
        draw_rounded_card(dc, top_card, 12.0, rgba(36, 36, 38, 0.96), rgba(255, 255, 255, 0.08), 1.0)?;
        draw_daily_progress_card(dc, factory, dwrite, top_card, layout, view)?;

        let bot_y = top_card.y + top_card.h + 14.0;
        let bot_card = RectF {
            x: rx,
            y: bot_y,
            w: rw,
            h: (left.y + left.h - bot_y).max(210.0),
        };
        draw_rounded_card(dc, bot_card, 12.0, rgba(36, 36, 38, 0.96), rgba(255, 255, 255, 0.08), 1.0)?;
        draw_right_guard_card(dc, factory, dwrite, bot_card, layout, view)?;
    }

    // Optional Settings & Preferences, Soundscape Mixer, Edit Goal, or Focus Statistics modal
    if let (Some(tab), Some(mr)) = (view.settings_tab, layout.modal_rect) {
        draw_settings_modal(dc, factory, dwrite, card, mr, layout, view, tab)?;
    } else if view.show_ambient_mixer {
        if let Some(mr) = layout.modal_rect {
            draw_ambient_mixer_modal(dc, factory, dwrite, card, mr, layout, view)?;
        }
    } else if view.show_goal_modal {
        if let Some(mr) = layout.modal_rect {
            draw_goal_modal(dc, dwrite, card, mr, layout, view)?;
        }
    } else if view.show_share_modal {
        if let Some(mr) = layout.modal_rect {
            draw_share_modal(dc, factory, dwrite, card, mr, layout, view)?;
        }
    } else if view.show_stats_modal {
        if let Some(mr) = layout.modal_rect {
            draw_stats_modal(dc, factory, dwrite, card, mr, layout, view)?;
        }
    } else if view.show_prayer_modal {
        if let Some(mr) = layout.modal_rect {
            draw_prayer_modal(dc, dwrite, card, mr, layout, view)?;
        }
    }

    Ok(())
}

fn draw_left_focus_card(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    left: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let header = match view.timer.phase() {
        SessionPhase::Idle => "Get ready to focus".to_string(),
        SessionPhase::Focusing => view.timer.period_header_label(),
        SessionPhase::Resting => {
            if view.config.is_health_mode {
                "Posture Break".to_string()
            } else {
                "Mandatory Break".to_string()
            }
        }
    };
    draw_text_shadow_nowrap(
        dc,
        dwrite,
        &header,
        RectF { x: left.x + 16.0, y: left.y + 13.0, w: left.w - 56.0, h: 24.0 },
        16.5,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_vector_pill(
        dc,
        find_hit(layout, ControlId::BtnTogglePip),
        VectorIcon::PopOut,
        view.hovered == Some(ControlId::BtnTogglePip),
        false,
        view.effective_accent,
    )?;

    if view.timer.phase() == SessionPhase::Idle {
        let lx = left.x + 16.0;
        let lw = left.w - 32.0;
        let mode_track = RectF { x: lx, y: left.y + 48.0, w: lw, h: 34.0 };
        draw_rounded_card(dc, mode_track, 9.0, rgba(22, 22, 25, 0.88), rgba(255, 255, 255, 0.08), 1.0)?;

        draw_neutral_segment_button(
            dc,
            Some(factory),
            dwrite,
            find_hit(layout, ControlId::BtnModeFocus),
            "Productive Focus",
            Some(VectorIcon::Target),
            !view.config.is_health_mode,
            view.hovered == Some(ControlId::BtnModeFocus),
        )?;
        draw_neutral_segment_button(
            dc,
            Some(factory),
            dwrite,
            find_hit(layout, ControlId::BtnModeHealth),
            "Posture Guard",
            Some(VectorIcon::Person),
            view.config.is_health_mode,
            view.hovered == Some(ControlId::BtnModeHealth),
        )?;

        let sub = if view.config.is_health_mode {
            "Periodic posture, eye & hydration reminders keep your body energized while working."
        } else {
            "We'll block unallowed applications during each session. For longer sessions, we'll enforce short breaks so you can recharge."
        };
        draw_text_shadow(
            dc,
            dwrite,
            sub,
            RectF { x: lx, y: mode_track.y + 38.0, w: lw, h: 42.0 },
            10.8,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(218, 218, 226, 0.92),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;

        let ticker_reserve = if let Some(track) = view.track {
            let pill = RectF { x: lx, y: mode_track.y + 84.0, w: lw, h: 28.0 };
            draw_music_ticker_pill(dc, factory, dwrite, pill, layout, view, track)?;
            36.0
        } else {
            0.0
        };

        // Big inset stepper box matching main_dashboard.png ("360 mins focus")
        let step_y = mode_track.y + 84.0 + ticker_reserve;
        let step_h = if view.track.is_some() && left.h < 515.0 { 78.0 } else { 88.0 };
        let step_box = RectF { x: lx, y: step_y, w: lw, h: step_h };
        draw_rounded_card(dc, step_box, 10.0, rgba(30, 30, 34, 0.84), rgba(255, 255, 255, 0.10), 1.0)?;

        let editing_total = view.active_num_edit.filter(|(f, _)| *f == NumericEditField::TotalMins);
        let total_text = if let Some((_, buf)) = editing_total {
            buf.to_string()
        } else {
            view.config.total_focus_minutes.to_string()
        };
        if let Some(in_r) = find_hit(layout, ControlId::InputTotalMins) {
            if editing_total.is_some() {
                draw_rounded_card(
                    dc,
                    in_r,
                    8.0,
                    rgba(10, 12, 16, 0.55),
                    hex_color(view.effective_accent, 0.85),
                    1.3,
                )?;
            } else if view.hovered == Some(ControlId::InputTotalMins) {
                draw_rounded_card(
                    dc,
                    in_r,
                    8.0,
                    rgba(255, 255, 255, 0.06),
                    rgba(255, 255, 255, 0.16),
                    1.0,
                )?;
            }
            let font_sz = if total_text.len() >= 4 { 38.0 } else if step_h < 82.0 { 42.0 } else { 46.0 };
            let shown = if editing_total.is_some() {
                format!("{total_text}|")
            } else {
                total_text
            };
            draw_text_nowrap(
                dc,
                dwrite,
                &shown,
                in_r,
                font_sz,
                DWRITE_FONT_WEIGHT_BOLD,
                rgba(255, 255, 255, 0.98),
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;
            draw_text_nowrap(
                dc,
                dwrite,
                if view.config.is_health_mode { "mins guard" } else { "mins focus" },
                RectF {
                    x: in_r.x + in_r.w + 8.0,
                    y: in_r.y + in_r.h * 0.30,
                    w: (step_box.w - in_r.w - 78.0).max(56.0),
                    h: 26.0,
                },
                12.5,
                DWRITE_FONT_WEIGHT_MEDIUM,
                rgba(195, 195, 206, 0.88),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        }

        draw_vector_button(dc, find_hit(layout, ControlId::BtnTotalUp), VectorIcon::ChevronUp, view.hovered == Some(ControlId::BtnTotalUp))?;
        draw_vector_button(dc, find_hit(layout, ControlId::BtnTotalDown), VectorIcon::ChevronDown, view.hovered == Some(ControlId::BtnTotalDown))?;

        // Inline duration rows matching main_dashboard.png
        let row1_y = step_box.y + step_box.h + 10.0;
        let editing_period = view.active_num_edit.filter(|(f, _)| *f == NumericEditField::PeriodMins);
        let period_text = if let Some((_, buf)) = editing_period {
            format!("{buf}|")
        } else {
            view.config.focus_minutes.to_string()
        };
        draw_accent_stepper_row(
            dc,
            dwrite,
            lx,
            row1_y,
            lw,
            "Focus period length",
            &period_text,
            find_hit(layout, ControlId::BtnChunkDown),
            find_hit(layout, ControlId::InputPeriodMins),
            find_hit(layout, ControlId::BtnChunkUp),
            view.hovered,
            ControlId::BtnChunkDown,
            ControlId::InputPeriodMins,
            ControlId::BtnChunkUp,
            editing_period.is_some(),
            view.effective_accent,
        )?;

        let row2_y = row1_y + 40.0;
        let editing_break = view.active_num_edit.filter(|(f, _)| *f == NumericEditField::BreakMins);
        let break_text = if let Some((_, buf)) = editing_break {
            format!("{buf}|")
        } else {
            view.config.break_minutes.to_string()
        };
        draw_accent_stepper_row(
            dc,
            dwrite,
            lx,
            row2_y,
            lw,
            "Break duration",
            &break_text,
            find_hit(layout, ControlId::BtnBreakDown),
            find_hit(layout, ControlId::InputBreakMins),
            find_hit(layout, ControlId::BtnBreakUp),
            view.hovered,
            ControlId::BtnBreakDown,
            ControlId::InputBreakMins,
            ControlId::BtnBreakUp,
            editing_break.is_some(),
            view.effective_accent,
        )?;

        let break_cnt = view.timer.planned_break_count();
        let periods = view.timer.planned_periods();
        let summary_1 = if view.config.skip_breaks || break_cnt == 0 {
            format!("Continuous focus ({periods} periods, no breaks)")
        } else {
            format!(
                "You'll have {break_cnt} breaks ({} mins each) across {periods} focus periods ({} mins each)",
                view.config.break_minutes, view.config.focus_minutes
            )
        };
        let total_str = FocusTimer::formatted_total_duration(view.timer.total_remaining_sec());

        let sum_y = row2_y + 41.0;
        draw_text_shadow(
            dc,
            dwrite,
            &summary_1,
            RectF { x: lx + 4.0, y: sum_y, w: lw - 8.0, h: 30.0 },
            10.5,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(208, 208, 218, 0.90),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        // Measured centered pill: adaptive icon & colors based on finish time of day (Morning/Afternoon/Night)
        let end_y = sum_y + 33.0;
        let pre_str = "Ends at ";
        let mid_str = view.ends_at_text;
        let post_str = format!(" • {total_str} total");
        let w_pre = measure_text_width(dwrite, pre_str, 10.6, DWRITE_FONT_WEIGHT_NORMAL);
        let w_mid = measure_text_width(dwrite, mid_str, 10.6, DWRITE_FONT_WEIGHT_BOLD);
        let w_post = measure_text_width(dwrite, &post_str, 10.6, DWRITE_FONT_WEIGHT_NORMAL);
        let icon_w = 15.0;
        let total_content_w = icon_w + w_pre + w_mid + w_post;
        let pill_w = (total_content_w + 26.0).clamp(170.0, lw);
        let (icon, icon_col, pill_bg, pill_border, mid_col) = match view.ends_at_period {
            DayPeriod::Morning => (
                VectorIcon::Sunrise,
                rgba(249, 115, 22, 0.98), // Sunrise Warm Coral/Amber #f97316
                rgba(245, 158, 11, 0.12),
                rgba(245, 158, 11, 0.32),
                rgba(251, 191, 36, 0.98), // Golden Sunrise #fbbf24
            ),
            DayPeriod::Afternoon => (
                VectorIcon::Sun,
                rgba(234, 179, 8, 0.98), // Radiant Sun Gold #eab308
                rgba(234, 179, 8, 0.12),
                rgba(234, 179, 8, 0.32),
                rgba(250, 204, 21, 0.98), // Vivid Sunshine Yellow #facc15
            ),
            DayPeriod::Night => (
                VectorIcon::Moon,
                rgba(192, 132, 252, 0.98), // Lavender/purple #c084fc
                rgba(168, 85, 247, 0.14),
                rgba(168, 85, 247, 0.35),
                rgba(216, 180, 254, 0.98), // Soft violet #d8b4fe
            ),
        };
        let pill_r = RectF { x: lx + (lw - pill_w) * 0.5, y: end_y - 1.0, w: pill_w, h: 22.0 };
        draw_rounded_card(dc, pill_r, 11.0, pill_bg, pill_border, 1.0)?;

        let mut cx_t = pill_r.x + (pill_r.w - total_content_w) * 0.5;
        draw_vector_icon(
            dc,
            Some(factory),
            RectF { x: cx_t - 1.0, y: pill_r.y + 4.5, w: 13.0, h: 13.0 },
            icon,
            icon_col,
        )?;
        cx_t += icon_w;
        draw_text_nowrap(
            dc,
            dwrite,
            pre_str,
            RectF { x: cx_t, y: pill_r.y, w: w_pre + 2.0, h: pill_r.h },
            10.6,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(205, 205, 215, 0.90),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        cx_t += w_pre;
        draw_text_nowrap(
            dc,
            dwrite,
            mid_str,
            RectF { x: cx_t, y: pill_r.y - 0.8, w: w_mid + 2.0, h: pill_r.h },
            10.6,
            DWRITE_FONT_WEIGHT_BOLD,
            mid_col,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        cx_t += w_mid;
        draw_text_nowrap(
            dc,
            dwrite,
            &post_str,
            RectF { x: cx_t, y: pill_r.y, w: w_post + 4.0, h: pill_r.h },
            10.6,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(205, 205, 215, 0.90),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;

        // Centered Skip breaks checkbox matching main_dashboard.png
        if let Some(chk_rect) = find_hit(layout, ControlId::BtnToggleSkipBreaks) {
            let box_r = RectF { x: chk_rect.x + 10.0, y: chk_rect.y + 3.0, w: 18.0, h: 18.0 };
            let box_fill = if view.config.skip_breaks {
                hex_color(view.effective_accent, 0.95)
            } else {
                rgba(46, 46, 52, 0.9)
            };
            draw_rounded_card(dc, box_r, 5.0, box_fill, rgba(255, 255, 255, 0.25), 1.0)?;
            if view.config.skip_breaks {
                draw_text_nowrap(
                    dc,
                    dwrite,
                    "✓",
                    box_r,
                    12.0,
                    DWRITE_FONT_WEIGHT_BOLD,
                    contrast_fg(view.effective_accent),
                    DWRITE_TEXT_ALIGNMENT_CENTER,
                )?;
            }
            draw_text_nowrap(
                dc,
                dwrite,
                "Skip breaks",
                RectF { x: box_r.x + 26.0, y: chk_rect.y, w: chk_rect.w - 30.0, h: chk_rect.h },
                12.5,
                DWRITE_FONT_WEIGHT_MEDIUM,
                rgba(225, 225, 232, 0.92),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        }

        if let Some(cta) = find_hit(layout, ControlId::BtnPlayPause) {
            let hover_alpha = if view.hovered == Some(ControlId::BtnPlayPause) { 1.0 } else { 0.94 };
            draw_rounded_card(
                dc,
                cta,
                10.0,
                hex_color(view.effective_accent, hover_alpha),
                rgba(255, 255, 255, 0.14),
                1.0,
            )?;
            draw_text_nowrap(
                dc,
                dwrite,
                "▶   Start focus session",
                cta,
                13.5,
                DWRITE_FONT_WEIGHT_BOLD,
                contrast_fg(view.effective_accent),
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;
        }
    } else {
        draw_focus_ring(dc, factory, dwrite, layout, view, false)?;

        let play_icon = if view.timer.is_running() { VectorIcon::PauseBars } else { VectorIcon::PlayTri };
        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnPlayPause),
            play_icon,
            hex_color(view.effective_accent, 0.96),
            rgba(0, 0, 0, 0.0),
            contrast_fg(view.effective_accent),
            view.hovered == Some(ControlId::BtnPlayPause),
        )?;
        draw_icon_circle_button(
            dc,
            factory,
            find_hit(layout, ControlId::BtnReset),
            VectorIcon::StopSquare,
            rgba(48, 48, 54, 0.94),
            rgba(255, 255, 255, 0.14),
            rgba(244, 244, 248, 0.95),
            view.hovered == Some(ControlId::BtnReset),
        )?;

        if let Some(pp_r) = find_hit(layout, ControlId::BtnPlayPause) {
            let sub_line = format_timer_subtitle(view);
            let sub_col = if view.hovered == Some(ControlId::BtnCycleTimerSubtitle) {
                hex_color(view.effective_accent, 0.98)
            } else {
                rgba(225, 225, 235, 0.90)
            };
            draw_text_shadow_nowrap(
                dc,
                dwrite,
                &sub_line,
                RectF {
                    x: left.x + 12.0,
                    y: pp_r.y + pp_r.h + 10.0,
                    w: left.w - 24.0,
                    h: 20.0,
                },
                11.5,
                DWRITE_FONT_WEIGHT_MEDIUM,
                sub_col,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;
        }
        if let Some(track) = view.track {
            let pill = RectF {
                x: left.x + 12.0,
                y: left.y + left.h - 40.0,
                w: (left.w - 24.0).max(140.0),
                h: 28.0,
            };
            draw_music_ticker_pill(dc, factory, dwrite, pill, layout, view, track)?;
        }
    }

    Ok(())
}

fn draw_daily_progress_card(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    top_card: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_text_nowrap(
        dc,
        dwrite,
        "Daily progress",
        RectF { x: top_card.x + 16.0, y: top_card.y + 13.0, w: 180.0, h: 22.0 },
        15.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(245, 245, 248, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    let sound_active = view.show_ambient_mixer || (view.ambient_playing && !view.ambient_muted);
    draw_vector_pill_fx(dc, factory, find_hit(layout, ControlId::BtnTopToolbarSound), VectorIcon::Headphones, view.hovered == Some(ControlId::BtnTopToolbarSound), sound_active, view.effective_accent)?;
    draw_vector_pill_fx(dc, factory, find_hit(layout, ControlId::BtnTopToolbarMusic), VectorIcon::Smartphone, view.hovered == Some(ControlId::BtnTopToolbarMusic), view.settings_tab == Some(SettingsTab::MobileSync), "#10B981")?;
    draw_vector_pill_fx(dc, factory, find_hit(layout, ControlId::BtnTopToolbarPip), VectorIcon::PipPop, view.hovered == Some(ControlId::BtnTopToolbarPip), view.config.auto_pip_on_blur, "#38BDF8")?;
    if let Some(gear_r) = find_hit(layout, ControlId::BtnTopToolbarAppearance) {
        let gear_active = view.settings_tab == Some(SettingsTab::Appearance);
        let gear_hov = view.hovered == Some(ControlId::BtnTopToolbarAppearance);
        let gear_deg = if gear_active { 45.0 } else if gear_hov { 22.0 } else { 0.0 };
        draw_rotated_pill_fx(dc, factory, gear_r, VectorIcon::Gear, gear_hov, gear_active, view.effective_accent, gear_deg)?;
    }
    draw_vector_pill_fx(dc, factory, find_hit(layout, ControlId::BtnOpenStats), VectorIcon::BarChart, view.hovered == Some(ControlId::BtnOpenStats), view.show_stats_modal, view.effective_accent)?;
    draw_vector_pill_fx(dc, factory, find_hit(layout, ControlId::BtnEditGoal), VectorIcon::Pencil, view.hovered == Some(ControlId::BtnEditGoal), view.show_goal_modal, view.effective_accent)?;

    let cy = top_card.y + 84.0;
    let col_w = (top_card.w - 32.0) / 3.0;
    let left_x = top_card.x + 16.0;
    let mid_x = left_x + col_w;
    let right_x = mid_x + col_w;

    // Yesterday stat
    let (yest_val, yest_unit) = if view.yesterday_mins >= 60 {
        (format!("{:.1}", view.yesterday_mins as f32 / 60.0), "hours")
    } else {
        (view.yesterday_mins.to_string(), "minutes")
    };
    draw_text_nowrap(dc, dwrite, "Yesterday", RectF { x: left_x, y: cy - 28.0, w: col_w, h: 18.0 }, 11.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.9), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, &yest_val, RectF { x: left_x, y: cy - 8.0, w: col_w, h: 32.0 }, 24.0, DWRITE_FONT_WEIGHT_BOLD, rgba(245, 245, 248, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, yest_unit, RectF { x: left_x, y: cy + 24.0, w: col_w, h: 16.0 }, 11.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.85), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    // Center Daily Goal ring (clickable to open Edit Goal) + Dual-Layer Multi-Tier Lap System
    let goal_cx = mid_x + col_w * 0.5;
    let goal_r = 46.0f32;
    let goal_hover = view.hovered == Some(ControlId::BtnEditGoal);
    let goal_mins = view.config.daily_goal_minutes.max(1);
    let daily_ratio = view.daily_completed_mins as f32 / goal_mins as f32;

    const LAP_COLORS: [&str; 5] = [
        "#22C55E", // Lap 2 (1.0x..2.0x): Emerald Green
        "#EC4899", // Lap 3 (2.0x..3.0x): Neon Pink
        "#EAB308", // Lap 4 (3.0x..4.0x): Amber Gold
        "#EF4444", // Lap 5 (4.0x..5.0x): Crimson Flame
        "#38BDF8", // Lap 6+ (5.0x+): Electric Blue
    ];

    let (track_col, fill_col, goal_prog, mult_badge) = if daily_ratio < 1.0 {
        let base = if goal_hover { rgba(78, 78, 90, 0.92) } else { rgba(60, 60, 68, 0.85) };
        let fill = hex_color(view.effective_accent, 0.92);
        let prog = daily_ratio.max(view.timer.elapsed_fraction() * 0.15).clamp(0.03, 1.0);
        (base, fill, prog, None)
    } else {
        let full_laps = daily_ratio.floor() as usize;
        let lap_frac = daily_ratio - full_laps as f32;
        let tier_idx = (full_laps - 1) % LAP_COLORS.len();
        let active_hex = LAP_COLORS[tier_idx];
        let base_hex = if full_laps == 1 {
            view.effective_accent
        } else {
            LAP_COLORS[(full_laps - 2) % LAP_COLORS.len()]
        };
        let fill_ratio = if lap_frac < 0.005 { 1.0 } else { lap_frac.clamp(0.03, 1.0) };
        let badge_txt = if daily_ratio >= 10.0 {
            format!("{:.0}x", daily_ratio)
        } else {
            format!("{:.1}x", daily_ratio)
        };
        (
            hex_color(base_hex, if goal_hover { 0.82 } else { 0.62 }),
            hex_color(active_hex, 0.98),
            fill_ratio,
            Some((badge_txt, active_hex)),
        )
    };

    let track_brush = unsafe { dc.CreateSolidColorBrush(&track_col, None) }?;
    unsafe {
        dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(goal_cx, cy), radiusX: goal_r, radiusY: goal_r }, &track_brush, 7.5, None);
    }
    draw_progress_arc(dc, factory, goal_cx, cy, goal_r, 7.5, goal_prog, fill_col)?;
    let (goal_num, goal_unit) = if goal_mins >= 60 {
        (format!("{}", goal_mins / 60), "hour".to_string())
    } else {
        (format!("{}", goal_mins), "mins".to_string())
    };
    draw_text_nowrap(dc, dwrite, "Daily goal", RectF { x: mid_x, y: cy - 26.0, w: col_w, h: 15.0 }, 10.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(175, 175, 184, 0.9), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, &goal_num, RectF { x: mid_x, y: cy - 9.0, w: col_w, h: 24.0 }, 20.0, DWRITE_FONT_WEIGHT_BOLD, rgba(245, 245, 248, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, &goal_unit, RectF { x: mid_x, y: cy + 14.0, w: col_w, h: 14.0 }, 10.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(175, 175, 184, 0.9), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    if let Some((badge_txt, badge_hex)) = mult_badge {
        let br = RectF { x: goal_cx - 18.0, y: cy + goal_r - 9.0, w: 36.0, h: 16.0 };
        draw_rounded_card(dc, br, 8.0, rgba(16, 17, 22, 0.96), hex_color(badge_hex, 0.92), 1.1)?;
        draw_text_nowrap(dc, dwrite, &badge_txt, br, 9.2, DWRITE_FONT_WEIGHT_BOLD, hex_color(badge_hex, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }

    // Streak stat (+ emerald Shield badge when posture goal shielded)
    let is_shielded = view.stats.is_shielded_today();
    if is_shielded {
        let tw = measure_text_width(dwrite, "Streak", 11.5, DWRITE_FONT_WEIGHT_NORMAL);
        let sx = right_x + (col_w - (tw + 15.0)) * 0.5;
        draw_text_nowrap(dc, dwrite, "Streak", RectF { x: sx, y: cy - 28.0, w: tw + 2.0, h: 18.0 }, 11.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
        draw_vector_icon(dc, Some(factory), RectF { x: sx + tw + 3.0, y: cy - 25.0, w: 12.0, h: 12.0 }, VectorIcon::Shield, rgba(52, 211, 153, 0.96))?;
    } else {
        draw_text_nowrap(dc, dwrite, "Streak", RectF { x: right_x, y: cy - 28.0, w: col_w, h: 18.0 }, 11.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.9), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    draw_text_nowrap(dc, dwrite, &view.streak_days.to_string(), RectF { x: right_x, y: cy - 8.0, w: col_w, h: 32.0 }, 24.0, DWRITE_FONT_WEIGHT_BOLD, rgba(245, 245, 248, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(
        dc,
        dwrite,
        if is_shielded { "days (shielded)" } else { "days" },
        RectF { x: right_x, y: cy + 24.0, w: col_w, h: 16.0 },
        if is_shielded { 10.5 } else { 11.5 },
        DWRITE_FONT_WEIGHT_NORMAL,
        if is_shielded { rgba(52, 211, 153, 0.92) } else { rgba(161, 161, 170, 0.85) },
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;

    // Completed status line matching main_dashboard.png ("Completed: <accent>value</accent> ↻")
    let comp_pre = "Completed: ";
    let comp_val = format_completed_value(view.daily_completed_mins, &view.config.completed_time_format);
    let w_cp = measure_text_width(dwrite, comp_pre, 11.5, DWRITE_FONT_WEIGHT_NORMAL);
    let w_cv = measure_text_width(dwrite, &comp_val, 11.5, DWRITE_FONT_WEIGHT_BOLD);
    let icon_gap = 15.0;
    let comp_x = top_card.x + (top_card.w - (w_cp + w_cv + icon_gap)) * 0.5;
    let comp_hover = view.hovered == Some(ControlId::BtnToggleCompletedFormat);
    draw_text_nowrap(
        dc,
        dwrite,
        comp_pre,
        RectF { x: comp_x, y: top_card.y + 146.0, w: w_cp + 2.0, h: 18.0 },
        11.5,
        DWRITE_FONT_WEIGHT_NORMAL,
        if comp_hover { rgba(220, 220, 232, 0.98) } else { rgba(175, 175, 185, 0.9) },
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        &comp_val,
        RectF { x: comp_x + w_cp, y: top_card.y + 146.0, w: w_cv + 4.0, h: 18.0 },
        11.5,
        DWRITE_FONT_WEIGHT_BOLD,
        hex_color(view.effective_accent, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_rotated_vector_icon(
        dc,
        Some(factory),
        RectF { x: comp_x + w_cp + w_cv + 4.0, y: top_card.y + 149.0, w: 12.0, h: 12.0 },
        VectorIcon::Refresh,
        if comp_hover { hex_color(view.effective_accent, 0.98) } else { rgba(145, 145, 158, 0.85) },
        view.completed_spin_deg,
    )?;

    // Bottom Posture Habit green bar matching main_dashboard.png & SHIELD ACTIVE badge
    let habit_r = RectF {
        x: top_card.x + 16.0,
        y: top_card.y + top_card.h - 52.0,
        w: top_card.w - 32.0,
        h: 38.0,
    };
    draw_rounded_card(dc, habit_r, 9.0, rgba(16, 185, 129, 0.08), rgba(16, 185, 129, 0.25), 1.0)?;
    draw_vector_icon(
        dc,
        Some(factory),
        RectF { x: habit_r.x + 11.0, y: habit_r.y + 12.0, w: 14.0, h: 14.0 },
        VectorIcon::Person,
        rgba(52, 211, 153, 0.96),
    )?;
    let habit_title_x = habit_r.x + 30.0;
    let habit_title_w = measure_text_width(dwrite, "Posture Habit", 11.5, DWRITE_FONT_WEIGHT_SEMI_BOLD);
    draw_text_nowrap(
        dc,
        dwrite,
        "Posture Habit",
        RectF { x: habit_title_x, y: habit_r.y + 4.0, w: habit_title_w + 4.0, h: 16.0 },
        11.5,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(235, 248, 242, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    if is_shielded {
        let shield_pill = RectF {
            x: habit_title_x + habit_title_w + 8.0,
            y: habit_r.y + 4.5,
            w: 78.0,
            h: 15.0,
        };
        draw_rounded_card(dc, shield_pill, 4.0, rgba(16, 185, 129, 0.16), rgba(16, 185, 129, 0.45), 1.0)?;
        draw_text_nowrap(
            dc,
            dwrite,
            "SHIELD ACTIVE",
            shield_pill,
            8.2,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(52, 211, 153, 0.98),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    let stands_done = view.stats.health_completed_breaks_today;
    let stands_goal = view.stats.health_daily_goal_breaks.max(1);
    let habit_sub = if !view.next_prayer_label.is_empty() {
        format!("{stands_done} / {stands_goal} stands today  •  {}", view.next_prayer_label)
    } else {
        format!("{stands_done} / {stands_goal} stands today")
    };
    draw_text_nowrap(
        dc,
        dwrite,
        &habit_sub,
        RectF { x: habit_title_x, y: habit_r.y + 19.0, w: habit_r.w - 118.0, h: 14.0 },
        10.0,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(156, 175, 168, 0.9),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    let badge_r = RectF { x: habit_r.x + habit_r.w - 82.0, y: habit_r.y + 8.0, w: 70.0, h: 22.0 };
    draw_rounded_card(dc, badge_r, 11.0, rgba(16, 185, 129, 0.16), rgba(16, 185, 129, 0.45), 1.0)?;
    draw_vector_icon(
        dc,
        Some(factory),
        RectF { x: badge_r.x + 8.0, y: badge_r.y + 5.0, w: 12.0, h: 12.0 },
        VectorIcon::Shield,
        rgba(52, 211, 153, 0.96),
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("{} days", view.stats.health_streak_days),
        RectF { x: badge_r.x + 21.0, y: badge_r.y, w: badge_r.w - 25.0, h: badge_r.h },
        10.2,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(52, 211, 153, 0.96),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;

    Ok(())
}

fn draw_right_guard_card(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    bot_card: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_guard_tab_pill_icon(
        dc,
        Some(factory),
        dwrite,
        find_hit(layout, ControlId::BtnGuardTabApps),
        "Blocked Apps",
        Some(VectorIcon::Ban),
        view.guard_tab == GuardTab::BlockedApps,
        view.hovered == Some(ControlId::BtnGuardTabApps),
        view.effective_accent,
    )?;
    draw_guard_tab_pill_icon(
        dc,
        Some(factory),
        dwrite,
        find_hit(layout, ControlId::BtnGuardTabSites),
        "Restricted Sites",
        Some(VectorIcon::Globe),
        view.guard_tab == GuardTab::RestrictedSites,
        view.hovered == Some(ControlId::BtnGuardTabSites),
        view.effective_accent,
    )?;
    draw_guard_tab_pill_icon(
        dc,
        Some(factory),
        dwrite,
        find_hit(layout, ControlId::BtnGuardTabPrayer),
        "Prayer Times",
        Some(VectorIcon::Moon),
        view.guard_tab == GuardTab::PrayerTimes,
        view.hovered == Some(ControlId::BtnGuardTabPrayer),
        view.effective_accent,
    )?;

    // Extension Sync badge + animated spin refresh icon matching main_dashboard.png
    if let Some(ref_r) = find_hit(layout, ControlId::BtnGuardRefreshApps) {
        let sync_r = RectF { x: ref_r.x - 98.0, y: ref_r.y + 2.0, w: 92.0, h: 22.0 };
        let synced = view.bridge_port > 0;
        draw_rounded_card(
            dc,
            sync_r,
            6.0,
            if synced { rgba(16, 185, 129, 0.12) } else { rgba(70, 70, 78, 0.35) },
            if synced { rgba(16, 185, 129, 0.45) } else { rgba(255, 255, 255, 0.12) },
            1.0,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            if synced { "Extension Sync" } else { "Bridge Off" },
            sync_r,
            10.0,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            if synced { rgba(52, 211, 153, 0.96) } else { rgba(161, 161, 170, 0.9) },
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        draw_rotated_pill_fx(
            dc,
            factory,
            ref_r,
            VectorIcon::Refresh,
            view.hovered == Some(ControlId::BtnGuardRefreshApps),
            false,
            view.effective_accent,
            view.guard_refresh_spin_deg,
        )?;
    }

    let content_rect = RectF {
        x: bot_card.x + 16.0,
        y: bot_card.y + 52.0,
        w: bot_card.w - 32.0,
        h: (bot_card.y + bot_card.h - (bot_card.y + 52.0) - 12.0).max(120.0),
    };

    match view.guard_tab {
        GuardTab::BlockedApps => draw_blocked_apps_tab(dc, dwrite, content_rect, layout, view)?,
        GuardTab::RestrictedSites => draw_restricted_sites_tab(dc, dwrite, content_rect, layout, view)?,
        GuardTab::PrayerTimes => draw_prayer_times_tab(dc, dwrite, content_rect, layout, view)?,
    }

    Ok(())
}

fn app_monogram_color(exe_name: &str) -> (D2D1_COLOR_F, D2D1_COLOR_F) {
    let mut hash: u32 = 5381;
    for b in exe_name.bytes() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(b.to_ascii_lowercase() as u32);
    }
    let palette = [
        (rgba(16, 185, 129, 0.28), rgba(16, 185, 129, 0.75)), // Emerald Green
        (rgba(59, 130, 246, 0.28), rgba(59, 130, 246, 0.75)), // Blue
        (rgba(139, 92, 246, 0.28), rgba(139, 92, 246, 0.75)), // Violet
        (rgba(245, 158, 11, 0.28), rgba(245, 158, 11, 0.75)), // Amber
        (rgba(236, 72, 153, 0.28), rgba(236, 72, 153, 0.75)), // Pink
        (rgba(6, 182, 212, 0.28), rgba(6, 182, 212, 0.75)),   // Cyan
        (rgba(99, 102, 241, 0.28), rgba(99, 102, 241, 0.75)), // Indigo
        (rgba(239, 68, 68, 0.28), rgba(239, 68, 68, 0.75)),   // Red
    ];
    palette[(hash as usize) % palette.len()]
}

fn draw_blocked_apps_tab(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    area: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_text_nowrap(
        dc,
        dwrite,
        "Checked applications or browser tabs will be blocked during focus sessions. Unchecked apps stay accessible.",
        RectF { x: area.x, y: area.y, w: area.w, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(161, 161, 170, 0.9),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    // Interactive search box matching main_dashboard.png
    let search_bar = RectF { x: area.x, y: area.y + 20.0, w: area.w, h: 28.0 };
    let search_border = if view.search_focused {
        hex_color(view.effective_accent, 0.78)
    } else if view.hovered == Some(ControlId::BtnAppSearchBox) {
        rgba(255, 255, 255, 0.16)
    } else {
        rgba(255, 255, 255, 0.08)
    };
    draw_rounded_card(dc, search_bar, 7.0, rgba(22, 22, 25, 0.92), search_border, 1.0)?;
    draw_vector_icon(dc, None, RectF { x: search_bar.x + 8.0, y: search_bar.y + 6.0, w: 16.0, h: 16.0 }, VectorIcon::Search, rgba(161, 161, 170, 0.85))?;

    let (search_label, search_col) = if view.search_query.is_empty() {
        (
            if view.search_focused { "Type to filter running apps..." } else { "Search applications or browser tabs..." },
            rgba(135, 135, 146, 0.88),
        )
    } else {
        (view.search_query, rgba(244, 244, 250, 0.96))
    };
    draw_text_nowrap(
        dc,
        dwrite,
        search_label,
        RectF { x: search_bar.x + 30.0, y: search_bar.y, w: (search_bar.w - 60.0).max(60.0), h: search_bar.h },
        11.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        search_col,
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    if !view.search_query.is_empty() {
        draw_vector_pill(dc, find_hit(layout, ControlId::BtnAppSearchClear), VectorIcon::Close, view.hovered == Some(ControlId::BtnAppSearchClear), false, view.effective_accent)?;
    }

    draw_segment_button(dc, dwrite, find_hit(layout, ControlId::BtnFilterAll), "All", view.app_filter == AppFilter::All, view.hovered == Some(ControlId::BtnFilterAll), view.effective_accent)?;
    draw_segment_button(dc, dwrite, find_hit(layout, ControlId::BtnFilterBlocked), "Blocked", view.app_filter == AppFilter::Blocked, view.hovered == Some(ControlId::BtnFilterBlocked), view.effective_accent)?;
    draw_segment_button(dc, dwrite, find_hit(layout, ControlId::BtnFilterAccessible), "Accessible", view.app_filter == AppFilter::Accessible, view.hovered == Some(ControlId::BtnFilterAccessible), view.effective_accent)?;

    // Sort pill on right matching main_dashboard.png ("Sort: Latest Open")
    if let Some(sort_r) = find_hit(layout, ControlId::BtnAppSort) {
        let hovered = view.hovered == Some(ControlId::BtnAppSort);
        draw_rounded_card(
            dc,
            sort_r,
            6.0,
            if hovered { rgba(52, 52, 60, 0.92) } else { rgba(32, 32, 37, 0.92) },
            rgba(255, 255, 255, 0.08),
            1.0,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            &format!("Sort: {} ▾", view.app_sort.label()),
            RectF { x: sort_r.x + 6.0, y: sort_r.y, w: sort_r.w - 12.0, h: sort_r.h },
            10.2,
            DWRITE_FONT_WEIGHT_MEDIUM,
            rgba(210, 210, 220, 0.92),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }

    if let Some(sec) = view.block_countdown_sec {
        draw_text_nowrap(
            dc,
            dwrite,
            &format!("Closing {} in {}s", view.active_exe, sec),
            RectF { x: area.x + 210.0, y: area.y + 58.0, w: (area.w - 350.0).max(40.0), h: 18.0 },
            10.0,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(248, 113, 113, 0.96),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }

    if view.running_apps.is_empty() {
        draw_text_nowrap(
            dc,
            dwrite,
            "No matching applications. Click Refresh or clear search.",
            RectF { x: area.x, y: area.y + 114.0, w: area.w, h: 24.0 },
            11.5,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(155, 155, 166, 0.8),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        return Ok(());
    }

    for (i, app) in view.running_apps.iter().enumerate() {
        let Some(row_r) = find_hit(layout, ControlId::GuardAppToggle(i)) else {
            continue;
        };
        let hovered = view.hovered == Some(ControlId::GuardAppToggle(i));
        let bg = if app.blocked {
            hex_color(view.effective_accent, 0.10)
        } else if hovered {
            rgba(48, 48, 56, 0.92)
        } else {
            rgba(26, 26, 30, 0.92)
        };
        let border = if app.blocked {
            hex_color(view.effective_accent, 0.55)
        } else {
            rgba(255, 255, 255, 0.06)
        };
        draw_rounded_card(dc, row_r, 8.0, bg, border, 1.0)?;

        // Real extracted app HICON bitmap (or rich monogram avatar)
        let icon_r = RectF { x: row_r.x + 10.0, y: row_r.y + (row_r.h - 23.0) * 0.5, w: 23.0, h: 23.0 };
        if let Some(bmp) = app.icon {
            unsafe {
                dc.DrawBitmap(bmp, Some(&icon_r.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
            }
        } else {
            let (tile_bg, tile_border) = app_monogram_color(app.exe_name);
            draw_rounded_card(dc, icon_r, 6.0, tile_bg, tile_border, 1.0)?;
            let ch = app.exe_name.chars().find(|c| c.is_alphanumeric()).unwrap_or('A').to_ascii_uppercase().to_string();
            draw_text_nowrap(dc, dwrite, &ch, RectF { x: icon_r.x, y: icon_r.y - 0.5, w: icon_r.w, h: icon_r.h }, 11.0, DWRITE_FONT_WEIGHT_BOLD, rgba(255, 255, 255, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
        }

        let tx = icon_r.x + 33.0;
        let tw = (row_r.w - 92.0).max(80.0);
        let text_top = row_r.y + (row_r.h - 31.0) * 0.5;
        draw_text_nowrap(
            dc,
            dwrite,
            if app.title.is_empty() { app.exe_name } else { app.title },
            RectF { x: tx, y: text_top, w: tw, h: 16.0 },
            11.2,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(240, 240, 246, 0.96),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            app.exe_name,
            RectF { x: tx, y: text_top + 16.0, w: tw, h: 14.0 },
            9.8,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(148, 148, 160, 0.88),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;

        draw_switch_pill(
            dc,
            RectF { x: row_r.x + row_r.w - 44.0, y: row_r.y + (row_r.h - 18.0) * 0.5, w: 32.0, h: 18.0 },
            app.blocked,
            view.effective_accent,
        )?;
    }

    let cap = blocked_apps_capacity(layout.card_rect.h + 4.0);
    if view.total_matching_apps > cap {
        if let Some(list_r) = layout.guard_list_rect {
            let track = RectF { x: list_r.x + list_r.w - 3.5, y: list_r.y + 2.0, w: 3.5, h: (list_r.h - 4.0).max(20.0) };
            let max_scroll = view.total_matching_apps.saturating_sub(cap).max(1);
            let thumb_h = ((cap as f32 / view.total_matching_apps as f32) * track.h).clamp(24.0, track.h);
            let ratio = (view.app_scroll as f32 / max_scroll as f32).clamp(0.0, 1.0);
            let thumb_y = track.y + (track.h - thumb_h) * ratio;
            draw_rounded_card(dc, RectF { x: track.x, y: thumb_y, w: 3.5, h: thumb_h }, 2.0, rgba(255, 255, 255, 0.24), rgba(0, 0, 0, 0.0), 0.0)?;
        }
    }

    Ok(())
}

pub fn build_site_entries(view: &RenderView<'_>) -> Vec<String> {
    let mut entries: Vec<String> = COMMON_SITE_PRESETS.iter().map(|s| (*s).to_string()).collect();
    for s in &view.config.restricted_sites {
        if !entries.iter().any(|e| e.eq_ignore_ascii_case(s)) {
            entries.push(s.clone());
        }
    }
    entries
}

fn draw_restricted_sites_tab(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    area: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let sub_line = if view.active_domain.is_empty() {
        "Websites checked below are synced to the FocusGrow browser extension (:8766).".to_string()
    } else {
        format!("Browser tab: {} (emergency passes active: {})", view.active_domain, view.active_pass_count)
    };
    draw_text_nowrap(
        dc,
        dwrite,
        &sub_line,
        RectF { x: area.x, y: area.y, w: area.w, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(161, 161, 170, 0.9),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    // 1. Add Domain Input Bar + Button
    let bar_y = area.y + 18.0;
    let bar_h = 28.0;
    let add_btn_w = 68.0;
    let input_w = area.w - add_btn_w - 8.0;
    let input_r = RectF { x: area.x, y: bar_y, w: input_w, h: bar_h };
    let add_r = RectF { x: area.x + input_w + 8.0, y: bar_y, w: add_btn_w, h: bar_h };

    let input_border = if view.site_input_focused {
        hex_color(view.effective_accent, 0.78)
    } else if view.hovered == Some(ControlId::BtnSiteSearchBox) {
        rgba(255, 255, 255, 0.16)
    } else {
        rgba(255, 255, 255, 0.08)
    };
    draw_rounded_card(dc, input_r, 6.0, rgba(22, 22, 25, 0.92), input_border, 1.0)?;
    draw_vector_icon(dc, None, RectF { x: input_r.x + 8.0, y: input_r.y + 6.0, w: 16.0, h: 16.0 }, VectorIcon::Search, rgba(161, 161, 170, 0.85))?;

    let (label_text, label_col) = if view.site_input_query.is_empty() {
        (
            if view.site_input_focused { "Enter domain (e.g. netflix.com, twitch.tv)..." } else { "Add domain to block (e.g. netflix.com, twitch.tv)..." },
            rgba(135, 135, 146, 0.88),
        )
    } else {
        (view.site_input_query, rgba(244, 244, 250, 0.98))
    };
    draw_text_nowrap(
        dc,
        dwrite,
        label_text,
        RectF { x: input_r.x + 28.0, y: input_r.y, w: (input_r.w - 56.0).max(40.0), h: input_r.h },
        11.0,
        DWRITE_FONT_WEIGHT_NORMAL,
        label_col,
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    if !view.site_input_query.is_empty() {
        draw_vector_pill(dc, find_hit(layout, ControlId::BtnSiteClear), VectorIcon::Close, view.hovered == Some(ControlId::BtnSiteClear), false, view.effective_accent)?;
    }

    let add_hovered = view.hovered == Some(ControlId::BtnSiteAdd);
    draw_rounded_card(
        dc,
        add_r,
        6.0,
        if add_hovered { hex_color(view.effective_accent, 0.35) } else { hex_color(view.effective_accent, 0.22) },
        hex_color(view.effective_accent, 0.65),
        1.0,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "+ Add",
        add_r,
        11.2,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(255, 255, 255, 0.98),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;

    // 2. Grid of site cards
    let entries = build_site_entries(view);
    let grid_y = bar_y + bar_h + 8.0;
    let row_h = 32.0;
    let gap = 6.0;
    let bottom_reserved = 34.0;
    let max_avail_h = area.y + area.h - bottom_reserved - grid_y;
    let visible_rows = ((max_avail_h / (row_h + gap)).floor() as usize).max(1);
    let visible_cap = visible_rows * 2;
    let start_idx = (view.site_scroll * 2).min(entries.len().saturating_sub(visible_cap));

    for vis_i in 0..visible_cap {
        let i = start_idx + vis_i;
        if i >= entries.len() {
            break;
        }
        let domain = &entries[i];
        let Some(r) = find_hit(layout, ControlId::SitePresetToggle(i)) else {
            continue;
        };
        let active = view.config.restricted_sites.iter().any(|s| s.eq_ignore_ascii_case(domain));
        let hovered = view.hovered == Some(ControlId::SitePresetToggle(i));
        let bg = if active { hex_color(view.effective_accent, 0.14) } else if hovered { rgba(55, 55, 64, 0.9) } else { rgba(32, 32, 37, 0.9) };
        let border = if active { hex_color(view.effective_accent, 0.65) } else { rgba(255, 255, 255, 0.07) };
        draw_rounded_card(dc, r, 7.0, bg, border, 1.0)?;

        let is_custom = i >= COMMON_SITE_PRESETS.len();
        let text_max_w = if is_custom { r.w - 78.0 } else { r.w - 52.0 };
        draw_text_nowrap(
            dc,
            dwrite,
            domain,
            RectF { x: r.x + 10.0, y: r.y, w: text_max_w.max(30.0), h: r.h },
            11.2,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(238, 238, 246, 0.95),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;

        if is_custom {
            if let Some(del_r) = find_hit(layout, ControlId::SiteCustomDelete(i)) {
                let del_hovered = view.hovered == Some(ControlId::SiteCustomDelete(i));
                draw_rounded_card(
                    dc,
                    del_r,
                    4.0,
                    if del_hovered { rgba(239, 68, 68, 0.3) } else { rgba(255, 255, 255, 0.05) },
                    if del_hovered { rgba(239, 68, 68, 0.6) } else { rgba(255, 255, 255, 0.08) },
                    1.0,
                )?;
                draw_text_nowrap(
                    dc,
                    dwrite,
                    "✕",
                    del_r,
                    9.5,
                    DWRITE_FONT_WEIGHT_BOLD,
                    if del_hovered { rgba(248, 113, 113, 0.98) } else { rgba(161, 161, 170, 0.8) },
                    DWRITE_TEXT_ALIGNMENT_CENTER,
                )?;
            }
        }

        draw_switch_pill(
            dc,
            RectF { x: r.x + r.w - 38.0, y: r.y + (r.h - 16.0) * 0.5, w: 28.0, h: 16.0 },
            active,
            view.effective_accent,
        )?;
    }

    // 3. Bottom info & Clear Passes
    let active_count = view.config.restricted_sites.len();
    let info_text = format!("{active_count} domain{} restricted (synced to extension)", if active_count == 1 { "" } else { "s" });
    draw_text_nowrap(
        dc,
        dwrite,
        &info_text,
        RectF { x: area.x, y: area.y + area.h - 26.0, w: (area.w - 160.0).max(60.0), h: 20.0 },
        10.2,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(156, 163, 175, 0.9),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_subtle_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnGuardClearPasses),
        &format!("Clear Passes ({})", view.active_pass_count),
        view.hovered == Some(ControlId::BtnGuardClearPasses),
    )?;

    Ok(())
}

fn draw_prayer_times_tab(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    area: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let accent = view.effective_accent;

    // 1. Hero Header Card matching WebView2 Prayer Times UI
    let hero_r = RectF { x: area.x, y: area.y, w: area.w, h: 62.0 };
    draw_rounded_card(dc, hero_r, 12.0, rgba(40, 49, 59, 0.72), rgba(255, 255, 255, 0.08), 1.0)?;

    draw_text_nowrap(
        dc,
        dwrite,
        view.local_clock_text,
        RectF { x: hero_r.x + 16.0, y: hero_r.y + 7.0, w: (hero_r.w - 165.0).max(110.0), h: 28.0 },
        22.0,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        view.local_date_text,
        RectF { x: hero_r.x + 16.0, y: hero_r.y + 37.0, w: (hero_r.w - 165.0).max(110.0), h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(165, 175, 188, 0.90),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    // Next / Selected Prayer Navigator (<  NEXT PRAYER  Name  HH:MM  >)
    let mut selected_idx = view.prayer_times.iter().position(|p| view.next_prayer_label.starts_with(p.name)).unwrap_or(0);
    let mut header_tag = "NEXT PRAYER";
    if let Some(custom_idx) = view.prayer_view_idx {
        if custom_idx < view.prayer_times.len() {
            if custom_idx != selected_idx {
                header_tag = "SCHEDULE";
            }
            selected_idx = custom_idx;
        }
    }
    let (p_name, p_time) = view
        .prayer_times
        .get(selected_idx)
        .map(|p| (p.name, p.formatted.as_str()))
        .unwrap_or(("Maghrib", "--:--"));

    if let Some(prev_r) = find_hit(layout, ControlId::BtnPrayerPrev) {
        draw_text_nowrap(
            dc,
            dwrite,
            "‹",
            prev_r,
            15.0,
            DWRITE_FONT_WEIGHT_BOLD,
            if view.hovered == Some(ControlId::BtnPrayerPrev) { rgba(255, 255, 255, 0.98) } else { rgba(165, 175, 188, 0.85) },
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    if let Some(next_r) = find_hit(layout, ControlId::BtnPrayerNext) {
        draw_text_nowrap(
            dc,
            dwrite,
            "›",
            next_r,
            15.0,
            DWRITE_FONT_WEIGHT_BOLD,
            if view.hovered == Some(ControlId::BtnPrayerNext) { rgba(255, 255, 255, 0.98) } else { rgba(165, 175, 188, 0.85) },
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    let info_box = RectF { x: hero_r.x + hero_r.w - 112.0, y: hero_r.y + 6.0, w: 80.0, h: 50.0 };
    draw_text_nowrap(dc, dwrite, header_tag, RectF { x: info_box.x, y: info_box.y, w: info_box.w, h: 13.0 }, 8.6, DWRITE_FONT_WEIGHT_SEMI_BOLD, hex_color(accent, 0.92), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, p_name, RectF { x: info_box.x, y: info_box.y + 14.0, w: info_box.w, h: 19.0 }, 14.2, DWRITE_FONT_WEIGHT_BOLD, hex_color(accent, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, p_time, RectF { x: info_box.x, y: info_box.y + 34.0, w: info_box.w, h: 14.0 }, 10.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(244, 244, 250, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    // 2. Settings Card (Enable Prayer Notifications | Auto Break | Notify before | Time Offset)
    let card_y = area.y + 68.0;
    let settings_card = RectF { x: area.x, y: card_y, w: area.w, h: 112.0 };
    draw_rounded_card(dc, settings_card, 12.0, rgba(34, 34, 38, 0.92), rgba(255, 255, 255, 0.06), 1.0)?;

    draw_text_nowrap(dc, dwrite, "Enable Prayer Notifications", RectF { x: area.x + 14.0, y: card_y + 9.0, w: area.w - 80.0, h: 20.0 }, 11.2, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(240, 240, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_switch_pill(dc, RectF { x: area.x + area.w - 46.0, y: card_y + 10.0, w: 32.0, h: 18.0 }, view.config.prayer_enabled, accent)?;

    draw_text_nowrap(dc, dwrite, "Auto Break (Screen Block)", RectF { x: area.x + 14.0, y: card_y + 35.0, w: area.w - 80.0, h: 20.0 }, 11.2, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(240, 240, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_switch_pill(dc, RectF { x: area.x + area.w - 46.0, y: card_y + 36.0, w: 32.0, h: 18.0 }, view.config.prayer_break_enabled, accent)?;

    draw_text_nowrap(dc, dwrite, "Notify before (mins)", RectF { x: area.x + 14.0, y: card_y + 60.0, w: area.w - 130.0, h: 20.0 }, 11.0, DWRITE_FONT_WEIGHT_NORMAL, rgba(215, 215, 226, 0.92), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    if let Some(dr) = find_hit(layout, ControlId::BtnPrayerAdvanceDown) {
        let fill = if view.hovered == Some(ControlId::BtnPrayerAdvanceDown) { rgba(64, 64, 74, 0.95) } else { rgba(48, 48, 56, 0.92) };
        draw_rounded_card(dc, dr, 5.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "−", dr, 12.0, DWRITE_FONT_WEIGHT_BOLD, rgba(240, 240, 248, 0.95), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    let adv_pill = RectF { x: area.x + area.w - 78.0, y: card_y + 59.0, w: 38.0, h: 22.0 };
    draw_rounded_card(dc, adv_pill, 5.0, rgba(22, 22, 26, 0.92), rgba(255, 255, 255, 0.12), 1.0)?;
    draw_text_nowrap(dc, dwrite, &view.config.prayer_advance_mins.to_string(), adv_pill, 11.0, DWRITE_FONT_WEIGHT_BOLD, rgba(248, 248, 252, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    if let Some(ur) = find_hit(layout, ControlId::BtnPrayerAdvanceUp) {
        let fill = if view.hovered == Some(ControlId::BtnPrayerAdvanceUp) { rgba(64, 64, 74, 0.95) } else { rgba(48, 48, 56, 0.92) };
        draw_rounded_card(dc, ur, 5.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "+", ur, 12.0, DWRITE_FONT_WEIGHT_BOLD, rgba(240, 240, 248, 0.95), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }

    draw_text_nowrap(dc, dwrite, "Schedule Offset / Ihtiyat (mins)", RectF { x: area.x + 14.0, y: card_y + 86.0, w: area.w - 130.0, h: 20.0 }, 11.0, DWRITE_FONT_WEIGHT_NORMAL, rgba(215, 215, 226, 0.92), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    if let Some(dr) = find_hit(layout, ControlId::BtnPrayerOffsetDown) {
        let fill = if view.hovered == Some(ControlId::BtnPrayerOffsetDown) { rgba(64, 64, 74, 0.95) } else { rgba(48, 48, 56, 0.92) };
        draw_rounded_card(dc, dr, 5.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "−", dr, 12.0, DWRITE_FONT_WEIGHT_BOLD, rgba(240, 240, 248, 0.95), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    let off_pill = RectF { x: area.x + area.w - 78.0, y: card_y + 85.0, w: 38.0, h: 22.0 };
    draw_rounded_card(dc, off_pill, 5.0, rgba(22, 22, 26, 0.92), rgba(255, 255, 255, 0.12), 1.0)?;
    let off_txt = if view.config.prayer_offset_mins == 0 {
        "0m".to_string()
    } else {
        format!("{:+}m", view.config.prayer_offset_mins)
    };
    draw_text_nowrap(dc, dwrite, &off_txt, off_pill, 10.6, DWRITE_FONT_WEIGHT_BOLD, rgba(248, 248, 252, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    if let Some(ur) = find_hit(layout, ControlId::BtnPrayerOffsetUp) {
        let fill = if view.hovered == Some(ControlId::BtnPrayerOffsetUp) { rgba(64, 64, 74, 0.95) } else { rgba(48, 48, 56, 0.92) };
        draw_rounded_card(dc, ur, 5.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "+", ur, 12.0, DWRITE_FONT_WEIGHT_BOLD, rgba(240, 240, 248, 0.95), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }

    // 3. Location Settings & Daily Schedule strip if vertical space allows
    let loc_y = card_y + 118.0;
    let avail_bot = (area.y + area.h - loc_y).max(0.0);
    if avail_bot >= 36.0 {
        let loc_h = avail_bot.min(72.0);
        let loc_card = RectF { x: area.x, y: loc_y, w: area.w, h: loc_h };
        draw_rounded_card(dc, loc_card, 12.0, rgba(34, 34, 38, 0.92), rgba(255, 255, 255, 0.06), 1.0)?;
        draw_text_nowrap(
            dc,
            dwrite,
            &format!(
                "Location  •  Lat {:.4}, Lng {:.4} (UTC{:+})",
                view.config.prayer_lat, view.config.prayer_lng, view.config.prayer_tz
            ),
            RectF { x: loc_card.x + 14.0, y: loc_card.y + 8.0, w: (loc_card.w - 140.0).max(80.0), h: 16.0 },
            10.2,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(182, 188, 200, 0.90),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_subtle_button(
            dc,
            dwrite,
            find_hit(layout, ControlId::BtnPrayerLocationCycle),
            "Change City ›",
            view.hovered == Some(ControlId::BtnPrayerLocationCycle),
        )?;
        if loc_h >= 60.0 && !view.prayer_times.is_empty() {
            let cnt = view.prayer_times.len().min(6);
            let sw = (loc_card.w - 28.0 - 5.0 * (cnt as f32 - 1.0)) / cnt as f32;
            for (idx, p) in view.prayer_times.iter().take(6).enumerate() {
                let chip = RectF {
                    x: loc_card.x + 14.0 + (sw + 5.0) * idx as f32,
                    y: loc_card.y + 29.0,
                    w: sw,
                    h: 26.0,
                };
                let active = idx == selected_idx;
                draw_rounded_card(
                    dc,
                    chip,
                    6.0,
                    if active { hex_color(accent, 0.18) } else { rgba(24, 24, 28, 0.85) },
                    if active { hex_color(accent, 0.65) } else { rgba(255, 255, 255, 0.06) },
                    1.0,
                )?;
                draw_text_nowrap(
                    dc,
                    dwrite,
                    &format!("{} {}", p.name, p.formatted),
                    chip,
                    8.8,
                    if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL },
                    if active { hex_color(accent, 0.98) } else { rgba(190, 190, 202, 0.90) },
                    DWRITE_TEXT_ALIGNMENT_CENTER,
                )?;
            }
        }
    }

    Ok(())
}

fn draw_settings_modal(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
    tab: SettingsTab,
) -> Result<()> {
    // Dim backdrop behind modal dialog
    draw_rounded_card(dc, card, layout.card_radius, rgba(8, 8, 12, 0.72), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_rounded_card(dc, mr, 14.0, rgba(28, 28, 32, 0.99), rgba(255, 255, 255, 0.12), 1.0)?;

    // Header matching settings_appearance.png: glowing Gear icon badge + Settings & Preferences + subtitle
    draw_icon_badge(
        dc,
        factory,
        RectF { x: mr.x + 18.0, y: mr.y + 13.0, w: 30.0, h: 30.0 },
        9.0,
        VectorIcon::Gear,
        view.effective_accent,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Settings & Preferences",
        RectF { x: mr.x + 56.0, y: mr.y + 11.0, w: mr.w - 106.0, h: 20.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Configure timer rules, audio alerts, PiP window, and appearance",
        RectF { x: mr.x + 56.0, y: mr.y + 30.0, w: mr.w - 106.0, h: 16.0 },
        10.5,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 166, 0.90),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    // 6-tab segmented bar matching settings_appearance.png
    let tab_bar = RectF { x: mr.x + 18.0, y: mr.y + 58.0, w: mr.w - 36.0, h: 32.0 };
    draw_rounded_card(dc, tab_bar, 10.0, rgba(19, 19, 22, 0.96), rgba(255, 255, 255, 0.06), 1.0)?;
    let tabs = [
        (SettingsTab::Appearance, "Appearance", VectorIcon::Palette),
        (SettingsTab::Session, "Session", VectorIcon::Clock),
        (SettingsTab::PipMode, "PiP Mode", VectorIcon::PipPop),
        (SettingsTab::Sound, "Sound", VectorIcon::Bell),
        (SettingsTab::MobileSync, "Mobile Sync", VectorIcon::Smartphone),
        (SettingsTab::System, "System", VectorIcon::Gear),
    ];
    for (t, label, icon) in tabs {
        let Some(tr) = find_hit(layout, ControlId::BtnSettingsTab(t)) else { continue; };
        let active = tab == t;
        let hov = view.hovered == Some(ControlId::BtnSettingsTab(t));
        let (fill, border, fg) = if active {
            (hex_color(view.effective_accent, 0.95), rgba(255, 255, 255, 0.14), contrast_fg(view.effective_accent))
        } else if hov {
            (rgba(52, 52, 60, 0.85), rgba(255, 255, 255, 0.08), rgba(240, 240, 248, 0.96))
        } else {
            (rgba(0, 0, 0, 0.0), rgba(0, 0, 0, 0.0), rgba(186, 186, 198, 0.90))
        };
        if fill.a > 0.0 || border.a > 0.0 {
            draw_rounded_card(dc, tr, 7.0, fill, border, 1.0)?;
        }
        let wt = if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM };
        let tw = measure_text_width(dwrite, label, 10.5, wt);
        let sx = tr.x + ((tr.w - (12.0 + 4.0 + tw)).max(4.0)) * 0.5;
        draw_vector_icon(dc, Some(factory), RectF { x: sx, y: tr.y + (tr.h - 12.0) * 0.5, w: 12.0, h: 12.0 }, icon, fg)?;
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF { x: sx + 16.0, y: tr.y, w: (tr.x + tr.w - 3.0 - (sx + 16.0)).max(16.0), h: tr.h },
            10.5,
            wt,
            fg,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
    }

    let Some(body) = layout.modal_body_rect else {
        return Ok(());
    };

    unsafe {
        dc.PushAxisAlignedClip(&body.to_d2d(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }
    let body_res = match tab {
        SettingsTab::Appearance => draw_settings_appearance(dc, factory, dwrite, body, layout, view),
        SettingsTab::Session => draw_settings_session(dc, dwrite, body, layout, view),
        SettingsTab::PipMode => draw_settings_pip(dc, dwrite, body, layout, view),
        SettingsTab::Sound => draw_settings_sound(dc, dwrite, body, layout, view),
        SettingsTab::MobileSync => draw_settings_mobile(dc, factory, dwrite, body, layout, view),
        SettingsTab::System => draw_settings_system(dc, dwrite, body, layout, view),
    };
    unsafe {
        dc.PopAxisAlignedClip();
    }
    body_res?;

    // Slim Fluent scrollbar thumb on right edge when scrollable
    if layout.settings_max_scroll > 1.0 {
        let track = RectF { x: body.x + body.w - 4.0, y: body.y + 2.0, w: 4.0, h: body.h - 4.0 };
        draw_rounded_card(dc, track, 2.0, rgba(255, 255, 255, 0.06), rgba(0, 0, 0, 0.0), 0.0)?;
        let total_h = body.h + layout.settings_max_scroll;
        let thumb_h = ((body.h / total_h) * track.h).clamp(28.0, track.h);
        let ratio = (view.settings_scroll / layout.settings_max_scroll).clamp(0.0, 1.0);
        let thumb_y = track.y + (track.h - thumb_h) * ratio;
        draw_rounded_card(dc, RectF { x: track.x, y: thumb_y, w: 4.0, h: thumb_h }, 2.0, rgba(255, 255, 255, 0.28), rgba(0, 0, 0, 0.0), 0.0)?;
    }

    Ok(())
}

fn draw_settings_appearance(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    body: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let cards = appearance_cards(body, view.config, view.settings_scroll);

    // --- Section 1: Timer Ring Theme ---
    draw_rounded_card(dc, cards.sec1, 12.0, rgba(36, 36, 40, 0.94), rgba(255, 255, 255, 0.07), 1.0)?;
    let p1x = cards.sec1.x + 14.0;
    let p1w = cards.sec1.w - 28.0;
    draw_text_nowrap(dc, dwrite, "Timer Ring Theme", RectF { x: p1x, y: cards.sec1.y + 12.0, w: p1w, h: 18.0 }, 12.8, DWRITE_FONT_WEIGHT_BOLD, rgba(246, 246, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "Choose a visual animation style for the circular focus timer:", RectF { x: p1x, y: cards.sec1.y + 31.0, w: p1w, h: 15.0 }, 10.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let chip_w = (p1w - 16.0) / 3.0;
    let mut y = cards.sec1.y + 52.0;
    let theme_icons = [
        VectorIcon::RingCircle,
        VectorIcon::HourglassMini,
        VectorIcon::Waves,
        VectorIcon::BarsMini,
        VectorIcon::DotsMini,
        VectorIcon::OrbitMini,
    ];
    for (i, (key, label)) in TIMER_THEMES.iter().enumerate() {
        let col = (i % 3) as f32;
        let row = (i / 3) as f32;
        let r = RectF { x: p1x + (chip_w + 8.0) * col, y: y + row * 38.0, w: chip_w, h: 32.0 };
        let active = view.config.timer_theme.eq_ignore_ascii_case(key);
        let hovered = view.hovered == Some(ControlId::BtnRingTheme(i));
        let bg = if active { hex_color(view.effective_accent, 0.14) } else if hovered { rgba(56, 56, 64, 0.92) } else { rgba(46, 46, 52, 0.88) };
        let border = if active { hex_color(view.effective_accent, 0.92) } else { rgba(255, 255, 255, 0.07) };
        draw_rounded_card(dc, r, 8.0, bg, border, 1.0)?;
        let wt = if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM };
        let fg = if active { hex_color(view.effective_accent, 0.98) } else { rgba(244, 244, 250, 0.96) };
        let tw = measure_text_width(dwrite, label, 11.0, wt);
        let sx = r.x + ((r.w - (13.0 + 6.0 + tw)).max(6.0)) * 0.5;
        let ic = theme_icons.get(i).copied().unwrap_or(VectorIcon::RingCircle);
        draw_vector_icon(dc, Some(factory), RectF { x: sx, y: r.y + (r.h - 13.0) * 0.5, w: 13.0, h: 13.0 }, ic, fg)?;
        draw_text_nowrap(dc, dwrite, label, RectF { x: sx + 19.0, y: r.y, w: (r.x + r.w - 4.0 - (sx + 19.0)).max(20.0), h: r.h }, 11.0, wt, rgba(244, 244, 250, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    }
    y += 78.0;
    draw_dashed_divider(dc, p1x, y, p1w)?;
    y += 4.0;
    if view.config.timer_theme == "hourglass" {
        draw_setting_inline_row(dc, dwrite, RectF { x: p1x, y, w: p1w, h: 36.0 }, "Auto-Rotate Hourglass Every 5s", None, view.config.auto_rotate_hourglass, view.hovered == Some(ControlId::BtnToggleHourglassRotate), view.effective_accent)?;
        y += 38.0;
        draw_dashed_divider(dc, p1x, y - 2.0, p1w)?;
    }
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p1x, y, w: p1w, h: 40.0 },
        "Stealth Mode (Click Timer to Toggle)",
        Some("Hide countdown timer digits to avoid clock-watching anxiety"),
        view.config.stealth_active,
        view.hovered == Some(ControlId::BtnToggleStealthTimer),
        view.effective_accent,
    )?;
    y += 42.0;
    draw_dashed_divider(dc, p1x, y - 2.0, p1w)?;
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p1x, y, w: p1w, h: 40.0 },
        "Hover to Peek Digits",
        Some("Temporarily fade in hidden countdown digits on mouse hover"),
        view.config.stealth_hover_peek,
        view.hovered == Some(ControlId::BtnToggleStealthPeek),
        view.effective_accent,
    )?;

    // --- Section 2: Accent Color Palette ---
    draw_rounded_card(dc, cards.sec2, 12.0, rgba(36, 36, 40, 0.94), rgba(255, 255, 255, 0.07), 1.0)?;
    let p2x = cards.sec2.x + 14.0;
    let p2w = cards.sec2.w - 28.0;
    draw_text_nowrap(dc, dwrite, "Accent Color Palette", RectF { x: p2x, y: cards.sec2.y + 12.0, w: p2w - 110.0, h: 18.0 }, 12.8, DWRITE_FONT_WEIGHT_BOLD, rgba(246, 246, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    let synced_badge = view.config.dynamic_album_enabled && view.track.is_some();
    let badge_r = RectF { x: p2x + p2w - 98.0, y: cards.sec2.y + 11.0, w: 98.0, h: 22.0 };
    draw_rounded_card(dc, badge_r, 11.0, if synced_badge { hex_color(view.effective_accent, 0.16) } else { rgba(48, 48, 56, 0.85) }, if synced_badge { hex_color(view.effective_accent, 0.65) } else { rgba(255, 255, 255, 0.1) }, 1.0)?;
    draw_text_nowrap(dc, dwrite, if synced_badge { "♫ Track Synced" } else { "Manual Preset" }, badge_r, 10.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, if synced_badge { hex_color(view.effective_accent, 0.96) } else { rgba(175, 175, 186, 0.9) }, DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_text_nowrap(dc, dwrite, "Default UI accent color used across gauges, highlights, and buttons:", RectF { x: p2x, y: cards.sec2.y + 31.0, w: p2w, h: 15.0 }, 10.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let sw_w = (p2w - 30.0) / 6.0;
    let sw_y = cards.sec2.y + 54.0;
    for (i, (name, hex)) in ACCENT_PRESETS.iter().enumerate() {
        let r = RectF { x: p2x + (sw_w + 6.0) * i as f32, y: sw_y, w: sw_w, h: 32.0 };
        let selected = view.config.accent_color.eq_ignore_ascii_case(hex);
        let hovered = view.hovered == Some(ControlId::AccentPreset(i));
        let bg = if selected { hex_color(hex, 0.16) } else if hovered { rgba(56, 56, 64, 0.92) } else { rgba(44, 44, 50, 0.88) };
        let border = if selected { hex_color(hex, 0.95) } else { rgba(255, 255, 255, 0.08) };
        draw_rounded_card(dc, r, 8.0, bg, border, 1.0)?;
        // Left vertical accent pill + colored dot matching settings_appearance.png
        draw_rounded_card(dc, RectF { x: r.x, y: r.y + 2.0, w: 3.5, h: r.h - 4.0 }, 2.0, hex_color(hex, 0.98), rgba(0, 0, 0, 0.0), 0.0)?;
        let dot_b = unsafe { dc.CreateSolidColorBrush(&hex_color(hex, 0.98), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(r.x + 14.0, r.y + r.h * 0.5), radiusX: 4.5, radiusY: 4.5 }, &dot_b);
        }
        draw_text_nowrap(dc, dwrite, name, RectF { x: r.x + 22.0, y: r.y, w: (r.w - 25.0).max(20.0), h: r.h }, 10.5, if selected { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM }, rgba(235, 235, 245, 0.95), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    }

    draw_dashed_divider(dc, p2x, sw_y + 40.0, p2w)?;
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p2x, y: sw_y + 44.0, w: p2w, h: 42.0 },
        "Dynamic Album Accent Sync",
        Some("Extracts accent palette from album art in real-time"),
        view.config.dynamic_album_enabled,
        view.hovered == Some(ControlId::BtnToggleDynamicAlbum),
        view.effective_accent,
    )?;
    draw_dashed_divider(dc, p2x, sw_y + 89.0, p2w)?;
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p2x, y: sw_y + 92.0, w: p2w, h: 42.0 },
        "High-Contrast Complementary Accent",
        Some("Generates contrasting gauge & background colors"),
        view.config.use_complementary_color,
        view.hovered == Some(ControlId::BtnToggleComplementary),
        view.effective_accent,
    )?;
    draw_dashed_divider(dc, p2x, sw_y + 137.0, p2w)?;
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p2x, y: sw_y + 140.0, w: p2w, h: 42.0 },
        "Browser Music Sync (:8765)",
        Some("Runs built-in WebSocket server for YT Music & extensions"),
        view.config.media_sync_enabled,
        view.hovered == Some(ControlId::BtnToggleMediaSync),
        view.effective_accent,
    )?;
    draw_dashed_divider(dc, p2x, sw_y + 185.0, p2w)?;
    draw_setting_inline_row(
        dc,
        dwrite,
        RectF { x: p2x, y: sw_y + 188.0, w: p2w, h: 42.0 },
        "Spotify & Desktop Media Sync",
        Some("Syncs desktop Spotify and Windows system media player"),
        view.config.spotify_sync_enabled,
        view.hovered == Some(ControlId::BtnToggleSpotifySync),
        view.effective_accent,
    )?;

    // --- Section 3: Mode-specific Timer Ambient Backdrop ---
    draw_rounded_card(dc, cards.sec3, 12.0, rgba(36, 36, 40, 0.94), rgba(255, 255, 255, 0.07), 1.0)?;
    let p3x = cards.sec3.x + 14.0;
    let p3w = cards.sec3.w - 28.0;
    draw_text_nowrap(dc, dwrite, "Timer Ambient Backdrop", RectF { x: p3x, y: cards.sec3.y + 12.0, w: p3w, h: 18.0 }, 12.8, DWRITE_FONT_WEIGHT_BOLD, rgba(246, 246, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "Visual theme displayed inside or behind the focus card:", RectF { x: p3x, y: cards.sec3.y + 31.0, w: p3w, h: 15.0 }, 10.5, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let mut y3 = cards.sec3.y + 52.0;
    let seg3 = (p3w - 12.0) / 3.0;
    let amb = [
        (ControlId::BtnAmbientDefault, "Default Plant Sprout", view.config.ambient_mode == AmbientMode::Default),
        (ControlId::BtnAmbientYtMusic, "YT Music Vinyl", view.config.ambient_mode == AmbientMode::YtMusic),
        (ControlId::BtnAmbientCustom, "Custom GIF / File...", view.config.ambient_mode == AmbientMode::Custom),
    ];
    for (i, (id, label, active)) in amb.into_iter().enumerate() {
        draw_segment_button(dc, dwrite, Some(RectF { x: p3x + (seg3 + 6.0) * i as f32, y: y3, w: seg3, h: 30.0 }), label, active, view.hovered == Some(id), view.effective_accent)?;
    }
    y3 += 40.0;

    match view.config.ambient_mode {
        AmbientMode::Default => {
            let badge = RectF { x: p3x, y: y3, w: p3w, h: 48.0 };
            draw_rounded_card(dc, badge, 8.0, rgba(16, 185, 129, 0.08), rgba(52, 211, 153, 0.28), 1.0)?;
            draw_text(
                dc,
                dwrite,
                "Plant Growth: Sprout will grow and blossom dynamically in real-time as your focus session progresses.",
                RectF { x: badge.x + 12.0, y: badge.y + 8.0, w: badge.w - 24.0, h: 32.0 },
                10.8,
                DWRITE_FONT_WEIGHT_MEDIUM,
                rgba(167, 243, 208, 0.94),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        }
        AmbientMode::YtMusic => {
            draw_text_nowrap(dc, dwrite, "GIF Display Style:", RectF { x: p3x, y: y3, w: p3w, h: 15.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(168, 168, 178, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let mw = (p3w - 10.0) * 0.5;
            draw_segment_button(dc, dwrite, Some(RectF { x: p3x, y: y3 + 18.0, w: mw, h: 30.0 }), "Circle Inner Ring", view.config.gif_display_mode == GifDisplayMode::Circle, view.hovered == Some(ControlId::BtnGifCircle), view.effective_accent)?;
            draw_segment_button(dc, dwrite, Some(RectF { x: p3x + mw + 10.0, y: y3 + 18.0, w: mw, h: 30.0 }), "Full Panel Fill", view.config.gif_display_mode == GifDisplayMode::Full, view.hovered == Some(ControlId::BtnGifFull), view.effective_accent)?;
            y3 += 58.0;

            draw_text_nowrap(dc, dwrite, "Vinyl Rotation Speed:", RectF { x: p3x, y: y3, w: p3w * 0.6, h: 15.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(168, 168, 178, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            draw_text_nowrap(dc, dwrite, &format!("{:.0}s", view.config.vinyl_speed_sec), RectF { x: p3x + p3w * 0.6, y: y3, w: p3w * 0.4, h: 15.0 }, 11.0, DWRITE_FONT_WEIGHT_BOLD, hex_color(view.effective_accent, 0.98), DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            draw_slider_track(dc, RectF { x: p3x, y: y3 + 18.0, w: p3w, h: 16.0 }, ((view.config.vinyl_speed_sec - 1.0) / 29.0).clamp(0.0, 1.0), view.effective_accent)?;
            y3 += 40.0;
            let sw = (p3w - 18.0) / 4.0;
            for (i, (spd, lbl)) in [(3.0f32, "Fast (3s)"), (6.0, "Normal (6s)"), (10.0, "Slow (10s)"), (16.0, "Relaxed (16s)")].into_iter().enumerate() {
                let active = (view.config.vinyl_speed_sec - spd).abs() < 0.6;
                draw_segment_button(dc, dwrite, Some(RectF { x: p3x + (sw + 6.0) * i as f32, y: y3, w: sw, h: 26.0 }), lbl, active, view.hovered == Some(ControlId::BtnVinylSpeedPreset(i)), view.effective_accent)?;
            }
            y3 += 36.0;

            draw_text_nowrap(dc, dwrite, "Vinyl Disc Size:", RectF { x: p3x, y: y3, w: p3w * 0.6, h: 15.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(168, 168, 178, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            draw_text_nowrap(dc, dwrite, &format!("{}px", view.config.vinyl_size_px), RectF { x: p3x + p3w * 0.6, y: y3, w: p3w * 0.4, h: 15.0 }, 11.0, DWRITE_FONT_WEIGHT_BOLD, hex_color(view.effective_accent, 0.98), DWRITE_TEXT_ALIGNMENT_TRAILING)?;
            draw_slider_track(dc, RectF { x: p3x, y: y3 + 18.0, w: p3w, h: 16.0 }, ((view.config.vinyl_size_px as f32 - 120.0) / (850.0 - 120.0)).clamp(0.0, 1.0), view.effective_accent)?;
            y3 += 40.0;
            let zw = (p3w - 12.0) / 3.0;
            for (i, (sz, lbl)) in [(150i32, "Small (150px)"), (340, "Medium (340px)"), (680, "Full Panel (680px)")].into_iter().enumerate() {
                let active = (view.config.vinyl_size_px - sz).abs() <= 15;
                draw_segment_button(dc, dwrite, Some(RectF { x: p3x + (zw + 6.0) * i as f32, y: y3, w: zw, h: 26.0 }), lbl, active, view.hovered == Some(ControlId::BtnVinylSizePreset(i)), view.effective_accent)?;
            }
            y3 += 34.0;
            draw_setting_inline_row(dc, dwrite, RectF { x: p3x, y: y3, w: p3w, h: 36.0 }, "Show Center Spindle (White Disc)", None, view.config.show_vinyl_spindle, view.hovered == Some(ControlId::BtnToggleVinylSpindle), view.effective_accent)?;
        }
        AmbientMode::Custom => {
            draw_text_nowrap(dc, dwrite, "Recent GIFs (Max 5):", RectF { x: p3x, y: y3, w: p3w - 205.0, h: 18.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(168, 168, 178, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            draw_subtle_button(dc, dwrite, Some(RectF { x: p3x + p3w - 196.0, y: y3 - 2.0, w: 96.0, h: 22.0 }), "+ Upload New", view.hovered == Some(ControlId::BtnUploadGif))?;
            draw_subtle_button(dc, dwrite, Some(RectF { x: p3x + p3w - 94.0, y: y3 - 2.0, w: 94.0, h: 22.0 }), "Clear history", view.hovered == Some(ControlId::BtnClearRecentGifs))?;
            y3 += 24.0;

            if view.config.recent_gifs.is_empty() {
                draw_text_nowrap(dc, dwrite, "No custom GIF uploaded yet", RectF { x: p3x, y: y3, w: p3w, h: 24.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(148, 148, 158, 0.85), DWRITE_TEXT_ALIGNMENT_LEADING)?;
                y3 += 30.0;
            } else {
                for (i, item) in view.config.recent_gifs.iter().take(5).enumerate() {
                    let active = view.config.custom_gif_id.as_deref() == Some(item.id.as_str());
                    let rr = RectF { x: p3x, y: y3, w: p3w - 28.0, h: 26.0 };
                    draw_rounded_card(dc, rr, 6.0, if active { hex_color(view.effective_accent, 0.20) } else { rgba(44, 44, 50, 0.88) }, if active { hex_color(view.effective_accent, 0.72) } else { rgba(255, 255, 255, 0.06) }, 1.0)?;
                    draw_text_nowrap(dc, dwrite, &item.name, RectF { x: rr.x + 8.0, y: rr.y, w: rr.w - 16.0, h: rr.h }, 11.0, if active { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL }, rgba(240, 240, 246, 0.95), DWRITE_TEXT_ALIGNMENT_LEADING)?;
                    draw_subtle_button(dc, dwrite, Some(RectF { x: p3x + p3w - 24.0, y: y3 + 2.0, w: 22.0, h: 22.0 }), "×", view.hovered == Some(ControlId::RecentGifDelete(i)))?;
                    y3 += 30.0;
                }
            }

            y3 += 8.0;
            draw_text_nowrap(dc, dwrite, "GIF Display Style:", RectF { x: p3x, y: y3, w: p3w, h: 15.0 }, 10.8, DWRITE_FONT_WEIGHT_NORMAL, rgba(168, 168, 178, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let mw = (p3w - 10.0) * 0.5;
            draw_segment_button(dc, dwrite, Some(RectF { x: p3x, y: y3 + 18.0, w: mw, h: 30.0 }), "Circle Inner Ring", view.config.gif_display_mode == GifDisplayMode::Circle, view.hovered == Some(ControlId::BtnGifCircle), view.effective_accent)?;
            draw_segment_button(dc, dwrite, Some(RectF { x: p3x + mw + 10.0, y: y3 + 18.0, w: mw, h: 30.0 }), "Full Panel Fill", view.config.gif_display_mode == GifDisplayMode::Full, view.hovered == Some(ControlId::BtnGifFull), view.effective_accent)?;
            y3 += 56.0;

            let thumb_size = 72.0f32;
            let thumb_r = RectF { x: p3x + (p3w - thumb_size) * 0.5, y: y3, w: thumb_size, h: thumb_size };
            if let Some(anim) = view.gif_anim {
                if let Some(frame) = anim.frames.get(view.gif_frame_index % anim.frames.len().max(1)) {
                    let opacity = (view.config.gif_opacity as f32 / 100.0).clamp(0.15, 1.0);
                    if view.config.gif_display_mode == GifDisplayMode::Circle {
                        draw_cover_bitmap_circle(dc, factory, &frame.bitmap, thumb_r.x + thumb_size * 0.5, thumb_r.y + thumb_size * 0.5, thumb_size * 0.5, opacity)?;
                        let b = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, 0.95), None) }?;
                        unsafe {
                            dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(thumb_r.x + thumb_size * 0.5, thumb_r.y + thumb_size * 0.5), radiusX: thumb_size * 0.5, radiusY: thumb_size * 0.5 }, &b, 2.0, None);
                        }
                    } else {
                        draw_cover_bitmap_rounded(dc, factory, &frame.bitmap, thumb_r, 8.0, opacity)?;
                        draw_rounded_card(dc, thumb_r, 8.0, rgba(0, 0, 0, 0.0), hex_color(view.effective_accent, 0.95), 2.0)?;
                    }
                }
            } else {
                draw_rounded_card(dc, thumb_r, 8.0, rgba(38, 38, 44, 0.9), hex_color(view.effective_accent, 0.5), 1.5)?;
                draw_text_nowrap(dc, dwrite, "GIF", thumb_r, 12.0, DWRITE_FONT_WEIGHT_BOLD, rgba(161, 161, 170, 0.8), DWRITE_TEXT_ALIGNMENT_CENTER)?;
            }
            let active_gif_label = if view.config.custom_gif_name.is_empty() {
                "Click + Upload New to load GIF"
            } else {
                view.config.custom_gif_name.as_str()
            };
            draw_text_nowrap(dc, dwrite, active_gif_label, RectF { x: p3x, y: y3 + 76.0, w: p3w, h: 16.0 }, 11.0, DWRITE_FONT_WEIGHT_MEDIUM, hex_color(view.effective_accent, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;
            y3 += 100.0;

            draw_text_nowrap(dc, dwrite, &format!("GIF Opacity: {}%", view.config.gif_opacity), RectF { x: p3x, y: y3, w: p3w, h: 16.0 }, 11.0, DWRITE_FONT_WEIGHT_MEDIUM, rgba(210, 210, 220, 0.92), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            if let Some(slider) = layout.opacity_slider_rect {
                let ratio = ((view.config.gif_opacity as f32 - 15.0) / 85.0).clamp(0.0, 1.0);
                draw_slider_track(dc, slider, ratio, view.effective_accent)?;
            }
        }
    }

    Ok(())
}

fn draw_settings_session(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    body: RectF,
    _layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let w = (body.w - 10.0).max(200.0);
    let mut y = body.y - view.settings_scroll;
    let rows = [
        (ControlId::BtnToggleAutoPause, "Auto-pause on Inactivity", "Pauses focus timer if no keyboard or mouse activity is detected", Some(view.config.auto_pause_enabled), None),
        (ControlId::BtnAutoPauseThreshold, "Inactivity Threshold", "Click to cycle time before auto-pausing active session", None, Some(format!("{}s", view.config.auto_pause_sec))),
        (ControlId::BtnToggleNeutralOverstay, "Neutral App Overstay Guard", "Nudges you when spending too long in browsers or neutral apps", Some(view.config.neutral_overstay_enabled), None),
        (ControlId::BtnNeutralThreshold, "Reminder Interval", "Frequency of neutral app check-in reminders (click to cycle)", None, Some(format!("{}m", view.config.neutral_overstay_mins))),
        (ControlId::BtnToggleNeutralSoftblock, "Soft-Block on 2x Ignored", "Temporarily restricts screen if 2 check-ins are ignored", Some(view.config.neutral_softblock_enabled), None),
        (ControlId::BtnToggleAutoClose, "Auto-Close Blocked Apps", "Gracefully closes forbidden apps after 5s warning during focus", Some(view.config.auto_close_blocked_apps), None),
        (ControlId::BtnToggleBlockAllMonitors, "Block All Monitors", "Cover all displays when restricted (OFF: active monitor only)", Some(view.config.block_all_monitors), None),
    ];
    for (id, title, sub, toggle, badge) in rows {
        draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, title, sub, toggle, badge.as_deref(), view.hovered == Some(id), view.effective_accent)?;
        y += 54.0;
    }
    Ok(())
}

fn draw_settings_pip(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    body: RectF,
    _layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let w = (body.w - 10.0).max(200.0);
    let mut y = body.y - view.settings_scroll;

    draw_text_nowrap(dc, dwrite, "PiP Window Format", RectF { x: body.x, y, w, h: 16.0 }, 11.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(225, 225, 236, 0.94), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    let sw_mode = (w - 10.0) * 0.5;
    draw_segment_button(dc, dwrite, Some(RectF { x: body.x, y: y + 20.0, w: sw_mode, h: 30.0 }), "Standard Card", view.config.pip_style == PipStyle::Card, view.hovered == Some(ControlId::BtnPipStyleCard), view.effective_accent)?;
    draw_segment_button(dc, dwrite, Some(RectF { x: body.x + sw_mode + 10.0, y: y + 20.0, w: sw_mode, h: 30.0 }), "Dynamic Capsule", view.config.pip_style == PipStyle::Capsule, view.hovered == Some(ControlId::BtnPipStyleCapsule), view.effective_accent)?;
    y += 60.0;

    if view.config.pip_style == PipStyle::Capsule {
        draw_setting_card_row(
            dc,
            dwrite,
            RectF { x: body.x, y, w, h: 48.0 },
            "Center Album Artwork",
            "Display YouTube Music album art in the middle of the capsule",
            Some(view.config.pip_capsule_album_bg),
            None,
            view.hovered == Some(ControlId::BtnToggleCapsuleAlbumBg),
            view.effective_accent,
        )?;
        y += 54.0;

        if view.config.pip_capsule_album_bg {
            draw_setting_card_row(
                dc,
                dwrite,
                RectF { x: body.x, y, w, h: 48.0 },
                "Spinning Disc (Vinyl Effect)",
                "Smoothly rotate album art like a spinning vinyl disc",
                Some(view.config.pip_capsule_album_spin),
                None,
                view.hovered == Some(ControlId::BtnToggleCapsuleAlbumSpin),
                view.effective_accent,
            )?;
            y += 54.0;

            draw_setting_card_row(
                dc,
                dwrite,
                RectF { x: body.x, y, w, h: 48.0 },
                "Album Art Transparency",
                "Adjust background artwork opacity (click to cycle)",
                None,
                Some(&format!("{}%", view.config.pip_capsule_album_opacity)),
                view.hovered == Some(ControlId::BtnCapsuleAlbumOpacity),
                view.effective_accent,
            )?;
            y += 54.0;
        }
    }

    let rows = [
        (ControlId::BtnToggleAutoPip, "Auto-PIP on Session Start", "Automatically transforms into floating PiP when focus session begins", view.config.auto_pip),
        (ControlId::BtnToggleAutoPipBlur, "Auto-PIP on Blur (Click Outside)", "Switches to floating PiP when window loses focus during a session", view.config.auto_pip_on_blur),
        (ControlId::BtnToggleHideTaskbarInPip, "Hide Taskbar Icon in PiP", "Keep your Windows taskbar ultra-clean while in PiP mini mode", view.config.hide_taskbar_in_pip),
    ];
    for (id, title, sub, enabled) in rows {
        draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, title, sub, Some(enabled), None, view.hovered == Some(id), view.effective_accent)?;
        y += 54.0;
    }

    if view.config.pip_style == PipStyle::Card {
        draw_setting_card_row(
            dc,
            dwrite,
            RectF { x: body.x, y, w, h: 48.0 },
            "Peeking Vinyl in PiP Mode",
            "Spinning vinyl sleeve smoothly pokes out behind the floating card",
            Some(view.config.pip_peeking_vinyl_enabled),
            None,
            view.hovered == Some(ControlId::BtnTogglePeekingVinyl),
            view.effective_accent,
        )?;
        y += 54.0;

        draw_text_nowrap(dc, dwrite, "Vinyl Peeking Side", RectF { x: body.x, y, w, h: 16.0 }, 11.5, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(225, 225, 236, 0.94), DWRITE_TEXT_ALIGNMENT_LEADING)?;
        let sw = (w - 10.0) * 0.5;
        draw_segment_button(dc, dwrite, Some(RectF { x: body.x, y: y + 20.0, w: sw, h: 30.0 }), "Left", view.config.pip_peeking_vinyl_side == VinylSide::Left, view.hovered == Some(ControlId::BtnVinylSideLeft), view.effective_accent)?;
        draw_segment_button(dc, dwrite, Some(RectF { x: body.x + sw + 10.0, y: y + 20.0, w: sw, h: 30.0 }), "Right", view.config.pip_peeking_vinyl_side == VinylSide::Right, view.hovered == Some(ControlId::BtnVinylSideRight), view.effective_accent)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct AlertSoundRowRects {
    card: RectF,
    cycle: RectF,
    test: RectF,
    sub_card: Option<RectF>,
    upload_btn: Option<RectF>,
}

struct SoundTabLayout {
    notif_hdr_y: f32,
    notif_row: RectF,
    effects_hdr_y: f32,
    alert_focus: AlertSoundRowRects,
    alert_break: AlertSoundRowRects,
    alert_warn: AlertSoundRowRects,
    countdown_card: RectF,
    countdown_secs: RectF,
    countdown_style: RectF,
    countdown_toggle: RectF,
    total_h: f32,
}

fn sound_tab_layout(body: RectF, config: &AppConfig, scroll: f32) -> SoundTabLayout {
    let w = (body.w - 10.0).max(200.0);
    let x = body.x;
    let y0 = body.y - scroll;
    let mut y = y0;

    let notif_hdr_y = y + 2.0;
    y += 22.0;
    let notif_row = RectF { x, y, w, h: 52.0 };
    y += 66.0;

    let effects_hdr_y = y;
    y += 22.0;

    let make_alert_block = |style: &str, y_cur: &mut f32| -> AlertSoundRowRects {
        let card = RectF { x, y: *y_cur, w, h: 54.0 };
        let test = RectF { x: card.x + card.w - 38.0, y: card.y + 13.0, w: 26.0, h: 28.0 };
        let cycle = RectF { x: test.x - 134.0, y: card.y + 13.0, w: 128.0, h: 28.0 };
        *y_cur += 58.0;
        let (sub_card, upload_btn) = if style == "custom" {
            let sr = RectF { x, y: *y_cur, w, h: 34.0 };
            let ub = RectF { x: sr.x + 12.0, y: sr.y + 6.0, w: 92.0, h: 22.0 };
            *y_cur += 42.0;
            (Some(sr), Some(ub))
        } else {
            *y_cur += 4.0;
            (None, None)
        };
        AlertSoundRowRects {
            card,
            cycle,
            test,
            sub_card,
            upload_btn,
        }
    };

    let alert_focus = make_alert_block(&config.focus_sound, &mut y);
    let alert_break = make_alert_block(&config.break_sound, &mut y);
    let alert_warn = make_alert_block(&config.warning_sound, &mut y);

    y += 4.0;
    let countdown_card = RectF { x, y, w, h: 58.0 };
    let countdown_toggle = RectF { x: countdown_card.x + countdown_card.w - 48.0, y: countdown_card.y + 19.0, w: 36.0, h: 20.0 };
    let countdown_style = RectF { x: countdown_toggle.x - 104.0, y: countdown_card.y + 16.0, w: 96.0, h: 26.0 };
    let countdown_secs = RectF { x: countdown_style.x - 92.0, y: countdown_card.y + 16.0, w: 86.0, h: 26.0 };
    y += 66.0;

    SoundTabLayout {
        notif_hdr_y,
        notif_row,
        effects_hdr_y,
        alert_focus,
        alert_break,
        alert_warn,
        countdown_card,
        countdown_secs,
        countdown_style,
        countdown_toggle,
        total_h: (y - y0).max(120.0),
    }
}

fn draw_settings_sound(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    body: RectF,
    _layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let sl = sound_tab_layout(body, view.config, view.settings_scroll);
    let accent = view.effective_accent;

    draw_text_nowrap(
        dc,
        dwrite,
        "NOTIFICATIONS",
        RectF { x: body.x, y: sl.notif_hdr_y, w: body.w - 16.0, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_BOLD,
        hex_color(accent, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_setting_card_row(
        dc,
        dwrite,
        sl.notif_row,
        "Desktop Toast Notifications",
        "Show Windows system notifications on timer starts, breaks, and finishes",
        Some(view.config.notifications_enabled),
        None,
        view.hovered == Some(ControlId::BtnToggleNotifications),
        accent,
    )?;

    draw_text_nowrap(
        dc,
        dwrite,
        "FOCUS & BREAK SOUND EFFECTS",
        RectF { x: body.x, y: sl.effects_hdr_y, w: body.w - 16.0, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_BOLD,
        hex_color(accent, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    let sound_cards = [
        (
            "Work / Focus Sound",
            "Plays when a focus period begins",
            view.config.focus_sound.as_str(),
            view.config.custom_focus_audio_name.as_deref().or_else(|| {
                view.config.custom_focus_audio_path.as_deref().and_then(|p| std::path::Path::new(p).file_name()?.to_str())
            }),
            sl.alert_focus,
            ControlId::BtnFocusSoundCycle,
            ControlId::BtnFocusSoundTest,
            ControlId::BtnFocusSoundUpload,
        ),
        (
            "Break Session Sound",
            "Plays when break time begins",
            view.config.break_sound.as_str(),
            view.config.custom_break_audio_name.as_deref().or_else(|| {
                view.config.custom_break_audio_path.as_deref().and_then(|p| std::path::Path::new(p).file_name()?.to_str())
            }),
            sl.alert_break,
            ControlId::BtnBreakSoundCycle,
            ControlId::BtnBreakSoundTest,
            ControlId::BtnBreakSoundUpload,
        ),
        (
            "Reminder / Warning Sound",
            "Plays 60s before break or focus begins",
            view.config.warning_sound.as_str(),
            view.config.custom_warning_audio_name.as_deref().or_else(|| {
                view.config.custom_warning_audio_path.as_deref().and_then(|p| std::path::Path::new(p).file_name()?.to_str())
            }),
            sl.alert_warn,
            ControlId::BtnWarningSoundCycle,
            ControlId::BtnWarningSoundTest,
            ControlId::BtnWarningSoundUpload,
        ),
    ];

    for (title, sub, style, custom_name, rects, cid_cycle, cid_test, cid_upload) in sound_cards {
        draw_rounded_card(dc, rects.card, 10.0, rgba(34, 34, 38, 0.92), rgba(255, 255, 255, 0.06), 1.0)?;
        let text_w = (rects.cycle.x - rects.card.x - 18.0).max(80.0);
        draw_text_nowrap(
            dc,
            dwrite,
            title,
            RectF { x: rects.card.x + 14.0, y: rects.card.y + 8.0, w: text_w, h: 18.0 },
            12.0,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(244, 244, 250, 0.96),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            sub,
            RectF { x: rects.card.x + 14.0, y: rects.card.y + 27.0, w: text_w, h: 15.0 },
            10.0,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(155, 155, 168, 0.88),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;

        let drop_fill = if view.hovered == Some(cid_cycle) { rgba(56, 56, 65, 0.95) } else { rgba(44, 44, 51, 0.95) };
        draw_rounded_card(dc, rects.cycle, 6.0, drop_fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(
            dc,
            dwrite,
            &format!("{}  ▾", sound_style_label(style)),
            RectF { x: rects.cycle.x + 8.0, y: rects.cycle.y, w: rects.cycle.w - 16.0, h: rects.cycle.h },
            10.8,
            DWRITE_FONT_WEIGHT_MEDIUM,
            rgba(240, 240, 248, 0.95),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        draw_subtle_button(dc, dwrite, Some(rects.test), "▶", view.hovered == Some(cid_test))?;

        if let (Some(sr), Some(ub)) = (rects.sub_card, rects.upload_btn) {
            draw_rounded_card(dc, sr, 8.0, rgba(30, 30, 34, 0.88), rgba(255, 255, 255, 0.05), 1.0)?;
            draw_subtle_button(dc, dwrite, Some(ub), "Select File", view.hovered == Some(cid_upload))?;
            let fname = custom_name.unwrap_or("No file chosen");
            draw_text_nowrap(
                dc,
                dwrite,
                fname,
                RectF { x: ub.x + ub.w + 10.0, y: sr.y, w: (sr.x + sr.w - 14.0 - (ub.x + ub.w + 10.0)).max(60.0), h: sr.h },
                10.5,
                DWRITE_FONT_WEIGHT_MEDIUM,
                hex_color(accent, 0.96),
                DWRITE_TEXT_ALIGNMENT_TRAILING,
            )?;
        }
    }

    // Bottom Card: Transition Countdown Chime
    draw_rounded_card(dc, sl.countdown_card, 10.0, rgba(34, 34, 38, 0.92), rgba(255, 255, 255, 0.06), 1.0)?;
    let cd_text_w = (sl.countdown_secs.x - sl.countdown_card.x - 18.0).max(80.0);
    draw_text_nowrap(
        dc,
        dwrite,
        "Transition Countdown Chime",
        RectF { x: sl.countdown_card.x + 14.0, y: sl.countdown_card.y + 9.0, w: cd_text_w, h: 18.0 },
        11.8,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(244, 244, 250, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text(
        dc,
        dwrite,
        "Audio countdown before Focus, Break & Prayer locks",
        RectF { x: sl.countdown_card.x + 14.0, y: sl.countdown_card.y + 26.0, w: cd_text_w, h: 26.0 },
        9.6,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 168, 0.88),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    let secs_fill = if view.hovered == Some(ControlId::BtnCycleCountdownSecs) { rgba(56, 56, 65, 0.95) } else { rgba(44, 44, 51, 0.95) };
    draw_rounded_card(dc, sl.countdown_secs, 6.0, secs_fill, rgba(255, 255, 255, 0.10), 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("{} Seconds  ▾", view.config.countdown_duration_secs),
        sl.countdown_secs,
        10.2,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(240, 240, 248, 0.95),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;

    let style_fill = if view.hovered == Some(ControlId::BtnCycleCountdownStyle) { rgba(56, 56, 65, 0.95) } else { rgba(44, 44, 51, 0.95) };
    draw_rounded_card(dc, sl.countdown_style, 6.0, style_fill, rgba(255, 255, 255, 0.10), 1.0)?;
    let cd_style_label = if view.config.countdown_sound_style == "digital" { "Digital Pip  ▾" } else { "Zen Chime  ▾" };
    draw_text_nowrap(
        dc,
        dwrite,
        cd_style_label,
        sl.countdown_style,
        10.2,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(240, 240, 248, 0.95),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;
    draw_switch_pill(dc, sl.countdown_toggle, view.config.countdown_sound_enabled, accent)?;

    Ok(())
}

#[derive(Clone, Copy)]
struct TrackCardRects {
    card: RectF,
    toggle: RectF,
    slider: RectF,
}

struct AmbientMixerLayout {
    master_toggle: RectF,
    mute_all: RectF,
    master_slider: RectF,
    preset_rects: Vec<RectF>,
    custom_rects: [RectF; 2],
    track_cards: Vec<TrackCardRects>,
    auto_play_row: RectF,
    auto_play_toggle: RectF,
    total_h: f32,
}

fn ambient_mixer_layout(body: RectF, scroll: f32) -> AmbientMixerLayout {
    let w = (body.w - 10.0).max(200.0);
    let x = body.x;
    let y0 = body.y - scroll;
    let mut y = y0;

    let master_toggle = RectF { x, y, w: 122.0, h: 28.0 };
    let master_slider = RectF { x: x + 190.0, y: y + 6.0, w: (w - 318.0).max(80.0), h: 16.0 };
    let mute_all = RectF { x: x + w - 74.0, y, w: 74.0, h: 28.0 };
    y += 42.0;

    let preset_y0 = y + 18.0;
    let chip_gap = 6.0;
    let chip_w = (w - chip_gap * 3.0) / 4.0;
    let mut preset_rects = Vec::with_capacity(AMBIENT_PRESETS.len());
    for i in 0..AMBIENT_PRESETS.len() {
        let row = i / 4;
        let col = i % 4;
        preset_rects.push(RectF {
            x: x + col as f32 * (chip_w + chip_gap),
            y: preset_y0 + row as f32 * 30.0,
            w: chip_w,
            h: 25.0,
        });
    }

    let custom_y0 = preset_y0 + 60.0;
    let custom_w = (w - chip_gap) * 0.5;
    let custom_rects = [
        RectF { x, y: custom_y0, w: custom_w, h: 25.0 },
        RectF { x: x + custom_w + chip_gap, y: custom_y0, w: custom_w, h: 25.0 },
    ];

    let tracks_y0 = custom_y0 + 36.0;
    let col_gap = 10.0;
    let card_w = (w - col_gap) * 0.5;
    let card_h = 68.0;
    let row_gap = 8.0;
    let mut track_cards = Vec::with_capacity(AMBIENT_TRACKS.len());
    for i in 0..AMBIENT_TRACKS.len() {
        let row = i / 2;
        let col = i % 2;
        let cx = x + col as f32 * (card_w + col_gap);
        let cy = tracks_y0 + row as f32 * (card_h + row_gap);
        track_cards.push(TrackCardRects {
            card: RectF { x: cx, y: cy, w: card_w, h: card_h },
            toggle: RectF { x: cx + card_w - 46.0, y: cy + 10.0, w: 36.0, h: 20.0 },
            slider: RectF { x: cx + 10.0, y: cy + 44.0, w: (card_w - 58.0).max(60.0), h: 16.0 },
        });
    }

    let track_rows = (AMBIENT_TRACKS.len() + 1) / 2;
    let footer_y = tracks_y0 + track_rows as f32 * (card_h + row_gap) + 6.0;
    let auto_play_row = RectF { x, y: footer_y, w, h: 42.0 };
    let auto_play_toggle = RectF { x: x + w - 48.0, y: footer_y + 11.0, w: 36.0, h: 20.0 };
    let total_h = (footer_y + 50.0 - y0).max(120.0);

    AmbientMixerLayout {
        master_toggle,
        mute_all,
        master_slider,
        preset_rects,
        custom_rects,
        track_cards,
        auto_play_row,
        auto_play_toggle,
        total_h,
    }
}

fn push_ambient_mixer_hits(body: RectF, scroll: f32, hits: &mut Vec<HitRect>) -> f32 {
    let ml = ambient_mixer_layout(body, scroll);
    push_modal_hit(body, hits, ControlId::BtnAmbientMasterToggle, ml.master_toggle);
    push_modal_hit(body, hits, ControlId::BtnAmbientMuteAll, ml.mute_all);
    push_modal_hit(body, hits, ControlId::AmbientMasterSlider, ml.master_slider);
    for (i, r) in ml.preset_rects.iter().copied().enumerate() {
        push_modal_hit(body, hits, ControlId::BtnAmbientPreset(i), r);
    }
    for slot in 0..2 {
        push_modal_hit(body, hits, ControlId::BtnCustomPreset(slot), ml.custom_rects[slot]);
    }
    for (i, card) in ml.track_cards.iter().enumerate() {
        push_modal_hit(body, hits, ControlId::AmbientTrackSlider(i), card.slider);
        push_modal_hit(body, hits, ControlId::AmbientTrackSlider(i), card.slider);
        push_modal_hit(body, hits, ControlId::BtnAmbientTrackToggle(i), card.toggle);
    }
    push_modal_hit(body, hits, ControlId::BtnAmbientAutoPlay, ml.auto_play_row);
    (ml.total_h - body.h).max(0.0)
}

fn draw_ambient_mixer_modal(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let accent = view.effective_accent;
    let dim_brush = unsafe { dc.CreateSolidColorBrush(&rgba(0, 0, 0, 0.58), None) }?;
    let dim_r = D2D1_ROUNDED_RECT {
        rect: card.to_d2d(),
        radiusX: layout.card_radius,
        radiusY: layout.card_radius,
    };
    unsafe {
        dc.FillRoundedRectangle(&dim_r, &dim_brush);
    }

    draw_rounded_card(dc, mr, 16.0, rgba(24, 24, 28, 0.99), rgba(255, 255, 255, 0.11), 1.0)?;

    draw_icon_badge(
        dc,
        factory,
        RectF { x: mr.x + 18.0, y: mr.y + 13.0, w: 30.0, h: 30.0 },
        9.0,
        VectorIcon::Headphones,
        accent,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Focus Soundscape Mixer",
        RectF { x: mr.x + 56.0, y: mr.y + 12.0, w: mr.w - 106.0, h: 20.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Pure Procedural Native Audio • 100% Offline Synthesis",
        RectF { x: mr.x + 56.0, y: mr.y + 31.0, w: mr.w - 106.0, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 166, 0.90),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_vector_pill_fx(
        dc,
        factory,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    let body = layout.modal_body_rect.unwrap_or(RectF {
        x: mr.x + 18.0,
        y: mr.y + 62.0,
        w: mr.w - 36.0,
        h: mr.h - 76.0,
    });
    let ml = ambient_mixer_layout(body, view.settings_scroll);

    unsafe {
        dc.PushAxisAlignedClip(&body.to_d2d(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }

    let (master_label, master_icon) = if view.ambient_playing && !view.ambient_muted {
        ("Audio Active", VectorIcon::PauseBars)
    } else if view.ambient_muted {
        ("Muted", VectorIcon::Bell)
    } else {
        ("Start Audio", VectorIcon::PlayTri)
    };
    draw_segment_button_icon(
        dc,
        Some(factory),
        dwrite,
        Some(ml.master_toggle),
        master_label,
        Some(master_icon),
        view.ambient_playing && !view.ambient_muted,
        view.hovered == Some(ControlId::BtnAmbientMasterToggle),
        accent,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Master:",
        RectF { x: ml.master_slider.x - 56.0, y: ml.master_slider.y - 1.0, w: 50.0, h: 18.0 },
        11.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(205, 205, 215, 0.92),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_slider_track(dc, ml.master_slider, view.config.ambient.master_volume as f32 / 100.0, accent)?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("{}%", view.config.ambient.master_volume),
        RectF { x: ml.master_slider.x + ml.master_slider.w + 4.0, y: ml.master_slider.y - 1.0, w: 38.0, h: 18.0 },
        10.8,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(240, 240, 248, 0.95),
        DWRITE_TEXT_ALIGNMENT_TRAILING,
    )?;
    draw_subtle_button(
        dc,
        dwrite,
        Some(ml.mute_all),
        if view.ambient_muted { "Unmute" } else { "Mute All" },
        view.hovered == Some(ControlId::BtnAmbientMuteAll),
    )?;

    draw_text_nowrap(
        dc,
        dwrite,
        "Presets:",
        RectF { x: body.x, y: ml.master_toggle.y + 38.0, w: 120.0, h: 16.0 },
        10.5,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(175, 175, 186, 0.90),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    let preset_icons = [
        VectorIcon::Moon,
        VectorIcon::Flame,
        VectorIcon::CloudRain,
        VectorIcon::CloudLightning,
        VectorIcon::Keyboard,
        VectorIcon::PulseWave,
        VectorIcon::Waves,
        VectorIcon::Clock,
    ];
    for (i, preset) in AMBIENT_PRESETS.iter().enumerate() {
        let active = view.config.ambient.active_preset == preset.id;
        let p_icon = preset_icons.get(i).copied();
        draw_segment_button_icon(
            dc,
            Some(factory),
            dwrite,
            Some(ml.preset_rects[i]),
            preset.name,
            p_icon,
            active,
            view.hovered == Some(ControlId::BtnAmbientPreset(i)),
            accent,
        )?;
    }

    for slot in 0..2 {
        let preset_id = format!("custom_{}", slot + 1);
        let default_name = format!("Custom {}", slot + 1);
        let name = view
            .config
            .ambient
            .custom_presets
            .get(slot)
            .map(|p| p.name.as_str())
            .unwrap_or(&default_name);
        let active = view.config.ambient.active_preset == preset_id;
        draw_segment_button_icon(
            dc,
            Some(factory),
            dwrite,
            Some(ml.custom_rects[slot]),
            name,
            Some(VectorIcon::Headphones),
            active,
            view.hovered == Some(ControlId::BtnCustomPreset(slot)),
            accent,
        )?;
    }

    let track_styles: [(&str, VectorIcon); 11] = [
        ("#38BDF8", VectorIcon::Keyboard),
        ("#F59E0B", VectorIcon::Flame),
        ("#10B981", VectorIcon::Cricket),
        ("#60A5FA", VectorIcon::CloudRain),
        ("#818CF8", VectorIcon::CloudLightning),
        ("#22D3EE", VectorIcon::Wind),
        ("#FB923C", VectorIcon::Clock),
        ("#F472B6", VectorIcon::Person),
        ("#EC4899", VectorIcon::Person),
        ("#2DD4BF", VectorIcon::Waves),
        ("#A78BFA", VectorIcon::PulseWave),
    ];
    for (i, meta) in AMBIENT_TRACKS.iter().enumerate() {
        let cr = ml.track_cards[i];
        let tr = view.config.ambient.track(meta.id);
        let (badge_hex, track_icon) = track_styles.get(i).copied().unwrap_or((accent, VectorIcon::Bell));
        let border = if tr.active { hex_color(accent, 0.65) } else { rgba(255, 255, 255, 0.06) };
        let bg = if tr.active { rgba(40, 44, 52, 0.96) } else { rgba(28, 28, 33, 0.92) };
        draw_rounded_card(dc, cr.card, 10.0, bg, border, 1.0)?;

        let badge_r = RectF { x: cr.card.x + 10.0, y: cr.card.y + 8.0, w: 30.0, h: 30.0 };
        draw_icon_badge(dc, factory, badge_r, 8.0, track_icon, badge_hex)?;

        draw_text_nowrap(
            dc,
            dwrite,
            meta.name,
            RectF { x: cr.card.x + 48.0, y: cr.card.y + 7.0, w: cr.card.w - 102.0, h: 16.0 },
            11.2,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(244, 244, 250, 0.96),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            meta.desc,
            RectF { x: cr.card.x + 48.0, y: cr.card.y + 23.0, w: cr.card.w - 102.0, h: 14.0 },
            9.3,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(161, 161, 170, 0.85),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_switch_pill(dc, cr.toggle, tr.active, accent)?;
        draw_slider_track(dc, cr.slider, tr.volume as f32 / 100.0, accent)?;
        draw_text_nowrap(
            dc,
            dwrite,
            &format!("{}%", tr.volume),
            RectF { x: cr.slider.x + cr.slider.w + 6.0, y: cr.slider.y - 1.0, w: 36.0, h: 16.0 },
            10.2,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(218, 218, 228, 0.92),
            DWRITE_TEXT_ALIGNMENT_TRAILING,
        )?;
    }

    draw_rounded_card(
        dc,
        ml.auto_play_row,
        10.0,
        if view.hovered == Some(ControlId::BtnAmbientAutoPlay) { rgba(46, 46, 54, 0.96) } else { rgba(32, 32, 38, 0.92) },
        rgba(255, 255, 255, 0.08),
        1.0,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Auto-play ambient sounds when Focus Session starts",
        RectF { x: ml.auto_play_row.x + 12.0, y: ml.auto_play_row.y, w: ml.auto_play_row.w - 72.0, h: ml.auto_play_row.h },
        11.0,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(232, 232, 242, 0.94),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_switch_pill(dc, ml.auto_play_toggle, view.config.ambient.auto_play, accent)?;

    unsafe {
        dc.PopAxisAlignedClip();
    }

    if layout.settings_max_scroll > 0.0 {
        let track = RectF { x: body.x + body.w - 4.0, y: body.y + 2.0, w: 4.0, h: (body.h - 4.0).max(20.0) };
        let thumb_h = (track.h * (body.h / (body.h + layout.settings_max_scroll))).clamp(28.0, track.h);
        let ratio = (view.settings_scroll / layout.settings_max_scroll).clamp(0.0, 1.0);
        let thumb_y = track.y + (track.h - thumb_h) * ratio;
        draw_rounded_card(dc, track, 2.0, rgba(255, 255, 255, 0.06), rgba(0, 0, 0, 0.0), 0.0)?;
        draw_rounded_card(dc, RectF { x: track.x, y: thumb_y, w: track.w, h: thumb_h }, 2.0, rgba(160, 160, 175, 0.45), rgba(0, 0, 0, 0.0), 0.0)?;
    }

    Ok(())
}

fn draw_slider_track(dc: &ID2D1DeviceContext, rect: RectF, norm: f32, accent: &str) -> Result<()> {
    let cy = rect.y + rect.h * 0.5;
    let t = norm.clamp(0.0, 1.0);
    let track = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F { left: rect.x, top: cy - 3.0, right: rect.x + rect.w, bottom: cy + 3.0 },
        radiusX: 3.0,
        radiusY: 3.0,
    };
    let fill = D2D1_ROUNDED_RECT {
        rect: D2D_RECT_F {
            left: rect.x,
            top: cy - 3.0,
            right: (rect.x + rect.w * t).max(rect.x + 3.0),
            bottom: cy + 3.0,
        },
        radiusX: 3.0,
        radiusY: 3.0,
    };
    let bg_b = unsafe { dc.CreateSolidColorBrush(&rgba(60, 60, 70, 0.95), None) }?;
    let ac_b = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.96), None) }?;
    let knob_b = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.98), None) }?;
    unsafe {
        dc.FillRoundedRectangle(&track, &bg_b);
        dc.FillRoundedRectangle(&fill, &ac_b);
        dc.FillEllipse(&D2D1_ELLIPSE { point: pt(rect.x + rect.w * t, cy), radiusX: 6.0, radiusY: 6.0 }, &knob_b);
    }
    Ok(())
}

fn draw_settings_mobile(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    body: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let w = (body.w - 10.0).max(200.0);
    let y0 = body.y - view.settings_scroll;
    let accent = view.effective_accent;

    // 1. YTMPX Live Player Card (crisp vector controls + synced badge)
    draw_rounded_card(dc, RectF { x: body.x, y: y0, w, h: 150.0 }, 12.0, rgba(34, 34, 40, 0.94), rgba(255, 255, 255, 0.08), 1.0)?;
    draw_vinyl_disc(dc, body.x + 74.0, y0 + 75.0, 52.0, view.in_card_vinyl_angle, view.cover_bitmap, view.config.show_vinyl_spindle, accent)?;

    let ix = body.x + 148.0;
    let iw = (w - 162.0).max(140.0);
    let (conn, conn_col, conn_bg) = if let Some(ref tr) = view.track {
        if tr.art_url.starts_with("smtc://") {
            let label = if tr.source.to_lowercase().contains("spotify-web") {
                "● Spotify Web Synced"
            } else if tr.source.to_lowercase().contains("spotify") {
                "● Spotify Desktop Synced"
            } else {
                "● System Media Synced"
            };
            (label, rgba(52, 211, 153, 0.98), rgba(16, 185, 129, 0.12))
        } else {
            ("● Browser / YT Music Synced (:8765)", rgba(52, 211, 153, 0.98), rgba(16, 185, 129, 0.12))
        }
    } else if view.ytmpx_connected {
        ("● WebSocket Active (:8765)", rgba(52, 211, 153, 0.98), rgba(16, 185, 129, 0.12))
    } else {
        ("○ Waiting for Media (Spotify / YT Music :8765)", rgba(161, 161, 170, 0.88), rgba(48, 48, 56, 0.72))
    };
    let conn_pill = RectF { x: ix, y: y0 + 12.0, w: (measure_text_width(dwrite, conn, 10.2, DWRITE_FONT_WEIGHT_SEMI_BOLD) + 18.0).min(iw), h: 20.0 };
    let conn_border = D2D1_COLOR_F { r: conn_col.r, g: conn_col.g, b: conn_col.b, a: 0.35 };
    draw_rounded_card(dc, conn_pill, 10.0, conn_bg, conn_border, 1.0)?;
    draw_text_nowrap(dc, dwrite, conn, conn_pill, 10.2, DWRITE_FONT_WEIGHT_SEMI_BOLD, conn_col, DWRITE_TEXT_ALIGNMENT_CENTER)?;

    let title = view.track.map(|t| t.title.as_str()).unwrap_or("No active track detected");
    let artist = view.track.map(|t| t.artist.as_str()).unwrap_or("Play Spotify or YouTube Music to sync vinyl & accent");
    draw_text_nowrap(dc, dwrite, title, RectF { x: ix, y: y0 + 40.0, w: iw, h: 22.0 }, 14.5, DWRITE_FONT_WEIGHT_BOLD, rgba(248, 248, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, artist, RectF { x: ix, y: y0 + 64.0, w: iw, h: 18.0 }, 11.2, DWRITE_FONT_WEIGHT_NORMAL, rgba(175, 175, 186, 0.92), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let btn_y = y0 + 102.0;
    let bs = 36.0;
    for (id, icon, is_primary) in [
        (ControlId::BtnMediaPrev, VectorIcon::SkipPrev, false),
        (
            ControlId::BtnMediaPlayPause,
            if view.playback == PlaybackState::Playing { VectorIcon::PauseBars } else { VectorIcon::PlayTri },
            true,
        ),
        (ControlId::BtnMediaNext, VectorIcon::SkipNext, false),
    ] {
        let Some(r) = find_hit(layout, id) else { continue; };
        let hov = view.hovered == Some(id);
        let fill = if is_primary {
            hex_color(accent, if hov { 1.0 } else { 0.95 })
        } else if hov {
            rgba(68, 68, 78, 0.96)
        } else {
            rgba(48, 48, 56, 0.94)
        };
        let fg = if is_primary { contrast_fg(accent) } else { rgba(244, 244, 248, 0.96) };
        draw_rounded_card(dc, r, bs * 0.5, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_vector_icon(dc, Some(factory), r, icon, fg)?;
    }
    draw_subtle_button(dc, dwrite, find_hit(layout, ControlId::BtnMediaToggleWindow), "Open Player", view.hovered == Some(ControlId::BtnMediaToggleWindow))?;

    // 2. Connect FocusGrow Android App (QR Code + LAN HTTP Bridge)
    let qr_card = RectF { x: body.x, y: y0 + 162.0, w, h: 198.0 };
    draw_rounded_card(dc, qr_card, 12.0, rgba(34, 34, 40, 0.94), rgba(255, 255, 255, 0.08), 1.0)?;

    let qr_box = RectF { x: qr_card.x + 16.0, y: qr_card.y + 18.0, w: 146.0, h: 146.0 };
    draw_rounded_card(dc, qr_box, 12.0, rgba(255, 255, 255, 0.98), rgba(255, 255, 255, 0.2), 1.0)?;
    if let Some(qr) = view.qr_matrix {
        let pad = 11.0;
        let mod_s = (qr_box.w - pad * 2.0) / qr.size as f32;
        let dark_b = unsafe { dc.CreateSolidColorBrush(&rgba(16, 18, 24, 0.98), None) }?;
        unsafe {
            for r in 0..qr.size {
                for c in 0..qr.size {
                    if qr.get(r, c) {
                        let mr = RectF {
                            x: qr_box.x + pad + c as f32 * mod_s,
                            y: qr_box.y + pad + r as f32 * mod_s,
                            w: mod_s + 0.15,
                            h: mod_s + 0.15,
                        };
                        dc.FillRectangle(&mr.to_d2d(), &dark_b);
                    }
                }
            }
        }
    }
    draw_text_nowrap(dc, dwrite, "SCAN WITH PHONE", RectF { x: qr_box.x, y: qr_box.y + qr_box.h + 6.0, w: qr_box.w, h: 14.0 }, 8.8, DWRITE_FONT_WEIGHT_BOLD, rgba(155, 160, 174, 0.88), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    let rx = qr_card.x + 180.0;
    let rw = (w - 196.0).max(140.0);
    draw_text_nowrap(dc, dwrite, "Connect FocusGrow Android App", RectF { x: rx, y: qr_card.y + 16.0, w: rw, h: 20.0 }, 13.0, DWRITE_FONT_WEIGHT_BOLD, rgba(246, 246, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "Scan QR from FocusGrow Android (Settings → Scan QR)", RectF { x: rx, y: qr_card.y + 38.0, w: rw, h: 16.0 }, 10.4, DWRITE_FONT_WEIGHT_NORMAL, hex_color(accent, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let port = if view.bridge_port > 0 { view.bridge_port } else { 8766 };
    let ip_badge = format!("PC IP: {}  (Port {})", view.lan_ip, port);
    let ip_r = RectF { x: rx, y: qr_card.y + 62.0, w: (measure_text_width(dwrite, &ip_badge, 10.6, DWRITE_FONT_WEIGHT_SEMI_BOLD) + 22.0).min(rw), h: 24.0 };
    draw_rounded_card(dc, ip_r, 12.0, rgba(16, 185, 129, 0.12), rgba(16, 185, 129, 0.38), 1.0)?;
    draw_text_nowrap(dc, dwrite, &ip_badge, ip_r, 10.6, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(52, 211, 153, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    let bridge_status = if view.bridge_port > 0 {
        format!("• Mobile & Browser Bridge active  ({} site passes)\n• Limit: Up to 2 Mobile Devices simultaneously on Wi-Fi", view.active_pass_count)
    } else {
        "• HTTP Bridge initializing on LAN...".to_string()
    };
    draw_text(dc, dwrite, &bridge_status, RectF { x: rx, y: qr_card.y + 94.0, w: rw, h: 40.0 }, 10.2, DWRITE_FONT_WEIGHT_NORMAL, rgba(175, 178, 190, 0.92), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    draw_subtle_button(dc, dwrite, find_hit(layout, ControlId::BtnCopyMobileUrl), "Copy URL", view.hovered == Some(ControlId::BtnCopyMobileUrl))?;
    draw_segment_button(dc, dwrite, find_hit(layout, ControlId::BtnOpenMobileUrl), "Open Web Remote", true, view.hovered == Some(ControlId::BtnOpenMobileUrl), accent)?;
    let _ = btn_y;
    Ok(())
}

fn draw_settings_system(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    body: RectF,
    _layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let w = (body.w - 10.0).max(200.0);
    let mut y = body.y - view.settings_scroll;

    // Software Update Card Row
    let (upd_title, upd_sub, upd_btn, upd_col) = match view.update_status {
        crate::updater::UpdateStatus::Idle => (
            format!("FocusGrow v{}", env!("CARGO_PKG_VERSION")),
            "Click to check for the latest releases on GitHub".to_string(),
            "Check Update",
            view.effective_accent,
        ),
        crate::updater::UpdateStatus::Checking => (
            format!("FocusGrow v{}", env!("CARGO_PKG_VERSION")),
            "Checking for updates from GitHub...".to_string(),
            "Checking...",
            "#60A5FA",
        ),
        crate::updater::UpdateStatus::UpToDate { current } => (
            format!("FocusGrow v{}", current),
            "You are using the latest version of FocusGrow".to_string(),
            "Up to Date",
            "#10B981",
        ),
        crate::updater::UpdateStatus::Available { latest, changelog, .. } => (
            format!("New Update Available: v{}", latest),
            changelog.clone(),
            "Download Update",
            "#34D399",
        ),
        crate::updater::UpdateStatus::Downloading { latest } => (
            format!("Downloading v{}...", latest),
            "Downloading installer in background, please wait...".to_string(),
            "Downloading...",
            "#60A5FA",
        ),
        crate::updater::UpdateStatus::ReadyToInstall { latest, .. } => (
            format!("Ready to Install v{}", latest),
            "Installer downloaded. Click to install and restart FocusGrow".to_string(),
            "Install & Restart",
            "#10B981",
        ),
        crate::updater::UpdateStatus::Error(err) => (
            format!("FocusGrow v{}", env!("CARGO_PKG_VERSION")),
            err.clone(),
            "Retry",
            "#F87171",
        ),
    };

    draw_setting_card_row(
        dc,
        dwrite,
        RectF { x: body.x, y, w, h: 48.0 },
        &upd_title,
        &upd_sub,
        None,
        Some(upd_btn),
        view.hovered == Some(ControlId::BtnCheckUpdate),
        upd_col,
    )?;
    y += 54.0;

    draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, "Minimize to System Tray on Close", "Keep FocusGrow running in background when closed", Some(view.config.minimize_to_tray), None, view.hovered == Some(ControlId::BtnToggleMinimizeTray), view.effective_accent)?;
    y += 54.0;
    let lang = if view.config.app_language.eq_ignore_ascii_case("id") { "Bahasa Indonesia" } else { "English" };
    draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, "App Language / Bahasa", "Select preferred interface language (click to switch)", None, Some(lang), view.hovered == Some(ControlId::BtnToggleLanguage), view.effective_accent)?;
    y += 54.0;
    draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, "Reset Today's Progress", "Clear completed focus minutes for today back to 0", None, Some("Reset"), view.hovered == Some(ControlId::BtnSysResetToday), "#FB923C")?;
    y += 54.0;
    draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, "Clear Blocked Apps List", "Remove all applications currently in the blacklist", None, Some("Clear All"), view.hovered == Some(ControlId::BtnSysClearApps), "#F87171")?;
    y += 54.0;
    draw_setting_card_row(dc, dwrite, RectF { x: body.x, y, w, h: 48.0 }, "Cleanup GIF Storage Cache", "Clear all cached recent GIFs from local storage", None, Some("Cleanup"), view.hovered == Some(ControlId::BtnSysClearGifs), view.effective_accent)?;
    Ok(())
}

fn draw_backdrop_media(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    area: RectF,
    radius: f32,
    layout: &UiLayout,
    view: &RenderView<'_>,
    is_pip: bool,
) -> Result<()> {
    let opacity = (view.config.gif_opacity as f32 / 100.0).clamp(0.15, 1.0);
    let draw_full = view.config.gif_display_mode == GifDisplayMode::Full;

    // 1. Custom GIF ambient backdrop
    if view.config.ambient_mode == AmbientMode::Custom {
        if let Some(anim) = view.gif_anim {
            if !anim.frames.is_empty() {
                let bmp = &anim.frames[view.gif_frame_index % anim.frames.len()].bitmap;
                if draw_full {
                    let bmp_alpha = if is_pip { (opacity * 0.92).clamp(0.25, 0.96) } else { opacity * 0.72 };
                    draw_cover_bitmap_rounded(dc, factory, bmp, area, radius, bmp_alpha)?;
                    draw_rounded_card(dc, area, radius, rgba(10, 10, 14, if is_pip { 0.14 } else { 0.44 }), rgba(0, 0, 0, 0.0), 0.0)?;
                } else if !is_pip && view.timer.phase() == SessionPhase::Idle {
                    // Subtle full backdrop when Dashboard is Idle so uploaded GIF is visible behind setup boxes
                    draw_cover_bitmap_rounded(dc, factory, bmp, area, radius, opacity * 0.42)?;
                    draw_rounded_card(dc, area, radius, rgba(16, 16, 20, 0.36), rgba(0, 0, 0, 0.0), 0.0)?;
                }
                return Ok(());
            }
        }
    }

    // 2. YT Music Vinyl Record backdrop (Full Panel Fill renders giant spinning vinyl disc behind gauge)
    if view.config.ambient_mode == AmbientMode::YtMusic {
        if draw_full {
            let rr_geo = unsafe {
                factory.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                    rect: area.to_d2d(),
                    radiusX: radius,
                    radiusY: radius,
                })
            }?;
            let mask: ID2D1Geometry = rr_geo.into();
            let params = D2D1_LAYER_PARAMETERS1 {
                contentBounds: area.to_d2d(),
                geometricMask: core::mem::ManuallyDrop::new(Some(mask)),
                maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                maskTransform: Matrix3x2::identity(),
                opacity: 1.0,
                opacityBrush: core::mem::ManuallyDrop::new(None),
                layerOptions: D2D1_LAYER_OPTIONS1_NONE,
            };
            unsafe {
                dc.PushLayer(&params, None);
            }
            let (vx, vy) = if !is_pip && view.timer.phase() == SessionPhase::Idle {
                (area.x + area.w * 0.5, area.y + area.h * 0.48)
            } else {
                (layout.gauge_cx, layout.gauge_cy)
            };
            let v_radius = (view.config.vinyl_size_px as f32 * 0.42).clamp(area.w * 0.44, area.h * 0.62);
            let _ = draw_vinyl_disc(
                dc,
                vx,
                vy,
                v_radius,
                view.in_card_vinyl_angle,
                view.cover_bitmap,
                view.config.show_vinyl_spindle,
                view.effective_accent,
            );
            let veil_a = if is_pip { 0.14 } else { 0.24 };
            draw_rounded_card(dc, area, radius, rgba(10, 10, 14, veil_a), rgba(0, 0, 0, 0.0), 0.0)?;
            unsafe {
                dc.PopLayer();
                drop(core::mem::ManuallyDrop::into_inner(params.geometricMask));
            }
        } else if let Some(cover) = view.cover_bitmap {
            let alpha = if is_pip { 0.45 } else { 0.24 };
            draw_cover_bitmap_rounded(dc, factory, cover, area, radius, alpha)?;
            draw_rounded_card(dc, area, radius, rgba(12, 12, 16, if is_pip { 0.34 } else { 0.52 }), rgba(0, 0, 0, 0.0), 0.0)?;
        }
    }

    let _ = layout;
    Ok(())
}

fn draw_focus_ring(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    layout: &UiLayout,
    view: &RenderView<'_>,
    is_pip: bool,
) -> Result<()> {
    let cx = layout.gauge_cx;
    let cy = layout.gauge_cy;
    let r = layout.gauge_radius;
    let draw_full = view.config.gif_display_mode == GifDisplayMode::Full;
    let has_full_media = (view.config.ambient_mode == AmbientMode::Custom && draw_full && view.gif_anim.is_some())
        || (view.config.ambient_mode == AmbientMode::YtMusic && view.cover_bitmap.is_some());
    let stroke_w = if is_pip { 4.8 } else { 9.5 };

    let arc_ratio = if view.timer.phase() == SessionPhase::Idle {
        1.0
    } else {
        view.timer.smooth_remaining_fraction(std::time::Instant::now()).clamp(0.0, 1.0)
    };
    let progress_ratio = if view.timer.phase() == SessionPhase::Idle {
        0.25
    } else {
        (1.0 - arc_ratio).clamp(0.0, 1.0)
    };
    let glass_r = (r - stroke_w * 0.6).max(16.0);

    // Optional circular GIF, YT Music vinyl, or Sprout Plant Growth backdrop inside ring
    if view.config.ambient_mode == AmbientMode::Custom
        && view.config.gif_display_mode == GifDisplayMode::Circle
    {
        if let Some(anim) = view.gif_anim {
            if !anim.frames.is_empty() {
                let bmp = &anim.frames[view.gif_frame_index % anim.frames.len()].bitmap;
                let inner_r = (r - stroke_w * 0.65).max(16.0);
                let opacity = (view.config.gif_opacity as f32 / 100.0).clamp(0.15, 1.0);
                draw_cover_bitmap_circle(dc, factory, bmp, cx, cy, inner_r, opacity)?;
            }
        }
    } else if view.config.ambient_mode == AmbientMode::YtMusic && !draw_full {
        let inner_r = (r - stroke_w * 0.7).max(16.0);
        let _ = draw_vinyl_disc(
            dc,
            cx,
            cy,
            inner_r,
            view.in_card_vinyl_angle,
            view.cover_bitmap,
            view.config.show_vinyl_spindle,
            view.effective_accent,
        );
    } else if view.config.ambient_mode == AmbientMode::Default {
        let inner_r = (r - stroke_w * 0.7).max(16.0);
        let base_b = unsafe { dc.CreateSolidColorBrush(&rgba(18, 20, 26, 0.72), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: inner_r, radiusY: inner_r }, &base_b);
        }
        let growth = if view.timer.phase() == SessionPhase::Resting {
            1.0
        } else if view.timer.phase() == SessionPhase::Idle {
            0.62
        } else {
            progress_ratio
        };
        draw_sprout_plant(dc, factory, cx, cy, inner_r * 0.94, growth, view.plant_sway_phase)?;
    }

    // Translucent frosted accent disc inside gauge matching WebView2 screenshots
    let is_wave = view.config.timer_theme == "wave";
    let is_hourglass = view.config.timer_theme == "hourglass";
    let disc_r = if is_wave { r } else { glass_r };
    if is_hourglass {
        // Hourglass has its own crisp tinted glass silhouette without outer dark disc
    } else if view.config.ambient_mode == AmbientMode::Default {
        let soft_tint = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, 0.08), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &soft_tint);
        }
    } else if has_full_media {
        let base_alpha = if is_pip { 0.22 } else { 0.36 };
        let tint_alpha = if is_wave { 0.16 } else if is_pip { 0.16 } else { 0.14 };
        let base_veil = unsafe { dc.CreateSolidColorBrush(&rgba(16, 16, 20, base_alpha), None) }?;
        let accent_veil = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, tint_alpha), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &base_veil);
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &accent_veil);
        }
    } else if is_pip {
        let tint_b = unsafe { dc.CreateSolidColorBrush(&hex_color(view.effective_accent, 0.22), None) }?;
        let base_b = unsafe { dc.CreateSolidColorBrush(&rgba(18, 18, 22, 0.56), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &base_b);
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &tint_b);
        }
    } else {
        let glass_brush = unsafe { dc.CreateSolidColorBrush(&rgba(16, 16, 20, 0.54), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: disc_r, radiusY: disc_r }, &glass_brush);
        }
    }
    let arc_alpha = if is_pip && has_full_media { 0.88 } else { 0.96 };
    let ring_color = hex_color(view.effective_accent, arc_alpha);

    match view.config.timer_theme.as_str() {
        "wave" => {
            draw_wave_fill(dc, factory, cx, cy, disc_r, progress_ratio, view.wave_phase, view.effective_accent)?;
        }
        "hourglass" => {
            let flowing = view.timer.phase() != SessionPhase::Idle && view.timer.is_running();
            draw_hourglass_fill(dc, factory, cx, cy, r, progress_ratio, flowing, view.wave_phase, view.effective_accent)?;
        }
        "blocks" => {
            draw_blocks_ring(dc, factory, cx, cy, r, stroke_w, arc_ratio, ring_color)?;
        }
        "dots" => {
            draw_dots_ring(dc, cx, cy, r, stroke_w, arc_ratio, ring_color)?;
        }
        _ => {
            let track_alpha = if is_pip && has_full_media { 0.14 } else { 0.16 };
            let track_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, track_alpha), None) }?;
            unsafe {
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &track_brush, stroke_w, None);
            }

            let tick_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.16), None) }?;
            let tick_outer = r - stroke_w * 0.85;
            let tick_inner = tick_outer - if is_pip { 2.5 } else { 4.0 };
            unsafe {
                for k in 0..60 {
                    let a = (k as f32) * std::f32::consts::TAU / 60.0 - std::f32::consts::FRAC_PI_2;
                    let len = if k % 5 == 0 { tick_inner - 1.2 } else { tick_inner };
                    dc.DrawLine(
                        pt(cx + tick_outer * a.cos(), cy + tick_outer * a.sin()),
                        pt(cx + len * a.cos(), cy + len * a.sin()),
                        &tick_brush,
                        if k % 5 == 0 { 1.1 } else { 0.7 },
                        None,
                    );
                }
            }

            draw_progress_arc(dc, factory, cx, cy, r, stroke_w, arc_ratio, ring_color)?;
        }
    }

    // Bold countdown time with strong 4-way black outline + shadow matching WebView2
    // Stealth Mode hides digits on click and peeks back when hovering timer gauge
    let peeking_stealth = view.config.stealth_hover_peek
        && view.hovered == Some(ControlId::BtnToggleStealthTimer);
    let show_digits = !view.config.stealth_active || peeking_stealth;
    if show_digits {
        let time_str = view.timer.formatted_time();
        let time_size = if is_pip { (r * 0.46).clamp(16.0, 32.0) } else { 40.0 };
        let label_size = if is_pip { (r * 0.145).clamp(6.8, 9.0) } else { 9.5 };
        let two_line_sub = !is_pip && view.timer.phase() == SessionPhase::Resting;
        let sub_text = if view.is_auto_paused {
            "AUTO-PAUSED"
        } else if view.timer.phase() != SessionPhase::Idle && !view.timer.is_running() {
            "PAUSED"
        } else if view.timer.phase() == SessionPhase::Resting {
            if is_pip { "BREAK" } else { "RESTING & STEP\nOUTSIDE" }
        } else {
            "REMAINING"
        };

        // Vertically center the combined [digits + gap + sub_text] visual ink stack on cy
        let vis_t = time_size * 0.72;
        let vis_l = if two_line_sub { label_size * 2.35 } else { label_size * 0.74 };
        let stack_gap = if is_pip { (r * 0.10).clamp(3.0, 5.5) } else { 6.0 };
        let stack_top = cy - (vis_t + stack_gap + vis_l) * 0.5;
        let digits_cy = stack_top + vis_t * 0.5 - time_size * 0.06;
        let time_rect_h = time_size * 1.15;
        draw_text_outline(
            dc,
            dwrite,
            &time_str,
            RectF {
                x: cx - r,
                y: digits_cy - time_rect_h * 0.5,
                w: r * 2.0,
                h: time_rect_h,
            },
            time_size,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(255, 255, 255, 0.99),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;

        let sub_cy = stack_top + vis_t + stack_gap + vis_l * 0.5 - label_size * 0.06;
        let sub_rect_h = if two_line_sub { label_size * 2.8 } else { (label_size * 1.6).max(11.0) };
        draw_text_outline_wrap(
            dc,
            dwrite,
            sub_text,
            RectF {
                x: cx - r * 0.85,
                y: sub_cy - sub_rect_h * 0.5,
                w: r * 1.7,
                h: sub_rect_h,
            },
            label_size,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(245, 245, 250, 0.96),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    } else if view.is_auto_paused {
        let label_size = if is_pip { (r * 0.145).clamp(6.8, 9.0) } else { 9.5 };
        draw_text_outline_wrap(
            dc,
            dwrite,
            "AUTO-PAUSED",
            RectF { x: cx - r * 0.85, y: cy - 8.0, w: r * 1.7, h: 16.0 },
            label_size,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(245, 245, 250, 0.96),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }

    Ok(())
}

fn fill_clipped_wave_band(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    x0: f32,
    x1: f32,
    surface_y: f32,
    bottom_y: f32,
    amp: f32,
    cycles: f32,
    phase: f32,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let sink = unsafe { path.Open() }?;
    let steps = 48usize;
    unsafe {
        sink.BeginFigure(pt(x0, bottom_y), D2D1_FIGURE_BEGIN_FILLED);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let px = x0 + (x1 - x0) * t;
            let py = surface_y + (t * std::f32::consts::TAU * cycles - phase).sin() * amp;
            sink.AddLine(pt(px, py));
        }
        sink.AddLine(pt(x1, bottom_y));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
    }
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    unsafe {
        dc.FillGeometry(&path, &brush, None);
    }
    Ok(())
}

fn draw_wave_fill(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    cx: f32,
    cy: f32,
    radius: f32,
    fill_ratio: f32,
    phase: f32,
    accent: &str,
) -> Result<()> {
    let geo = unsafe {
        factory.CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: pt(cx, cy),
            radiusX: radius,
            radiusY: radius,
        })
    }?;
    let bounds = RectF {
        x: cx - radius,
        y: cy - radius,
        w: radius * 2.0,
        h: radius * 2.0,
    };
    let mask: ID2D1Geometry = geo.into();
    let params = D2D1_LAYER_PARAMETERS1 {
        contentBounds: bounds.to_d2d(),
        geometricMask: core::mem::ManuallyDrop::new(Some(mask)),
        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        maskTransform: Matrix3x2::identity(),
        opacity: 1.0,
        opacityBrush: core::mem::ManuallyDrop::new(None),
        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
    };
    unsafe {
        dc.PushLayer(&params, None);
    }

    let water_y = (cy + radius) - (radius * 2.0) * fill_ratio.clamp(0.02, 0.98);
    let amp = (radius * 0.07).clamp(3.2, 7.2);
    let x0 = cx - radius - 2.0;
    let x1 = cx + radius + 2.0;
    let bottom_y = cy + radius + 4.0;
    let p_back = (phase * 4.1 + 1.8).rem_euclid(std::f32::consts::TAU);
    let p_front = (phase * 2.8).rem_euclid(std::f32::consts::TAU);
    let _ = fill_clipped_wave_band(
        dc,
        factory,
        x0,
        x1,
        water_y,
        bottom_y,
        amp * 0.9,
        1.75,
        p_back,
        hex_color(accent, 0.40),
    );
    let _ = fill_clipped_wave_band(
        dc,
        factory,
        x0,
        x1,
        water_y,
        bottom_y,
        amp,
        1.5,
        p_front,
        hex_color(accent, 0.82),
    );

    unsafe {
        dc.PopLayer();
        drop(core::mem::ManuallyDrop::into_inner(params.geometricMask));
    }
    Ok(())
}

fn draw_hourglass_fill(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    cx: f32,
    cy: f32,
    radius: f32,
    progress_ratio: f32,
    flowing: bool,
    phase: f32,
    accent: &str,
) -> Result<()> {
    // Map WebView2 SVG [0..200] coordinates cleanly around (cx, cy) with 76 -> radius
    let s = radius / 76.0;
    let map = |vx: f32, vy: f32| pt(cx + (vx - 100.0) * s, cy + (vy - 100.0) * s);

    // 1. Glass bulb geometries matching ui/index.html lines 346-353
    let top_geo: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let ts = unsafe { top_geo.Open() }?;
    unsafe {
        ts.BeginFigure(map(50.0, 40.0), D2D1_FIGURE_BEGIN_FILLED);
        ts.AddLine(map(150.0, 40.0));
        ts.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(150.0, 40.0), point2: map(120.0, 100.0), point3: map(105.0, 100.0) });
        ts.AddLine(map(95.0, 100.0));
        ts.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(80.0, 100.0), point2: map(50.0, 40.0), point3: map(50.0, 40.0) });
        ts.EndFigure(D2D1_FIGURE_END_CLOSED);
        ts.Close()?;
    }

    let bot_geo: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let bs = unsafe { bot_geo.Open() }?;
    unsafe {
        bs.BeginFigure(map(95.0, 100.0), D2D1_FIGURE_BEGIN_FILLED);
        bs.AddLine(map(105.0, 100.0));
        bs.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(120.0, 100.0), point2: map(150.0, 160.0), point3: map(150.0, 160.0) });
        bs.AddLine(map(50.0, 160.0));
        bs.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(50.0, 160.0), point2: map(80.0, 100.0), point3: map(95.0, 100.0) });
        bs.EndFigure(D2D1_FIGURE_END_CLOSED);
        bs.Close()?;
    }

    // Glass interior tint
    let glass_bg = unsafe { dc.CreateSolidColorBrush(&rgba(12, 12, 16, 0.42), None) }?;
    unsafe {
        dc.FillGeometry(&top_geo, &glass_bg, None);
        dc.FillGeometry(&bot_geo, &glass_bg, None);
    }

    let p = progress_ratio.clamp(0.0, 1.0);
    let x0 = cx - 56.0 * s;
    let x1 = cx + 56.0 * s;
    let amp = (3.6 * s).clamp(2.0, 4.5);

    // 2. Top Sand draining from y=40 -> y=100
    let top_surface_vy = 40.0 + 60.0 * p;
    if p < 0.99 {
        let top_bounds = RectF {
            x: cx - 54.0 * s,
            y: cy - 62.0 * s,
            w: 108.0 * s,
            h: 64.0 * s,
        };
        let top_mask: ID2D1Geometry = top_geo.clone().into();
        let top_params = D2D1_LAYER_PARAMETERS1 {
            contentBounds: top_bounds.to_d2d(),
            geometricMask: core::mem::ManuallyDrop::new(Some(top_mask)),
            maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            maskTransform: Matrix3x2::identity(),
            opacity: 1.0,
            opacityBrush: core::mem::ManuallyDrop::new(None),
            layerOptions: D2D1_LAYER_OPTIONS1_NONE,
        };
        unsafe { dc.PushLayer(&top_params, None); }
        let sy = map(100.0, top_surface_vy).Y;
        let neck_y = map(100.0, 102.0).Y;
        let _ = fill_clipped_wave_band(dc, factory, x0, x1, sy, neck_y, amp * 0.85, 1.6, phase + 1.5, hex_color(accent, 0.36));
        let _ = fill_clipped_wave_band(dc, factory, x0, x1, sy, neck_y, amp, 1.4, phase, hex_color(accent, 0.90));
        unsafe {
            dc.PopLayer();
            drop(core::mem::ManuallyDrop::into_inner(top_params.geometricMask));
        }
    }

    // 3. Bottom Sand filling from y=160 -> y=100
    let bot_surface_vy = 160.0 - 60.0 * p;
    if p > 0.01 {
        let bot_bounds = RectF {
            x: cx - 54.0 * s,
            y: cy - 2.0 * s,
            w: 108.0 * s,
            h: 64.0 * s,
        };
        let bot_mask: ID2D1Geometry = bot_geo.clone().into();
        let bot_params = D2D1_LAYER_PARAMETERS1 {
            contentBounds: bot_bounds.to_d2d(),
            geometricMask: core::mem::ManuallyDrop::new(Some(bot_mask)),
            maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            maskTransform: Matrix3x2::identity(),
            opacity: 1.0,
            opacityBrush: core::mem::ManuallyDrop::new(None),
            layerOptions: D2D1_LAYER_OPTIONS1_NONE,
        };
        unsafe { dc.PushLayer(&bot_params, None); }
        let sy = map(100.0, bot_surface_vy).Y;
        let floor_y = map(100.0, 162.0).Y;
        let _ = fill_clipped_wave_band(dc, factory, x0, x1, sy, floor_y, amp * 0.85, 1.6, -phase + 1.2, hex_color(accent, 0.35));
        let _ = fill_clipped_wave_band(dc, factory, x0, x1, sy, floor_y, amp, 1.4, -phase, hex_color(accent, 0.88));
        unsafe {
            dc.PopLayer();
            drop(core::mem::ManuallyDrop::into_inner(bot_params.geometricMask));
        }
    }

    // 4. Flowing wave stream down from neck to bottom sand
    if flowing && p < 0.985 {
        let stream_b = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.88), None) }?;
        let end_y = map(100.0, bot_surface_vy.max(102.0)).Y;
        let start_y = map(100.0, 100.0).Y;
        let segs = 12usize;
        let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let ss = unsafe { path.Open() }?;
        unsafe {
            ss.BeginFigure(pt(cx, start_y), D2D1_FIGURE_BEGIN_HOLLOW);
            for i in 1..=segs {
                let t = i as f32 / segs as f32;
                let py = start_y + (end_y - start_y) * t;
                let wobble = (t * std::f32::consts::TAU * 2.0 - phase * 2.4).sin() * (1.4 * s);
                ss.AddLine(pt(cx + wobble, py));
            }
            ss.EndFigure(D2D1_FIGURE_END_OPEN);
            ss.Close()?;
            let style = create_round_stroke_style(factory)?;
            dc.DrawGeometry(&path, &stream_b, (2.2 * s).clamp(1.2, 2.8), &style);
        }
    }

    // 5. Crisp glass silhouette outline + top/bottom wooden/accent caps
    let frame_b = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.92), None) }?;
    let cap_rim_b = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.55), None) }?;
    unsafe {
        dc.DrawGeometry(&top_geo, &frame_b, (2.2 * s).max(1.4), None);
        dc.DrawGeometry(&bot_geo, &frame_b, (2.2 * s).max(1.4), None);
    }
    for cap_y in [34.0f32, 158.0f32] {
        let p0 = map(42.0, cap_y);
        let p1 = map(158.0, cap_y + 8.0);
        let cr = RectF { x: p0.X, y: p0.Y, w: p1.X - p0.X, h: p1.Y - p0.Y };
        let rr = D2D1_ROUNDED_RECT { rect: cr.to_d2d(), radiusX: 4.0 * s, radiusY: 4.0 * s };
        unsafe {
            dc.FillRoundedRectangle(&rr, &frame_b);
            dc.DrawRoundedRectangle(&rr, &cap_rim_b, (1.2 * s).max(0.8), None);
        }
    }

    Ok(())
}

fn draw_goal_modal(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_rounded_card(dc, card, layout.card_radius, rgba(8, 8, 12, 0.72), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_rounded_card(dc, mr, 14.0, rgba(28, 28, 32, 0.99), rgba(255, 255, 255, 0.12), 1.0)?;

    draw_vector_icon(
        dc,
        None,
        RectF { x: mr.x + 18.0, y: mr.y + 15.0, w: 18.0, h: 18.0 },
        VectorIcon::Target,
        hex_color(view.effective_accent, 0.96),
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Edit Daily Goals",
        RectF { x: mr.x + 42.0, y: mr.y + 11.0, w: mr.w - 90.0, h: 20.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Set your daily work target & posture break shield goal",
        RectF { x: mr.x + 42.0, y: mr.y + 30.0, w: mr.w - 90.0, h: 16.0 },
        10.5,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 166, 0.90),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_vector_pill(
        dc,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    draw_text_nowrap(
        dc,
        dwrite,
        "Daily Focus Target",
        RectF { x: mr.x + 20.0, y: mr.y + 72.0, w: mr.w - 40.0, h: 18.0 },
        12.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(235, 235, 244, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "0.1h tolerance applies (e.g. 3.9h counts for a 4h goal)",
        RectF { x: mr.x + 20.0, y: mr.y + 91.0, w: mr.w - 40.0, h: 15.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 168, 0.88),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    let cur_goal_hrs = (view.config.daily_goal_minutes / 60).max(1);
    for h_val in [1u32, 2, 3, 4, 5, 6, 8] {
        let id = ControlId::BtnGoalHours(h_val);
        draw_segment_button(
            dc,
            dwrite,
            find_hit(layout, id),
            &format!("{h_val}h"),
            cur_goal_hrs == h_val,
            view.hovered == Some(id),
            view.effective_accent,
        )?;
    }

    draw_text_nowrap(
        dc,
        dwrite,
        "Daily Posture Stand Goal (Shield)",
        RectF { x: mr.x + 20.0, y: mr.y + 174.0, w: mr.w - 40.0, h: 18.0 },
        12.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(235, 235, 244, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Completing this many posture breaks shields your work streak on rest days",
        RectF { x: mr.x + 20.0, y: mr.y + 193.0, w: mr.w - 40.0, h: 15.0 },
        10.2,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(155, 155, 168, 0.88),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    let cur_stands = view.stats.health_daily_goal_breaks.max(1);
    for b_val in [2u32, 3, 4, 6, 8] {
        let id = ControlId::BtnHealthGoalBreaks(b_val);
        draw_segment_button(
            dc,
            dwrite,
            find_hit(layout, id),
            &format!("{b_val}x"),
            cur_stands == b_val,
            view.hovered == Some(id),
            "#10B981",
        )?;
    }

    if let Some(done_r) = find_hit(layout, ControlId::BtnCloseModal) {
        draw_segment_button(
            dc,
            dwrite,
            Some(done_r),
            "Done",
            true,
            view.hovered == Some(ControlId::BtnCloseModal),
            view.effective_accent,
        )?;
    }
    Ok(())
}

fn push_stats_modal_hits(
    mc: RectF,
    scroll: f32,
    stats_app_rows: usize,
    hits: &mut Vec<HitRect>,
) -> f32 {
    let w = (mc.w - 10.0).max(240.0);
    let y0 = mc.y - scroll;
    let y1 = y0 + 68.0 + 10.0;  // 7-column KPI strip
    let y2 = y1 + 62.0 + 12.0;  // Goal progress
    let y3 = y2 + 158.0 + 12.0; // 7-day trend
    let next_r = RectF { x: mc.x + w - 24.0, y: y3 + 2.0, w: 22.0, h: 20.0 };
    let prev_r = RectF { x: next_r.x - 134.0, y: y3 + 2.0, w: 22.0, h: 20.0 };
    if prev_r.y + prev_r.h >= mc.y && prev_r.y <= mc.y + mc.h {
        hits.push(HitRect { id: ControlId::BtnHeatmapPrev, rect: prev_r });
        hits.push(HitRect { id: ControlId::BtnHeatmapNext, rect: next_r });
    }
    let y4 = y3 + 172.0 + 14.0; // Contribution heatmap
    let y5 = y4 + 178.0 + 14.0; // 24h flow
    let rows = stats_app_rows.min(12);
    for i in 0..rows {
        let row_y = y5 + 32.0 + i as f32 * 54.0;
        let cat_r = RectF {
            x: mc.x + 10.0,
            y: row_y + 4.0,
            w: (w - 196.0).max(120.0),
            h: 24.0,
        };
        let blk_r = RectF {
            x: mc.x + w - 76.0,
            y: row_y + 6.0,
            w: 64.0,
            h: 22.0,
        };
        if row_y + 46.0 >= mc.y && row_y <= mc.y + mc.h {
            hits.push(HitRect {
                id: ControlId::BtnCycleAppCategory(i),
                rect: cat_r,
            });
            hits.push(HitRect {
                id: ControlId::BtnBlockAppFromStats(i),
                rect: blk_r,
            });
        }
    }
    let app_section_h = 32.0 + (rows.max(1) as f32) * 54.0;
    let total_h = (y5 - y0) + app_section_h + 10.0;
    (total_h - mc.h).max(0.0)
}

pub fn period_app_breakdown(
    stats: &DailyStats,
    today_date: &str,
    period: StatsPeriod,
) -> Vec<(String, u32)> {
    let mut totals: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut cur = period.end_date(today_date);
    for _ in 0..period.days() {
        if let Some(map) = stats.app_history.get(&cur) {
            for (exe, &secs) in map {
                if secs > 0 {
                    *totals.entry(exe.clone()).or_insert(0) += secs;
                }
            }
        }
        cur = prev_iso_date(&cur);
    }
    let mut list: Vec<(String, u32)> = totals.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

fn format_secs_compact(secs: u32) -> String {
    let mins = secs / 60;
    if mins >= 60 {
        format!("{}h {}m", mins / 60, mins % 60)
    } else if mins >= 1 {
        format!("{mins}m")
    } else {
        format!("{secs}s")
    }
}

fn evaluate_chronotype(hourly_mins: &[u32; 24], peak_hour: usize, total_mins: u32) -> (&'static str, D2D1_COLOR_F, String) {
    if total_mins == 0 {
        return (
            "Awaiting Data",
            rgba(165, 170, 184, 0.9),
            "Start focus sessions throughout the day to calculate your peak productivity curve.".to_string(),
        );
    }
    let morning: u32 = hourly_mins[5..12].iter().sum();
    let afternoon: u32 = hourly_mins[12..18].iter().sum();
    let night: u32 = hourly_mins[18..24].iter().sum();
    let eff = (morning + afternoon + night + hourly_mins[0..5].iter().sum::<u32>()).max(1);
    let solid_morn = morning >= 25 || (morning as f32 / eff as f32) >= 0.15;
    let solid_aft = afternoon >= 35 || (afternoon as f32 / eff as f32) >= 0.20;
    let solid_night = night >= 35 || (night as f32 / eff as f32) >= 0.20;
    let p_end = (peak_hour + 2) % 24;
    if total_mins >= 240 && solid_morn && (solid_aft || solid_night) {
        (
            "All-Day Focus Titan",
            rgba(251, 191, 36, 0.98),
            format!("Marathon deep worker maintaining endurance from morning into evening, peaking around {:02}:00 — {:02}:00.", peak_hour, p_end),
        )
    } else if (5..12).contains(&peak_hour) {
        (
            "Early Bird Champion",
            rgba(251, 191, 36, 0.98),
            format!("Prime focus peaks during fresh morning hours ({:02}:00 — {:02}:00).", peak_hour, p_end),
        )
    } else if (12..18).contains(&peak_hour) {
        (
            "Afternoon Flow Master",
            rgba(245, 158, 11, 0.98),
            format!("Unlocks strongest deep work momentum during afternoon hours ({:02}:00 — {:02}:00).", peak_hour, p_end),
        )
    } else if (18..=23).contains(&peak_hour) {
        (
            "Night Owl Coder",
            rgba(192, 132, 252, 0.98),
            format!("Achieves effortless flow state in late evenings and night hours ({:02}:00 — {:02}:00).", peak_hour, p_end),
        )
    } else {
        (
            "Midnight Hacker",
            rgba(167, 139, 250, 0.98),
            format!("Late night specialist thriving in zero-distraction quiet hours ({:02}:00 — {:02}:00).", peak_hour, p_end),
        )
    }
}

fn draw_stats_modal(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_rounded_card(dc, card, layout.card_radius, rgba(8, 8, 12, 0.72), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_rounded_card(dc, mr, 14.0, rgba(24, 24, 28, 0.99), rgba(255, 255, 255, 0.12), 1.0)?;

    let accent = view.effective_accent;
    draw_vector_icon(
        dc,
        None,
        RectF { x: mr.x + 18.0, y: mr.y + 15.0, w: 18.0, h: 18.0 },
        VectorIcon::BarChart,
        hex_color(accent, 0.98),
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        "Focus Statistics & App Usage Analytics",
        RectF { x: mr.x + 42.0, y: mr.y + 12.0, w: mr.w - 200.0, h: 22.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    if let Some(share_r) = find_hit(layout, ControlId::BtnOpenShareCard) {
        let share_hov = view.hovered == Some(ControlId::BtnOpenShareCard);
        draw_rounded_card(
            dc,
            share_r,
            8.0,
            if share_hov { hex_color(accent, 0.24) } else { hex_color(accent, 0.14) },
            hex_color(accent, if share_hov { 0.85 } else { 0.50 }),
            1.0,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            "Share Card",
            share_r,
            10.5,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(242, 248, 255, 0.98),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    draw_vector_pill(
        dc,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    let tab_bar = RectF { x: mr.x + 18.0, y: mr.y + 52.0, w: mr.w - 36.0, h: 32.0 };
    draw_rounded_card(dc, tab_bar, 9.0, rgba(30, 30, 36, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
    for p in [
        StatsPeriod::Today,
        StatsPeriod::Yesterday,
        StatsPeriod::Week,
        StatsPeriod::Month,
        StatsPeriod::Quarter,
    ] {
        let id = ControlId::BtnStatsPeriod(p);
        draw_segment_button(
            dc,
            dwrite,
            find_hit(layout, id),
            p.label(),
            view.stats_period == p,
            view.hovered == Some(id),
            accent,
        )?;
    }

    let Some(mc) = layout.modal_body_rect else {
        return Ok(());
    };
    unsafe {
        dc.PushAxisAlignedClip(&mc.to_d2d(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }

    let w = (mc.w - 10.0).max(240.0);
    let y0 = mc.y - view.settings_scroll;

    // Aggregate metrics across selected StatsPeriod
    let days_count = view.stats_period.days();
    let end_date = view.stats_period.end_date(view.today_date);
    let mut period_focus_mins = 0u32;
    let mut period_ignored = 0u32;
    let mut earliest_date = end_date.clone();
    let mut cur = end_date.clone();
    for d_idx in 0..days_count {
        let m = view.stats.day_minutes(&cur, view.today_date);
        period_focus_mins += m;
        period_ignored += view.stats.ignored_nudges.get(&cur).copied().unwrap_or(0);
        if d_idx + 1 == days_count {
            earliest_date = cur.clone();
        }
        cur = prev_iso_date(&cur);
    }
    let goal_mins = view.config.daily_goal_minutes.max(1);
    let target_goal_mins = goal_mins * days_count as u32;
    let goal_pct = ((period_focus_mins as f32 / target_goal_mins.max(1) as f32) * 100.0).round() as u32;

    let mut prod_secs = 0u64;
    let mut neutral_secs = 0u64;
    let mut total_app_secs = 0u64;
    for app in view.stats_apps {
        let s = app.secs as u64;
        total_app_secs += s;
        match app.category {
            "productive" => prod_secs += s,
            "neutral" => neutral_secs += s,
            _ => {}
        }
    }
    let eff_pct = if total_app_secs > 0 {
        (((prod_secs as f64 + neutral_secs as f64 * 0.5) / total_app_secs as f64) * 100.0)
            .round()
            .min(100.0) as u32
    } else {
        100
    };
    let eff_col = if eff_pct >= 80 {
        rgba(52, 211, 153, 0.98)
    } else if eff_pct >= 50 {
        rgba(96, 205, 255, 0.98)
    } else {
        rgba(248, 113, 113, 0.98)
    };
    let (win_title, win_val, win_sub) = if matches!(view.stats_period, StatsPeriod::Today | StatsPeriod::Yesterday) {
        let wv = view
            .stats
            .work_windows
            .get(&end_date)
            .filter(|(a, b)| !a.is_empty() && !b.is_empty())
            .map(|(a, b)| format!("{a} — {b}"))
            .unwrap_or_else(|| {
                if view.stats_period == StatsPeriod::Yesterday {
                    "No session".to_string()
                } else {
                    "Not started".to_string()
                }
            });
        (
            "WORK WINDOW",
            wv,
            if view.stats_period == StatsPeriod::Yesterday {
                format_full_date_label(&end_date)
            } else {
                "Active hours".to_string()
            },
        )
    } else {
        (
            "TIMEFRAME",
            format!("{} — {}", date_month_year(&earliest_date), date_month_year(&end_date)),
            format!("{days_count} days span"),
        )
    };
    let stands_done = view.stats.health_completed_breaks_today;
    let stands_goal = view.stats.health_daily_goal_breaks.max(1);
    let focus_label = if period_focus_mins >= 60 {
        format!("{}h {}m", period_focus_mins / 60, period_focus_mins % 60)
    } else {
        format!("{}m", period_focus_mins)
    };
    let kpis = [
        ("TOTAL FOCUS", focus_label, "Active duration".to_string(), rgba(244, 244, 250, 0.98)),
        ("EFFICIENCY", format!("{eff_pct}%"), "Productivity".to_string(), eff_col),
        ("GOAL PROGRESS", format!("{goal_pct}%"), format!("Target: {} hr", target_goal_mins / 60), rgba(244, 244, 250, 0.98)),
        (win_title, win_val, win_sub, hex_color(accent, 0.98)),
        (
            "FOCUS STREAK",
            format!("{} Days", view.streak_days),
            if view.stats.is_shielded_today() { "Shielded via Posture".to_string() } else { "Productive work".to_string() },
            rgba(244, 244, 250, 0.98),
        ),
        (
            "POSTURE STREAK",
            format!("{} Days", view.stats.health_streak_days),
            format!("Posture habit ({stands_done}/{stands_goal})"),
            rgba(52, 211, 153, 0.98),
        ),
        (
            "DISTRACTIONS",
            format!("{period_ignored}x"),
            "Ignored Nudges".to_string(),
            if period_ignored == 0 { rgba(52, 211, 153, 0.98) } else { rgba(251, 146, 60, 0.98) },
        ),
    ];

    // 1. 7-Card Single-Row KPI Strip (matching analytics_heatmap.png)
    let kpi_gap = 7.0;
    let kpi_w = (w - kpi_gap * 6.0) / 7.0;
    let kpi_h = 68.0;
    for (idx, (title, val, sub, val_col)) in kpis.iter().enumerate() {
        let kr = RectF {
            x: mc.x + (kpi_w + kpi_gap) * idx as f32,
            y: y0,
            w: kpi_w,
            h: kpi_h,
        };
        draw_rounded_card(dc, kr, 8.0, rgba(32, 32, 37, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
        draw_text_nowrap(dc, dwrite, title, RectF { x: kr.x + 4.0, y: kr.y + 7.0, w: kr.w - 8.0, h: 12.0 }, 7.6, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(142, 146, 160, 0.92), DWRITE_TEXT_ALIGNMENT_CENTER)?;
        draw_text_nowrap(
            dc,
            dwrite,
            val,
            RectF { x: kr.x + 4.0, y: kr.y + 22.0, w: kr.w - 8.0, h: 24.0 },
            if idx == 3 && val.len() > 11 { 9.2 } else { 14.5 },
            DWRITE_FONT_WEIGHT_BOLD,
            *val_col,
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        draw_text_nowrap(dc, dwrite, sub, RectF { x: kr.x + 4.0, y: kr.y + 48.0, w: kr.w - 8.0, h: 13.0 }, 8.0, DWRITE_FONT_WEIGHT_NORMAL, rgba(142, 146, 160, 0.85), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }

    // 2. Goal Progress Card
    let y1 = y0 + kpi_h + 10.0;
    let goal_card = RectF { x: mc.x, y: y1, w, h: 62.0 };
    draw_rounded_card(dc, goal_card, 8.0, rgba(32, 32, 37, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
    draw_text_nowrap(dc, dwrite, "Goal Progress", RectF { x: goal_card.x + 14.0, y: goal_card.y + 8.0, w: 180.0, h: 16.0 }, 11.2, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("{:.1} hrs ({})", target_goal_mins as f32 / 60.0, view.stats_period.label().to_lowercase()),
        RectF { x: goal_card.x + goal_card.w - 190.0, y: goal_card.y + 8.0, w: 176.0, h: 16.0 },
        10.2,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(232, 232, 240, 0.94),
        DWRITE_TEXT_ALIGNMENT_TRAILING,
    )?;
    let bar_bg = RectF { x: goal_card.x + 14.0, y: goal_card.y + 29.0, w: goal_card.w - 28.0, h: 7.0 };
    draw_rounded_card(dc, bar_bg, 3.5, rgba(48, 48, 56, 0.92), rgba(255, 255, 255, 0.05), 0.5)?;
    let fill_w = (bar_bg.w * (period_focus_mins as f32 / target_goal_mins.max(1) as f32).clamp(0.0, 1.0)).max(6.0);
    draw_rounded_card(dc, RectF { x: bar_bg.x, y: bar_bg.y, w: fill_w, h: bar_bg.h }, 3.5, hex_color(accent, 0.96), rgba(0, 0, 0, 0.0), 0.0)?;
    let rem_mins = target_goal_mins.saturating_sub(period_focus_mins);
    let rem_str = if rem_mins > 0 {
        format!("{:.1} hrs remaining", rem_mins as f32 / 60.0)
    } else {
        "Goal Met!".to_string()
    };
    draw_text_nowrap(dc, dwrite, &rem_str, RectF { x: goal_card.x + goal_card.w - 170.0, y: goal_card.y + 40.0, w: 156.0, h: 15.0 }, 9.4, DWRITE_FONT_WEIGHT_NORMAL, rgba(162, 166, 180, 0.90), DWRITE_TEXT_ALIGNMENT_TRAILING)?;

    // 3. 7-Day Activity Trend Bar Chart (dark track containers + bottom fill bars matching analytics_heatmap.png)
    let y2 = y1 + 62.0 + 12.0;
    draw_text_nowrap(dc, dwrite, "7-Day Activity Trend", RectF { x: mc.x + 2.0, y: y2, w: 200.0, h: 18.0 }, 11.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    let trend_card = RectF { x: mc.x, y: y2 + 24.0, w, h: 134.0 };
    draw_rounded_card(dc, trend_card, 8.0, rgba(30, 30, 34, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;

    let mut week_days = Vec::with_capacity(7);
    let mut cd = view.today_date.to_string();
    for _ in 0..7 {
        week_days.push((cd.clone(), view.stats.day_minutes(&cd, view.today_date)));
        cd = prev_iso_date(&cd);
    }
    week_days.reverse();
    let max_bar_mins = week_days.iter().map(|(_, m)| *m).max().unwrap_or(goal_mins).max(goal_mins).max(60) as f32;
    let slot_gap = 10.0;
    let slot_area_x = trend_card.x + 14.0;
    let slot_area_w = trend_card.w - 28.0;
    let slot_w = (slot_area_w - slot_gap * 6.0) / 7.0;
    let bar_top = trend_card.y + 12.0;
    let bar_h_max = 94.0;
    for (idx, (date_s, mins)) in week_days.iter().enumerate() {
        let sx = slot_area_x + (slot_w + slot_gap) * idx as f32;
        let track_r = RectF { x: sx, y: bar_top, w: slot_w, h: bar_h_max };
        draw_rounded_card(dc, track_r, 5.0, rgba(44, 44, 49, 0.72), rgba(0, 0, 0, 0.0), 0.0)?;
        if *mins > 0 {
            let bh = ((*mins as f32 / max_bar_mins) * bar_h_max).max(16.0);
            let by = bar_top + bar_h_max - bh;
            let met = DailyStats::is_goal_met_with_tolerance(*mins, goal_mins);
            let bar_col = if met { hex_color(accent, 0.96) } else { rgba(110, 114, 126, 0.88) };
            draw_rounded_card(dc, RectF { x: sx, y: by, w: slot_w, h: bh }, 5.0, bar_col, rgba(0, 0, 0, 0.0), 0.0)?;
            draw_text_nowrap(
                dc,
                dwrite,
                &format!("{mins}m"),
                RectF { x: sx + 2.0, y: by + 2.0, w: slot_w - 4.0, h: 15.0 },
                8.8,
                DWRITE_FONT_WEIGHT_BOLD,
                contrast_fg(if met { accent } else { "#6E727E" }),
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;
        }
        let lbl = if idx == 6 { "Today" } else { weekday_short(date_s) };
        draw_text_nowrap(
            dc,
            dwrite,
            lbl,
            RectF { x: sx, y: bar_top + bar_h_max + 5.0, w: slot_w, h: 15.0 },
            9.2,
            DWRITE_FONT_WEIGHT_MEDIUM,
            rgba(160, 165, 178, 0.92),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }

    // 4. 34-Week GitHub-style Contribution Activity Heatmap (matching analytics_heatmap.png)
    let y3 = y2 + 158.0 + 12.0;
    let cols = 34usize;
    let dow_today = weekday_index(view.today_date); // 0=Sun..6=Sat
    let mut end_sat = view.today_date.to_string();
    for _ in 0..(6 - dow_today) {
        end_sat = next_iso_date(&end_sat);
    }
    for _ in 0..(view.heatmap_offset_months * 84) {
        end_sat = prev_iso_date(&end_sat);
    }
    let total_cells = cols * 7;
    let mut cell_dates = vec![String::new(); total_cells];
    let mut cell_mins = vec![0u32; total_cells];
    let mut hcur = end_sat.clone();
    let mut heat_total_mins = 0u32;
    let mut heat_active_days = 0u32;
    for i in (0..total_cells).rev() {
        let m = view.stats.day_minutes(&hcur, view.today_date);
        heat_total_mins += m;
        if m > 0 {
            heat_active_days += 1;
        }
        cell_dates[i] = hcur.clone();
        cell_mins[i] = m;
        hcur = prev_iso_date(&hcur);
    }
    let start_date_str = cell_dates.first().cloned().unwrap_or_default();

    draw_text_nowrap(
        dc,
        dwrite,
        &format!(
            "{:.1} hours focused in 34 weeks ({} active days)",
            heat_total_mins as f32 / 60.0,
            heat_active_days
        ),
        RectF { x: mc.x + 2.0, y: y3 + 2.0, w: 360.0, h: 18.0 },
        11.2,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(242, 242, 248, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_subtle_button(dc, dwrite, find_hit(layout, ControlId::BtnHeatmapPrev), "‹", view.hovered == Some(ControlId::BtnHeatmapPrev))?;
    draw_subtle_button(dc, dwrite, find_hit(layout, ControlId::BtnHeatmapNext), "›", view.hovered == Some(ControlId::BtnHeatmapNext))?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("{} — {}", date_month_year(&start_date_str), date_month_year(&end_sat)),
        RectF { x: mc.x + w - 134.0, y: y3 + 2.0, w: 108.0, h: 20.0 },
        9.4,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(175, 180, 194, 0.92),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )?;

    let heat_card = RectF { x: mc.x, y: y3 + 26.0, w, h: 146.0 };
    draw_rounded_card(dc, heat_card, 8.0, rgba(22, 22, 26, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;

    let label_left = 34.0;
    let grid_x0 = heat_card.x + label_left + 8.0;
    let grid_w = (heat_card.w - label_left - 22.0).max(200.0);
    let col_step = grid_w / cols as f32;
    let cell_s = (col_step * 0.68).clamp(8.0, 13.0);
    let row_step = cell_s + 3.0;
    let gy0 = heat_card.y + 24.0;

    let cursor_in_mc = view.cursor_dip.filter(|&(cx, cy)| mc.contains(cx, cy));
    let mut hovered_heat: Option<(String, u32, RectF)> = None;
    let mut last_mon = "";
    for c in 0..cols {
        let top_date = &cell_dates[c * 7];
        let m_lbl = month_short(top_date);
        if m_lbl != last_mon {
            last_mon = m_lbl;
            draw_text_nowrap(
                dc,
                dwrite,
                m_lbl,
                RectF { x: grid_x0 + c as f32 * col_step, y: heat_card.y + 6.0, w: 32.0, h: 14.0 },
                8.6,
                DWRITE_FONT_WEIGHT_NORMAL,
                rgba(155, 160, 174, 0.88),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        }
        for r in 0..7 {
            let mins = cell_mins[c * 7 + r];
            let cell_r = RectF {
                x: grid_x0 + c as f32 * col_step,
                y: gy0 + r as f32 * row_step,
                w: cell_s,
                h: cell_s,
            };
            let hit_r = RectF {
                x: cell_r.x - 1.5,
                y: cell_r.y - 1.5,
                w: col_step,
                h: row_step,
            };
            let is_cell_hov = cursor_in_mc.is_some_and(|(cx, cy)| hit_r.contains(cx, cy));
            if is_cell_hov {
                hovered_heat = Some((cell_dates[c * 7 + r].clone(), mins, cell_r));
            }
            let fill = if mins == 0 {
                rgba(42, 42, 48, 0.78)
            } else if mins >= goal_mins || mins >= 120 {
                hex_color(accent, 0.98)
            } else if mins >= 60 {
                hex_color(accent, 0.72)
            } else if mins >= 30 {
                hex_color(accent, 0.46)
            } else {
                hex_color(accent, 0.24)
            };
            draw_rounded_card(
                dc,
                cell_r,
                2.5,
                fill,
                if is_cell_hov { rgba(248, 250, 255, 0.92) } else { rgba(255, 255, 255, 0.04) },
                if is_cell_hov { 1.4 } else { 0.5 },
            )?;
        }
    }
    for (r, d_lbl) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
        draw_text_nowrap(
            dc,
            dwrite,
            d_lbl,
            RectF { x: heat_card.x + 10.0, y: gy0 + r as f32 * row_step - 1.0, w: 26.0, h: cell_s + 2.0 },
            8.4,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(150, 155, 168, 0.86),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
    }
    let leg_y = gy0 + 7.0 * row_step + 4.0;
    let leg_x = heat_card.x + heat_card.w - 130.0;
    draw_text_nowrap(dc, dwrite, "Less", RectF { x: leg_x, y: leg_y, w: 28.0, h: 12.0 }, 8.4, DWRITE_FONT_WEIGHT_NORMAL, rgba(150, 155, 168, 0.85), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    for (li, col) in [
        rgba(42, 42, 48, 0.78),
        hex_color(accent, 0.24),
        hex_color(accent, 0.46),
        hex_color(accent, 0.72),
        hex_color(accent, 0.98),
    ]
    .into_iter()
    .enumerate()
    {
        draw_rounded_card(dc, RectF { x: leg_x + 28.0 + li as f32 * 13.0, y: leg_y + 1.0, w: 10.0, h: 10.0 }, 2.0, col, rgba(0, 0, 0, 0.0), 0.0)?;
    }
    draw_text_nowrap(dc, dwrite, "More", RectF { x: leg_x + 95.0, y: leg_y, w: 32.0, h: 12.0 }, 8.4, DWRITE_FONT_WEIGHT_NORMAL, rgba(150, 155, 168, 0.85), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    // 5. 24-Hour Productivity Flow Smooth Curve (matching productivity_flow_breakdown.png)
    let y4 = y3 + 172.0 + 14.0;
    let mut hourly_secs = [0u32; 24];
    let mut h_cursor = end_date.clone();
    for _ in 0..days_count {
        if let Some(arr) = view.stats.hourly_history.get(&h_cursor) {
            for (h_i, &v) in arr.iter().enumerate() {
                hourly_secs[h_i] += v;
            }
        }
        h_cursor = prev_iso_date(&h_cursor);
    }
    let mut hourly_mins = [0u32; 24];
    for i in 0..24 {
        hourly_mins[i] = if hourly_secs[i] > 0 { (hourly_secs[i] / 60).max(1) } else { 0 };
    }
    let mut best_win_h = 0usize;
    let mut best_win_sum = 0u32;
    for h in 0..24 {
        let sum = hourly_mins[h] + hourly_mins[(h + 1) % 24];
        if sum > best_win_sum {
            best_win_sum = sum;
            best_win_h = h;
        }
    }
    let total_flow_mins: u32 = hourly_mins.iter().sum();
    let (persona, p_col, insight_text) = evaluate_chronotype(&hourly_mins, best_win_h, total_flow_mins);

    draw_text_nowrap(
        dc,
        dwrite,
        &format!("24-Hour Productivity Flow  •  {}", view.stats_period.label()),
        RectF { x: mc.x + 2.0, y: y4 + 2.0, w: 340.0, h: 18.0 },
        11.6,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(242, 242, 248, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    let with_alpha = |c: D2D1_COLOR_F, a: f32| D2D1_COLOR_F { a, ..c };
    let badge_w = (measure_text_width(dwrite, persona, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD) + 26.0).clamp(110.0, 210.0);
    let badge_r = RectF { x: mc.x + w - badge_w, y: y4, w: badge_w, h: 22.0 };
    draw_rounded_card(dc, badge_r, 11.0, with_alpha(p_col, 0.14), with_alpha(p_col, 0.45), 1.0)?;
    draw_text_nowrap(dc, dwrite, persona, badge_r, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, p_col, DWRITE_TEXT_ALIGNMENT_CENTER)?;

    let flow_card = RectF { x: mc.x, y: y4 + 28.0, w, h: 150.0 };
    draw_rounded_card(dc, flow_card, 8.0, rgba(22, 22, 26, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
    let f_area = RectF { x: flow_card.x + 20.0, y: flow_card.y + 18.0, w: flow_card.w - 40.0, h: 78.0 };
    let max_hs = (*hourly_secs.iter().max().unwrap_or(&60)).max(60) as f32;
    let pts: Vec<Vector2> = (0..24)
        .map(|h| {
            let x = f_area.x + (h as f32 / 23.0) * f_area.w;
            let y = f_area.y + f_area.h - (hourly_secs[h] as f32 / max_hs) * (f_area.h - 10.0);
            pt(x, y)
        })
        .collect();

    // Build smooth cubic Bezier curve through all 24 points
    let flow_fill = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.18), None) }?;
    let flow_stroke = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.96), None) }?;
    let area_geo: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let line_geo: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let a_sink = unsafe { area_geo.Open() }?;
    let l_sink = unsafe { line_geo.Open() }?;
    unsafe {
        a_sink.BeginFigure(pt(f_area.x, f_area.y + f_area.h), D2D1_FIGURE_BEGIN_FILLED);
        a_sink.AddLine(pts[0]);
        l_sink.BeginFigure(pts[0], D2D1_FIGURE_BEGIN_HOLLOW);
        for i in 0..(pts.len() - 1) {
            let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
            let p1 = pts[i];
            let p2 = pts[i + 1];
            let p3 = if i + 2 < pts.len() { pts[i + 2] } else { pts[i + 1] };
            let c1 = pt(p1.X + (p2.X - p0.X) / 6.0, (p1.Y + (p2.Y - p0.Y) / 6.0).min(f_area.y + f_area.h));
            let c2 = pt(p2.X - (p3.X - p1.X) / 6.0, (p2.Y - (p3.Y - p1.Y) / 6.0).min(f_area.y + f_area.h));
            let seg = D2D1_BEZIER_SEGMENT { point1: c1, point2: c2, point3: p2 };
            a_sink.AddBezier(&seg);
            l_sink.AddBezier(&seg);
        }
        a_sink.AddLine(pt(f_area.x + f_area.w, f_area.y + f_area.h));
        a_sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        l_sink.EndFigure(D2D1_FIGURE_END_OPEN);
        a_sink.Close()?;
        l_sink.Close()?;
        dc.FillGeometry(&area_geo, &flow_fill, None);
        dc.DrawGeometry(&line_geo, &flow_stroke, 2.2, None);
    }

    let (peak_idx, &peak_sec_val) = hourly_secs.iter().enumerate().max_by_key(|&(_, s)| s).unwrap_or((view.cur_hour.min(23), &0));
    if peak_sec_val > 0 {
        let pk = pts[peak_idx];
        let halo_b = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.28), None) }?;
        let dot_b = unsafe { dc.CreateSolidColorBrush(&rgba(245, 250, 255, 0.98), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: pk, radiusX: 8.5, radiusY: 8.5 }, &halo_b);
            dc.DrawEllipse(&D2D1_ELLIPSE { point: pk, radiusX: 4.5, radiusY: 4.5 }, &flow_stroke, 2.0, None);
            dc.FillEllipse(&D2D1_ELLIPSE { point: pk, radiusX: 3.0, radiusY: 3.0 }, &dot_b);
        }
    }

    let flow_hover_rect = RectF {
        x: f_area.x - 8.0,
        y: f_area.y - 8.0,
        w: f_area.w + 16.0,
        h: f_area.h + 18.0,
    };
    let hovered_flow: Option<(usize, u32, Vector2)> = cursor_in_mc
        .filter(|&(cx, cy)| flow_hover_rect.contains(cx, cy))
        .map(|(cx, _)| {
            let h_idx = (((cx - f_area.x) / f_area.w.max(1.0)) * 23.0)
                .round()
                .clamp(0.0, 23.0) as usize;
            (h_idx, hourly_secs[h_idx], pts[h_idx])
        });
    if let Some((_, _, hp)) = hovered_flow {
        let guide_b = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.22), None) }?;
        let h_halo = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.36), None) }?;
        let h_dot = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.98), None) }?;
        unsafe {
            dc.DrawLine(pt(hp.X, f_area.y), pt(hp.X, f_area.y + f_area.h), &guide_b, 1.0, None);
            dc.FillEllipse(&D2D1_ELLIPSE { point: hp, radiusX: 8.0, radiusY: 8.0 }, &h_halo);
            dc.FillEllipse(&D2D1_ELLIPSE { point: hp, radiusX: 4.0, radiusY: 4.0 }, &h_dot);
        }
    }

    for (idx, lbl) in [(0, "00:00"), (4, "04:00"), (8, "08:00"), (12, "12:00"), (16, "16:00"), (20, "20:00"), (23, "24:00")] {
        let lx = (f_area.x + (idx as f32 / 23.0) * f_area.w - 18.0).clamp(flow_card.x + 4.0, flow_card.x + flow_card.w - 40.0);
        draw_text_nowrap(dc, dwrite, lbl, RectF { x: lx, y: f_area.y + f_area.h + 4.0, w: 36.0, h: 14.0 }, 8.6, DWRITE_FONT_WEIGHT_NORMAL, rgba(155, 160, 174, 0.88), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    draw_text_nowrap(
        dc,
        dwrite,
        &insight_text,
        RectF { x: flow_card.x + 14.0, y: flow_card.y + flow_card.h - 24.0, w: flow_card.w - 28.0, h: 16.0 },
        9.4,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(182, 188, 200, 0.92),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    // 6. Application & Tool Breakdown (real icons, clickable PRODUCTIVE/NEUTRAL/DISTRACTING pills, Ranked by focus time)
    let y5 = y4 + 178.0 + 14.0;
    draw_text_nowrap(dc, dwrite, "Application & Tool Breakdown", RectF { x: mc.x + 2.0, y: y5 + 2.0, w: 260.0, h: 18.0 }, 11.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "Ranked by focus time (click category to switch)", RectF { x: mc.x + w - 280.0, y: y5 + 3.0, w: 278.0, h: 16.0 }, 9.4, DWRITE_FONT_WEIGHT_NORMAL, rgba(150, 155, 168, 0.88), DWRITE_TEXT_ALIGNMENT_TRAILING)?;

    let shown = view.stats_apps.len().min(12);
    let total_secs_all: u32 = view.stats_apps.iter().map(|a| a.secs).sum::<u32>().max(1);
    let max_app_secs: u32 = view.stats_apps.first().map(|a| a.secs).unwrap_or(1).max(1);
    if shown == 0 {
        let empty_card = RectF { x: mc.x, y: y5 + 32.0, w, h: 46.0 };
        draw_rounded_card(dc, empty_card, 8.0, rgba(32, 32, 37, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
        draw_text_nowrap(dc, dwrite, "No tracked app usage recorded in this period yet.", RectF { x: empty_card.x + 14.0, y: empty_card.y + 12.0, w: empty_card.w - 28.0, h: 22.0 }, 10.6, DWRITE_FONT_WEIGHT_NORMAL, rgba(155, 160, 174, 0.85), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    } else {
        for (i, app) in view.stats_apps.iter().take(12).enumerate() {
            let row_y = y5 + 32.0 + i as f32 * 54.0;
            let row_card = RectF { x: mc.x, y: row_y, w, h: 46.0 };
            draw_rounded_card(dc, row_card, 8.0, rgba(32, 32, 37, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;

            // App Icon (real extracted bitmap or colorful monogram avatar)
            let ico_s = 20.0;
            let ico_r = RectF { x: row_card.x + 12.0, y: row_card.y + 7.0, w: ico_s, h: ico_s };
            if let Some(bmp) = app.icon {
                unsafe {
                    dc.DrawBitmap(bmp, Some(&ico_r.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
                }
            } else {
                let (tile_bg, tile_border) = app_monogram_color(app.exe_name);
                draw_rounded_card(dc, ico_r, 5.0, tile_bg, tile_border, 1.0)?;
                let initial = app.exe_name.chars().find(|c| c.is_alphanumeric()).unwrap_or('A').to_ascii_uppercase().to_string();
                draw_text_nowrap(
                    dc,
                    dwrite,
                    &initial,
                    RectF { x: ico_r.x, y: ico_r.y - 0.5, w: ico_r.w, h: ico_r.h },
                    10.5,
                    DWRITE_FONT_WEIGHT_BOLD,
                    rgba(255, 255, 255, 0.96),
                    DWRITE_TEXT_ALIGNMENT_CENTER,
                )?;
            }

            let exe_w = measure_text_width(dwrite, app.exe_name, 11.2, DWRITE_FONT_WEIGHT_SEMI_BOLD).min(w - 300.0);
            draw_text_nowrap(
                dc,
                dwrite,
                app.exe_name,
                RectF { x: row_card.x + 36.0, y: row_card.y + 7.0, w: exe_w + 6.0, h: 18.0 },
                11.2,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                rgba(244, 244, 250, 0.96),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;

            // Clickable Category Pill (● PRODUCTIVE / ● NEUTRAL / ● DISTRACTING)
            let (cat_col, cat_bg, cat_lbl) = match app.category {
                "productive" => (rgba(52, 211, 153, 0.98), rgba(16, 185, 129, 0.14), "PRODUCTIVE"),
                "distracting" => (rgba(248, 113, 113, 0.98), rgba(239, 68, 68, 0.14), "DISTRACTING"),
                _ => (rgba(251, 191, 36, 0.98), rgba(245, 158, 11, 0.14), "NEUTRAL"),
            };
            let cat_hovered = view.hovered == Some(ControlId::BtnCycleAppCategory(i));
            let pill_w = if app.category == "neutral" { 74.0 } else { 88.0 };
            let pill_r = RectF {
                x: row_card.x + 36.0 + exe_w + 10.0,
                y: row_card.y + 8.0,
                w: pill_w,
                h: 17.0,
            };
            draw_rounded_card(
                dc,
                pill_r,
                8.5,
                if cat_hovered { with_alpha(cat_col, 0.24) } else { cat_bg },
                with_alpha(cat_col, if cat_hovered { 0.85 } else { 0.45 }),
                1.0,
            )?;
            let dot_b = unsafe { dc.CreateSolidColorBrush(&cat_col, None) }?;
            unsafe {
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(pill_r.x + 8.0, pill_r.y + 8.5), radiusX: 2.6, radiusY: 2.6 }, &dot_b);
            }
            draw_text_nowrap(
                dc,
                dwrite,
                cat_lbl,
                RectF { x: pill_r.x + 13.0, y: pill_r.y, w: pill_r.w - 16.0, h: pill_r.h },
                8.2,
                DWRITE_FONT_WEIGHT_BOLD,
                cat_col,
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;

            // Time + share % + Block button
            let share_pct = (app.secs as f32 / total_secs_all as f32 * 100.0).round();
            draw_text_nowrap(
                dc,
                dwrite,
                &format!("{} ({:.0}%)", format_secs_compact(app.secs), share_pct),
                RectF { x: row_card.x + row_card.w - 182.0, y: row_card.y + 8.0, w: 96.0, h: 17.0 },
                10.2,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                hex_color(accent, 0.96),
                DWRITE_TEXT_ALIGNMENT_TRAILING,
            )?;
            let btn_id = ControlId::BtnBlockAppFromStats(i);
            draw_segment_button(
                dc,
                dwrite,
                find_hit(layout, btn_id),
                if app.is_blocked { "Blocked" } else { "Block" },
                app.is_blocked,
                view.hovered == Some(btn_id),
                if app.is_blocked { "#EF4444" } else { accent },
            )?;

            // Full-width bottom progress bar
            let rel = app.secs as f32 / max_app_secs as f32;
            let prog_bg = RectF { x: row_card.x + 12.0, y: row_card.y + 34.0, w: row_card.w - 24.0, h: 4.5 };
            draw_rounded_card(dc, prog_bg, 2.2, rgba(20, 20, 26, 0.92), rgba(0, 0, 0, 0.0), 0.0)?;
            draw_rounded_card(
                dc,
                RectF { x: prog_bg.x, y: prog_bg.y, w: (prog_bg.w * rel).max(6.0), h: prog_bg.h },
                2.2,
                hex_color(accent, 0.94),
                rgba(0, 0, 0, 0.0),
                0.0,
            )?;
        }
    }

    // Floating hover tooltip on Contribution Activity heatmap square
    if let Some((date_s, mins, cell_r)) = hovered_heat {
        let hrs = mins as f32 / 60.0;
        let line1 = format_full_date_label(&date_s);
        let line2 = if mins > 0 {
            format!("{hrs:.1} hrs  ({})", format_secs_compact(mins * 60))
        } else {
            "0.0 hrs  (No focus logged)".to_string()
        };
        let tw = measure_text_width(dwrite, &line1, 10.0, DWRITE_FONT_WEIGHT_SEMI_BOLD)
            .max(measure_text_width(dwrite, &line2, 9.8, DWRITE_FONT_WEIGHT_NORMAL))
            + 20.0;
        let th = 42.0;
        let tx = (cell_r.x + cell_r.w * 0.5 - tw * 0.5).clamp(mc.x + 4.0, mc.x + mc.w - tw - 4.0);
        let ty = if cell_r.y - th - 6.0 >= mc.y + 2.0 {
            cell_r.y - th - 6.0
        } else {
            (cell_r.y + cell_r.h + 6.0).min(mc.y + mc.h - th - 2.0)
        };
        let tip = RectF { x: tx, y: ty, w: tw, h: th };
        draw_rounded_card(dc, tip, 8.0, rgba(16, 17, 22, 0.97), hex_color(accent, 0.65), 1.0)?;
        draw_text_nowrap(dc, dwrite, &line1, RectF { x: tip.x + 10.0, y: tip.y + 5.0, w: tip.w - 20.0, h: 15.0 }, 10.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(244, 246, 252, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
        draw_text_nowrap(dc, dwrite, &line2, RectF { x: tip.x + 10.0, y: tip.y + 21.0, w: tip.w - 20.0, h: 15.0 }, 9.8, DWRITE_FONT_WEIGHT_BOLD, if mins > 0 { hex_color(accent, 0.98) } else { rgba(160, 165, 178, 0.88) }, DWRITE_TEXT_ALIGNMENT_LEADING)?;
    }

    // Floating hover tooltip on 24-Hour Productivity Flow curve (with small focused app icons!)
    if let Some((h_idx, h_secs, hp)) = hovered_flow {
        let top_apps = view.stats.period_hourly_apps(&end_date, days_count, h_idx);
        let shown_apps: Vec<&(String, u32)> = top_apps.iter().take(4).collect();
        let row_cnt = shown_apps.len().max(1);
        let tw = 224.0;
        let th = 32.0 + row_cnt as f32 * 20.0 + 6.0;
        let tx = if hp.X + 14.0 + tw <= mc.x + mc.w - 4.0 {
            hp.X + 14.0
        } else {
            (hp.X - tw - 14.0).max(mc.x + 4.0)
        };
        let ty = (hp.Y - th * 0.5).clamp(mc.y + 4.0, mc.y + mc.h - th - 4.0);
        let tip = RectF { x: tx, y: ty, w: tw, h: th };
        draw_rounded_card(dc, tip, 9.0, rgba(16, 17, 22, 0.97), hex_color(accent, 0.72), 1.0)?;

        let next_h = (h_idx + 1) % 24;
        draw_text_nowrap(
            dc,
            dwrite,
            &format!("{:02}:00 — {:02}:00", h_idx, next_h),
            RectF { x: tip.x + 10.0, y: tip.y + 6.0, w: 110.0, h: 16.0 },
            10.2,
            DWRITE_FONT_WEIGHT_BOLD,
            rgba(244, 246, 252, 0.98),
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            &format_secs_compact(h_secs),
            RectF { x: tip.x + tip.w - 86.0, y: tip.y + 6.0, w: 76.0, h: 16.0 },
            10.2,
            DWRITE_FONT_WEIGHT_BOLD,
            hex_color(accent, 0.98),
            DWRITE_TEXT_ALIGNMENT_TRAILING,
        )?;
        draw_rounded_card(dc, RectF { x: tip.x + 10.0, y: tip.y + 25.0, w: tip.w - 20.0, h: 1.0 }, 0.5, rgba(255, 255, 255, 0.08), rgba(0, 0, 0, 0.0), 0.0)?;

        if shown_apps.is_empty() {
            let empty_msg = if h_secs > 0 {
                "No per-hour app details saved"
            } else {
                "No focus activity this hour"
            };
            draw_text_nowrap(
                dc,
                dwrite,
                empty_msg,
                RectF { x: tip.x + 10.0, y: tip.y + 30.0, w: tip.w - 20.0, h: 18.0 },
                9.5,
                DWRITE_FONT_WEIGHT_NORMAL,
                rgba(155, 160, 175, 0.88),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
        } else {
            for (ai, (exe, a_secs)) in shown_apps.iter().enumerate() {
                let ry = tip.y + 30.0 + ai as f32 * 20.0;
                let ico_r = RectF { x: tip.x + 10.0, y: ry + 2.0, w: 14.0, h: 14.0 };
                let low = exe.to_ascii_lowercase();
                let icon_bmp = if low == "focusgrow.exe" || low == "focusgrow-app.exe" {
                    view.app_icon_logo
                } else {
                    view.app_icons_map.get(&low)
                };
                if let Some(bmp) = icon_bmp {
                    unsafe {
                        dc.DrawBitmap(bmp, Some(&ico_r.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
                    }
                } else {
                    draw_rounded_card(dc, ico_r, 3.0, rgba(58, 60, 72, 0.92), rgba(255, 255, 255, 0.14), 0.5)?;
                    let ch = exe.chars().next().unwrap_or('A').to_ascii_uppercase().to_string();
                    draw_text_nowrap(dc, dwrite, &ch, ico_r, 8.2, DWRITE_FONT_WEIGHT_BOLD, rgba(236, 238, 246, 0.94), DWRITE_TEXT_ALIGNMENT_CENTER)?;
                }
                draw_text_nowrap(
                    dc,
                    dwrite,
                    exe,
                    RectF { x: tip.x + 29.0, y: ry + 1.0, w: tip.w - 92.0, h: 16.0 },
                    9.8,
                    DWRITE_FONT_WEIGHT_MEDIUM,
                    rgba(232, 235, 244, 0.95),
                    DWRITE_TEXT_ALIGNMENT_LEADING,
                )?;
                draw_text_nowrap(
                    dc,
                    dwrite,
                    &format_secs_compact(*a_secs),
                    RectF { x: tip.x + tip.w - 64.0, y: ry + 1.0, w: 54.0, h: 16.0 },
                    9.4,
                    DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    rgba(170, 176, 190, 0.92),
                    DWRITE_TEXT_ALIGNMENT_TRAILING,
                )?;
            }
        }
    }

    unsafe {
        dc.PopAxisAlignedClip();
    }
    Ok(())
}

struct ShareCardSummary {
    date_label: String,
    total_secs: u32,
    eff_pct: u32,
    goal_pct: u32,
    goal_hours: f32,
    streak_days: u32,
    window_str: String,
    top_apps: Vec<(String, u32)>,
}

fn compute_share_summary(view: &RenderView<'_>, yesterday: bool) -> ShareCardSummary {
    let target_date = if yesterday {
        prev_iso_date(view.today_date)
    } else {
        view.today_date.to_string()
    };
    let mut apps: Vec<(String, u32)> = view
        .stats
        .app_history
        .get(&target_date)
        .map(|m| m.iter().filter(|(_, &s)| s > 0).map(|(k, &v)| (k.clone(), v)).collect())
        .unwrap_or_default();
    apps.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let app_secs_total: u32 = apps.iter().map(|(_, s)| *s).sum();
    let day_mins = view.stats.day_minutes(&target_date, view.today_date);
    let total_secs = app_secs_total.max(day_mins * 60);

    let mut prod_secs = 0u64;
    let mut neutral_secs = 0u64;
    for (exe, secs) in &apps {
        let cat = view.config.app_category(exe);
        if cat == "productive" {
            prod_secs += *secs as u64;
        } else if cat == "neutral" {
            neutral_secs += *secs as u64;
        }
    }
    let eff_pct = if app_secs_total > 0 {
        (((prod_secs as f64 + neutral_secs as f64 * 0.5) / app_secs_total as f64) * 100.0)
            .round()
            .min(100.0) as u32
    } else {
        100
    };

    let goal_mins = view.config.daily_goal_minutes.max(1);
    let total_mins = (total_secs / 60).max(day_mins);
    let goal_pct = ((total_mins as f32 / goal_mins as f32) * 100.0).round().min(100.0) as u32;
    let window_str = view
        .stats
        .work_windows
        .get(&target_date)
        .filter(|(a, b)| !a.is_empty() && !b.is_empty())
        .map(|(a, b)| format!("{a} — {b}"))
        .unwrap_or_else(|| {
            if yesterday {
                "No logged session".to_string()
            } else {
                "Active session".to_string()
            }
        });

    ShareCardSummary {
        date_label: format_full_date_label(&target_date),
        total_secs,
        eff_pct,
        goal_pct,
        goal_hours: goal_mins as f32 / 60.0,
        streak_days: view.streak_days,
        window_str,
        top_apps: apps.into_iter().take(3).collect(),
    }
}

pub fn build_share_report_text(view: &RenderView<'_>, yesterday: bool) -> String {
    let s = compute_share_summary(view, yesterday);
    let mut app_lines = String::new();
    for (idx, (exe, secs)) in s.top_apps.iter().enumerate() {
        let pct = if s.total_secs > 0 {
            ((*secs as f32 / s.total_secs as f32) * 100.0).round() as u32
        } else {
            0
        };
        app_lines.push_str(&format!("\n  {}. {} — {} ({}%)", idx + 1, exe, format_secs_compact(*secs), pct));
    }
    if app_lines.is_empty() {
        app_lines = "\n  No focus apps logged yet.".to_string();
    }
    format!(
        "FocusGrow Productivity Report — {}\nTotal Focus: {}\nFocus Score: {}% | Goal Met: {}% ({:.1} hrs)\nStreak: {} Days\nActive Window: {}\nTop Applications:{}\n\n#StayFocused #FocusGrow",
        s.date_label,
        format_secs_compact(s.total_secs),
        s.eff_pct,
        s.goal_pct,
        s.goal_hours,
        s.streak_days,
        s.window_str,
        app_lines
    )
}

fn draw_share_card_content(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    card_r: RectF,
    view: &RenderView<'_>,
    summary: &ShareCardSummary,
) -> Result<()> {
    let accent = view.effective_accent;
    draw_rounded_card(dc, card_r, 14.0, rgba(14, 18, 28, 1.0), hex_color(accent, 0.35), 1.2)?;

    // Ambient neon corner glows
    let glow_tr = unsafe { dc.CreateSolidColorBrush(&hex_color(accent, 0.14), None) }?;
    let glow_bl = unsafe { dc.CreateSolidColorBrush(&rgba(16, 185, 129, 0.12), None) }?;
    unsafe {
        dc.FillEllipse(&D2D1_ELLIPSE { point: pt(card_r.x + card_r.w - 40.0, card_r.y + 36.0), radiusX: 84.0, radiusY: 54.0 }, &glow_tr);
        dc.FillEllipse(&D2D1_ELLIPSE { point: pt(card_r.x + 52.0, card_r.y + card_r.h - 32.0), radiusX: 90.0, radiusY: 54.0 }, &glow_bl);
    }

    let pad = 20.0;
    let inner_w = card_r.w - pad * 2.0;

    // Header: logo + FocusGrow / Daily Focus Intelligence + Date Pill
    if let Some(logo) = view.app_icon_logo {
        let lr = RectF { x: card_r.x + pad, y: card_r.y + 18.0, w: 26.0, h: 26.0 };
        unsafe {
            dc.DrawBitmap(logo, Some(&lr.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
        }
    }
    draw_text_nowrap(dc, dwrite, "FocusGrow", RectF { x: card_r.x + pad + 34.0, y: card_r.y + 15.0, w: 140.0, h: 18.0 }, 13.5, DWRITE_FONT_WEIGHT_BOLD, rgba(246, 248, 255, 0.98), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "Daily Focus Intelligence", RectF { x: card_r.x + pad + 34.0, y: card_r.y + 32.0, w: 160.0, h: 14.0 }, 9.6, DWRITE_FONT_WEIGHT_NORMAL, rgba(160, 170, 188, 0.9), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let day_tag = if view.share_yesterday {
        format!("Yesterday  •  {}", summary.date_label)
    } else {
        summary.date_label.clone()
    };
    let pill_w = (measure_text_width(dwrite, &day_tag, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD) + 22.0).clamp(110.0, 210.0);
    let date_pill = RectF { x: card_r.x + card_r.w - pad - pill_w, y: card_r.y + 18.0, w: pill_w, h: 24.0 };
    draw_rounded_card(dc, date_pill, 12.0, rgba(30, 36, 52, 0.95), hex_color(accent, 0.45), 1.0)?;
    draw_text_nowrap(dc, dwrite, &day_tag, date_pill, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, hex_color(accent, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    // Hero Time + Score/Streak Badges
    let hero_y = card_r.y + 62.0;
    draw_text_nowrap(dc, dwrite, "TOTAL FOCUS TIME", RectF { x: card_r.x + pad, y: hero_y, w: 160.0, h: 13.0 }, 9.2, DWRITE_FONT_WEIGHT_BOLD, rgba(148, 160, 180, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    let hero_time = if summary.total_secs > 0 {
        format_secs_compact(summary.total_secs)
    } else {
        "0m".to_string()
    };
    draw_text_nowrap(dc, dwrite, &hero_time, RectF { x: card_r.x + pad, y: hero_y + 14.0, w: 220.0, h: 36.0 }, 29.0, DWRITE_FONT_WEIGHT_BOLD, rgba(248, 250, 255, 0.99), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    let b1 = RectF { x: card_r.x + card_r.w - pad - 150.0, y: hero_y + 2.0, w: 150.0, h: 22.0 };
    let b2 = RectF { x: card_r.x + card_r.w - pad - 150.0, y: hero_y + 28.0, w: 150.0, h: 22.0 };
    draw_rounded_card(dc, b1, 11.0, rgba(16, 185, 129, 0.14), rgba(52, 211, 153, 0.45), 1.0)?;
    draw_text_nowrap(dc, dwrite, &format!("{}% Focus Score", summary.eff_pct), b1, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(52, 211, 153, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    draw_rounded_card(dc, b2, 11.0, rgba(245, 158, 11, 0.14), rgba(251, 191, 36, 0.45), 1.0)?;
    draw_text_nowrap(dc, dwrite, &format!("{} Days Streak", summary.streak_days), b2, 9.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(251, 191, 36, 0.98), DWRITE_TEXT_ALIGNMENT_CENTER)?;

    // Goal Progress & Work Window Box
    let prog_y = hero_y + 62.0;
    let prog_box = RectF { x: card_r.x + pad, y: prog_y, w: inner_w, h: 46.0 };
    draw_rounded_card(dc, prog_box, 9.0, rgba(24, 30, 44, 0.92), rgba(255, 255, 255, 0.08), 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("Goal Met: {}% ({:.1} hrs)", summary.goal_pct, summary.goal_hours),
        RectF { x: prog_box.x + 12.0, y: prog_box.y + 6.0, w: inner_w * 0.48, h: 15.0 },
        9.6,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(232, 236, 246, 0.95),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    draw_text_nowrap(
        dc,
        dwrite,
        &format!("Window: {}", summary.window_str),
        RectF { x: prog_box.x + inner_w * 0.48, y: prog_box.y + 6.0, w: inner_w * 0.52 - 12.0, h: 15.0 },
        9.4,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(165, 175, 192, 0.90),
        DWRITE_TEXT_ALIGNMENT_TRAILING,
    )?;
    let bar_r = RectF { x: prog_box.x + 12.0, y: prog_box.y + 27.0, w: inner_w - 24.0, h: 8.0 };
    draw_rounded_card(dc, bar_r, 4.0, rgba(38, 45, 62, 0.95), rgba(0, 0, 0, 0.0), 0.0)?;
    let fill_w = (bar_r.w * (summary.goal_pct as f32 / 100.0)).max(if summary.goal_pct > 0 { 8.0 } else { 0.0 });
    if fill_w > 0.0 {
        draw_rounded_card(dc, RectF { x: bar_r.x, y: bar_r.y, w: fill_w, h: bar_r.h }, 4.0, hex_color(accent, 0.96), rgba(0, 0, 0, 0.0), 0.0)?;
    }

    // Top 3 Focus Applications
    let apps_y = prog_y + 58.0;
    draw_text_nowrap(dc, dwrite, "TOP FOCUS APPLICATIONS", RectF { x: card_r.x + pad, y: apps_y, w: 220.0, h: 14.0 }, 9.2, DWRITE_FONT_WEIGHT_BOLD, rgba(148, 160, 180, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    if summary.top_apps.is_empty() {
        let er = RectF { x: card_r.x + pad, y: apps_y + 20.0, w: inner_w, h: 36.0 };
        draw_rounded_card(dc, er, 8.0, rgba(24, 30, 44, 0.88), rgba(255, 255, 255, 0.06), 1.0)?;
        draw_text_nowrap(dc, dwrite, "No focus applications recorded for this day.", er, 10.0, DWRITE_FONT_WEIGHT_NORMAL, rgba(155, 165, 182, 0.86), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    } else {
        for (i, (exe, secs)) in summary.top_apps.iter().take(3).enumerate() {
            let ry = apps_y + 18.0 + i as f32 * 32.0;
            let row = RectF { x: card_r.x + pad, y: ry, w: inner_w, h: 27.0 };
            draw_rounded_card(dc, row, 7.0, rgba(24, 30, 44, 0.92), rgba(255, 255, 255, 0.06), 1.0)?;
            let ico_r = RectF { x: row.x + 8.0, y: row.y + 5.5, w: 16.0, h: 16.0 };
            let low = exe.to_ascii_lowercase();
            let icon_bmp = if low == "focusgrow.exe" || low == "focusgrow-app.exe" {
                view.app_icon_logo
            } else {
                view.app_icons_map.get(&low)
            };
            if let Some(bmp) = icon_bmp {
                unsafe {
                    dc.DrawBitmap(bmp, Some(&ico_r.to_d2d()), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None);
                }
            } else {
                draw_rounded_card(dc, ico_r, 4.0, rgba(56, 62, 78, 0.92), rgba(255, 255, 255, 0.12), 0.5)?;
            }
            draw_text_nowrap(dc, dwrite, exe, RectF { x: row.x + 30.0, y: row.y + 4.0, w: row.w - 140.0, h: 19.0 }, 10.4, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 246, 254, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
            let pct = if summary.total_secs > 0 {
                ((*secs as f32 / summary.total_secs as f32) * 100.0).round() as u32
            } else {
                0
            };
            draw_text_nowrap(
                dc,
                dwrite,
                &format!("{} ({}%)", format_secs_compact(*secs), pct),
                RectF { x: row.x + row.w - 116.0, y: row.y + 4.0, w: 106.0, h: 19.0 },
                10.0,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                hex_color(accent, 0.98),
                DWRITE_TEXT_ALIGNMENT_TRAILING,
            )?;
        }
    }

    // Footer quote & tag
    let foot_y = card_r.y + card_r.h - 28.0;
    draw_rounded_card(dc, RectF { x: card_r.x + pad, y: foot_y - 6.0, w: inner_w, h: 1.0 }, 0.5, rgba(255, 255, 255, 0.08), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_text_nowrap(dc, dwrite, "\"Deep focus creates masterpiece results.\"", RectF { x: card_r.x + pad, y: foot_y, w: inner_w * 0.64, h: 16.0 }, 9.2, DWRITE_FONT_WEIGHT_NORMAL, rgba(162, 172, 190, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, "FocusGrow OS • Windows", RectF { x: card_r.x + pad + inner_w * 0.64, y: foot_y, w: inner_w * 0.36, h: 16.0 }, 9.2, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(148, 160, 180, 0.9), DWRITE_TEXT_ALIGNMENT_TRAILING)?;

    let _ = factory;
    Ok(())
}

fn draw_share_modal(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_rounded_card(dc, card, layout.card_radius, rgba(8, 8, 12, 0.78), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_rounded_card(dc, mr, 14.0, rgba(22, 24, 30, 0.99), rgba(255, 255, 255, 0.14), 1.0)?;

    let accent = view.effective_accent;
    draw_text_nowrap(
        dc,
        dwrite,
        "Share Performance Card",
        RectF { x: mr.x + 20.0, y: mr.y + 13.0, w: mr.w - 230.0, h: 24.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_segment_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnShareDayToday),
        "Today",
        !view.share_yesterday,
        view.hovered == Some(ControlId::BtnShareDayToday),
        accent,
    )?;
    draw_segment_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnShareDayYesterday),
        "Yesterday",
        view.share_yesterday,
        view.hovered == Some(ControlId::BtnShareDayYesterday),
        accent,
    )?;
    draw_vector_pill(
        dc,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    let preview_r = RectF {
        x: mr.x + 18.0,
        y: mr.y + 52.0,
        w: mr.w - 36.0,
        h: (mr.h - 110.0).max(280.0),
    };
    let summary = compute_share_summary(view, view.share_yesterday);
    draw_share_card_content(dc, factory, dwrite, preview_r, view, &summary)?;

    draw_subtle_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnShareCopyText),
        "Copy Text Report",
        view.hovered == Some(ControlId::BtnShareCopyText),
    )?;
    draw_subtle_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnShareCopyImage),
        "Copy Card Image",
        view.hovered == Some(ControlId::BtnShareCopyImage),
    )?;
    draw_segment_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnShareSavePng),
        "Save PNG",
        true,
        view.hovered == Some(ControlId::BtnShareSavePng),
        accent,
    )?;
    Ok(())
}

fn draw_prayer_modal(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    card: RectF,
    mr: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    draw_rounded_card(dc, card, layout.card_radius, rgba(8, 8, 12, 0.76), rgba(0, 0, 0, 0.0), 0.0)?;
    draw_rounded_card(dc, mr, 14.0, rgba(24, 24, 29, 0.99), rgba(255, 255, 255, 0.13), 1.0)?;

    let accent = view.effective_accent;
    draw_text_nowrap(
        dc,
        dwrite,
        "Prayer Location & Auto GPS",
        RectF { x: mr.x + 20.0, y: mr.y + 13.0, w: mr.w - 165.0, h: 24.0 },
        14.5,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 248, 252, 0.98),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    if let Some(gps_r) = find_hit(layout, ControlId::BtnPrayerAutoGps) {
        let gps_hov = view.hovered == Some(ControlId::BtnPrayerAutoGps);
        draw_rounded_card(
            dc,
            gps_r,
            8.0,
            if gps_hov { hex_color(accent, 0.24) } else { hex_color(accent, 0.14) },
            hex_color(accent, if gps_hov { 0.85 } else { 0.50 }),
            1.0,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            if view.prayer_loc_busy { "Locating..." } else { "Auto GPS" },
            gps_r,
            10.2,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            rgba(242, 248, 255, 0.98),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    draw_vector_pill(
        dc,
        find_hit(layout, ControlId::BtnCloseModal),
        VectorIcon::Close,
        view.hovered == Some(ControlId::BtnCloseModal),
        false,
        "#EF4444",
    )?;

    if let Some(sr) = find_hit(layout, ControlId::BtnPrayerSearchBox) {
        draw_rounded_card(
            dc,
            sr,
            8.0,
            rgba(18, 18, 22, 0.96),
            if view.prayer_search_focused { hex_color(accent, 0.78) } else { rgba(255, 255, 255, 0.10) },
            1.0,
        )?;
        let (txt, col) = if view.prayer_search_query.is_empty() {
            ("Type city or area (e.g. Makassar, Padang Batung)...", rgba(140, 144, 156, 0.86))
        } else {
            (view.prayer_search_query, rgba(244, 246, 252, 0.98))
        };
        draw_text_nowrap(
            dc,
            dwrite,
            txt,
            RectF { x: sr.x + 10.0, y: sr.y, w: (sr.w - 38.0).max(60.0), h: sr.h },
            11.0,
            DWRITE_FONT_WEIGHT_NORMAL,
            col,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )?;
        if !view.prayer_search_query.is_empty() {
            draw_vector_pill(
                dc,
                find_hit(layout, ControlId::BtnPrayerSearchClear),
                VectorIcon::Close,
                view.hovered == Some(ControlId::BtnPrayerSearchClear),
                false,
                accent,
            )?;
        }
    }
    draw_segment_button(
        dc,
        dwrite,
        find_hit(layout, ControlId::BtnPrayerSearchOnline),
        "Search",
        true,
        view.hovered == Some(ControlId::BtnPrayerSearchOnline),
        accent,
    )?;

    if view.prayer_city_results.is_empty() {
        draw_text_nowrap(
            dc,
            dwrite,
            "No offline match. Click Search to lookup online via GPS/Map!",
            RectF { x: mr.x + 20.0, y: mr.y + 132.0, w: mr.w - 40.0, h: 24.0 },
            10.8,
            DWRITE_FONT_WEIGHT_NORMAL,
            rgba(165, 170, 182, 0.88),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    } else {
        for (i, city) in view.prayer_city_results.iter().take(6).enumerate() {
            let Some(row_r) = find_hit(layout, ControlId::BtnPrayerCityPick(i)) else {
                continue;
            };
            let is_cur = (city.lat - view.config.prayer_lat).abs() < 0.08
                && (city.lng - view.config.prayer_lng).abs() < 0.08;
            let hov = view.hovered == Some(ControlId::BtnPrayerCityPick(i));
            draw_rounded_card(
                dc,
                row_r,
                8.0,
                if is_cur { hex_color(accent, 0.16) } else if hov { rgba(48, 50, 60, 0.94) } else { rgba(32, 33, 40, 0.92) },
                if is_cur { hex_color(accent, 0.72) } else { rgba(255, 255, 255, 0.07) },
                1.0,
            )?;
            draw_text_nowrap(
                dc,
                dwrite,
                &city.name,
                RectF { x: row_r.x + 12.0, y: row_r.y + 2.0, w: row_r.w - 155.0, h: 17.0 },
                11.0,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                rgba(244, 246, 252, 0.97),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
            draw_text_nowrap(
                dc,
                dwrite,
                &format!("Lat {:.4}, Lng {:.4}", city.lat, city.lng),
                RectF { x: row_r.x + 12.0, y: row_r.y + 18.0, w: row_r.w - 155.0, h: 14.0 },
                9.2,
                DWRITE_FONT_WEIGHT_NORMAL,
                rgba(155, 160, 175, 0.88),
                DWRITE_TEXT_ALIGNMENT_LEADING,
            )?;
            draw_text_nowrap(
                dc,
                dwrite,
                &format!("UTC{:+}", city.tz),
                RectF { x: row_r.x + row_r.w - 78.0, y: row_r.y + 8.0, w: 66.0, h: 18.0 },
                9.8,
                DWRITE_FONT_WEIGHT_BOLD,
                if is_cur { hex_color(accent, 0.98) } else { rgba(175, 180, 196, 0.9) },
                DWRITE_TEXT_ALIGNMENT_TRAILING,
            )?;
        }
    }

    // Active coordinates footer strip
    let foot_y = mr.y + mr.h - 40.0;
    let status_txt = view
        .prayer_loc_status
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            format!(
                "Active: Lat {:.4}, Lng {:.4} (UTC{:+})",
                view.config.prayer_lat, view.config.prayer_lng, view.config.prayer_tz
            )
        });
    draw_rounded_card(dc, RectF { x: mr.x + 18.0, y: foot_y, w: mr.w - 36.0, h: 28.0 }, 7.0, rgba(18, 19, 24, 0.94), rgba(255, 255, 255, 0.06), 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        &status_txt,
        RectF { x: mr.x + 28.0, y: foot_y + 4.0, w: mr.w - 56.0, h: 20.0 },
        10.0,
        DWRITE_FONT_WEIGHT_MEDIUM,
        hex_color(accent, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    Ok(())
}

/// Renders the Share Performance Card cleanly to an 800x520 PNG image via Direct2D offscreen target + WIC PNG encoder.
pub fn export_share_card_png(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    view: &RenderView<'_>,
    out_path: &Path,
) -> Result<()> {
    let w = 800u32;
    let h = 520u32;
    let props_target = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
        colorContext: Default::default(),
    };
    let props_read = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: Default::default(),
    };
    let size = D2D_SIZE_U { width: w, height: h };
    let target_bmp = unsafe { dc.CreateBitmap(size, None, 0, &props_target) }
        .context("CreateBitmap(TARGET) for share card failed")?;
    let read_bmp = unsafe { dc.CreateBitmap(size, None, 0, &props_read) }
        .context("CreateBitmap(CPU_READ) for share card failed")?;

    let prev_target = unsafe { dc.GetTarget() }.ok();
    let mut prev_dx = 96.0f32;
    let mut prev_dy = 96.0f32;
    unsafe {
        dc.GetDpi(&mut prev_dx, &mut prev_dy);
        dc.SetTarget(&target_bmp);
        dc.SetDpi(96.0, 96.0);
        dc.BeginDraw();
        dc.Clear(Some(&rgba(10, 14, 22, 1.0)));
    }
    let summary = compute_share_summary(view, view.share_yesterday);
    let draw_res = draw_share_card_content(
        dc,
        factory,
        dwrite,
        RectF { x: 16.0, y: 16.0, w: w as f32 - 32.0, h: h as f32 - 32.0 },
        view,
        &summary,
    );
    let end_res = unsafe { dc.EndDraw(None, None) };
    unsafe {
        dc.SetTarget(prev_target.as_ref());
        dc.SetDpi(prev_dx, prev_dy);
    }
    draw_res?;
    end_res.context("Offscreen Share Card EndDraw failed")?;

    unsafe { read_bmp.CopyFromBitmap(None, &target_bmp, None) }
        .context("CopyFromBitmap for share card failed")?;
    let mapped = unsafe { read_bmp.Map(D2D1_MAP_OPTIONS_READ) }
        .context("Map(CPU_READ) for share card failed")?;
    let row_bytes = (w * 4) as usize;
    let mut pixels = vec![0u8; row_bytes * h as usize];
    for row in 0..h as usize {
        unsafe {
            let src_ptr = mapped.bits.add(row * mapped.pitch as usize);
            std::ptr::copy_nonoverlapping(src_ptr, pixels.as_mut_ptr().add(row * row_bytes), row_bytes);
        }
    }
    unsafe {
        let _ = read_bmp.Unmap();
    }

    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .context("WIC factory creation failed")?;
    let stream = unsafe { wic.CreateStream() }.context("WIC CreateStream failed")?;
    unsafe {
        stream.InitializeFromFilename(&HSTRING::from(out_path.as_os_str()), GENERIC_WRITE.0)
    }
    .context("WIC stream InitializeFromFilename failed")?;
    let encoder = unsafe { wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()) }
        .context("WIC CreateEncoder(PNG) failed")?;
    unsafe { encoder.Initialize(&stream, WICBitmapEncoderNoCache) }
        .context("WIC encoder Initialize failed")?;
    let mut frame = None;
    let mut bag = None;
    unsafe { encoder.CreateNewFrame(&mut frame, &mut bag) }.context("WIC CreateNewFrame failed")?;
    let frame = frame.context("WIC frame is None")?;
    unsafe {
        frame.Initialize(bag.as_ref())?;
        frame.SetSize(w, h)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WritePixels(h, w * 4, &pixels)?;
        frame.Commit()?;
        encoder.Commit()?;
    }
    Ok(())
}

fn draw_blocks_ring(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    cx: f32,
    cy: f32,
    r: f32,
    stroke_w: f32,
    ratio: f32,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let total = 24usize;
    let lit = ((total as f32) * ratio).ceil() as usize;
    let track_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.14), None) }?;
    let lit_brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let step = std::f32::consts::TAU / total as f32;
    let gap = step * 0.28;
    for i in 0..total {
        let a0 = -std::f32::consts::FRAC_PI_2 + i as f32 * step + gap * 0.5;
        let a1 = -std::f32::consts::FRAC_PI_2 + (i + 1) as f32 * step - gap * 0.5;
        let p0 = pt(cx + r * a0.cos(), cy + r * a0.sin());
        let p1 = pt(cx + r * a1.cos(), cy + r * a1.sin());
        let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let sink = unsafe { path.Open() }?;
        unsafe {
            sink.BeginFigure(p0, D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddArc(&D2D1_ARC_SEGMENT {
                point: p1,
                size: D2D_SIZE_F { width: r, height: r },
                rotationAngle: 0.0,
                sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                arcSize: D2D1_ARC_SIZE_SMALL,
            });
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close()?;
            let b = if i < lit { &lit_brush } else { &track_brush };
            dc.DrawGeometry(&path, b, stroke_w * 0.92, None);
        }
    }
    Ok(())
}

fn draw_dots_ring(
    dc: &ID2D1DeviceContext,
    cx: f32,
    cy: f32,
    r: f32,
    stroke_w: f32,
    ratio: f32,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let total = 20usize;
    let lit = ((total as f32) * ratio).ceil() as usize;
    let track_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.16), None) }?;
    let lit_brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let dot_r = (stroke_w * 0.52).clamp(2.4, 5.2);
    unsafe {
        for i in 0..total {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / total as f32) * std::f32::consts::TAU;
            let b = if i < lit { &lit_brush } else { &track_brush };
            dc.FillEllipse(
                &D2D1_ELLIPSE { point: pt(cx + r * a.cos(), cy + r * a.sin()), radiusX: dot_r, radiusY: dot_r },
                b,
            );
        }
    }
    Ok(())
}

/// Renders the animated ceramic-pot Sprout Plant Growth illustration inside `[0..100]` space.
fn draw_sprout_plant(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    cx: f32,
    cy: f32,
    radius: f32,
    growth: f32,
    sway_phase: f32,
) -> Result<()> {
    let g = growth.clamp(0.0, 1.0);
    let scale = (radius * 2.0) / 104.0;
    let ox = cx - 50.0 * scale;
    let oy = cy - 51.0 * scale;
    let map = |vx: f32, vy: f32| pt(ox + vx * scale, oy + vy * scale);

    let breeze = sway_phase.sin() * 2.2;
    let sway_pt = |vx: f32, vy: f32| {
        let h_factor = ((60.0 - vy) / 52.0).clamp(0.0, 1.1);
        map(vx + breeze * h_factor * h_factor, vy)
    };

    // 1. Royal Blue Ceramic Pot + Cavity
    let pot_path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let sink = unsafe { pot_path.Open() }?;
    unsafe {
        sink.BeginFigure(map(14.5, 60.0), D2D1_FIGURE_BEGIN_FILLED);
        sink.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(14.5, 85.0), point2: map(33.0, 95.0), point3: map(50.0, 95.0) });
        sink.AddBezier(&D2D1_BEZIER_SEGMENT { point1: map(67.0, 95.0), point2: map(85.5, 85.0), point3: map(85.5, 60.0) });
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
    }
    let pot_brush = unsafe { dc.CreateSolidColorBrush(&rgba(37, 99, 235, 0.96), None) }?;
    let rim_brush = unsafe { dc.CreateSolidColorBrush(&rgba(29, 78, 216, 0.98), None) }?;
    let soil_brush = unsafe { dc.CreateSolidColorBrush(&rgba(62, 39, 35, 0.96), None) }?;
    unsafe {
        dc.FillGeometry(&pot_path, &pot_brush, None);
        dc.FillEllipse(&D2D1_ELLIPSE { point: map(50.0, 60.0), radiusX: 35.5 * scale, radiusY: 6.2 * scale }, &rim_brush);
        dc.FillEllipse(&D2D1_ELLIPSE { point: map(50.0, 60.5), radiusX: 31.0 * scale, radiusY: 4.0 * scale }, &soil_brush);
    }

    // 2. Main Stem (Cubic curve 49,60 -> 49,46 -> 53,34 -> 48.5,12) clipped progressively by growth
    let stem_t = (g / 0.94).clamp(0.05, 1.0);
    let stem_path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let ssink = unsafe { stem_path.Open() }?;
    let mut tip = sway_pt(49.0, 60.0);
    unsafe {
        ssink.BeginFigure(tip, D2D1_FIGURE_BEGIN_HOLLOW);
        let segs = 14usize;
        for i in 1..=segs {
            let t = stem_t * (i as f32 / segs as f32);
            let u = 1.0 - t;
            let sx = u * u * u * 49.0 + 3.0 * u * u * t * 49.0 + 3.0 * u * t * t * 53.0 + t * t * t * 48.5;
            let sy = u * u * u * 60.0 + 3.0 * u * u * t * 46.0 + 3.0 * u * t * t * 34.0 + t * t * t * 12.0;
            tip = sway_pt(sx, sy);
            ssink.AddLine(tip);
        }
        ssink.EndFigure(D2D1_FIGURE_END_OPEN);
        ssink.Close()?;
    }
    let stem_style = create_round_stroke_style(factory)?;
    let stem_brush = unsafe { dc.CreateSolidColorBrush(&rgba(22, 163, 74, 0.98), None) }?;
    unsafe {
        dc.DrawGeometry(&stem_path, &stem_brush, (3.4 * scale).max(1.8), &stem_style);
    }

    let light_leaf = unsafe { dc.CreateSolidColorBrush(&rgba(74, 222, 128, 0.96), None) }?;
    let dark_leaf = unsafe { dc.CreateSolidColorBrush(&rgba(22, 163, 74, 0.98), None) }?;

    // Helper: draw scaled leaf lobe from anchor `(ax, ay)`
    let draw_leaf = |ax: f32, ay: f32, grow: f32, p1: (f32, f32), p2: (f32, f32), tip_pt: (f32, f32), top_mid: (f32, f32), top_mid2: (f32, f32), bot_mid1: (f32, f32), bot_mid2: (f32, f32)| -> Result<()> {
        if grow <= 0.02 {
            return Ok(());
        }
        let s = grow.clamp(0.0, 1.0);
        let lerp_pt = |px: f32, py: f32| sway_pt(ax + (px - ax) * s, ay + (py - ay) * s);
        // Upper bright lamina
        let top_g: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let ts = unsafe { top_g.Open() }?;
        unsafe {
            ts.BeginFigure(sway_pt(ax, ay), D2D1_FIGURE_BEGIN_FILLED);
            ts.AddBezier(&D2D1_BEZIER_SEGMENT { point1: lerp_pt(p1.0, p1.1), point2: lerp_pt(p2.0, p2.1), point3: lerp_pt(tip_pt.0, tip_pt.1) });
            ts.AddBezier(&D2D1_BEZIER_SEGMENT { point1: lerp_pt(top_mid.0, top_mid.1), point2: lerp_pt(top_mid2.0, top_mid2.1), point3: sway_pt(ax, ay) });
            ts.EndFigure(D2D1_FIGURE_END_CLOSED);
            ts.Close()?;
            dc.FillGeometry(&top_g, &light_leaf, None);
        }
        // Lower shadow lamina
        let bot_g: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let bs = unsafe { bot_g.Open() }?;
        unsafe {
            bs.BeginFigure(sway_pt(ax, ay), D2D1_FIGURE_BEGIN_FILLED);
            bs.AddBezier(&D2D1_BEZIER_SEGMENT { point1: lerp_pt(top_mid2.0, top_mid2.1), point2: lerp_pt(top_mid.0, top_mid.1), point3: lerp_pt(tip_pt.0, tip_pt.1) });
            bs.AddBezier(&D2D1_BEZIER_SEGMENT { point1: lerp_pt(bot_mid1.0, bot_mid1.1), point2: lerp_pt(bot_mid2.0, bot_mid2.1), point3: sway_pt(ax, ay) });
            bs.EndFigure(D2D1_FIGURE_END_CLOSED);
            bs.Close()?;
            dc.FillGeometry(&bot_g, &dark_leaf, None);
        }
        Ok(())
    };

    // 3. Leaf Pair 1 (Left Organic, appears >= 0.30)
    draw_leaf(50.0, 43.0, (g - 0.28) / 0.16, (40.0, 33.0), (29.0, 32.0), (22.0, 35.0), (33.0, 39.0), (44.0, 42.0), (30.0, 46.0), (41.0, 47.0))?;
    // 4. Leaf Pair 2 (Middle Right, appears >= 0.48)
    draw_leaf(51.0, 35.0, (g - 0.48) / 0.16, (61.0, 25.0), (71.0, 24.0), (81.0, 27.0), (67.0, 30.0), (56.0, 34.0), (73.0, 38.0), (61.0, 39.0))?;
    // 5. Leaf Pair 3 (Top Right, appears >= 0.78)
    draw_leaf(49.0, 19.0, (g - 0.78) / 0.15, (59.0, 9.0), (72.0, 8.0), (81.0, 11.0), (66.0, 15.0), (54.0, 18.0), (73.0, 22.0), (60.0, 23.0))?;

    // 6. Top Crown Buds (>= 0.90)
    let bud_g = ((g - 0.88) / 0.10).clamp(0.0, 1.0);
    if bud_g > 0.02 {
        draw_leaf(48.5, 14.0, bud_g, (43.0, 11.0), (41.0, 6.5), (43.0, 4.0), (46.5, 6.0), (48.0, 9.5), (49.5, 7.5), (48.5, 11.0))?;
        draw_leaf(48.5, 13.5, bud_g, (48.0, 9.5), (50.5, 6.5), (53.0, 6.5), (53.5, 8.5), (52.0, 11.0), (55.5, 7.5), (54.5, 10.5))?;
    }

    // 7. Golden Crown Flower (Blooms at completion / rest)
    let bloom = ((g - 0.96) / 0.04).clamp(0.0, 1.0);
    if bloom > 0.05 {
        let fpt = sway_pt(48.5, 7.5);
        let halo_b = unsafe { dc.CreateSolidColorBrush(&rgba(245, 158, 11, 0.38 * bloom), None) }?;
        let petal_b = unsafe { dc.CreateSolidColorBrush(&rgba(251, 191, 36, 0.98 * bloom), None) }?;
        let core_b = unsafe { dc.CreateSolidColorBrush(&rgba(239, 68, 68, 0.98 * bloom), None) }?;
        unsafe {
            dc.FillEllipse(&D2D1_ELLIPSE { point: fpt, radiusX: 8.5 * scale * bloom, radiusY: 8.5 * scale * bloom }, &halo_b);
            dc.FillEllipse(&D2D1_ELLIPSE { point: fpt, radiusX: 4.8 * scale * bloom, radiusY: 4.8 * scale * bloom }, &petal_b);
            dc.FillEllipse(&D2D1_ELLIPSE { point: fpt, radiusX: 2.1 * scale * bloom, radiusY: 2.1 * scale * bloom }, &core_b);
        }
    }

    Ok(())
}

fn draw_pip_controls(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    layout: &UiLayout,
    view: &RenderView<'_>,
) -> Result<()> {
    let play_icon = if view.timer.is_running() { VectorIcon::PauseBars } else { VectorIcon::PlayTri };
    draw_icon_circle_button(
        dc,
        factory,
        find_hit(layout, ControlId::BtnPlayPause),
        play_icon,
        hex_color(view.effective_accent, 0.96),
        rgba(0, 0, 0, 0.0),
        contrast_fg(view.effective_accent),
        view.hovered == Some(ControlId::BtnPlayPause),
    )?;
    if let Some(reset_r) = find_hit(layout, ControlId::BtnReset) {
        draw_icon_circle_button(
            dc,
            factory,
            Some(reset_r),
            VectorIcon::StopSquare,
            rgba(42, 42, 46, 0.92),
            rgba(255, 255, 255, 0.14),
            rgba(245, 245, 250, 0.96),
            view.hovered == Some(ControlId::BtnReset),
        )?;
    }

    if let Some(pp_r) = find_hit(layout, ControlId::BtnPlayPause) {
        let subtitle = if view.timer.phase() == SessionPhase::Idle {
            let total_str = FocusTimer::formatted_total_duration(view.timer.total_remaining_sec());
            format!("{total_str} total")
        } else {
            format_timer_subtitle(view)
        };
        let sub_size = if layout.card_rect.w < 215.0 { 8.6 } else { 9.5 };
        let sub_col = if view.hovered == Some(ControlId::BtnCycleTimerSubtitle) {
            hex_color(view.effective_accent, 0.98)
        } else {
            rgba(220, 220, 228, 0.92)
        };
        draw_text_shadow_nowrap(
            dc,
            dwrite,
            &subtitle,
            RectF {
                x: layout.card_rect.x + 6.0,
                y: pp_r.y + pp_r.h + 3.0,
                w: layout.card_rect.w - 12.0,
                h: 15.0,
            },
            sub_size,
            DWRITE_FONT_WEIGHT_MEDIUM,
            sub_col,
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
    }
    Ok(())
}

fn draw_music_ticker_pill(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    pill: RectF,
    layout: &UiLayout,
    view: &RenderView<'_>,
    track: &TrackInfo,
) -> Result<()> {
    draw_rounded_card(
        dc,
        pill,
        6.0,
        rgba(20, 20, 22, 0.82),
        hex_color(view.effective_accent, 0.90),
        1.0,
    )?;

    let btn_bg = rgba(255, 255, 255, 0.11);
    let btn_border = rgba(255, 255, 255, 0.22);
    let btn_fg = rgba(255, 255, 255, 0.94);
    draw_icon_circle_button(dc, factory, find_hit(layout, ControlId::BtnMediaPrev), VectorIcon::SkipPrev, btn_bg, btn_border, btn_fg, view.hovered == Some(ControlId::BtnMediaPrev))?;
    let pp = if view.playback == PlaybackState::Playing { VectorIcon::PauseBars } else { VectorIcon::PlayTri };
    draw_icon_circle_button(dc, factory, find_hit(layout, ControlId::BtnMediaPlayPause), pp, btn_bg, btn_border, btn_fg, view.hovered == Some(ControlId::BtnMediaPlayPause))?;
    draw_icon_circle_button(dc, factory, find_hit(layout, ControlId::BtnMediaNext), VectorIcon::SkipNext, btn_bg, btn_border, btn_fg, view.hovered == Some(ControlId::BtnMediaNext))?;

    if let Some(txt_r) = find_hit(layout, ControlId::BtnMediaToggleWindow) {
        let label = track.ticker_text();
        draw_marquee_text(
            dc,
            dwrite,
            &label,
            txt_r,
            11.0,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            hex_color(view.effective_accent, 0.98),
            view.marquee_offset,
        )?;
    }

    Ok(())
}

fn draw_marquee_text(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    offset_px: f32,
) -> Result<()> {
    let fmt = create_text_format_single_line(
        dwrite,
        "Segoe UI",
        size,
        weight,
        DWRITE_TEXT_ALIGNMENT_LEADING,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    )?;
    let _ = unsafe { fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP) };

    let wide: Vec<u16> = text.encode_utf16().collect();
    let tl = unsafe { dwrite.CreateTextLayout(&wide, &fmt, 4096.0, rect.h.max(16.0)) }?;
    let mut metrics = DWRITE_TEXT_METRICS::default();
    unsafe {
        tl.GetMetrics(&mut metrics)?;
    }

    let text_w = metrics.widthIncludingTrailingWhitespace.max(metrics.width);
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    unsafe {
        dc.PushAxisAlignedClip(&rect.to_d2d(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }

    if text_w <= rect.w {
        unsafe {
            dc.DrawTextLayout(pt(rect.x, rect.y), &tl, &brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
        }
    } else {
        let gap = 36.0f32;
        let cycle = (text_w + gap).max(1.0);
        let shift = offset_px % cycle;
        let x1 = rect.x - shift;
        let x2 = x1 + cycle;
        unsafe {
            dc.DrawTextLayout(pt(x1, rect.y), &tl, &brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
            if x2 < rect.x + rect.w {
                dc.DrawTextLayout(pt(x2, rect.y), &tl, &brush, D2D1_DRAW_TEXT_OPTIONS_CLIP);
            }
        }
    }

    unsafe {
        dc.PopAxisAlignedClip();
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VectorIcon {
    Minimize,
    Maximize,
    Close,
    CloseSoft,
    PipPop,
    CapsulePill,
    PopOut,
    Target,
    Person,
    Clock,
    Headphones,
    Smartphone,
    #[allow(dead_code)]
    PictureFrame,
    Gear,
    BarChart,
    Pencil,
    Shield,
    Refresh,
    Search,
    ChevronUp,
    ChevronDown,
    PlayTri,
    PauseBars,
    StopSquare,
    SkipPrev,
    SkipNext,
    Keyboard,
    Flame,
    Cricket,
    CloudRain,
    CloudLightning,
    Wind,
    Waves,
    PulseWave,
    Moon,
    Sun,
    Sunrise,
    Ban,
    Globe,
    Palette,
    Bell,
    RingCircle,
    HourglassMini,
    BarsMini,
    DotsMini,
    OrbitMini,
}

fn draw_filled_triangle(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    p0: Vector2,
    p1: Vector2,
    p2: Vector2,
    brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
) -> Result<()> {
    let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let sink = unsafe { path.Open() }?;
    unsafe {
        sink.BeginFigure(p0, D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(p1);
        sink.AddLine(p2);
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
        dc.FillGeometry(&path, brush, None);
    }
    Ok(())
}

fn draw_vector_icon(
    dc: &ID2D1DeviceContext,
    factory: Option<&ID2D1Factory1>,
    rect: RectF,
    icon: VectorIcon,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let cx = rect.x + rect.w * 0.5;
    let cy = rect.y + rect.h * 0.5;
    let s = rect.w.min(rect.h);
    let r = (s * 0.255).max(2.6);
    let round_s: Option<windows::Win32::Graphics::Direct2D::ID2D1StrokeStyle> = factory
        .and_then(|fac| create_round_stroke_style(fac).ok())
        .and_then(|s| windows::core::Interface::cast(&s).ok());
    let stroke_opt = round_s.as_ref();

    unsafe {
        match icon {
            VectorIcon::Minimize => {
                dc.DrawLine(pt(cx - r, cy), pt(cx + r, cy), &brush, 1.55, stroke_opt);
            }
            VectorIcon::Maximize => {
                let rr = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - r, y: cy - r, w: r * 2.0, h: r * 2.0 }.to_d2d(),
                    radiusX: 1.6,
                    radiusY: 1.6,
                };
                dc.DrawRoundedRectangle(&rr, &brush, 1.35, stroke_opt);
            }
            VectorIcon::Close | VectorIcon::CloseSoft => {
                let d = (r * 0.86).max(3.0);
                dc.DrawLine(pt(cx - d, cy - d), pt(cx + d, cy + d), &brush, 1.5, stroke_opt);
                dc.DrawLine(pt(cx + d, cy - d), pt(cx - d, cy + d), &brush, 1.5, stroke_opt);
            }
            VectorIcon::PopOut => {
                let bx = cx - r * 0.94;
                let by = cy - r * 0.62;
                let bw = r * 1.56;
                let bh = r * 1.56;
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(bx + bw * 0.45, by), D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddLine(pt(bx + 1.5, by));
                    sink.AddLine(pt(bx, by + 1.5));
                    sink.AddLine(pt(bx, by + bh - 1.5));
                    sink.AddLine(pt(bx + 1.5, by + bh));
                    sink.AddLine(pt(bx + bw - 1.5, by + bh));
                    sink.AddLine(pt(bx + bw, by + bh - 1.5));
                    sink.AddLine(pt(bx + bw, by + bh * 0.55));
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.4, stroke_opt);
                } else {
                    dc.DrawLine(pt(bx, by), pt(bx + bw * 0.45, by), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(bx, by), pt(bx, by + bh), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(bx, by + bh), pt(bx + bw, by + bh), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(bx + bw, by + bh), pt(bx + bw, by + bh * 0.55), &brush, 1.35, stroke_opt);
                }
                let ax = cx + r * 0.96;
                let ay = cy - r * 0.96;
                dc.DrawLine(pt(cx - r * 0.12, cy + r * 0.12), pt(ax, ay), &brush, 1.4, stroke_opt);
                dc.DrawLine(pt(ax - r * 0.62, ay), pt(ax, ay), &brush, 1.4, stroke_opt);
                dc.DrawLine(pt(ax, ay), pt(ax, ay + r * 0.62), &brush, 1.4, stroke_opt);
            }
            VectorIcon::Target => {
                let r1 = (r * 1.0).max(4.4);
                let r2 = (r * 0.56).max(2.4);
                let r3 = (r * 0.22).max(1.1);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r1, radiusY: r1 }, &brush, 1.35, stroke_opt);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r2, radiusY: r2 }, &brush, 1.2, stroke_opt);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r3, radiusY: r3 }, &brush);
            }
            VectorIcon::Person => {
                // Clean modern Posture / User silhouette: circle head with smooth rounded shoulder arc
                let rad = (r * 1.05).max(4.6);
                let head_r = rad * 0.44;
                let head_cy = cy - rad * 0.44;
                dc.DrawEllipse(
                    &D2D1_ELLIPSE {
                        point: pt(cx, head_cy),
                        radiusX: head_r,
                        radiusY: head_r,
                    },
                    &brush,
                    1.3,
                    stroke_opt,
                );
                let sh_w = rad * 0.88;
                let sh_h = rad * 0.72;
                let sh_y = cy + rad * 0.90;
                if let Some(fact) = factory {
                    if let Ok(path) = fact.CreatePathGeometry() {
                        if let Ok(sink) = path.Open() {
                            sink.BeginFigure(pt(cx - sh_w, sh_y), D2D1_FIGURE_BEGIN_HOLLOW);
                            sink.AddArc(&D2D1_ARC_SEGMENT {
                                point: pt(cx + sh_w, sh_y),
                                size: D2D_SIZE_F { width: sh_w, height: sh_h },
                                rotationAngle: 0.0,
                                sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                                arcSize: D2D1_ARC_SIZE_SMALL,
                            });
                            sink.EndFigure(D2D1_FIGURE_END_OPEN);
                            let _ = sink.Close();
                            dc.DrawGeometry(&path, &brush, 1.3, stroke_opt);
                        }
                    }
                } else {
                    // Fallback smooth shoulder arch if factory not provided
                    let p0 = pt(cx - sh_w, sh_y);
                    let p1 = pt(cx - sh_w * 0.45, cy + rad * 0.20);
                    let p2 = pt(cx + sh_w * 0.45, cy + rad * 0.20);
                    let p3 = pt(cx + sh_w, sh_y);
                    dc.DrawLine(p0, p1, &brush, 1.3, stroke_opt);
                    dc.DrawLine(p1, p2, &brush, 1.3, stroke_opt);
                    dc.DrawLine(p2, p3, &brush, 1.3, stroke_opt);
                }
            }
            VectorIcon::Clock => {
                let rad = (r * 0.96).max(4.2);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad, radiusY: rad }, &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx, cy - rad * 0.52), pt(cx, cy + 0.2), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx, cy + 0.2), pt(cx + rad * 0.44, cy + rad * 0.24), &brush, 1.35, stroke_opt);
            }
            VectorIcon::PipPop => {
                let rr = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - r * 1.12, y: cy - r * 0.88, w: r * 2.24, h: r * 1.76 }.to_d2d(),
                    radiusX: 2.0,
                    radiusY: 2.0,
                };
                dc.DrawRoundedRectangle(&rr, &brush, 1.35, stroke_opt);
                let inner = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx + r * 0.06, y: cy + r * 0.04, w: r * 0.86, h: r * 0.64 }.to_d2d(),
                    radiusX: 1.2,
                    radiusY: 1.2,
                };
                dc.FillRoundedRectangle(&inner, &brush);
            }
            VectorIcon::CapsulePill => {
                let pill_w = (r * 2.2).max(12.0);
                let pill_h = (r * 0.95).max(6.0);
                let rr = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - pill_w * 0.5, y: cy - pill_h * 0.5, w: pill_w, h: pill_h }.to_d2d(),
                    radiusX: pill_h * 0.5,
                    radiusY: pill_h * 0.5,
                };
                dc.DrawRoundedRectangle(&rr, &brush, 1.35, stroke_opt);
                let dot_r = (pill_h * 0.28).max(1.5);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx - pill_w * 0.5 + pill_h * 0.5, cy), radiusX: dot_r, radiusY: dot_r }, &brush);
            }
            VectorIcon::Headphones => {
                let arch_r = (r * 0.95).max(4.2);
                let arch_cy = cy + r * 0.14;
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(cx - arch_r, arch_cy + r * 0.12), D2D1_FIGURE_BEGIN_HOLLOW);
                    for i in 1..=16 {
                        let a = std::f32::consts::PI + (i as f32 / 16.0) * std::f32::consts::PI;
                        sink.AddLine(pt(cx + arch_r * a.cos(), arch_cy + arch_r * a.sin()));
                    }
                    sink.AddLine(pt(cx + arch_r, arch_cy + r * 0.12));
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.45, stroke_opt);
                } else {
                    let steps = 12;
                    for i in 0..steps {
                        let a0 = std::f32::consts::PI + (i as f32 / steps as f32) * std::f32::consts::PI;
                        let a1 = std::f32::consts::PI + ((i + 1) as f32 / steps as f32) * std::f32::consts::PI;
                        dc.DrawLine(
                            pt(cx + arch_r * a0.cos(), arch_cy + arch_r * a0.sin()),
                            pt(cx + arch_r * a1.cos(), arch_cy + arch_r * a1.sin()),
                            &brush,
                            1.45,
                            stroke_opt,
                        );
                    }
                }
                let cup_w = (r * 0.46).max(2.4);
                let cup_h = (r * 0.88).max(4.5);
                let left_cup = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - arch_r - 0.8, y: arch_cy - 0.3, w: cup_w, h: cup_h }.to_d2d(),
                    radiusX: 1.5,
                    radiusY: 1.5,
                };
                let right_cup = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx + arch_r - cup_w + 0.8, y: arch_cy - 0.3, w: cup_w, h: cup_h }.to_d2d(),
                    radiusX: 1.5,
                    radiusY: 1.5,
                };
                dc.DrawRoundedRectangle(&left_cup, &brush, 1.35, stroke_opt);
                dc.DrawRoundedRectangle(&right_cup, &brush, 1.35, stroke_opt);
            }
            VectorIcon::Smartphone => {
                let w_p = (r * 1.42).max(7.2);
                let h_p = (r * 2.15).max(10.5);
                let frame = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - w_p * 0.5, y: cy - h_p * 0.5, w: w_p, h: h_p }.to_d2d(),
                    radiusX: 2.0,
                    radiusY: 2.0,
                };
                dc.DrawRoundedRectangle(&frame, &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - w_p * 0.20, cy - h_p * 0.32), pt(cx + w_p * 0.20, cy - h_p * 0.32), &brush, 1.15, stroke_opt);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy + h_p * 0.31), radiusX: 0.9, radiusY: 0.9 }, &brush);
            }
            VectorIcon::PictureFrame => {
                let rr = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - r * 1.15, y: cy - r * 0.9, w: r * 2.3, h: r * 1.8 }.to_d2d(),
                    radiusX: 1.8,
                    radiusY: 1.8,
                };
                dc.DrawRoundedRectangle(&rr, &brush, 1.35, stroke_opt);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx - r * 0.48, cy - r * 0.25), radiusX: 1.3, radiusY: 1.3 }, &brush);
                dc.DrawLine(pt(cx - r * 0.85, cy + r * 0.6), pt(cx - r * 0.1, cy - r * 0.05), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.1, cy - r * 0.05), pt(cx + r * 0.35, cy + r * 0.32), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.25, cy + r * 0.32), pt(cx + r * 0.78, cy - r * 0.18), &brush, 1.35, stroke_opt);
            }
            VectorIcon::Gear => {
                // Scalloped 8-tooth Lucide-style cog silhouette
                let r_outer = (r * 1.06).max(4.8);
                let r_valley = (r * 0.76).max(3.4);
                let inner = (r * 0.36).max(1.7);
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    let teeth = 8usize;
                    let segs = teeth * 4;
                    for i in 0..=segs {
                        let step = i % 4;
                        let rad = if step == 0 || step == 1 { r_outer } else { r_valley };
                        let angle = (i as f32 / segs as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                        let p = pt(cx + rad * angle.cos(), cy + rad * angle.sin());
                        if i == 0 {
                            sink.BeginFigure(p, D2D1_FIGURE_BEGIN_HOLLOW);
                        } else {
                            sink.AddLine(p);
                        }
                    }
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.35, stroke_opt);
                } else {
                    dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r_valley, radiusY: r_valley }, &brush, 1.35, stroke_opt);
                    for k in 0..8 {
                        let a = (k as f32) * std::f32::consts::FRAC_PI_4;
                        dc.DrawLine(
                            pt(cx + (r_valley - 0.2) * a.cos(), cy + (r_valley - 0.2) * a.sin()),
                            pt(cx + r_outer * a.cos(), cy + r_outer * a.sin()),
                            &brush,
                            1.5,
                            stroke_opt,
                        );
                    }
                }
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: inner, radiusY: inner }, &brush, 1.3, stroke_opt);
            }
            VectorIcon::BarChart => {
                let x_gap = (r * 0.56).max(2.6);
                let base_y = cy + r * 0.85;
                dc.DrawLine(pt(cx - x_gap, base_y), pt(cx - x_gap, cy + r * 0.05), &brush, 1.8, stroke_opt);
                dc.DrawLine(pt(cx, base_y), pt(cx, cy - r * 0.88), &brush, 1.8, stroke_opt);
                dc.DrawLine(pt(cx + x_gap, base_y), pt(cx + x_gap, cy - r * 0.25), &brush, 1.8, stroke_opt);
            }
            VectorIcon::Pencil => {
                // Lucide square-pen Edit Note icon matching main_dashboard.png toolbar
                let bx = cx - r * 0.92;
                let by = cy - r * 0.72;
                let bw = r * 1.62;
                let bh = r * 1.62;
                dc.DrawLine(pt(bx, by), pt(bx + bw * 0.45, by), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(bx, by), pt(bx, by + bh), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(bx, by + bh), pt(bx + bw, by + bh), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(bx + bw, by + bh), pt(bx + bw, by + bh * 0.54), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.12, cy + r * 0.14), pt(cx + r * 0.94, cy - r * 0.92), &brush, 1.7, stroke_opt);
            }
            VectorIcon::Shield => {
                let sw = (r * 0.88).max(3.6);
                let sh = (r * 1.04).max(4.4);
                let top_y = cy - sh * 0.85;
                let mid_y = cy + sh * 0.04;
                let bot_y = cy + sh * 0.98;
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(cx, top_y), D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddLine(pt(cx + sw, top_y + sh * 0.26));
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx + sw * 0.96, mid_y + sh * 0.34),
                        point2: pt(cx + sw * 0.35, bot_y - sh * 0.14),
                        point3: pt(cx, bot_y),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx - sw * 0.35, bot_y - sh * 0.14),
                        point2: pt(cx - sw * 0.96, mid_y + sh * 0.34),
                        point3: pt(cx - sw, top_y + sh * 0.26),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.35, stroke_opt);
                } else {
                    dc.DrawLine(pt(cx, top_y), pt(cx + sw, top_y + sh * 0.28), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(cx + sw, top_y + sh * 0.28), pt(cx + sw * 0.82, mid_y), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(cx + sw * 0.82, mid_y), pt(cx, bot_y), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(cx, bot_y), pt(cx - sw * 0.82, mid_y), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(cx - sw * 0.82, mid_y), pt(cx - sw, top_y + sh * 0.28), &brush, 1.35, stroke_opt);
                    dc.DrawLine(pt(cx - sw, top_y + sh * 0.28), pt(cx, top_y), &brush, 1.35, stroke_opt);
                }
            }
            VectorIcon::Refresh => {
                let rad = (r * 0.94).max(4.0);
                let start_deg = 25.0f32.to_radians();
                let sweep_deg = 285.0f32.to_radians();
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(cx + rad * start_deg.cos(), cy + rad * start_deg.sin()), D2D1_FIGURE_BEGIN_HOLLOW);
                    let steps = 20usize;
                    for i in 1..=steps {
                        let a = start_deg + (i as f32 / steps as f32) * sweep_deg;
                        sink.AddLine(pt(cx + rad * a.cos(), cy + rad * a.sin()));
                    }
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.45, stroke_opt);
                } else {
                    let steps = 18usize;
                    for i in 0..steps {
                        let a0 = start_deg + (i as f32 / steps as f32) * sweep_deg;
                        let a1 = start_deg + ((i + 1) as f32 / steps as f32) * sweep_deg;
                        dc.DrawLine(pt(cx + rad * a0.cos(), cy + rad * a0.sin()), pt(cx + rad * a1.cos(), cy + rad * a1.sin()), &brush, 1.45, stroke_opt);
                    }
                }
                let end_a = start_deg + sweep_deg;
                let tx = cx + rad * end_a.cos();
                let ty = cy + rad * end_a.sin();
                let tip = pt(tx + 1.2, ty + 1.2);
                let wing1 = pt(tx - 0.6, ty - 3.2);
                let wing2 = pt(tx - 3.6, ty + 0.5);
                if let Some(fac) = factory {
                    draw_filled_triangle(dc, fac, tip, wing1, wing2, &brush)?;
                } else {
                    dc.DrawLine(tip, wing1, &brush, 1.45, stroke_opt);
                    dc.DrawLine(tip, wing2, &brush, 1.45, stroke_opt);
                }
            }
            VectorIcon::Search => {
                let rad = (r * 0.82).max(3.6);
                let gx = cx - 1.3;
                let gy = cy - 1.3;
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(gx, gy), radiusX: rad, radiusY: rad }, &brush, 1.45, stroke_opt);
                let off = rad * 0.72;
                dc.DrawLine(pt(gx + off, gy + off), pt(cx + r * 1.05, cy + r * 1.05), &brush, 1.55, stroke_opt);
            }
            VectorIcon::ChevronUp => {
                let d = (r * 0.78).max(3.2);
                dc.DrawLine(pt(cx - d, cy + d * 0.45), pt(cx, cy - d * 0.45), &brush, 1.65, stroke_opt);
                dc.DrawLine(pt(cx, cy - d * 0.45), pt(cx + d, cy + d * 0.45), &brush, 1.65, stroke_opt);
            }
            VectorIcon::ChevronDown => {
                let d = (r * 0.78).max(3.2);
                dc.DrawLine(pt(cx - d, cy - d * 0.45), pt(cx, cy + d * 0.45), &brush, 1.65, stroke_opt);
                dc.DrawLine(pt(cx, cy + d * 0.45), pt(cx + d, cy - d * 0.45), &brush, 1.65, stroke_opt);
            }
            VectorIcon::PlayTri => {
                let hr = (s * 0.20).max(3.0);
                let wr = (s * 0.22).max(3.2);
                let x_l = cx - wr * 0.55;
                let x_r = cx + wr * 0.85;
                if let Some(fac) = factory {
                    draw_filled_triangle(dc, fac, pt(x_l, cy - hr), pt(x_r, cy), pt(x_l, cy + hr), &brush)?;
                } else {
                    dc.DrawLine(pt(x_l, cy - hr), pt(x_r, cy), &brush, 1.8, None);
                    dc.DrawLine(pt(x_r, cy), pt(x_l, cy + hr), &brush, 1.8, None);
                    dc.DrawLine(pt(x_l, cy + hr), pt(x_l, cy - hr), &brush, 1.8, None);
                }
            }
            VectorIcon::PauseBars => {
                let bar_w = (s * 0.088).clamp(1.8, 3.6);
                let bar_h = (s * 0.36).clamp(6.5, 14.0);
                let gap = (s * 0.08).clamp(1.6, 3.6);
                let l_bar = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - gap * 0.5 - bar_w, y: cy - bar_h * 0.5, w: bar_w, h: bar_h }.to_d2d(),
                    radiusX: 0.8,
                    radiusY: 0.8,
                };
                let r_bar = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx + gap * 0.5, y: cy - bar_h * 0.5, w: bar_w, h: bar_h }.to_d2d(),
                    radiusX: 0.8,
                    radiusY: 0.8,
                };
                dc.FillRoundedRectangle(&l_bar, &brush);
                dc.FillRoundedRectangle(&r_bar, &brush);
            }
            VectorIcon::StopSquare => {
                let side = (s * 0.32).clamp(5.6, 12.0);
                let sq = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - side * 0.5, y: cy - side * 0.5, w: side, h: side }.to_d2d(),
                    radiusX: 1.2,
                    radiusY: 1.2,
                };
                dc.FillRoundedRectangle(&sq, &brush);
            }
            VectorIcon::SkipPrev => {
                let h_r = (s * 0.17).clamp(2.6, 6.0);
                let bar_w = (s * 0.07).clamp(1.3, 2.2);
                let bar_x = cx - h_r * 1.15;
                let bar = D2D1_ROUNDED_RECT {
                    rect: RectF { x: bar_x, y: cy - h_r, w: bar_w, h: h_r * 2.0 }.to_d2d(),
                    radiusX: 0.5,
                    radiusY: 0.5,
                };
                dc.FillRoundedRectangle(&bar, &brush);
                let tip_x = bar_x + bar_w + 0.6;
                let base_x = cx + h_r * 1.05;
                if let Some(fac) = factory {
                    draw_filled_triangle(dc, fac, pt(base_x, cy - h_r), pt(tip_x, cy), pt(base_x, cy + h_r), &brush)?;
                }
            }
            VectorIcon::SkipNext => {
                let h_r = (s * 0.17).clamp(2.6, 6.0);
                let bar_w = (s * 0.07).clamp(1.3, 2.2);
                let bar_x = cx + h_r * 1.15 - bar_w;
                let bar = D2D1_ROUNDED_RECT {
                    rect: RectF { x: bar_x, y: cy - h_r, w: bar_w, h: h_r * 2.0 }.to_d2d(),
                    radiusX: 0.5,
                    radiusY: 0.5,
                };
                dc.FillRoundedRectangle(&bar, &brush);
                let base_x = cx - h_r * 1.05;
                let tip_x = bar_x - 0.6;
                if let Some(fac) = factory {
                    draw_filled_triangle(dc, fac, pt(base_x, cy - h_r), pt(tip_x, cy), pt(base_x, cy + h_r), &brush)?;
                }
            }
            VectorIcon::Keyboard => {
                let rr = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - r * 1.25, y: cy - r * 0.78, w: r * 2.5, h: r * 1.56 }.to_d2d(),
                    radiusX: 2.0,
                    radiusY: 2.0,
                };
                dc.DrawRoundedRectangle(&rr, &brush, 1.35, stroke_opt);
                for kx in [-0.72f32, -0.24, 0.24, 0.72] {
                    dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx + r * kx, cy - r * 0.26), radiusX: 0.75, radiusY: 0.75 }, &brush);
                }
                dc.DrawLine(pt(cx - r * 0.65, cy + r * 0.34), pt(cx + r * 0.65, cy + r * 0.34), &brush, 1.3, stroke_opt);
            }
            VectorIcon::Flame => {
                let sx = r * 0.85;
                let sy = r * 1.08;
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(cx, cy - sy), D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx + sx * 1.15, cy - sy * 0.25),
                        point2: pt(cx + sx * 1.1, cy + sy * 0.85),
                        point3: pt(cx, cy + sy * 0.96),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx - sx * 1.1, cy + sy * 0.85),
                        point2: pt(cx - sx * 1.05, cy - sy * 0.2),
                        point3: pt(cx, cy - sy),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.4, stroke_opt);
                    dc.DrawLine(pt(cx - sx * 0.2, cy + sy * 0.55), pt(cx + sx * 0.15, cy + sy * 0.05), &brush, 1.25, stroke_opt);
                } else {
                    dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy + r * 0.2), radiusX: sx * 0.75, radiusY: sy * 0.78 }, &brush, 1.35, stroke_opt);
                }
            }
            VectorIcon::Cricket => {
                // Stylized acoustic cricket / bug silhouette
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy + r * 0.12), radiusX: r * 0.42, radiusY: r * 0.66 }, &brush, 1.3, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.18, cy - r * 0.52), pt(cx - r * 0.52, cy - r * 0.96), &brush, 1.15, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.18, cy - r * 0.52), pt(cx + r * 0.52, cy - r * 0.96), &brush, 1.15, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.42, cy), pt(cx - r * 0.98, cy - r * 0.25), &brush, 1.2, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.42, cy), pt(cx + r * 0.98, cy - r * 0.25), &brush, 1.2, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.40, cy + r * 0.4), pt(cx - r * 0.95, cy + r * 0.78), &brush, 1.2, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.40, cy + r * 0.4), pt(cx + r * 0.95, cy + r * 0.78), &brush, 1.2, stroke_opt);
            }
            VectorIcon::CloudRain => {
                let cw = r * 1.75;
                let ch = r * 0.78;
                let cloud = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - cw * 0.5, y: cy - r * 0.68, w: cw, h: ch }.to_d2d(),
                    radiusX: ch * 0.48,
                    radiusY: ch * 0.48,
                };
                dc.DrawRoundedRectangle(&cloud, &brush, 1.3, stroke_opt);
                for dx in [-0.45f32, 0.0, 0.45] {
                    dc.DrawLine(pt(cx + r * dx, cy + r * 0.35), pt(cx + r * (dx - 0.14), cy + r * 0.88), &brush, 1.25, stroke_opt);
                }
            }
            VectorIcon::CloudLightning => {
                let cw = r * 1.75;
                let ch = r * 0.76;
                let cloud = D2D1_ROUNDED_RECT {
                    rect: RectF { x: cx - cw * 0.5, y: cy - r * 0.74, w: cw, h: ch }.to_d2d(),
                    radiusX: ch * 0.48,
                    radiusY: ch * 0.48,
                };
                dc.DrawRoundedRectangle(&cloud, &brush, 1.3, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.14, cy + r * 0.08), pt(cx - r * 0.18, cy + r * 0.52), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.18, cy + r * 0.52), pt(cx + r * 0.16, cy + r * 0.52), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.16, cy + r * 0.52), pt(cx - r * 0.14, cy + r * 1.02), &brush, 1.35, stroke_opt);
            }
            VectorIcon::Wind => {
                for (dy, end_x) in [(-0.46f32, 0.85f32), (0.0, 1.05), (0.46, 0.65)] {
                    let py = cy + r * dy;
                    dc.DrawLine(pt(cx - r * 1.0, py), pt(cx + r * end_x, py), &brush, 1.35, stroke_opt);
                }
            }
            VectorIcon::Waves => {
                for row_y in [-0.36f32, 0.28] {
                    let py = cy + r * row_y;
                    let sx = r * 0.96;
                    if let Some(fac) = factory {
                        let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                        let sink = path.Open()?;
                        sink.BeginFigure(pt(cx - sx, py), D2D1_FIGURE_BEGIN_HOLLOW);
                        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                            point1: pt(cx - sx * 0.35, py - r * 0.38),
                            point2: pt(cx + sx * 0.35, py + r * 0.38),
                            point3: pt(cx + sx, py),
                        });
                        sink.EndFigure(D2D1_FIGURE_END_OPEN);
                        sink.Close()?;
                        dc.DrawGeometry(&path, &brush, 1.35, stroke_opt);
                    } else {
                        dc.DrawLine(pt(cx - sx, py), pt(cx + sx, py), &brush, 1.35, stroke_opt);
                    }
                }
            }
            VectorIcon::PulseWave => {
                dc.DrawLine(pt(cx - r * 1.08, cy + r * 0.1), pt(cx - r * 0.45, cy + r * 0.1), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.45, cy + r * 0.1), pt(cx - r * 0.15, cy - r * 0.72), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.15, cy - r * 0.72), pt(cx + r * 0.20, cy + r * 0.72), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.20, cy + r * 0.72), pt(cx + r * 0.48, cy - r * 0.05), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.48, cy - r * 0.05), pt(cx + r * 1.08, cy - r * 0.05), &brush, 1.35, stroke_opt);
            }
            VectorIcon::Moon => {
                let rm = (r * 0.92).max(4.0);
                if let Some(fac) = factory {
                    let path: ID2D1PathGeometry1 = fac.CreatePathGeometry()?;
                    let sink = path.Open()?;
                    sink.BeginFigure(pt(cx + rm * 0.25, cy - rm * 0.92), D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx - rm * 1.1, cy - rm * 0.75),
                        point2: pt(cx - rm * 1.05, cy + rm * 0.95),
                        point3: pt(cx + rm * 0.78, cy + rm * 0.55),
                    });
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: pt(cx - rm * 0.05, cy + rm * 0.55),
                        point2: pt(cx - rm * 0.18, cy - rm * 0.25),
                        point3: pt(cx + rm * 0.25, cy - rm * 0.92),
                    });
                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    sink.Close()?;
                    dc.DrawGeometry(&path, &brush, 1.35, stroke_opt);
                } else {
                    dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rm * 0.78, radiusY: rm * 0.78 }, &brush, 1.35, stroke_opt);
                }
            }
            VectorIcon::Sun => {
                // Classic radiant Sun (distinct from gear: compact core with clean gap & cardinal/diagonal rays)
                let r_core = (r * 0.33).max(1.8);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r_core, radiusY: r_core }, &brush, 1.2, stroke_opt);
                let r_in = r * 0.54;
                for i in 0..8 {
                    let angle = i as f32 * std::f32::consts::FRAC_PI_4;
                    let r_out = if i % 2 == 0 { r * 0.98 } else { r * 0.84 };
                    let cos = angle.cos();
                    let sin = angle.sin();
                    dc.DrawLine(
                        pt(cx + r_in * cos, cy + r_in * sin),
                        pt(cx + r_out * cos, cy + r_out * sin),
                        &brush,
                        1.15,
                        stroke_opt,
                    );
                }
            }
            VectorIcon::Sunrise => {
                // Morning sunrise: horizon baseline with rising sun dome and upward morning rays
                let horiz_y = cy + r * 0.38;
                dc.DrawLine(pt(cx - r * 0.92, horiz_y), pt(cx + r * 0.92, horiz_y), &brush, 1.25, stroke_opt);
                let r_dome = (r * 0.42).max(2.4);
                if let Some(fact) = factory {
                    if let Ok(path) = fact.CreatePathGeometry() {
                        if let Ok(sink) = path.Open() {
                            sink.BeginFigure(pt(cx - r_dome, horiz_y), D2D1_FIGURE_BEGIN_HOLLOW);
                            sink.AddArc(&D2D1_ARC_SEGMENT {
                                point: pt(cx + r_dome, horiz_y),
                                size: D2D_SIZE_F { width: r_dome, height: r_dome },
                                rotationAngle: 0.0,
                                sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                                arcSize: D2D1_ARC_SIZE_SMALL,
                            });
                            sink.EndFigure(D2D1_FIGURE_END_OPEN);
                            let _ = sink.Close();
                            dc.DrawGeometry(&path, &brush, 1.25, stroke_opt);
                        }
                    }
                } else {
                    dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, horiz_y), radiusX: r_dome, radiusY: r_dome }, &brush, 1.25, stroke_opt);
                }
                // Radiating upward morning rays
                dc.DrawLine(pt(cx, cy - r * 0.15), pt(cx, cy - r * 0.86), &brush, 1.2, stroke_opt);
                dc.DrawLine(pt(cx - r * 0.36, cy + r * 0.02), pt(cx - r * 0.68, cy - r * 0.42), &brush, 1.2, stroke_opt);
                dc.DrawLine(pt(cx + r * 0.36, cy + r * 0.02), pt(cx + r * 0.68, cy - r * 0.42), &brush, 1.2, stroke_opt);
            }
            VectorIcon::Ban => {
                let rad = (r * 0.92).max(4.0);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad, radiusY: rad }, &brush, 1.35, stroke_opt);
                let d = rad * 0.68;
                dc.DrawLine(pt(cx - d, cy - d), pt(cx + d, cy + d), &brush, 1.35, stroke_opt);
            }
            VectorIcon::Globe => {
                let rad = (r * 0.94).max(4.0);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad, radiusY: rad }, &brush, 1.3, stroke_opt);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad * 0.44, radiusY: rad }, &brush, 1.1, stroke_opt);
                dc.DrawLine(pt(cx - rad * 0.95, cy), pt(cx + rad * 0.95, cy), &brush, 1.15, stroke_opt);
            }
            VectorIcon::Palette => {
                let rad = (r * 0.96).max(4.2);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad, radiusY: rad * 0.88 }, &brush, 1.3, stroke_opt);
                for (dx, dy) in [(-0.42f32, -0.18f32), (-0.05, -0.44), (0.36, -0.16), (0.18, 0.36)] {
                    dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx + rad * dx, cy + rad * dy), radiusX: 1.0, radiusY: 1.0 }, &brush);
                }
            }
            VectorIcon::Bell => {
                let bw = r * 0.82;
                let bh = r * 0.94;
                dc.DrawLine(pt(cx - bw * 0.72, cy - bh * 0.15), pt(cx - bw * 0.98, cy + bh * 0.48), &brush, 1.3, stroke_opt);
                dc.DrawLine(pt(cx + bw * 0.72, cy - bh * 0.15), pt(cx + bw * 0.98, cy + bh * 0.48), &brush, 1.3, stroke_opt);
                dc.DrawLine(pt(cx - bw * 0.98, cy + bh * 0.48), pt(cx + bw * 0.98, cy + bh * 0.48), &brush, 1.35, stroke_opt);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy - bh * 0.25), radiusX: bw * 0.72, radiusY: bh * 0.56 }, &brush, 1.3, stroke_opt);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy + bh * 0.84), radiusX: 1.15, radiusY: 1.15 }, &brush);
            }
            VectorIcon::RingCircle => {
                let rad = (r * 0.88).max(3.8);
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rad, radiusY: rad }, &brush, 1.5, stroke_opt);
            }
            VectorIcon::HourglassMini => {
                let hw = r * 0.76;
                let hh = r * 0.92;
                dc.DrawLine(pt(cx - hw, cy - hh), pt(cx + hw, cy - hh), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - hw, cy + hh), pt(cx + hw, cy + hh), &brush, 1.35, stroke_opt);
                dc.DrawLine(pt(cx - hw, cy - hh), pt(cx + hw, cy + hh), &brush, 1.3, stroke_opt);
                dc.DrawLine(pt(cx + hw, cy - hh), pt(cx - hw, cy + hh), &brush, 1.3, stroke_opt);
            }
            VectorIcon::BarsMini => {
                for (dx, top) in [(-0.55f32, 0.25f32), (0.0, -0.55), (0.55, -0.15)] {
                    dc.DrawLine(pt(cx + r * dx, cy + r * 0.75), pt(cx + r * dx, cy + r * top), &brush, 1.65, stroke_opt);
                }
            }
            VectorIcon::DotsMini => {
                for dx in [-0.7f32, 0.0, 0.7] {
                    dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx + r * dx, cy), radiusX: 1.35, radiusY: 1.35 }, &brush);
                }
            }
            VectorIcon::OrbitMini => {
                dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r * 0.96, radiusY: r * 0.45 }, &brush, 1.3, stroke_opt);
                dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: 1.5, radiusY: 1.5 }, &brush);
            }
        }
    }
    Ok(())
}

fn draw_vector_pill_fx(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    rect_opt: Option<RectF>,
    icon: VectorIcon,
    hovered: bool,
    active: bool,
    accent: &str,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let is_close = icon == VectorIcon::Close;
    let fill = if active {
        hex_color(accent, 0.24)
    } else if hovered {
        if is_close { rgba(220, 53, 69, 0.85) } else { rgba(255, 255, 255, 0.14) }
    } else {
        rgba(0, 0, 0, 0.0)
    };
    let border = if active { hex_color(accent, 0.65) } else { rgba(0, 0, 0, 0.0) };
    if fill.a > 0.0 || border.a > 0.0 {
        draw_rounded_card(dc, rect, 7.0, fill, border, 1.0)?;
    }
    let fg = if active {
        hex_color(accent, 0.98)
    } else if hovered {
        rgba(255, 255, 255, 0.98)
    } else {
        rgba(218, 218, 228, 0.88)
    };
    draw_vector_icon(dc, Some(factory), rect, icon, fg)
}

fn draw_rotated_vector_icon(
    dc: &ID2D1DeviceContext,
    factory: Option<&ID2D1Factory1>,
    rect: RectF,
    icon: VectorIcon,
    color: D2D1_COLOR_F,
    angle_deg: f32,
) -> Result<()> {
    if angle_deg.abs() <= 0.1 {
        return draw_vector_icon(dc, factory, rect, icon, color);
    }
    let center = pt(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5);
    let mut prev = Matrix3x2::identity();
    unsafe {
        dc.GetTransform(&mut prev);
        dc.SetTransform(&(Matrix3x2::rotation_around(angle_deg, center) * prev));
    }
    let res = draw_vector_icon(dc, factory, rect, icon, color);
    unsafe {
        dc.SetTransform(&prev);
    }
    res
}

fn draw_rotated_pill_fx(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    rect: RectF,
    icon: VectorIcon,
    hovered: bool,
    active: bool,
    accent: &str,
    angle_deg: f32,
) -> Result<()> {
    let fill = if active {
        hex_color(accent, 0.24)
    } else if hovered {
        rgba(255, 255, 255, 0.14)
    } else {
        rgba(0, 0, 0, 0.0)
    };
    let border = if active { hex_color(accent, 0.65) } else { rgba(0, 0, 0, 0.0) };
    if fill.a > 0.0 || border.a > 0.0 {
        draw_rounded_card(dc, rect, 7.0, fill, border, 1.0)?;
    }
    let fg = if active {
        hex_color(accent, 0.98)
    } else if hovered {
        rgba(255, 255, 255, 0.98)
    } else {
        rgba(218, 218, 228, 0.88)
    };
    draw_rotated_vector_icon(dc, Some(factory), rect, icon, fg, angle_deg)
}

fn draw_icon_badge(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    rect: RectF,
    radius: f32,
    icon: VectorIcon,
    tint_hex: &str,
) -> Result<()> {
    draw_rounded_card(
        dc,
        rect,
        radius,
        hex_color(tint_hex, 0.14),
        hex_color(tint_hex, 0.36),
        1.0,
    )?;
    let pad = (rect.w.min(rect.h) * 0.22).clamp(3.0, 7.0);
    draw_vector_icon(
        dc,
        Some(factory),
        RectF {
            x: rect.x + pad,
            y: rect.y + pad,
            w: (rect.w - pad * 2.0).max(8.0),
            h: (rect.h - pad * 2.0).max(8.0),
        },
        icon,
        hex_color(tint_hex, 0.96),
    )
}

fn draw_vector_pill(
    dc: &ID2D1DeviceContext,
    rect_opt: Option<RectF>,
    icon: VectorIcon,
    hovered: bool,
    active: bool,
    accent: &str,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let is_close = icon == VectorIcon::Close;
    let fill = if active {
        hex_color(accent, 0.24)
    } else if hovered {
        if is_close { rgba(220, 53, 69, 0.85) } else { rgba(255, 255, 255, 0.14) }
    } else {
        rgba(0, 0, 0, 0.0)
    };
    let border = if active { hex_color(accent, 0.65) } else { rgba(0, 0, 0, 0.0) };
    if fill.a > 0.0 || border.a > 0.0 {
        draw_rounded_card(dc, rect, 7.0, fill, border, 1.0)?;
    }
    let fg = if active {
        hex_color(accent, 0.98)
    } else if hovered {
        rgba(255, 255, 255, 0.98)
    } else {
        rgba(218, 218, 228, 0.88)
    };
    draw_vector_icon(dc, None, rect, icon, fg)
}

fn draw_vector_button(
    dc: &ID2D1DeviceContext,
    rect_opt: Option<RectF>,
    icon: VectorIcon,
    hovered: bool,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let fill = if hovered { rgba(70, 70, 80, 0.95) } else { rgba(52, 52, 60, 0.90) };
    draw_rounded_card(dc, rect, 7.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
    draw_vector_icon(dc, None, rect, icon, rgba(242, 242, 248, 0.94))
}

fn draw_accent_stepper_row(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value_text: &str,
    minus_r: Option<RectF>,
    input_r: Option<RectF>,
    plus_r: Option<RectF>,
    hovered: Option<ControlId>,
    minus_id: ControlId,
    input_id: ControlId,
    plus_id: ControlId,
    is_editing: bool,
    accent: &str,
) -> Result<()> {
    let row = RectF { x, y, w, h: 36.0 };
    draw_rounded_card(dc, row, 9.0, rgba(34, 34, 38, 0.82), rgba(255, 255, 255, 0.08), 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        label,
        RectF { x: x + 12.0, y, w: (w - 154.0).max(60.0), h: 36.0 },
        12.0,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(232, 232, 240, 0.96),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;
    if let Some(mr) = minus_r {
        let fill = if hovered == Some(minus_id) { rgba(70, 70, 80, 0.96) } else { rgba(52, 52, 59, 0.94) };
        draw_rounded_card(dc, mr, 6.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "−", mr, 13.0, DWRITE_FONT_WEIGHT_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    if let Some(ir) = input_r {
        if is_editing {
            draw_rounded_card(dc, ir, 6.0, rgba(10, 12, 16, 0.58), hex_color(accent, 0.85), 1.2)?;
        } else if hovered == Some(input_id) {
            draw_rounded_card(dc, ir, 6.0, rgba(255, 255, 255, 0.06), rgba(255, 255, 255, 0.16), 1.0)?;
        }
        draw_text_nowrap(
            dc,
            dwrite,
            value_text,
            ir,
            14.2,
            DWRITE_FONT_WEIGHT_BOLD,
            hex_color(accent, 0.98),
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )?;
        if let Some(pr) = plus_r {
            let unit_x = ir.x + ir.w + 2.0;
            let unit_w = (pr.x - unit_x - 2.0).max(20.0);
            draw_text_nowrap(
                dc,
                dwrite,
                "mins",
                RectF { x: unit_x, y, w: unit_w, h: 36.0 },
                11.0,
                DWRITE_FONT_WEIGHT_NORMAL,
                rgba(178, 178, 190, 0.88),
                DWRITE_TEXT_ALIGNMENT_CENTER,
            )?;
        }
    }
    if let Some(pr) = plus_r {
        let fill = if hovered == Some(plus_id) { rgba(70, 70, 80, 0.96) } else { rgba(52, 52, 59, 0.94) };
        draw_rounded_card(dc, pr, 6.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
        draw_text_nowrap(dc, dwrite, "+", pr, 13.0, DWRITE_FONT_WEIGHT_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    Ok(())
}

fn draw_dashed_divider(dc: &ID2D1DeviceContext, x: f32, y: f32, w: f32) -> Result<()> {
    let brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.10), None) }?;
    let mut cur = x;
    unsafe {
        while cur < x + w {
            let end = (cur + 4.0).min(x + w);
            dc.DrawLine(pt(cur, y), pt(end, y), &brush, 1.0, None);
            cur += 8.0;
        }
    }
    Ok(())
}

fn draw_setting_inline_row(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    rect: RectF,
    title: &str,
    sub: Option<&str>,
    enabled: bool,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    if hovered {
        draw_rounded_card(dc, rect, 7.0, rgba(255, 255, 255, 0.05), rgba(0, 0, 0, 0.0), 0.0)?;
    }
    let tw = (rect.w - 52.0).max(80.0);
    if let Some(sub_txt) = sub {
        draw_text_nowrap(dc, dwrite, title, RectF { x: rect.x + 2.0, y: rect.y + 4.0, w: tw, h: 17.0 }, 11.6, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
        draw_text_nowrap(dc, dwrite, sub_txt, RectF { x: rect.x + 2.0, y: rect.y + 21.0, w: tw, h: 15.0 }, 10.2, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    } else {
        draw_text_nowrap(dc, dwrite, title, RectF { x: rect.x + 2.0, y: rect.y, w: tw, h: rect.h }, 11.6, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(242, 242, 248, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    }
    draw_switch_pill(
        dc,
        RectF { x: rect.x + rect.w - 38.0, y: rect.y + (rect.h - 18.0) * 0.5, w: 34.0, h: 18.0 },
        enabled,
        accent,
    )
}

fn draw_setting_card_row(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    rect: RectF,
    title: &str,
    sub: &str,
    toggle: Option<bool>,
    badge: Option<&str>,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    let fill = if hovered { rgba(48, 48, 54, 0.96) } else { rgba(38, 38, 42, 0.94) };
    let border = if toggle == Some(true) { hex_color(accent, 0.45) } else { rgba(255, 255, 255, 0.07) };
    draw_rounded_card(dc, rect, 10.0, fill, border, 1.0)?;
    let right_pad = if toggle.is_some() { 54.0 } else { 108.0 };
    let tw = (rect.w - 18.0 - right_pad).max(80.0);
    draw_text_nowrap(dc, dwrite, title, RectF { x: rect.x + 14.0, y: rect.y + 6.0, w: tw, h: 18.0 }, 12.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(244, 244, 250, 0.96), DWRITE_TEXT_ALIGNMENT_LEADING)?;
    draw_text_nowrap(dc, dwrite, sub, RectF { x: rect.x + 14.0, y: rect.y + 24.0, w: tw, h: 16.0 }, 10.3, DWRITE_FONT_WEIGHT_NORMAL, rgba(161, 161, 170, 0.88), DWRITE_TEXT_ALIGNMENT_LEADING)?;

    if let Some(enabled) = toggle {
        draw_switch_pill(
            dc,
            RectF { x: rect.x + rect.w - 48.0, y: rect.y + (rect.h - 18.0) * 0.5, w: 34.0, h: 18.0 },
            enabled,
            accent,
        )?;
    } else if let Some(btxt) = badge {
        let br = RectF { x: rect.x + rect.w - 102.0, y: rect.y + (rect.h - 26.0) * 0.5, w: 88.0, h: 26.0 };
        draw_rounded_card(dc, br, 7.0, hex_color(accent, 0.16), hex_color(accent, 0.55), 1.0)?;
        draw_text_nowrap(dc, dwrite, btxt, br, 10.8, DWRITE_FONT_WEIGHT_SEMI_BOLD, rgba(245, 245, 252, 0.96), DWRITE_TEXT_ALIGNMENT_CENTER)?;
    }
    Ok(())
}

// --- Shared drawing primitives ---

fn draw_rounded_card(
    dc: &ID2D1DeviceContext,
    rect: RectF,
    radius: f32,
    fill: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    border_w: f32,
) -> Result<()> {
    let rr = D2D1_ROUNDED_RECT {
        rect: rect.to_d2d(),
        radiusX: radius,
        radiusY: radius,
    };
    let fill_brush = unsafe { dc.CreateSolidColorBrush(&fill, None) }?;
    unsafe {
        dc.FillRoundedRectangle(&rr, &fill_brush);
    }
    if border_w > 0.0 && border.a > 0.0 {
        let stroke_brush = unsafe { dc.CreateSolidColorBrush(&border, None) }?;
        unsafe {
            dc.DrawRoundedRectangle(&rr, &stroke_brush, border_w, None);
        }
    }
    Ok(())
}

fn draw_neutral_segment_button(
    dc: &ID2D1DeviceContext,
    factory: Option<&ID2D1Factory1>,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    icon: Option<VectorIcon>,
    active: bool,
    hovered: bool,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let (fill, border, fg) = if active {
        (
            rgba(56, 56, 64, 0.96),
            rgba(255, 255, 255, 0.12),
            rgba(246, 246, 250, 0.98),
        )
    } else if hovered {
        (
            rgba(40, 40, 46, 0.72),
            rgba(255, 255, 255, 0.06),
            rgba(232, 232, 240, 0.94),
        )
    } else {
        (rgba(0, 0, 0, 0.0), rgba(0, 0, 0, 0.0), rgba(164, 164, 174, 0.88))
    };
    if fill.a > 0.0 || border.a > 0.0 {
        draw_rounded_card(dc, rect, 7.0, fill, border, 1.0)?;
    }
    let weight = if active { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM };
    if let Some(ic) = icon {
        let tw = measure_text_width(dwrite, label, 11.4, weight);
        let icon_w = 13.0;
        let gap = 5.0;
        let start_x = rect.x + ((rect.w - (icon_w + gap + tw)).max(6.0)) * 0.5;
        draw_vector_icon(dc, factory, RectF { x: start_x, y: rect.y + (rect.h - 13.0) * 0.5, w: 13.0, h: 13.0 }, ic, fg)?;
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF { x: start_x + icon_w + gap, y: rect.y, w: (rect.x + rect.w - 4.0 - (start_x + icon_w + gap)).max(20.0), h: rect.h },
            11.4,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )
    } else {
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF { x: rect.x + 6.0, y: rect.y, w: (rect.w - 12.0).max(16.0), h: rect.h },
            11.4,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )
    }
}

fn draw_guard_tab_pill_icon(
    dc: &ID2D1DeviceContext,
    factory: Option<&ID2D1Factory1>,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    icon: Option<VectorIcon>,
    active: bool,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let (fill, border, fg) = if active {
        (
            hex_color(accent, 0.96),
            rgba(255, 255, 255, 0.14),
            contrast_fg(accent),
        )
    } else if hovered {
        (
            rgba(52, 52, 60, 0.80),
            rgba(255, 255, 255, 0.08),
            rgba(236, 236, 244, 0.96),
        )
    } else {
        (rgba(0, 0, 0, 0.0), rgba(0, 0, 0, 0.0), rgba(182, 182, 192, 0.90))
    };
    if fill.a > 0.0 || border.a > 0.0 {
        draw_rounded_card(dc, rect, 7.0, fill, border, 1.0)?;
    }
    let weight = if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM };
    if let Some(ic) = icon {
        let tw = measure_text_width(dwrite, label, 11.1, weight);
        let icon_w = 12.0;
        let gap = 5.0;
        let start_x = rect.x + ((rect.w - (icon_w + gap + tw)).max(6.0)) * 0.5;
        draw_vector_icon(
            dc,
            factory,
            RectF { x: start_x, y: rect.y + (rect.h - 12.0) * 0.5, w: 12.0, h: 12.0 },
            ic,
            fg,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF {
                x: start_x + icon_w + gap,
                y: rect.y,
                w: (rect.x + rect.w - 4.0 - (start_x + icon_w + gap)).max(20.0),
                h: rect.h,
            },
            11.1,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )
    } else {
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF { x: rect.x + 6.0, y: rect.y, w: (rect.w - 12.0).max(16.0), h: rect.h },
            11.4,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )
    }
}

fn draw_segment_button_icon(
    dc: &ID2D1DeviceContext,
    factory: Option<&ID2D1Factory1>,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    icon: Option<VectorIcon>,
    active: bool,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let (fill, border, fg) = if active {
        (
            hex_color(accent, 0.95),
            rgba(255, 255, 255, 0.12),
            contrast_fg(accent),
        )
    } else if hovered {
        (
            rgba(58, 58, 66, 0.92),
            rgba(255, 255, 255, 0.14),
            rgba(240, 240, 248, 0.95),
        )
    } else {
        (
            rgba(43, 43, 49, 0.88),
            rgba(255, 255, 255, 0.07),
            rgba(205, 205, 215, 0.90),
        )
    };
    draw_rounded_card(dc, rect, 8.0, fill, border, 1.0)?;
    let weight = if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM };
    let font_sz = if icon.is_some() && rect.w < 135.0 { 10.6 } else { 11.2 };
    if let Some(ic) = icon {
        let tw = measure_text_width(dwrite, label, font_sz, weight);
        let icon_w = 11.5;
        let gap = 4.5;
        let start_x = rect.x + ((rect.w - (icon_w + gap + tw)).max(5.0)) * 0.5;
        draw_vector_icon(
            dc,
            factory,
            RectF { x: start_x, y: rect.y + (rect.h - 12.0) * 0.5, w: 12.0, h: 12.0 },
            ic,
            fg,
        )?;
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF {
                x: start_x + icon_w + gap,
                y: rect.y,
                w: (rect.x + rect.w - 4.0 - (start_x + icon_w + gap)).max(18.0),
                h: rect.h,
            },
            font_sz,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_LEADING,
        )
    } else {
        draw_text_nowrap(
            dc,
            dwrite,
            label,
            RectF { x: rect.x + 6.0, y: rect.y, w: (rect.w - 12.0).max(16.0), h: rect.h },
            font_sz,
            weight,
            fg,
            DWRITE_TEXT_ALIGNMENT_CENTER,
        )
    }
}

fn draw_segment_button(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    active: bool,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let (fill, border, fg) = if active {
        (
            hex_color(accent, 0.95),
            rgba(255, 255, 255, 0.12),
            contrast_fg(accent),
        )
    } else if hovered {
        (
            rgba(58, 58, 66, 0.92),
            rgba(255, 255, 255, 0.14),
            rgba(240, 240, 248, 0.95),
        )
    } else {
        (
            rgba(43, 43, 49, 0.88),
            rgba(255, 255, 255, 0.07),
            rgba(205, 205, 215, 0.90),
        )
    };
    draw_rounded_card(dc, rect, 8.0, fill, border, 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        label,
        RectF { x: rect.x + 6.0, y: rect.y, w: (rect.w - 12.0).max(16.0), h: rect.h },
        11.5,
        if active { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_MEDIUM },
        fg,
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )
}

fn draw_subtle_button(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    hovered: bool,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let fill = if hovered { rgba(70, 70, 80, 0.95) } else { rgba(52, 52, 60, 0.90) };
    draw_rounded_card(dc, rect, 7.0, fill, rgba(255, 255, 255, 0.10), 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        label,
        RectF { x: rect.x + 6.0, y: rect.y, w: (rect.w - 12.0).max(12.0), h: rect.h },
        11.5,
        DWRITE_FONT_WEIGHT_SEMI_BOLD,
        rgba(242, 242, 248, 0.94),
        DWRITE_TEXT_ALIGNMENT_CENTER,
    )
}

fn draw_switch_pill(
    dc: &ID2D1DeviceContext,
    pill: RectF,
    enabled: bool,
    accent: &str,
) -> Result<()> {
    let pill_fill = if enabled { hex_color(accent, 0.95) } else { rgba(70, 70, 78, 0.95) };
    draw_rounded_card(dc, pill, pill.h * 0.5, pill_fill, rgba(255, 255, 255, 0.12), 0.5)?;
    let knob_r = (pill.h - 5.0).max(4.0) * 0.5;
    let cy = pill.y + pill.h * 0.5;
    let knob_x = if enabled { pill.x + pill.w - pill.h * 0.5 } else { pill.x + pill.h * 0.5 };
    let knob_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.98), None) }?;
    unsafe {
        dc.FillEllipse(&D2D1_ELLIPSE { point: pt(knob_x, cy), radiusX: knob_r, radiusY: knob_r }, &knob_brush);
    }
    Ok(())
}

fn format_completed_value(mins: u32, fmt: &str) -> String {
    match fmt {
        "decimal" => format!("{:.1} hours", mins as f32 / 60.0),
        "smart" if mins >= 60 => format!("{}h {}m", mins / 60, mins % 60),
        _ => format!("{mins} minutes"),
    }
}

fn format_timer_subtitle(view: &RenderView<'_>) -> String {
    match view.config.timer_subtitle_mode.as_str() {
        "finishtime" => {
            let left = view.timer.total_periods().saturating_sub(view.timer.current_period());
            if left > 0 {
                let s = if left == 1 { "" } else { "s" };
                format!("Ends at {} • {} period{} left", view.ends_at_text, left, s)
            } else {
                format!("Ends at {} • Final period", view.ends_at_text)
            }
        }
        "totalrem" => {
            let total_str = FocusTimer::formatted_total_duration(view.timer.total_remaining_sec());
            format!("{total_str} total remaining")
        }
        _ => {
            let rem_sec = view.timer.remaining_sec();
            let cur = view.timer.current_period();
            let tot = view.timer.total_periods();
            match view.timer.phase() {
                SessionPhase::Resting => {
                    if rem_sec <= 60 && rem_sec > 0 {
                        format!("Break ends in {rem_sec}s")
                    } else {
                        let m = rem_sec.div_ceil(60);
                        let s = if m == 1 { "" } else { "s" };
                        format!("Break ends in {m} min{s}")
                    }
                }
                SessionPhase::Focusing => {
                    let has_more_breaks = cur < tot && !view.config.skip_breaks;
                    if has_more_breaks {
                        if rem_sec <= 600 {
                            if rem_sec <= 60 && rem_sec > 0 {
                                format!("Break in {rem_sec}s")
                            } else {
                                let m = rem_sec.div_ceil(60);
                                let s = if m == 1 { "" } else { "s" };
                                format!("Break in {m} min{s}")
                            }
                        } else if view.config.is_health_mode {
                            "Posture Guard active".to_string()
                        } else {
                            "Stay focused".to_string()
                        }
                    } else if view.config.skip_breaks && cur < tot {
                        if rem_sec <= 600 {
                            if rem_sec <= 60 && rem_sec > 0 {
                                format!("Next period in {rem_sec}s")
                            } else {
                                let m = rem_sec.div_ceil(60);
                                let s = if m == 1 { "" } else { "s" };
                                format!("Next period in {m} min{s}")
                            }
                        } else {
                            format!("Continuous focus ({cur}/{tot})")
                        }
                    } else if rem_sec <= 600 {
                        if rem_sec <= 60 && rem_sec > 0 {
                            format!("Final stretch ({rem_sec}s)")
                        } else {
                            let m = rem_sec.div_ceil(60);
                            format!("Final period ({m}m left)")
                        }
                    } else if view.config.is_health_mode {
                        "Final posture interval".to_string()
                    } else {
                        "Final focus period".to_string()
                    }
                }
                SessionPhase::Idle => "Focus session".to_string(),
            }
        }
    }
}

#[allow(dead_code)]
fn draw_toggle_chip(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    rect_opt: Option<RectF>,
    label: &str,
    enabled: bool,
    hovered: bool,
    accent: &str,
) -> Result<()> {
    let Some(r) = rect_opt else {
        return Ok(());
    };
    let bg = if hovered { rgba(55, 55, 64, 0.94) } else { rgba(42, 42, 49, 0.88) };
    let border = if enabled { hex_color(accent, 0.55) } else { rgba(255, 255, 255, 0.07) };
    draw_rounded_card(dc, r, 8.0, bg, border, 1.0)?;
    draw_text_nowrap(
        dc,
        dwrite,
        label,
        RectF { x: r.x + 10.0, y: r.y, w: (r.w - 50.0).max(36.0), h: r.h },
        11.2,
        DWRITE_FONT_WEIGHT_MEDIUM,
        rgba(236, 236, 244, 0.94),
        DWRITE_TEXT_ALIGNMENT_LEADING,
    )?;

    draw_switch_pill(
        dc,
        RectF { x: r.x + r.w - 42.0, y: r.y + (r.h - 18.0) * 0.5, w: 32.0, h: 18.0 },
        enabled,
        accent,
    )
}

fn draw_icon_circle_button(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    rect_opt: Option<RectF>,
    icon: VectorIcon,
    fill: D2D1_COLOR_F,
    border: D2D1_COLOR_F,
    fg: D2D1_COLOR_F,
    hovered: bool,
) -> Result<()> {
    let Some(rect) = rect_opt else {
        return Ok(());
    };
    let cx = rect.x + rect.w * 0.5;
    let cy = rect.y + rect.h * 0.5;
    let r = rect.w.min(rect.h) * 0.5;
    let fill_adj = if hovered {
        D2D1_COLOR_F {
            r: (fill.r + 0.08).min(1.0),
            g: (fill.g + 0.08).min(1.0),
            b: (fill.b + 0.08).min(1.0),
            a: (fill.a + 0.08).min(1.0),
        }
    } else {
        fill
    };
    let brush = unsafe { dc.CreateSolidColorBrush(&fill_adj, None) }?;
    unsafe {
        dc.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &brush);
    }
    if border.a > 0.0 {
        let stroke = unsafe { dc.CreateSolidColorBrush(&border, None) }?;
        unsafe {
            dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &stroke, 1.0, None);
        }
    }
    draw_vector_icon(dc, Some(factory), rect, icon, fg)
}

fn draw_text_with_fmt(
    dc: &ID2D1DeviceContext,
    fmt: &windows::Win32::Graphics::DirectWrite::IDWriteTextFormat,
    text: &str,
    rect: RectF,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let utf16: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        dc.DrawText(
            &utf16,
            fmt,
            &rect.to_d2d(),
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    Ok(())
}

fn draw_text(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format(dwrite, "Segoe UI", size, weight, align, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    draw_text_with_fmt(dc, &fmt, text, rect, color)
}

fn draw_text_shadow(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format(dwrite, "Segoe UI", size, weight, align, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    let shadow = RectF { x: rect.x + 1.0, y: rect.y + 1.0, w: rect.w, h: rect.h };
    draw_text_with_fmt(dc, &fmt, text, shadow, rgba(0, 0, 0, 0.72 * color.a))?;
    draw_text_with_fmt(dc, &fmt, text, rect, color)
}

fn draw_text_shadow_nowrap(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format_single_line(dwrite, "Segoe UI", size, weight, align, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    let shadow = RectF { x: rect.x + 1.0, y: rect.y + 1.0, w: rect.w, h: rect.h };
    draw_text_with_fmt(dc, &fmt, text, shadow, rgba(0, 0, 0, 0.72 * color.a))?;
    draw_text_with_fmt(dc, &fmt, text, rect, color)
}

fn draw_text_outlined_fmt(
    dc: &ID2D1DeviceContext,
    fmt: &windows::Win32::Graphics::DirectWrite::IDWriteTextFormat,
    text: &str,
    rect: RectF,
    color: D2D1_COLOR_F,
) -> Result<()> {
    // Clean soft multi-pass drop shadow (replaces harsh 4-way black outline stroke)
    for (dx, dy, alpha) in [(0.0f32, 2.2f32, 0.28f32), (0.0, 1.1, 0.56)] {
        draw_text_with_fmt(
            dc,
            fmt,
            text,
            RectF { x: rect.x + dx, y: rect.y + dy, w: rect.w, h: rect.h },
            rgba(0, 0, 0, alpha * color.a),
        )?;
    }
    draw_text_with_fmt(dc, fmt, text, rect, color)
}

fn draw_text_outline(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format_single_line(dwrite, "Segoe UI", size, weight, align, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    draw_text_outlined_fmt(dc, &fmt, text, rect, color)
}

fn draw_text_outline_wrap(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format(dwrite, "Segoe UI", size, weight, align, DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    draw_text_outlined_fmt(dc, &fmt, text, rect, color)
}

fn measure_text_width(
    dwrite: &IDWriteFactory,
    text: &str,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let Ok(fmt) = create_text_format_single_line(
        dwrite,
        "Segoe UI",
        size,
        weight,
        DWRITE_TEXT_ALIGNMENT_LEADING,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    ) else {
        return text.chars().count() as f32 * size * 0.52;
    };
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let Ok(tl) = (unsafe { dwrite.CreateTextLayout(&utf16, &fmt, 2000.0, size * 2.0) }) else {
        return text.chars().count() as f32 * size * 0.52;
    };
    let mut metrics = DWRITE_TEXT_METRICS::default();
    if unsafe { tl.GetMetrics(&mut metrics) }.is_ok() {
        metrics.widthIncludingTrailingWhitespace
    } else {
        text.chars().count() as f32 * size * 0.52
    }
}

fn draw_text_nowrap(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    rect: RectF,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    align: windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_ALIGNMENT,
) -> Result<()> {
    let fmt = create_text_format_single_line(
        dwrite,
        "Segoe UI",
        size,
        weight,
        align,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    )?;
    draw_text_with_fmt(dc, &fmt, text, rect, color)
}

fn draw_progress_arc(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    cx: f32,
    cy: f32,
    r: f32,
    stroke_w: f32,
    progress: f32,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let clamped = progress.clamp(0.0, 1.0);
    if clamped <= 0.001 {
        return Ok(());
    }
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    if clamped >= 0.999 {
        unsafe {
            dc.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &brush, stroke_w, None);
        }
        return Ok(());
    }

    let start_angle = -std::f32::consts::FRAC_PI_2;
    let end_angle = start_angle + std::f32::consts::TAU * clamped;
    let start_pt = pt(cx + r * start_angle.cos(), cy + r * start_angle.sin());
    let end_pt = pt(cx + r * end_angle.cos(), cy + r * end_angle.sin());

    let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
    let sink = unsafe { path.Open() }?;
    unsafe {
        sink.BeginFigure(start_pt, D2D1_FIGURE_BEGIN_HOLLOW);
        sink.AddArc(&D2D1_ARC_SEGMENT {
            point: end_pt,
            size: D2D_SIZE_F { width: r, height: r },
            rotationAngle: 0.0,
            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
            arcSize: if clamped > 0.5 { D2D1_ARC_SIZE_LARGE } else { D2D1_ARC_SIZE_SMALL },
        });
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
        sink.Close()?;
    }

    let stroke_style = create_round_stroke_style(factory)?;
    unsafe {
        dc.DrawGeometry(&path, &brush, stroke_w, &stroke_style);
    }
    Ok(())
}

fn create_round_stroke_style(factory: &ID2D1Factory1) -> Result<ID2D1StrokeStyle1> {
    let style: ID2D1StrokeStyle1 = unsafe {
        factory.CreateStrokeStyle(
            &D2D1_STROKE_STYLE_PROPERTIES1 {
                startCap: D2D1_CAP_STYLE_ROUND,
                endCap: D2D1_CAP_STYLE_ROUND,
                ..Default::default()
            },
            None,
        )
    }?;
    Ok(style)
}

fn draw_cover_bitmap_rounded(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    bitmap: &ID2D1Bitmap1,
    rect: RectF,
    radius: f32,
    opacity: f32,
) -> Result<()> {
    let geo = unsafe {
        factory.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
            rect: rect.to_d2d(),
            radiusX: radius,
            radiusY: radius,
        })
    }?;
    let mask: ID2D1Geometry = geo.into();
    let params = D2D1_LAYER_PARAMETERS1 {
        contentBounds: rect.to_d2d(),
        geometricMask: core::mem::ManuallyDrop::new(Some(mask)),
        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        maskTransform: Matrix3x2::identity(),
        opacity: 1.0,
        opacityBrush: core::mem::ManuallyDrop::new(None),
        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
    };
    unsafe {
        dc.PushLayer(&params, None);
    }
    draw_bitmap_cover_fit(dc, bitmap, rect, opacity);
    unsafe {
        dc.PopLayer();
        drop(core::mem::ManuallyDrop::into_inner(params.geometricMask));
    }
    Ok(())
}

fn draw_cover_bitmap_circle(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    bitmap: &ID2D1Bitmap1,
    cx: f32,
    cy: f32,
    radius: f32,
    opacity: f32,
) -> Result<()> {
    let geo = unsafe {
        factory.CreateEllipseGeometry(&D2D1_ELLIPSE {
            point: pt(cx, cy),
            radiusX: radius,
            radiusY: radius,
        })
    }?;
    let bounds = RectF {
        x: cx - radius,
        y: cy - radius,
        w: radius * 2.0,
        h: radius * 2.0,
    };
    let mask: ID2D1Geometry = geo.into();
    let params = D2D1_LAYER_PARAMETERS1 {
        contentBounds: bounds.to_d2d(),
        geometricMask: core::mem::ManuallyDrop::new(Some(mask)),
        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        maskTransform: Matrix3x2::identity(),
        opacity: 1.0,
        opacityBrush: core::mem::ManuallyDrop::new(None),
        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
    };
    unsafe {
        dc.PushLayer(&params, None);
    }
    draw_bitmap_cover_fit(dc, bitmap, bounds, opacity);
    unsafe {
        dc.PopLayer();
        drop(core::mem::ManuallyDrop::into_inner(params.geometricMask));
    }
    Ok(())
}

fn draw_bitmap_cover_fit(dc: &ID2D1DeviceContext, bitmap: &ID2D1Bitmap1, dest: RectF, opacity: f32) {
    let size = unsafe { bitmap.GetSize() };
    if size.width <= 0.0 || size.height <= 0.0 || dest.w <= 0.0 || dest.h <= 0.0 {
        return;
    }
    let scale = (dest.w / size.width).max(dest.h / size.height);
    let src_w = dest.w / scale;
    let src_h = dest.h / scale;
    let src_x = (size.width - src_w) * 0.5;
    let src_y = (size.height - src_h) * 0.5;
    let src_rect = D2D_RECT_F {
        left: src_x,
        top: src_y,
        right: src_x + src_w,
        bottom: src_y + src_h,
    };
    unsafe {
        dc.DrawBitmap(
            bitmap,
            Some(&dest.to_d2d()),
            opacity,
            D2D1_INTERPOLATION_MODE_LINEAR,
            Some(&src_rect),
            None,
        );
    }
}

fn find_hit(layout: &UiLayout, id: ControlId) -> Option<RectF> {
    layout.hit_rects.iter().rev().find(|h| h.id == id).map(|h| h.rect)
}

fn contrast_fg(hex: &str) -> D2D1_COLOR_F {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
            if lum > 0.56 {
                return rgba(18, 18, 22, 0.98);
            }
        }
    }
    rgba(255, 255, 255, 0.98)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_query<'a>(w: f32, h: f32, config: &'a AppConfig) -> LayoutQuery<'a> {
        LayoutQuery {
            w,
            h,
            config,
            guard_tab: GuardTab::BlockedApps,
            visible_app_rows: 0,
            site_rows: COMMON_SITE_PRESETS.len(),
            site_scroll: 0,
            settings_tab: None,
            settings_scroll: 0.0,
            show_ambient_mixer: false,
            show_stats_modal: false,
            show_share_modal: false,
            show_goal_modal: false,
            show_prayer_modal: false,
            prayer_city_rows: 0,
            stats_app_rows: 0,
            show_pip_drawer: false,
            has_track: false,
            phase: SessionPhase::Idle,
        }
    }

    #[test]
    fn pip_hit_test_returns_play_pause_and_media_controls() {
        let mut cfg = AppConfig::default();
        cfg.pip_mode = true;
        let mut q = base_query(195.0, 280.0, &cfg);
        q.has_track = true;
        q.phase = SessionPhase::Focusing;
        let layout = compute_layout(&q);

        let pp = layout.hit_rects.iter().find(|h| h.id == ControlId::BtnPlayPause).unwrap().rect;
        assert_eq!(
            layout.hit_test(pp.x + pp.w * 0.5, pp.y + pp.h * 0.5),
            Some(ControlId::BtnPlayPause)
        );
        let close = layout.hit_rects.iter().find(|h| h.id == ControlId::BtnClose).unwrap().rect;
        assert_eq!(
            layout.hit_test(close.x + close.w * 0.5, close.y + close.h * 0.5),
            Some(ControlId::BtnClose)
        );
        let next = layout.hit_rects.iter().find(|h| h.id == ControlId::BtnMediaNext).unwrap().rect;
        assert_eq!(
            layout.hit_test(next.x + next.w * 0.5, next.y + next.h * 0.5),
            Some(ControlId::BtnMediaNext)
        );
    }

    #[test]
    fn pip_capsule_hit_test_returns_controls_and_underbar() {
        let mut cfg = AppConfig::default();
        cfg.pip_mode = true;
        cfg.pip_style = PipStyle::Capsule;
        let mut q = base_query(280.0, 68.0, &cfg);
        q.has_track = true;
        q.phase = SessionPhase::Focusing;
        let layout = compute_layout(&q);

        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnPlayPause));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTogglePipStyle));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTogglePip));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnClose));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaPrev));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaPlayPause));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaNext));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::DragBar));

        // When no track is active, playback controls must be hidden
        q.has_track = false;
        let layout_no_track = compute_layout(&q);
        assert!(!layout_no_track.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaPlayPause));
        assert!(!layout_no_track.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaPrev));
        assert!(!layout_no_track.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaNext));
    }

    #[test]
    fn dashboard_idle_exposes_steppers_toolbar_and_settings_modal() {
        let cfg = AppConfig::default();
        let mut q = base_query(960.0, 660.0, &cfg);
        q.settings_tab = Some(SettingsTab::Appearance);
        let layout = compute_layout(&q);

        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTotalUp));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnRingTheme(0)));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnToggleStealthPeek));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnSettingsTab(SettingsTab::System)));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::AccentPreset(4)));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTopToolbarSound));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnOpenStats));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnEditGoal));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTopToolbarMusic));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTopToolbarPip));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnTopToolbarAppearance));
        assert!(layout.hit_rects.iter().any(|h| h.id == ControlId::BtnCloseModal));

        let mut q_goal = base_query(960.0, 660.0, &cfg);
        q_goal.show_goal_modal = true;
        let l_goal = compute_layout(&q_goal);
        assert!(l_goal.hit_rects.iter().any(|h| h.id == ControlId::BtnGoalHours(4)));
        assert!(l_goal.hit_rects.iter().any(|h| h.id == ControlId::BtnHealthGoalBreaks(4)));

        let mut q_stats = base_query(980.0, 680.0, &cfg);
        q_stats.show_stats_modal = true;
        q_stats.stats_app_rows = 3;
        let l_stats = compute_layout(&q_stats);
        assert!(l_stats.hit_rects.iter().any(|h| h.id == ControlId::BtnStatsPeriod(StatsPeriod::Yesterday)));
        assert!(l_stats.hit_rects.iter().any(|h| h.id == ControlId::BtnStatsPeriod(StatsPeriod::Week)));
        assert!(l_stats.hit_rects.iter().any(|h| h.id == ControlId::BtnOpenShareCard));
        assert!(l_stats.hit_rects.iter().any(|h| h.id == ControlId::BtnHeatmapPrev));
        let mr = l_stats.modal_rect.unwrap();
        assert_eq!(l_stats.hit_test(mr.x + 24.0, mr.y + 42.0), Some(ControlId::ModalSurface));
        assert_eq!(l_stats.hit_test(l_stats.card_rect.x + 4.0, l_stats.card_rect.y + 4.0), Some(ControlId::ModalBackdrop));

        let mut q_share = base_query(980.0, 680.0, &cfg);
        q_share.show_share_modal = true;
        let l_share = compute_layout(&q_share);
        assert!(l_share.hit_rects.iter().any(|h| h.id == ControlId::BtnShareDayYesterday));
        assert!(l_share.hit_rects.iter().any(|h| h.id == ControlId::BtnShareCopyImage));
        assert!(l_share.hit_rects.iter().any(|h| h.id == ControlId::BtnShareSavePng));

        let mut q_ploc = base_query(980.0, 680.0, &cfg);
        q_ploc.show_prayer_modal = true;
        q_ploc.prayer_city_rows = 3;
        let l_ploc = compute_layout(&q_ploc);
        assert!(l_ploc.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerAutoGps));
        assert!(l_ploc.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerSearchOnline));
        assert!(l_ploc.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerCityPick(2)));

        let mut q_pip = base_query(980.0, 680.0, &cfg);
        q_pip.settings_tab = Some(SettingsTab::PipMode);
        let l_pip = compute_layout(&q_pip);
        assert!(l_pip.hit_rects.iter().any(|h| h.id == ControlId::BtnTogglePeekingVinyl));
        assert!(l_pip.hit_rects.iter().any(|h| h.id == ControlId::BtnToggleHideTaskbarInPip));
    }

    #[test]
    fn dashboard_guard_tabs_expose_apps_sites_and_prayer_controls() {
        let cfg = AppConfig::default();

        let mut q_apps = base_query(980.0, 680.0, &cfg);
        q_apps.guard_tab = GuardTab::BlockedApps;
        q_apps.visible_app_rows = 2;
        let l_apps = compute_layout(&q_apps);
        assert!(l_apps.hit_rects.iter().any(|h| h.id == ControlId::BtnAppSearchBox));
        assert!(l_apps.hit_rects.iter().any(|h| h.id == ControlId::BtnAppSort));
        assert!(l_apps.hit_rects.iter().any(|h| h.id == ControlId::BtnFilterAll));
        assert!(l_apps.hit_rects.iter().any(|h| h.id == ControlId::GuardAppToggle(1)));

        let mut q_sites = base_query(980.0, 680.0, &cfg);
        q_sites.guard_tab = GuardTab::RestrictedSites;
        let l_sites = compute_layout(&q_sites);
        assert!(l_sites.hit_rects.iter().any(|h| h.id == ControlId::SitePresetToggle(0)));
        assert!(l_sites.hit_rects.iter().any(|h| h.id == ControlId::BtnGuardClearPasses));

        let mut q_prayer = base_query(980.0, 680.0, &cfg);
        q_prayer.guard_tab = GuardTab::PrayerTimes;
        let l_prayer = compute_layout(&q_prayer);
        assert!(l_prayer.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerPrev));
        assert!(l_prayer.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerNext));
        assert!(l_prayer.hit_rects.iter().any(|h| h.id == ControlId::BtnTogglePrayerEnabled));
        assert!(l_prayer.hit_rects.iter().any(|h| h.id == ControlId::BtnTogglePrayerBreak));
        assert!(l_prayer.hit_rects.iter().any(|h| h.id == ControlId::BtnPrayerAdvanceUp));

        let mut q_sound = base_query(960.0, 660.0, &cfg);
        q_sound.settings_tab = Some(SettingsTab::Sound);
        let l_sound = compute_layout(&q_sound);
        assert!(l_sound.hit_rects.iter().any(|h| h.id == ControlId::BtnFocusSoundTest));
        assert!(l_sound.hit_rects.iter().any(|h| h.id == ControlId::BtnCycleCountdownSecs));
        assert!(!l_sound.hit_rects.iter().any(|h| h.id == ControlId::BtnAmbientMasterToggle));

        let mut q_mixer = base_query(960.0, 660.0, &cfg);
        q_mixer.show_ambient_mixer = true;
        let l_mixer = compute_layout(&q_mixer);
        assert!(l_mixer.hit_rects.iter().any(|h| h.id == ControlId::BtnAmbientMasterToggle));
        assert!(l_mixer.hit_rects.iter().any(|h| h.id == ControlId::BtnAmbientPreset(7)));
        assert!(l_mixer.hit_rects.iter().any(|h| h.id == ControlId::BtnCustomPreset(0)));
        assert!(l_mixer.hit_rects.iter().any(|h| h.id == ControlId::BtnCustomPreset(1)));
        q_mixer.settings_scroll = l_mixer.settings_max_scroll;
        let l_mixer_bot = compute_layout(&q_mixer);
        assert!(l_mixer_bot.hit_rects.iter().any(|h| h.id == ControlId::AmbientTrackSlider(8)));

        let mut q_mob = base_query(960.0, 660.0, &cfg);
        q_mob.settings_tab = Some(SettingsTab::MobileSync);
        let l_mob = compute_layout(&q_mob);
        assert!(l_mob.hit_rects.iter().any(|h| h.id == ControlId::BtnMediaToggleWindow));
        assert!(l_mob.hit_rects.iter().any(|h| h.id == ControlId::BtnCopyMobileUrl));
        assert!(l_mob.hit_rects.iter().any(|h| h.id == ControlId::BtnOpenMobileUrl));
        assert_eq!(SettingsTab::from_key(SettingsTab::MobileSync.key()), SettingsTab::MobileSync);
        assert_eq!(SettingsTab::from_key(SettingsTab::System.key()), SettingsTab::System);
    }
}
