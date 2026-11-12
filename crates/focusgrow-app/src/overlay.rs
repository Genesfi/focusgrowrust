use anyhow::{Context, Result};
use focusgrow_gfx::dcomp::{create_text_format, create_text_format_single_line, hex_color, rgba};
use focusgrow_gfx::DCompSurface;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_MEDIUM,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_WORD_WRAPPING_WRAP,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_1, VK_2, VK_3, VK_ESCAPE, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetForegroundWindow,
    GetWindowLongPtrW, IsWindow, LoadCursorW, PostMessageW, RegisterClassW, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, CREATESTRUCTW, GWLP_USERDATA, HWND_TOPMOST,
    IDC_ARROW, SC_CLOSE, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_APP,
    WM_CLOSE, WM_KEYDOWN, WM_LBUTTONUP, WM_NCCREATE, WM_PAINT, WM_SYSCOMMAND, WNDCLASSW,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

/// Posted to `main_hwnd` when the user grants an emergency pass or dismisses the fullscreen overlay.
pub const WM_OVERLAY_EVENT: u32 = WM_APP + 14;

const OVERLAY_CLASS_NAME: windows::core::PCWSTR = w!("FocusGrowFullscreenOverlay");

const FOCUS_QUOTES: &[&str] = &[
    "\"Tetap fokus pada tujuanmu. Hasil besar dibangun dari langkah-langkah kecil setiap hari.\"",
    "\"Focus on being productive instead of busy. — Tim Ferriss\"",
    "\"Konsistensi adalah kunci keberhasilan. Kerjakan tugasmu dengan penuh kesungguhan.\"",
    "\"Deep work is the superpower of the 21st century. — Cal Newport\"",
    "\"Jangan biarkan gangguan kecil menghalangi impian besarmu.\"",
    "\"Do what you have to do until you can do what you want to do. — Oprah Winfrey\"",
    "\"Satu jam fokus penuh jauh lebih berharga daripada seharian bekerja setengah hati.\"",
    "\"Starve your distractions, feed your focus.\"",
    "\"Kerjakan yang paling penting terlebih dahulu, biarkan yang lain menyusul.\"",
    "\"Action is the foundational key to all success. — Pablo Picasso\"",
    "\"Impian tidak terwujud lewat keajaiban; itu membutuhkan keringat, tekad, dan kerja keras. — Colin Powell\"",
    "\"You don't need more time, you just need more focus.\"",
    "\"Fokus pada proses, hasil indah akan mengikuti dengan sendirinya.\"",
    "\"It's not that I'm so smart, it's just that I stay with problems longer. — Albert Einstein\"",
    "\"Fokus adalah seni mengatakan 'tidak' pada seribu hal baik lainnya. — Steve Jobs\"",
    "\"Disiplin adalah jembatan antara cita-cita dan pencapaian.\"",
    "\"Work hard in silence, let your success be your noise. — Frank Ocean\"",
    "\"Cara terbaik untuk memulai adalah berhenti berbicara dan mulai melakukan. — Walt Disney\"",
    "\"Success is not final, failure is not fatal: it is the courage to continue that counts. — Winston Churchill\"",
    "\"Jangan menunggu waktu yang tepat. Ciptakan waktu itu sekarang juga.\"",
    "\"Small steps in the right direction can turn out to be the biggest steps of your life.\"",
    "\"Your mind is for having ideas, not holding them. Focus on the task at hand. — David Allen\"",
    "\"Productivity is being able to do things that you were never able to do before. — Franz Kafka\"",
    "\"Energi dan ketekunan menaklukkan segala hal. — Benjamin Franklin\"",
    "\"Believe you can and you're halfway there. — Theodore Roosevelt\"",
];

