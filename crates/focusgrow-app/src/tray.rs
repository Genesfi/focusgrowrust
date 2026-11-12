use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIIF_INFO, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, InsertMenuW, SetForegroundWindow, TrackPopupMenu,
    HICON, MF_BYPOSITION, MF_SEPARATOR, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD, WM_APP,
};

pub const WM_TRAYICON: u32 = WM_APP + 18;
pub const IDM_TRAY_RESTORE: usize = 10001;
pub const IDM_TRAY_TOGGLE_PIP: usize = 10002;
pub const IDM_TRAY_PAUSE: usize = 10003;
pub const IDM_TRAY_STOP: usize = 10004;
pub const IDM_TRAY_EXIT: usize = 10005;

pub struct TrayIcon {
    nid: NOTIFYICONDATAW,
    active: bool,
}

impl TrayIcon {
    pub fn create(hwnd: HWND, icon: HICON) -> Self {
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1002;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = icon;

        let tip = "FocusGrow - Smart Focus & Posture Guard";
        let utf16: Vec<u16> = tip.encode_utf16().collect();
        let copy_len = utf16.len().min(nid.szTip.len().saturating_sub(1));
        nid.szTip[..copy_len].copy_from_slice(&utf16[..copy_len]);

        let active = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        Self { nid, active }
    }

    pub fn remove(&mut self) {
        if self.active {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            }
            self.active = false;
        }
    }

    pub fn show_notification(&mut self, title: &str, body: &str) {
        if !self.active {
            return;
        }
        self.nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_INFO;
        self.nid.dwInfoFlags = NIIF_INFO;
        self.nid.szInfoTitle.fill(0);
        self.nid.szInfo.fill(0);

        let t_u16: Vec<u16> = title.encode_utf16().collect();
        let t_len = t_u16.len().min(self.nid.szInfoTitle.len().saturating_sub(1));
        self.nid.szInfoTitle[..t_len].copy_from_slice(&t_u16[..t_len]);

        let b_u16: Vec<u16> = body.encode_utf16().collect();
        let b_len = b_u16.len().min(self.nid.szInfo.len().saturating_sub(1));
        self.nid.szInfo[..b_len].copy_from_slice(&b_u16[..b_len]);

        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &self.nid);
        }
    }

    pub fn show_context_menu(
        &self,
        hwnd: HWND,
        is_pip_mode: bool,
        is_focus_active: bool,
        is_paused: bool,
    ) -> usize {
        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            let Ok(hmenu) = CreatePopupMenu() else {
                return 0;
            };
            let restore_lbl = HSTRING::from(if is_pip_mode {
                "Open Floating Timer (PiP)"
            } else {
                "Open FocusGrow Dashboard"
            });
            let toggle_lbl = HSTRING::from(if is_pip_mode {
                "Switch to Full Dashboard"
            } else {
                "Pop out Floating Timer (PiP)"
            });

            let mut pos = 0u32;
            let _ = InsertMenuW(hmenu, pos, MF_BYPOSITION | MF_STRING, IDM_TRAY_RESTORE, &restore_lbl);
            pos += 1;
            let _ = InsertMenuW(hmenu, pos, MF_BYPOSITION | MF_STRING, IDM_TRAY_TOGGLE_PIP, &toggle_lbl);
            pos += 1;

            if is_focus_active {
                let _ = InsertMenuW(hmenu, pos, MF_BYPOSITION | MF_SEPARATOR, 0, None);
                pos += 1;
                let pause_lbl = HSTRING::from(if is_paused {
                    "Resume Focus Session"
                } else {
                    "Pause Focus Session"
                });
                let _ = InsertMenuW(hmenu, pos, MF_BYPOSITION | MF_STRING, IDM_TRAY_PAUSE, &pause_lbl);
                pos += 1;
                let _ = InsertMenuW(
                    hmenu,
                    pos,
                    MF_BYPOSITION | MF_STRING,
                    IDM_TRAY_STOP,
                    windows::core::w!("Stop Focus Session"),
                );
                pos += 1;
            }

            let _ = InsertMenuW(hmenu, pos, MF_BYPOSITION | MF_SEPARATOR, 0, None);
            pos += 1;
            let _ = InsertMenuW(
                hmenu,
                pos,
                MF_BYPOSITION | MF_STRING,
                IDM_TRAY_EXIT,
                windows::core::w!("Exit FocusGrow"),
            );

            let _ = SetForegroundWindow(hwnd);
            let cmd = TrackPopupMenu(
                hmenu,
                TPM_RETURNCMD | TPM_NONOTIFY,
                pt.x,
                pt.y,
                Some(0),
                hwnd,
                None,
            );
            let _ = DestroyMenu(hmenu);
            cmd.0 as usize
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}
