//! Reads and writes settings and watch history as JSON files in the app data dir.
//! A corrupt or missing file falls back to defaults instead of breaking the app.

use crate::store::{HistoryEntry, Settings};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, path::Path, path::PathBuf, sync::Mutex};

// Playback threads and commands both edit history; serialise read-modify-write.
static HISTORY_LOCK: Mutex<()> = Mutex::new(());

fn settings_path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

fn history_path(dir: &Path) -> PathBuf {
    dir.join("history.json")
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Write to a temp file and rename, so a crash can't leave half a file behind.
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub fn load_settings(dir: &Path) -> Settings {
    read_json::<Settings>(&settings_path(dir)).sanitized()
}

pub fn save_settings(dir: &Path, settings: &Settings) -> io::Result<()> {
    write_json(&settings_path(dir), settings)
}

pub fn load_history(dir: &Path) -> Vec<HistoryEntry> {
    read_json(&history_path(dir))
}

/// Load history, apply `change`, save it, and return the new list.
pub fn update_history<F: FnOnce(&mut Vec<HistoryEntry>)>(
    dir: &Path,
    change: F,
) -> io::Result<Vec<HistoryEntry>> {
    let _guard = HISTORY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut entries: Vec<HistoryEntry> = read_json(&history_path(dir));
    change(&mut entries);
    write_json(&history_path(dir), &entries)?;
    Ok(entries)
}