const REST_QUOTES: &[&str] = &[
    "\"Istirahat bukan berarti berhenti, melainkan mengisi ulang energi untuk melangkah lebih jauh.\"",
    "\"Rest when you're weary. Refresh and renew yourself, your body, your mind, your spirit. — Ralph Marston\"",
    "\"Jauhkan pandangan dari layar, regangkan tubuhmu, dan hirup udara segar.\"",
    "\"Almost everything will work again if you unplug it for a few minutes, including you. — Anne Lamott\"",
    "\"Kesehatan dan ketenangan pikiranmu adalah investasi terbaik untuk masa depan.\"",
    "\"Take a break. A rested mind can solve problems that a tired mind cannot.\"",
    "\"Minum air putih, berdiri sejenak, dan biarkan matamu beristirahat.\"",
    "\"Rest is not idleness; to lie on the grass under trees is by no means a waste of time. — John Lubbock\"",
    "\"Tubuhmu butuh jeda agar bisa berlari kencang kembali nanti.\"",
    "\"He who holds his breath for too long will collapse. Breathe and rest now.\"",
    "\"Tidur adalah meditasi terbaik. Tapi saat ini, cukup regangkan badanmu saja. — Dalai Lama\"",
    "\"Resting is a part of the process, not a reward for the process.\"",
    "\"Sometimes the most productive thing you can do is relax. — Mark Black\"",
    "\"Jangan merasa bersalah karena beristirahat. Mesin pun butuh pendinginan.\"",
    "\"Tenangkan pikiranmu, dan jiwamu akan berbicara.\"",
    "\"Your body hears everything your mind says. Give it some peace.\"",
    "\"A change of pace is as good as a rest. Look away from the screen for a bit.\"",
    "\"The time to relax is when you don't have time for it. — Sydney J. Harris\"",
    "\"Refresh your mind, clear your vision, and recharge your soul.\"",
    "\"Breathe in confidence, breathe out doubt. Take this moment for yourself.\"",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayMode {
    None,
    FocusBlock,
    RestBreak,
    PrayerBreak,
    AutoCloseWarning,
    #[allow(dead_code)]
    NeutralOverstayBlock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayAction {
    Dismissed,
    GrantPass { domain: String, minutes: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OverlaySnapshot {
    mode: OverlayMode,
    timer_text: String,
    target_label: String,
    quote: String,
    passes_available: u32,
}

impl Default for OverlaySnapshot {
    fn default() -> Self {
        Self {
            mode: OverlayMode::None,
            timer_text: "00:00".to_string(),
            target_label: String::new(),
            quote: String::new(),
            passes_available: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct OverlayButtonHit {
    id: OverlayButtonId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl OverlayButtonHit {
    fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverlayButtonId {
    Pass5,
    Pass10,
    Pass15,
    Close,
}

struct OverlayShared {
    main_hwnd: isize,
    snapshot: OverlaySnapshot,
    actions: VecDeque<OverlayAction>,
    quote_seed: usize,
    dismissed_domain: String,
}

struct OverlayPane {
    hwnd: HWND,
    surface: DCompSurface,
    dpi: f32,
    rect_px: RECT,
    shared: Arc<Mutex<OverlayShared>>,
}

/// Multi-monitor Direct2D + DirectComposition fullscreen blocking & break overlay.
pub struct OverlayWindow {
    main_hwnd: HWND,
    block_all_monitors: bool,
    panes: Vec<Box<OverlayPane>>,
    shared: Arc<Mutex<OverlayShared>>,
    last_fg_hwnd: isize,
    last_cursor_pt: POINT,
}

impl OverlayWindow {
    pub fn new(main_hwnd: HWND, block_all_monitors: bool) -> Result<Self> {
        let hinstance = unsafe { GetModuleHandleW(None) }?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: hinstance.into(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
            lpszClassName: OVERLAY_CLASS_NAME,
            ..Default::default()
        };
        unsafe {
            let _ = RegisterClassW(&wc);
        }

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(1);
        let shared = Arc::new(Mutex::new(OverlayShared {
            main_hwnd: main_hwnd.0 as isize,
            snapshot: OverlaySnapshot::default(),
            actions: VecDeque::new(),
            quote_seed: nanos ^ (main_hwnd.0 as usize),
            dismissed_domain: String::new(),
        }));

        let mut pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }
        Ok(Self {
            main_hwnd,
            block_all_monitors,
            panes: Vec::new(),
            shared,
            last_fg_hwnd: 0,
            last_cursor_pt: pt,
        })
    }

    pub fn set_block_all_monitors(&mut self, block_all: bool) {
        if self.block_all_monitors == block_all {
            return;
        }
        self.block_all_monitors = block_all;
        let current = self.current_snapshot();
        if current.mode != OverlayMode::None {
            let monitors = self.detect_target_monitors();
            self.rebuild_panes_and_show(monitors, false);
        }
    }

    /// Fast ~1ms single-monitor tracking called on every 250ms tick & foreground change.
    pub fn tick_follow_active_monitor(&mut self) {
        if self.block_all_monitors || !self.is_visible() || self.panes.len() != 1 {
            return;
        }
        let desired = self.detect_target_monitors();
        if desired.len() == 1 {
            let cur = self.panes[0].rect_px;
            let nxt = desired[0];
            if cur.left != nxt.left
                || cur.top != nxt.top
                || cur.right != nxt.right
                || cur.bottom != nxt.bottom
            {
                self.move_single_pane_to(nxt);
            }
        }
    }

    pub fn mode(&self) -> OverlayMode {
        self.shared
            .lock()
            .map(|s| s.snapshot.mode)
            .unwrap_or(OverlayMode::None)
    }

    pub fn is_visible(&self) -> bool {
        self.mode() != OverlayMode::None && !self.panes.is_empty()
    }

    pub fn is_overlay_hwnd(&self, hwnd: HWND) -> bool {
        !hwnd.0.is_null() && self.panes.iter().any(|p| p.hwnd == hwnd)
    }

    #[allow(dead_code)]
    pub fn is_domain_dismissed(&self, domain: &str) -> bool {
        self.shared
            .lock()
            .is_ok_and(|s| !domain.is_empty() && s.dismissed_domain.eq_ignore_ascii_case(domain))
    }

    pub fn clear_dismissed_domain(&mut self) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.dismissed_domain.clear();
        }
    }

    pub fn drain_actions(&self) -> Vec<OverlayAction> {
        let Ok(mut guard) = self.shared.lock() else {
            return Vec::new();
        };
        guard.actions.drain(..).collect()
    }

    pub fn show(
        &mut self,
        mode: OverlayMode,
        secs_remaining: u32,
        target_label: &str,
        passes_available: u32,
    ) {
        if mode == OverlayMode::None {
            self.hide();
            return;
        }

        let timer_text = if mode == OverlayMode::AutoCloseWarning {
            format!("{secs_remaining}s")
        } else {
            format!("{:02}:{:02}", secs_remaining / 60, secs_remaining % 60)
        };
        let desired_monitors = self.detect_target_monitors();
        let current_rects: Vec<RECT> = self.panes.iter().map(|p| p.rect_px).collect();
        let monitors_changed = current_rects.len() != desired_monitors.len()
            || current_rects.iter().zip(desired_monitors.iter()).any(|(a, b)| {
                a.left != b.left || a.top != b.top || a.right != b.right || a.bottom != b.bottom
            });
        let mut need_rebuild = self.panes.is_empty();
        let mut need_redraw = false;

        if let Ok(mut guard) = self.shared.lock() {
            let mode_changed = guard.snapshot.mode != mode;
            if mode_changed {
                let mut x = (guard.quote_seed as u64)
                    ^ std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0x9E37_79B9_7F4A_7C15);
                if x == 0 {
                    x = 0x9E37_79B9_7F4A_7C15;
                }
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                guard.quote_seed = x as usize;
                guard.snapshot.quote =
                    pick_quote_avoiding(mode, guard.quote_seed, &guard.snapshot.quote).to_string();
                need_rebuild = true;
            }
            if guard.snapshot.timer_text != timer_text
                || guard.snapshot.target_label != target_label
                || guard.snapshot.passes_available != passes_available
                || mode_changed
            {
                guard.snapshot.mode = mode;
                guard.snapshot.timer_text = timer_text;
                guard.snapshot.target_label = target_label.to_string();
                guard.snapshot.passes_available = passes_available;
                need_redraw = true;
            }
        }

        if need_rebuild {
            let take_focus = matches!(
                mode,
                OverlayMode::FocusBlock | OverlayMode::NeutralOverstayBlock
            );
            self.rebuild_panes_and_show(desired_monitors, take_focus);
        } else if monitors_changed {
            if self.panes.len() == 1 && desired_monitors.len() == 1 {
                self.move_single_pane_to(desired_monitors[0]);
            } else {
                self.rebuild_panes_and_show(desired_monitors, false);
            }
        } else if need_redraw {
            for pane in &self.panes {
                pane.render_now();
            }
        }
    }

    fn move_single_pane_to(&mut self, rc: RECT) {
        let Some(pane) = self.panes.first_mut() else {
            return;
        };
        let w_px = (rc.right - rc.left).max(1);
        let h_px = (rc.bottom - rc.top).max(1);
        unsafe {
            let _ = SetWindowPos(
                pane.hwnd,
                Some(HWND_TOPMOST),
                rc.left,
                rc.top,
                w_px,
                h_px,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
        }
        let new_dpi = unsafe { GetDpiForWindow(pane.hwnd) }.max(96) as f32;
        let _ = pane.surface.resize(w_px as u32, h_px as u32, new_dpi);
        pane.dpi = new_dpi;
        pane.rect_px = rc;
        pane.render_now();
    }

    fn detect_target_monitors(&mut self) -> Vec<RECT> {
        if self.block_all_monitors {
            return collect_all_monitors();
        }
        let fg = unsafe { GetForegroundWindow() };
        let fg_is_pane = self.panes.iter().any(|p| p.hwnd == fg);
        let fg_valid_external = !fg.0.is_null()
            && fg != self.main_hwnd
            && !fg_is_pane
            && unsafe { IsWindow(Some(fg)).as_bool() };

        let mut pt = POINT::default();
        let pt_ok = unsafe { GetCursorPos(&mut pt).is_ok() };

        let hmon = if fg_valid_external && fg.0 as isize != self.last_fg_hwnd {
            self.last_fg_hwnd = fg.0 as isize;
            if pt_ok {
                self.last_cursor_pt = pt;
            }
            unsafe { MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST) }
        } else if pt_ok
            && (self.panes.is_empty()
                || (pt.x - self.last_cursor_pt.x).abs() >= 3
                || (pt.y - self.last_cursor_pt.y).abs() >= 3)
        {
            self.last_cursor_pt = pt;
            unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) }
        } else if let Some(first) = self.panes.first() {
            return vec![first.rect_px];
        } else if fg_valid_external {
            unsafe { MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST) }
        } else if pt_ok {
            unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) }
        } else {
            unsafe { MonitorFromWindow(self.main_hwnd, MONITOR_DEFAULTTONEAREST) }
        };

        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(hmon, &mut mi).as_bool() } {
            return vec![mi.rcMonitor];
        }
        vec![RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        }]
    }

    pub fn hide(&mut self) {
        if let Ok(mut guard) = self.shared.lock() {
            guard.snapshot.mode = OverlayMode::None;
        }
        self.destroy_panes();
    }

    pub fn destroy(&mut self) {
        self.hide();
    }

    fn current_snapshot(&self) -> OverlaySnapshot {
        self.shared
            .lock()
            .map(|s| s.snapshot.clone())
            .unwrap_or_default()
    }

    fn destroy_panes(&mut self) {
        for pane in self.panes.drain(..) {
            if !pane.hwnd.0.is_null() {
                unsafe {
                    SetWindowLongPtrW(pane.hwnd, GWLP_USERDATA, 0);
                    let _ = ShowWindow(pane.hwnd, SW_HIDE);
                    let _ = DestroyWindow(pane.hwnd);
                }
            }
        }
    }

    fn rebuild_panes_and_show(&mut self, monitors: Vec<RECT>, take_focus: bool) {
        self.destroy_panes();
        let Ok(hinstance) = (unsafe { GetModuleHandleW(None) }) else {
            return;
        };

        for (idx, rc) in monitors.into_iter().enumerate() {
            let w_px = (rc.right - rc.left).max(1);
            let h_px = (rc.bottom - rc.top).max(1);
            if let Ok(pane) = create_overlay_pane(
                hinstance.into(),
                rc,
                w_px,
                h_px,
                Arc::clone(&self.shared),
            ) {
                unsafe {
                    let flags = SWP_SHOWWINDOW
                        | if take_focus && idx == 0 {
                            SET_WINDOW_POS_FLAGS(0)
                        } else {
                            SWP_NOACTIVATE
                        };
                    let _ = SetWindowPos(
                        pane.hwnd,
                        Some(HWND_TOPMOST),
                        rc.left,
                        rc.top,
                        w_px,
                        h_px,
                        flags,
                    );
                    if take_focus && idx == 0 {
                        let _ = SetForegroundWindow(pane.hwnd);
                    }
                }
                pane.render_now();
                self.panes.push(pane);
            }
        }
    }
}

