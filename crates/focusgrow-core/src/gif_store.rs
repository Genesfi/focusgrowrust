use crate::config::{AmbientMode, AppConfig, RecentGif};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT_GIFS: usize = 5;

/// Copies a user-selected GIF/image into the local cache dir and activates it in config.
pub fn import_gif_into_store(
    source_path: &Path,
    gifs_dir: &Path,
    config: &mut AppConfig,
) -> Result<PathBuf> {
    fs::create_dir_all(gifs_dir)
        .with_context(|| format!("failed to create gifs dir {}", gifs_dir.display()))?;

    let original_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("custom.gif")
        .to_string();

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "gif" | "webp" | "png" | "jpg" | "jpeg"))
        .unwrap_or_else(|| "gif".to_string());

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let id = format!("gif_{stamp}");
    let dest_file_name = format!("{id}.{ext}");
    let dest_path = gifs_dir.join(&dest_file_name);

    fs::copy(source_path, &dest_path).with_context(|| {
        format!(
            "failed to copy GIF from {} to {}",
            source_path.display(),
            dest_path.display()
        )
    })?;

    // Deduplicate same display name so the recent strip stays clean, removing stale replaced files.
    let mut removed_files = Vec::new();
    config.recent_gifs.retain(|item| {
        if item.name.eq_ignore_ascii_case(&original_name) {
            removed_files.push(item.file_name.clone());
            false
        } else {
            true
        }
    });

    config.recent_gifs.insert(
        0,
        RecentGif {
            id: id.clone(),
            name: original_name.clone(),
            file_name: dest_file_name,
        },
    );

    if config.recent_gifs.len() > MAX_RECENT_GIFS {
        for overflow in config.recent_gifs.drain(MAX_RECENT_GIFS..) {
            removed_files.push(overflow.file_name);
        }
    }

    for stale in removed_files {
        let _ = fs::remove_file(gifs_dir.join(stale));
    }

    config.custom_gif_id = Some(id);
    config.custom_gif_name = original_name;
    config.ambient_mode = AmbientMode::Custom;

    Ok(dest_path)
}

/// Resolves the active custom GIF path on disk if present and existing.
pub fn resolve_active_gif_path(config: &AppConfig, gifs_dir: &Path) -> Option<PathBuf> {
    let active_id = config.custom_gif_id.as_deref()?;
    let entry = config.recent_gifs.iter().find(|g| g.id == active_id)?;
    let candidate = gifs_dir.join(&entry.file_name);
    candidate.exists().then_some(candidate)
}

/// Selects an entry from `recent_gifs` by id and returns its file path if it exists.
pub fn select_recent_gif(id: &str, gifs_dir: &Path, config: &mut AppConfig) -> Option<PathBuf> {
    let entry = config.recent_gifs.iter().find(|g| g.id == id)?.clone();
    let candidate = gifs_dir.join(&entry.file_name);
    if !candidate.exists() {
        return None;
    }
    config.custom_gif_id = Some(entry.id);
    config.custom_gif_name = entry.name;
    config.ambient_mode = AmbientMode::Custom;
    Some(candidate)
}

/// Removes a recent GIF from config and deletes its cached file.
pub fn remove_recent_gif(id: &str, gifs_dir: &Path, config: &mut AppConfig) {
    if let Some(pos) = config.recent_gifs.iter().position(|g| g.id == id) {
        let removed = config.recent_gifs.remove(pos);
        let _ = fs::remove_file(gifs_dir.join(&removed.file_name));
    }
    if config.custom_gif_id.as_deref() == Some(id) {
        if let Some(next) = config.recent_gifs.first() {
            config.custom_gif_id = Some(next.id.clone());
            config.custom_gif_name = next.name.clone();
        } else {
            config.custom_gif_id = None;
            config.custom_gif_name.clear();
            if config.ambient_mode == AmbientMode::Custom {
                config.ambient_mode = AmbientMode::Default;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_caps_recent_gifs_to_five_and_cleans_overflow() {
        let root = std::env::temp_dir().join("focusgrow_core_gif_store_test");
        let _ = fs::remove_dir_all(&root);
        let gifs_dir = root.join("gifs");
        let src_dir = root.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let mut cfg = AppConfig::default();
        for i in 0..6 {
            let src_file = src_dir.join(format!("clip_{i}.gif"));
            fs::write(&src_file, format!("GIF89a_{i}")).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
            import_gif_into_store(&src_file, &gifs_dir, &mut cfg).unwrap();
        }

        assert_eq!(cfg.recent_gifs.len(), 5);
        assert_eq!(cfg.ambient_mode, AmbientMode::Custom);
        assert!(resolve_active_gif_path(&cfg, &gifs_dir).is_some());

        let cached_count = fs::read_dir(&gifs_dir).unwrap().count();
        assert_eq!(cached_count, 5);

        let _ = fs::remove_dir_all(&root);
    }
}

