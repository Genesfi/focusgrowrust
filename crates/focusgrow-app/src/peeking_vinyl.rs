use anyhow::{Context, Result};
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_PER_PRIMITIVE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
    GetWindowRect, PostMessageW, RegisterClassW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    CREATESTRUCTW, GWLP_USERDATA, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW,
    SW_HIDE, WINDOWPOS, WM_APP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEWHEEL,
    WM_NCCREATE, WM_PAINT, WM_SIZE, WM_WINDOWPOSCHANGING, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use focusgrow_core::VinylSide;
use focusgrow_gfx::{draw_vinyl_disc, upload_frame_bitmap, DCompSurface, DecodedFrame};
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap1;

use crate::ytmpx::MediaCommand;

/// Posted from `PeekingVinylWindow` to `main_hwnd` when the user clicks or scrolls on the vinyl.
pub const WM_VINYL_COMMAND: u32 = WM_APP + 11;

const VINYL_CLASS_NAME: windows::core::PCWSTR = w!("FocusGrowPeekingVinyl");
const SLIDE_DURATION_SEC: f32 = 0.28;
const PEEK_FRACTION: f32 = 0.46;
const MARGIN_PX: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlideState {
    Hidden,
    Extending,
    Normal,
    Retracting,
}

/// GPU-composited (`DirectComposition`) companion window that slides a rotating vinyl disc
/// from behind FocusGrow's left or right card edge and stays glued behind `main_hwnd`.
pub struct PeekingVinylWindow {
    hwnd: HWND,
    main_hwnd: HWND,
    surface: Option<DCompSurface>,
    cover_frame: Option<DecodedFrame>,
    pending_frame: Option<DecodedFrame>,
    has_pending_swap: bool,
    cover_bitmap: Option<ID2D1Bitmap1>,
    cover_dirty: bool,
    slide_state: SlideState,
    slide_t: f32,
    angle_deg: f32,
    want_visible: bool,
    is_playing: bool,
    side: VinylSide,
    vinyl_size_px: i32,
    speed_sec_per_rev: f32,
    show_spindle: bool,
    accent_color: String,
    dpi: f32,
    last_tick: Instant,
    last_main_rect: RECT,
}