fn create_overlay_pane(
    hinstance: windows::Win32::Foundation::HINSTANCE,
    rc: RECT,
    w_px: i32,
    h_px: i32,
    shared: Arc<Mutex<OverlayShared>>,
) -> Result<Box<OverlayPane>> {
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
            OVERLAY_CLASS_NAME,
            w!("FocusGrow Overlay"),
            WS_POPUP,
            rc.left,
            rc.top,
            w_px,
            h_px,
            None,
            None,
            Some(hinstance),
            None,
        )
    }
    .context("CreateWindowExW failed for OverlayPane")?;

    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32;
    let surface = DCompSurface::new(hwnd, w_px as u32, h_px as u32, dpi)?;
    let mut pane = Box::new(OverlayPane {
        hwnd,
        surface,
        dpi,
        rect_px: rc,
        shared,
    });
    let ptr: *mut OverlayPane = &mut *pane;
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
    }
    let _ = pane.rect_px;
    Ok(pane)
}

impl OverlayPane {
    fn render_now(&self) {
        let snapshot = self
            .shared
            .lock()
            .map(|s| s.snapshot.clone())
            .unwrap_or_default();
        let _ = self.surface.render(|dc, _factory, dwrite, w, h| {
            draw_overlay(dc, dwrite, w, h, &snapshot)
        });
    }

