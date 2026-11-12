use anyhow::{Context, Result};
use focusgrow_gfx::dcomp::{create_text_format_single_line, rgba};
use focusgrow_gfx::DCompSurface;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_OPEN, D2D_SIZE_F,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1DeviceContext, ID2D1Factory1, ID2D1PathGeometry1, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_LARGE,
    D2D1_ARC_SIZE_SMALL, D2D1_CAP_STYLE_ROUND, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE,
    D2D1_ROUNDED_RECT, D2D1_STROKE_STYLE_PROPERTIES1, D2D1_SWEEP_DIRECTION_CLOCKWISE,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, DWRITE_FONT_WEIGHT_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, KillTimer, RegisterClassW,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, CREATESTRUCTW, GWLP_USERDATA,
    HTTRANSPARENT, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_NCCREATE,
    WM_NCHITTEST, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_NOREDIRECTIONBITMAP,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows_numerics::Vector2;

const HUD_CLASS_NAME: windows::core::PCWSTR = w!("FocusGrowHealthHud");
const HUD_W_DIP: i32 = 292;
const HUD_H_DIP: i32 = 38;
const TIMER_AUTO_HIDE_ID: usize = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
struct HudRenderState {
    is_prayer: bool,
    is_finished: bool,
    title: String,
    timer_text: String,
}

/// Click-through translucent Dynamic Island floating capsule at the top center of the screen
/// for Posture Guard breaks and advance Prayer reminders.
pub struct HealthHudWindow {
    hwnd: HWND,
    surface: Option<DCompSurface>,
    dpi: f32,
    visible: bool,
    state: HudRenderState,
}

impl HealthHudWindow {
    pub fn create() -> Result<Box<Self>> {
        let hinstance = unsafe { GetModuleHandleW(None) }?;
        let wc = WNDCLASSW {
            lpfnWndProc: Some(hud_wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: HUD_CLASS_NAME,
            ..Default::default()
        };
        unsafe {
            let _ = RegisterClassW(&wc);
        }

        let mut win = Box::new(Self {
            hwnd: HWND::default(),
            surface: None,
            dpi: 96.0,
            visible: false,
            state: HudRenderState {
                is_prayer: false,
                is_finished: false,
                title: "STAND & STRETCH".to_string(),
                timer_text: "05:00".to_string(),
            },
        });

        let ptr: *mut Self = &mut *win;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TRANSPARENT
                    | WS_EX_NOREDIRECTIONBITMAP,
                HUD_CLASS_NAME,
                w!(""),
                WS_POPUP,
                0,
                0,
                HUD_W_DIP,
                HUD_H_DIP,
                None,
                None,
                Some(hinstance.into()),
                Some(ptr as *const _),
            )
        }
        .context("CreateWindowExW failed for HealthHudWindow")?;

        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32;
        win.hwnd = hwnd;
        win.dpi = dpi;
        let px_w = ((HUD_W_DIP as f32) * dpi / 96.0).round() as u32;
        let px_h = ((HUD_H_DIP as f32) * dpi / 96.0).round() as u32;
        win.surface = Some(DCompSurface::new(hwnd, px_w.max(1), px_h.max(1), dpi)?);

        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        Ok(win)
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show_health_countdown(&mut self, secs: u32) {
        let next = HudRenderState {
            is_prayer: false,
            is_finished: false,
            title: "STAND & STRETCH".to_string(),
            timer_text: format_mmss(secs),
        };
        self.show_state(next, false);
    }

    pub fn show_health_finished(&mut self) {
        let next = HudRenderState {
            is_prayer: false,
            is_finished: true,
            title: "BREAK FINISHED".to_string(),
            timer_text: "DONE".to_string(),
        };
        self.show_state(next, true);
    }

    pub fn show_prayer_countdown(&mut self, prayer_name: &str, secs: u32) {
        let next = HudRenderState {
            is_prayer: true,
            is_finished: false,
            title: format!("PRAYER: {}", prayer_name.to_ascii_uppercase()),
            timer_text: format_mmss(secs),
        };
        self.show_state(next, false);
    }

    #[allow(dead_code)]
    pub fn show_prayer_finished(&mut self) {
        let next = HudRenderState {
            is_prayer: true,
            is_finished: true,
            title: "PRAYER FINISHED".to_string(),
            timer_text: "DONE".to_string(),
        };
        self.show_state(next, true);
    }

