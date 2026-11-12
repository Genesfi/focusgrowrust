use crate::synth::{AlertKind, SynthBank, SAMPLE_RATE};
use focusgrow_core::AmbientConfig;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioClient, IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
    AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;

#[link(name = "winmm")]
extern "system" {
    fn mciSendStringW(
        lpsz_command: PCWSTR,
        lpsz_return_string: *mut u16,
        cch_return: u32,
        hwnd_callback: *mut std::ffi::c_void,
    ) -> u32;
}

pub(crate) struct SharedAudioState {
    pub synth: SynthBank,
}

pub(crate) struct WasapiWorker {
    stop_flag: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl WasapiWorker {
    pub fn spawn(shared: Arc<Mutex<SharedAudioState>>) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_flag);
        let join = thread::Builder::new()
            .name("focusgrow-wasapi".to_string())
            .spawn(move || {
                run_wasapi_loop(shared, stop_clone);
            })
            .ok();
        Self { stop_flag, join }
    }
}

impl Drop for WasapiWorker {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn run_wasapi_loop(shared: Arc<Mutex<SharedAudioState>>, stop: Arc<AtomicBool>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    while !stop.load(Ordering::Relaxed) {
        if try_stream_session(&shared, &stop).is_err() {
            // Headless / no audio device or device change: back off calmly
            thread::sleep(Duration::from_millis(500));
        }
    }

    unsafe {
        CoUninitialize();
    }
}

fn try_stream_session(shared: &Arc<Mutex<SharedAudioState>>, stop: &Arc<AtomicBool>) -> anyhow::Result<()> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let wfx = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_IEEE_FLOAT,
            nChannels: 2,
            nSamplesPerSec: SAMPLE_RATE as u32,
            nAvgBytesPerSec: (SAMPLE_RATE as u32) * 8,
            nBlockAlign: 8,
            wBitsPerSample: 32,
            cbSize: 0,
        };

        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            500_000, // 50ms shared buffer
            0,
            &wfx,
            None,
        )?;

        let buffer_frames = client.GetBufferSize()?;
        let render: IAudioRenderClient = client.GetService()?;
        client.Start()?;

        let mut scratch = vec![0.0f32; (buffer_frames as usize) * 2];

        while !stop.load(Ordering::Relaxed) {
            let padding = client.GetCurrentPadding()?;
            let avail = buffer_frames.saturating_sub(padding);
            if avail == 0 {
                thread::sleep(Duration::from_millis(8));
                continue;
            }

            let want_samples = (avail as usize) * 2;
            if scratch.len() < want_samples {
                scratch.resize(want_samples, 0.0);
            }

            let is_silent = {
                let mut guard = shared.lock().unwrap();
                if guard.synth.is_idle() {
                    true
                } else {
                    guard.synth.render_stereo(&mut scratch[..want_samples]);
                    false
                }
            };

            let ptr = render.GetBuffer(avail)?;
            if is_silent {
                render.ReleaseBuffer(avail, AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)?;
                thread::sleep(Duration::from_millis(25));
            } else {
                let dst = std::slice::from_raw_parts_mut(ptr as *mut f32, want_samples);
                dst.copy_from_slice(&scratch[..want_samples]);
                render.ReleaseBuffer(avail, 0)?;
                thread::sleep(Duration::from_millis(12));
            }
        }

        let _ = client.Stop();
    }
    Ok(())
}

/// Immediately stops and closes all active MCI audio playbacks.
pub fn stop_custom_audio() {
    let cmd = wide_null("close all");
    unsafe {
        mciSendStringW(PCWSTR(cmd.as_ptr()), std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
}

/// Plays a user-uploaded WAV/MP3/M4A file natively using Windows MCI on a detached thread.
pub fn play_custom_audio_file(path: &Path) {
    stop_custom_audio();
    let owned = path.to_path_buf();
    let _ = thread::Builder::new()
        .name("focusgrow-custom-snd".to_string())
        .spawn(move || {
            if !owned.exists() {
                return;
            }
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            let alias = format!("fg_alert_{stamp}");
            let path_str = owned.to_string_lossy().replace('"', "");
            let open_cmd = wide_null(&format!("open \"{path_str}\" alias {alias}"));
            let play_cmd = wide_null(&format!("play {alias} wait"));
            let close_cmd = wide_null(&format!("close {alias}"));
            unsafe {
                if mciSendStringW(PCWSTR(open_cmd.as_ptr()), std::ptr::null_mut(), 0, std::ptr::null_mut()) == 0 {
                    mciSendStringW(PCWSTR(play_cmd.as_ptr()), std::ptr::null_mut(), 0, std::ptr::null_mut());
                    mciSendStringW(PCWSTR(close_cmd.as_ptr()), std::ptr::null_mut(), 0, std::ptr::null_mut());
                }
            }
        });
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn sync_synth(
    shared: &Arc<Mutex<SharedAudioState>>,
    cfg: &AmbientConfig,
    playing: bool,
    master_muted: bool,
) {
    if let Ok(mut guard) = shared.lock() {
        guard.synth.apply_config(cfg, playing, master_muted);
    }
}

pub(crate) fn trigger_synth_alert(shared: &Arc<Mutex<SharedAudioState>>, kind: AlertKind) {
    if let Ok(mut guard) = shared.lock() {
        guard.synth.trigger_alert(kind);
    }
}