    fn handle_click(&self, px_x: i32, px_y: i32) {
        let scale = (self.dpi / 96.0).max(1.0);
        let x = px_x as f32 / scale;
        let y = px_y as f32 / scale;
        let (w_dip, h_dip) = self.surface.dip_size();
        let snap = self
            .shared
            .lock()
            .map(|s| s.snapshot.clone())
            .unwrap_or_default();

        let buttons = overlay_buttons_layout(w_dip, h_dip, &snap);
        for btn in buttons {
            if btn.contains(x, y) {
                match btn.id {
                    OverlayButtonId::Pass5 => push_action(&self.shared, OverlayAction::GrantPass { domain: snap.target_label, minutes: 5 }),
                    OverlayButtonId::Pass10 => push_action(&self.shared, OverlayAction::GrantPass { domain: snap.target_label, minutes: 10 }),
                    OverlayButtonId::Pass15 => push_action(&self.shared, OverlayAction::GrantPass { domain: snap.target_label, minutes: 15 }),
                    OverlayButtonId::Close => {
                        if let Ok(mut guard) = self.shared.lock() {
                            guard.dismissed_domain = snap.target_label;
                        }
                        push_action(&self.shared, OverlayAction::Dismissed);
                    }
                }
                break;
            }
        }
    }
}