    pub fn hide(&mut self) {
        if self.visible && !self.hwnd.0.is_null() {
            unsafe {
                let _ = KillTimer(Some(self.hwnd), TIMER_AUTO_HIDE_ID);
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
            self.visible = false;
        }
    }

    pub fn destroy(&self) {
        if !self.hwnd.0.is_null() {
            unsafe {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }

    fn show_state(&mut self, next: HudRenderState, auto_hide: bool) {
        let changed = self.state != next || !self.visible;
        self.state = next;

        unsafe {
            let _ = KillTimer(Some(self.hwnd), TIMER_AUTO_HIDE_ID);
            if auto_hide {
                SetTimer(Some(self.hwnd), TIMER_AUTO_HIDE_ID, 5000, None);
            }
        }

        if !changed {
            return;
        }

        let (x, y, w_px, h_px) = self.compute_top_center_rect();
        if let Some(ref mut surface) = self.surface {
            let _ = surface.resize(w_px as u32, h_px as u32, self.dpi);
        }
        self.render_now();

        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                w_px,
                h_px,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
        self.visible = true;
    }

    fn compute_top_center_rect(&mut self) -> (i32, i32, i32, i32) {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) }.max(96) as f32;
        self.dpi = dpi;
        let scale = dpi / 96.0;
        let w_px = ((HUD_W_DIP as f32) * scale).round() as i32;
        let h_px = ((HUD_H_DIP as f32) * scale).round() as i32;

        let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if unsafe { GetMonitorInfoW(monitor, &mut mi).as_bool() } {
            mi.rcWork
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        };

        let x = work.left + ((work.right - work.left) - w_px) / 2;
        let y = work.top + (12.0 * scale).round() as i32;
        (x, y, w_px, h_px)
    }

    fn render_now(&self) {
        let Some(ref surface) = self.surface else {
            return;
        };
        let st = self.state.clone();
        let _ = surface.render(|dc, factory, dwrite, w, h| draw_hud(dc, factory, dwrite, w, h, &st));
    }
}

fn format_mmss(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn draw_hud(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    dwrite: &IDWriteFactory,
    w: f32,
    h: f32,
    st: &HudRenderState,
) -> Result<()> {
    let bg_rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
        left: 1.0,
        top: 1.0,
        right: (w - 1.0).max(2.0),
        bottom: (h - 1.0).max(2.0),
    };
    let pill = D2D1_ROUNDED_RECT {
        rect: bg_rect,
        radiusX: 10.0,
        radiusY: 10.0,
    };
    let fill_brush = unsafe { dc.CreateSolidColorBrush(&rgba(15, 23, 42, 0.80), None) }?;
    let border_col = if st.is_prayer {
        if st.is_finished {
            rgba(252, 211, 77, 0.82)
        } else {
            rgba(245, 158, 11, 0.72)
        }
    } else if st.is_finished {
        rgba(52, 211, 153, 0.80)
    } else {
        rgba(16, 185, 129, 0.68)
    };
    let border_brush = unsafe { dc.CreateSolidColorBrush(&border_col, None) }?;

    unsafe {
        dc.FillRoundedRectangle(&pill, &fill_brush);
        dc.DrawRoundedRectangle(&pill, &border_brush, 1.0, None);
    }

    // Left badge (26x26)
    let badge = D2D1_ROUNDED_RECT {
        rect: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
            left: 7.0,
            top: 6.0,
            right: 33.0,
            bottom: 32.0,
        },
        radiusX: 6.0,
        radiusY: 6.0,
    };
    let badge_fill_col = if st.is_prayer {
        rgba(245, 158, 11, 0.26)
    } else {
        rgba(16, 185, 129, 0.26)
    };
    let badge_border_col = if st.is_prayer {
        rgba(251, 191, 36, 0.52)
    } else {
        rgba(52, 211, 153, 0.48)
    };
    let badge_fill = unsafe { dc.CreateSolidColorBrush(&badge_fill_col, None) }?;
    let badge_border = unsafe { dc.CreateSolidColorBrush(&badge_border_col, None) }?;
    unsafe {
        dc.FillRoundedRectangle(&badge, &badge_fill);
        dc.DrawRoundedRectangle(&badge, &badge_border, 1.0, None);
    }

    draw_hud_badge_icon(dc, factory, st.is_prayer, st.is_finished)?;

