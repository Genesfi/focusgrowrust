use anyhow::{Context, Result};
use std::path::PathBuf;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, FileSaveDialog, IFileOpenDialog, IFileSaveDialog, FOS_FILEMUSTEXIST,
    FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, SIGDN_FILESYSPATH,
};

/// Opens a native Windows Explorer file picker for GIF / WebP / PNG / JPEG files.
pub fn pick_image_file(owner: HWND) -> Result<Option<PathBuf>> {
    let filters = [
        COMDLG_FILTERSPEC {
            pszName: w!("Animated GIF & Images (*.gif;*.webp;*.png;*.jpg)"),
            pszSpec: w!("*.gif;*.webp;*.png;*.jpg;*.jpeg"),
        },
        COMDLG_FILTERSPEC {
            pszName: w!("GIF Files (*.gif)"),
            pszSpec: w!("*.gif"),
        },
    ];
    pick_file(owner, &filters, "Select Custom GIF / Background Image - FocusGrow")
}

/// Opens a native Windows Explorer file picker for custom notification/alarm audio files.
pub fn pick_audio_file(owner: HWND) -> Result<Option<PathBuf>> {
    let filters = [
        COMDLG_FILTERSPEC {
            pszName: w!("Audio Files (*.wav;*.mp3;*.m4a;*.flac;*.wma)"),
            pszSpec: w!("*.wav;*.mp3;*.m4a;*.flac;*.wma"),
        },
        COMDLG_FILTERSPEC {
            pszName: w!("WAV Audio (*.wav)"),
            pszSpec: w!("*.wav"),
        },
    ];
    pick_file(owner, &filters, "Select Custom Notification Audio / Alarm - FocusGrow")
}

/// Opens a native Windows Save File dialog for exporting a Share Card PNG image.
pub fn save_png_file(owner: HWND, default_name: &str) -> Result<Option<PathBuf>> {
    let filters = [COMDLG_FILTERSPEC {
        pszName: w!("PNG Image (*.png)"),
        pszSpec: w!("*.png"),
    }];
    unsafe {
        let dialog: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                .context("CoCreateInstance(FileSaveDialog) failed")?;

        let opts = dialog.GetOptions()?;
        dialog.SetOptions(opts | FOS_OVERWRITEPROMPT | FOS_FORCEFILESYSTEM)?;
        dialog.SetFileTypes(&filters)?;
        dialog.SetDefaultExtension(w!("png"))?;
        dialog.SetFileName(&HSTRING::from(default_name))?;
        dialog.SetTitle(&HSTRING::from("Save FocusGrow Share Card PNG"))?;

        match dialog.Show(Some(owner)) {
            Ok(()) => {}
            Err(err) if err.code() == windows::Win32::Foundation::ERROR_CANCELLED.into() => {
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        }

        let item = dialog.GetResult()?;
        let pwstr = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path_str = pwstr.to_string().unwrap_or_default();
        CoTaskMemFree(Some(pwstr.0 as *const _));

        if path_str.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(path_str)))
        }
    }
}

fn pick_file(owner: HWND, filters: &[COMDLG_FILTERSPEC], title: &str) -> Result<Option<PathBuf>> {
    unsafe {
        let dialog: IFileOpenDialog =
            CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                .context("CoCreateInstance(FileOpenDialog) failed")?;

        let opts = dialog.GetOptions()?;
        dialog.SetOptions(opts | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM)?;

        dialog.SetFileTypes(filters)?;
        dialog.SetTitle(&HSTRING::from(title))?;

        match dialog.Show(Some(owner)) {
            Ok(()) => {}
            Err(err) if err.code() == windows::Win32::Foundation::ERROR_CANCELLED.into() => {
                return Ok(None);
            }
            Err(err) => return Err(err.into()),
        }

        let item = dialog.GetResult()?;
        let pwstr = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path_str = pwstr.to_string().unwrap_or_default();
        CoTaskMemFree(Some(pwstr.0 as *const _));

        if path_str.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(path_str)))
        }
    }
}