fn push_action(shared: &Arc<Mutex<OverlayShared>>, action: OverlayAction) {
    let main_hwnd = if let Ok(mut guard) = shared.lock() {
        guard.actions.push_back(action);
        guard.main_hwnd
    } else {
        0
    };
    if main_hwnd != 0 {
        unsafe {
            let _ = PostMessageW(
                Some(HWND(main_hwnd as *mut _)),
                WM_OVERLAY_EVENT,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}

fn collect_all_monitors() -> Vec<RECT> {

    unsafe extern "system" fn enum_mon_proc(
        _hmon: HMONITOR,
        _hdc: HDC,
        rc: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        if !rc.is_null() {
            let out = &mut *(data.0 as *mut Vec<RECT>);
            out.push(*rc);
        }
        BOOL(1)
    }

    let mut rects = Vec::<RECT>::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_mon_proc),
            LPARAM(&mut rects as *mut Vec<RECT> as isize),
        );
    }
    if rects.is_empty() {
        rects.push(RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        });
    }
    rects
}

#[cfg(test)]
fn pick_quote(mode: OverlayMode, seed: usize) -> &'static str {
    pick_quote_avoiding(mode, seed, "")
}

fn pick_quote_avoiding(mode: OverlayMode, seed: usize, previous: &str) -> &'static str {
    let list = match mode {
        OverlayMode::RestBreak | OverlayMode::PrayerBreak => REST_QUOTES,
        _ => FOCUS_QUOTES,
    };
    let idx = (seed ^ (seed >> 16)) % list.len();
    if list.len() > 1 && list[idx] == previous {
        let step = 1 + ((seed >> 8) % (list.len() - 1));
        list[(idx + step) % list.len()]
    } else {
        list[idx]
    }
}