    // Center title
    let title_col = if st.is_prayer {
        rgba(254, 243, 199, 0.98)
    } else {
        rgba(248, 250, 252, 0.98)
    };
    let title_fmt = create_text_format_single_line(
        dwrite,
        "Segoe UI",
        11.2,
        DWRITE_FONT_WEIGHT_BOLD,
        DWRITE_TEXT_ALIGNMENT_LEADING,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    )?;
    let title_brush = unsafe { dc.CreateSolidColorBrush(&title_col, None) }?;
    let title_rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
        left: 41.0,
        top: 0.0,
        right: (w - 78.0).max(60.0),
        bottom: h,
    };
    let title_u16: Vec<u16> = st.title.encode_utf16().collect();
    unsafe {
        dc.DrawText(
            &title_u16,
            &title_fmt,
            &title_rect,
            &title_brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    // Right timer / DONE label
    let timer_col = if st.is_prayer {
        if st.is_finished {
            rgba(252, 211, 77, 0.98)
        } else {
            rgba(251, 191, 36, 0.98)
        }
    } else if st.is_finished {
        rgba(110, 231, 183, 0.98)
    } else {
        rgba(52, 211, 153, 0.98)
    };
    let timer_fmt = create_text_format_single_line(
        dwrite,
        "Segoe UI",
        13.5,
        DWRITE_FONT_WEIGHT_BOLD,
        DWRITE_TEXT_ALIGNMENT_TRAILING,
        DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    )?;
    let timer_brush = unsafe { dc.CreateSolidColorBrush(&timer_col, None) }?;
    let timer_rect = windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F {
        left: (w - 82.0).max(40.0),
        top: 0.0,
        right: (w - 10.0).max(50.0),
        bottom: h,
    };
    let timer_u16: Vec<u16> = st.timer_text.encode_utf16().collect();
    unsafe {
        dc.DrawText(
            &timer_u16,
            &timer_fmt,
            &timer_rect,
            &timer_brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    Ok(())
}

fn draw_hud_badge_icon(
    dc: &ID2D1DeviceContext,
    factory: &ID2D1Factory1,
    is_prayer: bool,
    is_finished: bool,
) -> Result<()> {
    let stroke_style = unsafe {
        factory.CreateStrokeStyle(
            &D2D1_STROKE_STYLE_PROPERTIES1 {
                startCap: D2D1_CAP_STYLE_ROUND,
                endCap: D2D1_CAP_STYLE_ROUND,
                ..Default::default()
            },
            None,
        )
    }?;

    if is_prayer {
        let amber = unsafe { dc.CreateSolidColorBrush(&rgba(251, 191, 36, 0.98), None) }?;
        let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let sink = unsafe { path.Open() }?;
        unsafe {
            sink.BeginFigure(Vector2 { X: 20.5, Y: 12.2 }, D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddArc(&D2D1_ARC_SEGMENT {
                point: Vector2 { X: 20.5, Y: 25.0 },
                size: D2D_SIZE_F { width: 6.4, height: 6.4 },
                rotationAngle: 0.0,
                sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                arcSize: D2D1_ARC_SIZE_LARGE,
            });
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close()?;
            dc.DrawGeometry(&path, &amber, 1.8, &stroke_style);
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: Vector2 { X: 22.8, Y: 17.5 },
                    radiusX: 1.5,
                    radiusY: 1.5,
                },
                &amber,
            );
        }
    } else if !is_finished {
        let white = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.96), None) }?;
        let emerald = unsafe { dc.CreateSolidColorBrush(&rgba(52, 211, 153, 0.96), None) }?;
        unsafe {
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: Vector2 { X: 20.0, Y: 11.8 },
                    radiusX: 2.2,
                    radiusY: 2.2,
                },
                &white,
            );
            dc.DrawLine(
                Vector2 { X: 20.0, Y: 14.5 },
                Vector2 { X: 20.0, Y: 21.5 },
                &white,
                1.8,
                &stroke_style,
            );
            dc.DrawLine(
                Vector2 { X: 20.0, Y: 21.5 },
                Vector2 { X: 17.4, Y: 27.5 },
                &white,
                1.8,
                &stroke_style,
            );
            dc.DrawLine(
                Vector2 { X: 20.0, Y: 21.5 },
                Vector2 { X: 22.6, Y: 27.5 },
                &white,
                1.8,
                &stroke_style,
            );
            dc.DrawLine(
                Vector2 { X: 15.0, Y: 17.2 },
                Vector2 { X: 25.0, Y: 15.0 },
                &emerald,
                1.6,
                &stroke_style,
            );
        }
    } else {
        let emerald = unsafe { dc.CreateSolidColorBrush(&rgba(52, 211, 153, 0.98), None) }?;
        let path: ID2D1PathGeometry1 = unsafe { factory.CreatePathGeometry() }?;
        let sink = unsafe { path.Open() }?;
        unsafe {
            sink.BeginFigure(Vector2 { X: 13.5, Y: 19.5 }, D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddLine(Vector2 { X: 18.2, Y: 24.4 });
            sink.AddLine(Vector2 { X: 26.2, Y: 13.6 });
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close()?;
            dc.DrawGeometry(&path, &emerald, 2.2, &stroke_style);
        }
    }
    let _ = D2D1_ARC_SIZE_SMALL;
    Ok(())
}

unsafe extern "system" fn hud_wnd_proc(
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

    if msg == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HealthHudWindow;
    if !ptr.is_null() {
        let hud = &mut *ptr;
        if msg == WM_TIMER && wparam.0 == TIMER_AUTO_HIDE_ID {
            hud.hide();
            return LRESULT(0);
        }
        if msg == WM_PAINT {
            let _ = windows::Win32::Graphics::Gdi::ValidateRect(Some(hwnd), None);
            hud.render_now();
            return LRESULT(0);
        }
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::format_mmss;

    #[test]
    fn formats_hud_mmss_cleanly() {
        assert_eq!(format_mmss(300), "05:00");
        assert_eq!(format_mmss(59), "00:59");
    }
}
