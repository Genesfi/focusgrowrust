use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../ui/icon.png");
    println!("cargo:rerun-if-changed=../../../ui/icon.png");
    println!("cargo:rerun-if-changed=build.rs");

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let out = PathBuf::from(&out_dir);
    let ico_path = out.join("focusgrow.ico");

    let app_ico = manifest_dir.join("../../../app.ico");
    let app_ico_alt = manifest_dir.join("../../app.ico");
    if app_ico.is_file() {
        if fs::copy(&app_ico, &ico_path).is_err() {
            return;
        }
    } else if app_ico_alt.is_file() {
        if fs::copy(&app_ico_alt, &ico_path).is_err() {
            return;
        }
    } else {
        let png_path = if manifest_dir.join("../../ui/icon.png").is_file() {
            manifest_dir.join("../../ui/icon.png")
        } else {
            manifest_dir.join("../../../ui/icon.png")
        };
        let Ok(png_bytes) = fs::read(&png_path) else {
            return;
        };
        let (w, h) = parse_png_dims(&png_bytes).unwrap_or((0, 0));

        let mut ico = Vec::with_capacity(22 + png_bytes.len());
        ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
        ico.push(if w >= 256 { 0 } else { w as u8 });
        ico.push(if h >= 256 { 0 } else { h as u8 });
        ico.push(0);
        ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(png_bytes.len() as u32).to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&png_bytes);
        if fs::write(&ico_path, &ico).is_err() {
            return;
        }
    }

    let rc_path = out.join("focusgrow.rc");
    let res_path = out.join("focusgrow.res");
    let ico_escaped = ico_path.to_string_lossy().replace('\\', "\\\\");
    let rc_body = format!("1 ICON \"{ico_escaped}\"\n");
    if fs::write(&rc_path, rc_body).is_err() {
        return;
    }

    if let Some(rc_exe) = find_rc_exe() {
        let status = Command::new(rc_exe)
            .args(["/nologo", "/fo"])
            .arg(&res_path)
            .arg(&rc_path)
            .status();
        if status.is_ok_and(|s| s.success()) && res_path.exists() {
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
    }
}

fn parse_png_dims(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((w, h))
}

fn find_rc_exe() -> Option<PathBuf> {
    if let Ok(path_env) = env::var("PATH") {
        for dir in env::split_paths(&path_env) {
            let cand = dir.join("rc.exe");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    let kits = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if let Ok(entries) = fs::read_dir(kits) {
        let mut versions: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|x| x.path()))
            .filter(|p| p.is_dir())
            .collect();
        versions.sort();
        for dir in versions.into_iter().rev() {
            let cand = dir.join("x64").join("rc.exe");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