fn overlay_buttons_layout(w: f32, h: f32, snap: &OverlaySnapshot) -> Vec<OverlayButtonHit> {
    let mut hits = Vec::new();
    let cy = h * 0.5;
    let btn_y = cy + 190.0;
    if snap.mode == OverlayMode::FocusBlock && snap.passes_available > 0 && !snap.target_label.is_empty() {
        let bw = 132.0;
        let bh = 36.0;
        let gap = 12.0;
        let total_w = bw * 4.0 + gap * 3.0;
        let mut x = (w - total_w) * 0.5;
        for id in [
            OverlayButtonId::Pass5,
            OverlayButtonId::Pass10,
            OverlayButtonId::Pass15,
            OverlayButtonId::Close,
        ] {
            hits.push(OverlayButtonHit {
                id,
                x,
                y: btn_y,
                w: bw,
                h: bh,
            });
            x += bw + gap;
        }
    } else if matches!(snap.mode, OverlayMode::FocusBlock | OverlayMode::NeutralOverstayBlock) {
        let bw = 180.0;
        let bh = 36.0;
        hits.push(OverlayButtonHit {
            id: OverlayButtonId::Close,
            x: (w - bw) * 0.5,
            y: btn_y,
            w: bw,
            h: bh,
        });
    }
    hits
}