impl PeekingVinylWindow {
    pub fn create(main_hwnd: HWND) -> Result<Box<Self>> {
        let hinstance = unsafe { GetModuleHandleW(None) }?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(vinyl_wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: VINYL_CLASS_NAME,
            ..Default::default()
        };
        unsafe {
            let _ = RegisterClassW(&wc);
        }

        let init_dpi = unsafe { GetDpiForWindow(main_hwnd) }.max(96) as f32;

        let mut win = Box::new(Self {
            hwnd: HWND::default(),
            main_hwnd,
            surface: None,
            cover_frame: None,
            pending_frame: None,
            has_pending_swap: false,
            cover_bitmap: None,
            cover_dirty: false,
            slide_state: SlideState::Hidden,
            slide_t: 0.0,
            angle_deg: 0.0,
            want_visible: false,
            is_playing: false,
            side: VinylSide::Left,
            vinyl_size_px: 230,
            speed_sec_per_rev: 8.0,
            show_spindle: true,
            accent_color: "#F59E0B".to_string(),
            dpi: init_dpi,
            last_tick: Instant::now(),
            last_main_rect: RECT::default(),
        });

        let ptr: *mut Self = &mut *win;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                VINYL_CLASS_NAME,
                w!(""),
                WS_POPUP,
                0,
                0,
                260,
                260,
                None,
                None,
                Some(hinstance.into()),
                Some(ptr as *const _),
            )
        }
        .context("CreateWindowExW failed for PeekingVinylWindow")?;

        win.hwnd = hwnd;
        win.surface = Some(DCompSurface::new(hwnd, 260, 260, init_dpi)?);
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        Ok(win)
    }

    pub fn update_settings(
        &mut self,
        enabled: bool,
        has_track: bool,
        is_playing: bool,
        side: VinylSide,
        vinyl_size_px: i32,
        speed_sec: f32,
        show_spindle: bool,
        accent_color: &str,
    ) {
        self.is_playing = is_playing;
        self.side = side;
        self.vinyl_size_px = vinyl_size_px.clamp(120, 380);
        self.speed_sec_per_rev = speed_sec.clamp(2.0, 24.0);
        self.show_spindle = show_spindle;
        self.accent_color = accent_color.to_string();

        let next_visible = enabled && has_track;
        if next_visible != self.want_visible {
            self.want_visible = next_visible;
            if next_visible {
                self.slide_state = SlideState::Extending;
                self.last_tick = Instant::now();
                self.sync_bounds_and_zorder(true);
            } else if self.slide_state != SlideState::Hidden {
                self.slide_state = SlideState::Retracting;
                self.last_tick = Instant::now();
            }
        } else if next_visible {
            self.sync_bounds_and_zorder(false);
        }
    }

    pub fn set_cover_frame(&mut self, frame: Option<DecodedFrame>, is_visible_change: bool) {
        if !self.want_visible || !is_visible_change || matches!(self.slide_state, SlideState::Hidden) {
            self.cover_frame = frame;
            self.cover_dirty = true;
            self.pending_frame = None;
            self.has_pending_swap = false;
            return;
        }

        // Smooth sleeve tuck -> swap cover -> slide back out (matches PipVinylOverlay.hpp)
        self.pending_frame = frame;
        self.has_pending_swap = true;
        self.slide_state = SlideState::Retracting;
        self.last_tick = Instant::now();
    }

    pub fn is_animating(&self) -> bool {
        match self.slide_state {
            SlideState::Hidden => false,
            SlideState::Extending | SlideState::Retracting => true,
            SlideState::Normal => self.want_visible && self.is_playing,
        }
    }

    pub fn sync_bounds_and_zorder(&mut self, show_if_hidden: bool) {
        if matches!(self.slide_state, SlideState::Hidden) && !show_if_hidden {
            return;
        }

        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.main_hwnd, &mut rect) }.is_err() {
            return;
        }
        self.last_main_rect = rect;
        self.dpi = unsafe { GetDpiForWindow(self.main_hwnd) }.max(96) as f32;
        let scale = self.dpi / 96.0;

        let card_h_px = rect.bottom - rect.top;
        let card_h_dip = (card_h_px as f32 / scale).round() as i32;
        // Clamp disc diameter cleanly so it never looks taller than the host card
        let disc_dip = self.vinyl_size_px.min((card_h_dip - 28).max(120));
        let disc_px = (disc_dip as f32 * scale).round() as i32;
        let margin_px = (MARGIN_PX as f32 * scale).round() as i32;
        let peek_px = (disc_px as f32 * PEEK_FRACTION).round() as i32;
        let win_h = disc_px + margin_px * 2;
        let win_w = disc_px + margin_px * 2;
        let win_y = rect.top + (card_h_px - win_h) / 2;

        let win_x = match self.side {
            VinylSide::Left => rect.left - peek_px - margin_px,
            VinylSide::Right => {
                let target_right = rect.right + peek_px + margin_px;
                target_right - win_w
            }
        };

        let mut flags = SWP_NOACTIVATE;
        if show_if_hidden || matches!(self.slide_state, SlideState::Extending) {
            flags |= SWP_SHOWWINDOW;
        }

        unsafe {
            // Setting insertAfter = main_hwnd keeps the vinyl directly behind main_hwnd
            let _ = SetWindowPos(self.hwnd, Some(self.main_hwnd), win_x, win_y, win_w, win_h, flags);
        }
    }

    pub fn tick_and_render(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().clamp(0.0, 0.1);
        self.last_tick = now;

        match self.slide_state {
            SlideState::Hidden => return,
            SlideState::Extending => {
                self.slide_t = (self.slide_t + dt / SLIDE_DURATION_SEC).min(1.0);
                if self.slide_t >= 1.0 {
                    self.slide_state = SlideState::Normal;
                }
            }
            SlideState::Retracting => {
                self.slide_t = (self.slide_t - dt / SLIDE_DURATION_SEC).max(0.0);
                if self.slide_t <= 0.0 {
                    if self.has_pending_swap {
                        self.cover_frame = self.pending_frame.take();
                        self.has_pending_swap = false;
                        self.cover_dirty = true;
                        if self.want_visible {
                            self.slide_state = SlideState::Extending;
                        } else {
                            self.hide_window();
                            return;
                        }
                    } else {
                        self.hide_window();
                        return;
                    }
                }
            }
            SlideState::Normal => {}
        }

        if self.is_playing {
            let deg_per_sec = 360.0 / self.speed_sec_per_rev;
            self.angle_deg = (self.angle_deg + deg_per_sec * dt) % 360.0;
        }

        let _ = self.render();
    }

    pub fn render(&mut self) -> Result<()> {
        if matches!(self.slide_state, SlideState::Hidden) {
            return Ok(());
        }

        let mut win_rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut win_rect);
            let _ = GetWindowRect(self.main_hwnd, &mut self.last_main_rect);
        }

        let side = self.side;
        let show_spindle = self.show_spindle;
        let angle = self.angle_deg;
        let eased = ease_out_cubic(self.slide_t);
        let accent = self.accent_color.clone();

        let cover_dirty = self.cover_dirty;
        let cover_frame = self.cover_frame.clone();
        let mut new_bmp: Option<ID2D1Bitmap1> = None;
        let mut clear_bmp = false;
        let existing_bmp = if cover_dirty {
            None
        } else {
            self.cover_bitmap.clone()
        };

        let main_rect = self.last_main_rect;
        let dpi_scale = self.dpi / 96.0;
        let Some(ref surface) = self.surface else {
            return Ok(());
        };

        surface.render(|dc, _, _, w_dip, h_dip| {
            unsafe {
                dc.Clear(Some(&D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }));
            }

            let bmp_ref: Option<&ID2D1Bitmap1> = if cover_dirty {
                if let Some(ref frame) = cover_frame {
                    let uploaded = upload_frame_bitmap(dc, frame)?;
                    new_bmp = Some(uploaded);
                    new_bmp.as_ref()
                } else {
                    clear_bmp = true;
                    None
                }
            } else {
                existing_bmp.as_ref()
            };

            let disc_px = (w_dip - (MARGIN_PX * 2) as f32).min(h_dip - (MARGIN_PX * 2) as f32);
            let radius = (disc_px * 0.5).max(20.0);
            let cy = h_dip * 0.5;

            let (cx, clip_rect) = compute_disc_position_and_clip(
                side,
                win_rect,
                main_rect,
                dpi_scale,
                w_dip,
                h_dip,
                radius,
                eased,
            );

            if clip_rect.right <= clip_rect.left {
                return Ok(());
            }

            unsafe {
                dc.PushAxisAlignedClip(&clip_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            }
            let draw_res = draw_vinyl_disc(
                dc,
                cx,
                cy,
                radius,
                angle,
                bmp_ref,
                show_spindle,
                &accent,
            );
            unsafe {
                dc.PopAxisAlignedClip();
            }
            draw_res
        })?;

        if cover_dirty {
            if clear_bmp {
                self.cover_bitmap = None;
            } else if new_bmp.is_some() {
                self.cover_bitmap = new_bmp;
            }
            self.cover_dirty = false;
        }

        Ok(())
    }

    pub fn hide_window(&mut self) {
        self.slide_state = SlideState::Hidden;
        self.slide_t = 0.0;
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn destroy(&self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

fn compute_disc_position_and_clip(
    side: VinylSide,
    win_rect: RECT,
    main_rect: RECT,
    dpi_scale: f32,
    w_dip: f32,
    h_dip: f32,
    radius: f32,
    eased_t: f32,
) -> (f32, D2D_RECT_F) {
    let scale = dpi_scale.max(1.0);
    let card_left_local = (main_rect.left - win_rect.left) as f32 / scale;
    let card_right_local = (main_rect.right - win_rect.left) as f32 / scale;

    match side {
        VinylSide::Left => {
            // Tucked cleanly inside card when eased_t == 0, peeking 46% outside when eased_t == 1
            let start_cx = card_left_local + radius + 6.0;
            let end_cx = card_left_local + radius * (1.0 - PEEK_FRACTION * 2.0);
            let cx = start_cx + (end_cx - start_cx) * eased_t;
            let clip_right = card_right_local.min(w_dip).max(0.0);
            (
                cx,
                D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: clip_right,
                    bottom: h_dip,
                },
            )
        }
        VinylSide::Right => {
            let start_cx = card_right_local - radius - 6.0;
            let end_cx = card_right_local - radius * (1.0 - PEEK_FRACTION * 2.0);
            let cx = start_cx + (end_cx - start_cx) * eased_t;
            let clip_left = card_left_local.max(0.0).min(w_dip);
            (
                cx,
                D2D_RECT_F {
                    left: clip_left,
                    top: 0.0,
                    right: w_dip,
                    bottom: h_dip,
                },
            )
        }
    }
}

unsafe extern "system" fn vinyl_wnd_proc(
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

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PeekingVinylWindow;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let win = &mut *ptr;

    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_WINDOWPOSCHANGING => {
            let wp = &mut *(lparam.0 as *mut WINDOWPOS);
            if !win.main_hwnd.0.is_null() {
                wp.hwndInsertAfter = win.main_hwnd;
                wp.flags &= !SWP_NOZORDER;
                wp.flags |= SWP_NOACTIVATE;
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => LRESULT(0),
        WM_SIZE => {
            let mut rc = RECT::default();
            if GetClientRect(hwnd, &mut rc).is_ok() {
                let w = (rc.right - rc.left).max(1) as u32;
                let h = (rc.bottom - rc.top).max(1) as u32;
                win.dpi = GetDpiForWindow(win.main_hwnd).max(96) as f32;
                if let Some(ref mut surface) = win.surface {
                    let _ = surface.resize(w, h, win.dpi);
                }
                let _ = win.render();
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let _ = win.render();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            let _ = PostMessageW(
                Some(win.main_hwnd),
                WM_VINYL_COMMAND,
                WPARAM(MediaCommand::ToggleMusicWindow as usize),
                LPARAM(0),
            );
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as u16) as i16;
            let cmd = if delta >= 0 {
                MediaCommand::VolumeUp
            } else {
                MediaCommand::VolumeDown
            };
            let _ = PostMessageW(
                Some(win.main_hwnd),
                WM_VINYL_COMMAND,
                WPARAM(cmd as usize),
                LPARAM(0),
            );
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