fn draw_overlay(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    w: f32,
    h: f32,
    snap: &OverlaySnapshot,
) -> Result<()> {
    let (bg_col, accent_hex, quote_col, title, sub) = match snap.mode {
        OverlayMode::RestBreak => (
            rgba(15, 23, 42, 0.945),
            "#34D399",
            rgba(167, 243, 208, 0.96),
            "TIME TO REST & TAKE A BREAK".to_string(),
            "Step away from your screen. Stand up, stretch your body, get a drink, and enjoy fresh air!\nWork applications are locked until your break finishes.".to_string(),
        ),
        OverlayMode::PrayerBreak => (
            rgba(13, 41, 33, 0.945),
            "#FBBF24",
            rgba(167, 243, 208, 0.96),
            "WAKTUNYA SHOLAT & ISTIRAHAT".to_string(),
            "Mari sejenak menghadap Sang Pencipta. Tinggalkan pekerjaanmu sementara waktu.\nLayar akan kembali terbuka setelah waktu istirahat sholat selesai.".to_string(),
        ),
        OverlayMode::AutoCloseWarning => (
            rgba(20, 20, 22, 0.945),
            "#F87171",
            rgba(186, 230, 253, 0.95),
            format!("[!] RESTRICTED APPLICATION: {}", snap.target_label),
            format!(
                "Application {} is restricted during focus mode.\nIt will close automatically in the countdown above.\nSwitch back (Alt+Tab) to your work application to cancel.",
                snap.target_label
            ),
        ),
        OverlayMode::NeutralOverstayBlock => (
            rgba(24, 18, 14, 0.945),
            "#FB923C",
            rgba(254, 215, 170, 0.95),
            "TIME TO RETURN TO FOCUS".to_string(),
            "Take a breath and switch back to your work applications.\nPress [Esc] to dismiss this overlay.".to_string(),
        ),
        OverlayMode::FocusBlock | OverlayMode::None => {
            let title = if snap.target_label.is_empty() {
                "STAY FOCUSED ON YOUR WORK".to_string()
            } else if snap.passes_available == 0 {
                format!("[!] RESTRICTED APPLICATION: {}", snap.target_label)
            } else {
                format!("RESTRICTED SITE: {}", snap.target_label)
            };
            let sub = if snap.target_label.is_empty() {
                "This application is restricted during your active focus session.\nSwitch back to an allowed workspace window or press [Esc].".to_string()
            } else if snap.passes_available > 0 {
                format!(
                    "Website {} is blocked during focus mode ({} emergency pass left).\n[1] 5m   |   [2] 10m   |   [3] 15m   |   [Esc] Close",
                    snap.target_label, snap.passes_available
                )
            } else {
                format!(
                    "Application {} is restricted during your active focus session.\nSwitch back (Alt+Tab) to an allowed work application or press [Esc].",
                    snap.target_label
                )
            };
            (rgba(16, 18, 22, 0.945), "#60CDFF", rgba(186, 230, 253, 0.95), title, sub)
        }
    };

    let bg_brush = unsafe { dc.CreateSolidColorBrush(&bg_col, None) }?;
    unsafe {
        dc.FillRectangle(
            &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: w,
                bottom: h,
            },
            &bg_brush,
        );
    }

    let cy = h * 0.5;
    let panel_w = (w * 0.68).clamp(520.0, 860.0);
    let panel_x = (w - panel_w) * 0.5;

    // Title
    draw_text_line(
        dc,
        dwrite,
        &title,
        panel_x,
        cy - 190.0,
        panel_w,
        46.0,
        30.0,
        DWRITE_FONT_WEIGHT_BOLD,
        rgba(248, 250, 252, 0.98),
        true,
    )?;

    // Large accent countdown
    draw_text_line(
        dc,
        dwrite,
        &snap.timer_text,
        panel_x,
        cy - 130.0,
        panel_w,
        88.0,
        74.0,
        DWRITE_FONT_WEIGHT_BOLD,
        hex_color(accent_hex, 0.98),
        true,
    )?;

    // Subtitle explanation
    draw_text_line(
        dc,
        dwrite,
        &sub,
        panel_x,
        cy - 22.0,
        panel_w,
        84.0,
        17.5,
        DWRITE_FONT_WEIGHT_NORMAL,
        rgba(205, 210, 222, 0.94),
        false,
    )?;

    // Italic Quote cleanly on backdrop (matching fullscreen_break_overlay.png, no border box)
    if !snap.quote.is_empty() {
        draw_italic_quote(
            dc,
            dwrite,
            &snap.quote,
            panel_x + 20.0,
            cy + 78.0,
            (panel_w - 40.0).max(200.0),
            84.0,
            19.0,
            quote_col,
        )?;
    }

    // Action buttons
    for btn in overlay_buttons_layout(w, h, snap) {
        let label = match btn.id {
            OverlayButtonId::Pass5 => "[1] 5 Min Pass",
            OverlayButtonId::Pass10 => "[2] 10 Min Pass",
            OverlayButtonId::Pass15 => "[3] 15 Min Pass",
            OverlayButtonId::Close => "[Esc] Return to Work",
        };
        let is_close = btn.id == OverlayButtonId::Close;
        let fill = if is_close {
            rgba(255, 255, 255, 0.10)
        } else {
            hex_color(accent_hex, 0.22)
        };
        let border = if is_close {
            rgba(255, 255, 255, 0.22)
        } else {
            hex_color(accent_hex, 0.68)
        };
        let rr = D2D1_ROUNDED_RECT {
            rect: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: btn.x,
                top: btn.y,
                right: btn.x + btn.w,
                bottom: btn.y + btn.h,
            },
            radiusX: 9.0,
            radiusY: 9.0,
        };
        let b_fill = unsafe { dc.CreateSolidColorBrush(&fill, None) }?;
        let b_stroke = unsafe { dc.CreateSolidColorBrush(&border, None) }?;
        unsafe {
            dc.FillRoundedRectangle(&rr, &b_fill);
            dc.DrawRoundedRectangle(&rr, &b_stroke, 1.0, None);
        }
        draw_text_line(
            dc,
            dwrite,
            label,
            btn.x + 8.0,
            btn.y,
            btn.w - 16.0,
            btn.h,
            12.5,
            DWRITE_FONT_WEIGHT_MEDIUM,
            rgba(248, 250, 252, 0.96),
            true,
        )?;
    }

    Ok(())
}

fn draw_italic_quote(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    color: D2D1_COLOR_F,
) -> Result<()> {
    let fmt = unsafe {
        dwrite.CreateTextFormat(
            w!("Segoe UI"),
            None,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
            DWRITE_FONT_STYLE_ITALIC,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("en-us"),
        )
    }?;
    unsafe {
        fmt.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
    }
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let u16_text: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        dc.DrawText(
            &u16_text,
            &fmt,
            &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            },
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    let _ = DWRITE_FONT_STYLE_NORMAL;
    Ok(())
}

fn draw_text_line(
    dc: &ID2D1DeviceContext,
    dwrite: &IDWriteFactory,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    weight: windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT,
    color: D2D1_COLOR_F,
    single_line: bool,
) -> Result<()> {
    let fmt = if single_line {
        create_text_format_single_line(
            dwrite,
            "Segoe UI",
            size,
            weight,
            DWRITE_TEXT_ALIGNMENT_CENTER,
            DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
        )?
    } else {
        create_text_format(
            dwrite,
            "Segoe UI",
            size,
            weight,
            DWRITE_TEXT_ALIGNMENT_CENTER,
            DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
        )?
    };
    let brush = unsafe { dc.CreateSolidColorBrush(&color, None) }?;
    let u16_text: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        dc.DrawText(
            &u16_text,
            &fmt,
            &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
                left: x,
                top: y,
                right: x + w,
                bottom: y + h,
            },
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }
    Ok(())
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    if msg == WM_CLOSE {
        return LRESULT(0);
    }
    if msg == WM_SYSCOMMAND && (wparam.0 & 0xFFF0) == SC_CLOSE as usize {
        return LRESULT(0);
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayPane;
    if !ptr.is_null() {
        let pane = &*ptr;
        match msg {
            WM_PAINT => {
                let _ = windows::Win32::Graphics::Gdi::ValidateRect(Some(hwnd), None);
                pane.render_now();
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let px_x = (lparam.0 & 0xFFFF) as i16 as i32;
                let px_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                pane.handle_click(px_x, px_y);
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u16;
                let snap = pane
                    .shared
                    .lock()
                    .map(|s| s.snapshot.clone())
                    .unwrap_or_default();
                if matches!(
                    snap.mode,
                    OverlayMode::FocusBlock | OverlayMode::NeutralOverstayBlock
                ) {
                    if vk == VK_ESCAPE.0 {
                        if let Ok(mut guard) = pane.shared.lock() {
                            guard.dismissed_domain = snap.target_label;
                        }
                        push_action(&pane.shared, OverlayAction::Dismissed);
                        return LRESULT(0);
                    }
                    if snap.mode == OverlayMode::FocusBlock
                        && snap.passes_available > 0
                        && !snap.target_label.is_empty()
                    {
                        let mins = if vk == VK_1.0 || vk == VK_NUMPAD1.0 {
                            Some(5)
                        } else if vk == VK_2.0 || vk == VK_NUMPAD2.0 {
                            Some(10)
                        } else if vk == VK_3.0 || vk == VK_NUMPAD3.0 {
                            Some(15)
                        } else {
                            None
                        };
                        if let Some(minutes) = mins {
                            push_action(
                                &pane.shared,
                                OverlayAction::GrantPass {
                                    domain: snap.target_label,
                                    minutes,
                                },
                            );
                            return LRESULT(0);
                        }
                    }
                }
                return LRESULT(0);
            }
            _ => {}
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_block_with_domain_and_passes_exposes_four_buttons() {
        let snap = OverlaySnapshot {
            mode: OverlayMode::FocusBlock,
            timer_text: "24:10".to_string(),
            target_label: "youtube.com".to_string(),
            quote: pick_quote(OverlayMode::FocusBlock, 0).to_string(),
            passes_available: 2,
        };
        let btns = overlay_buttons_layout(1920.0, 1080.0, &snap);
        assert_eq!(btns.len(), 4);
        assert_eq!(btns[0].id, OverlayButtonId::Pass5);
        assert_eq!(btns[3].id, OverlayButtonId::Close);
    }

    #[test]
    fn rest_break_has_no_dismiss_buttons_and_picks_rest_quote() {
        let snap = OverlaySnapshot {
            mode: OverlayMode::RestBreak,
            timer_text: "04:50".to_string(),
            target_label: String::new(),
            quote: pick_quote(OverlayMode::RestBreak, 1).to_string(),
            passes_available: 0,
        };
        assert!(overlay_buttons_layout(1920.0, 1080.0, &snap).is_empty());
        assert!(REST_QUOTES.contains(&snap.quote.as_str()));
    }
}
