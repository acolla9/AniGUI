//! App settings and watch history: data types and pure logic.
//! File reading/writing lives in `persist.rs`.

use serde::{Deserialize, Serialize};

pub const MAX_HISTORY: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerChoice {
    #[default]
    Mpv,
    Vlc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Which player to use. mpv is the default.
    pub player: PlayerChoice,
    /// Optional full path to mpv. Empty = auto-detect.
    pub mpv_path: String,
    /// Optional full path to VLC. Empty = auto-detect.
    pub vlc_path: String,
    pub default_dub: bool,
    /// "best", "worst" or a label such as "720p".
    pub default_quality: String,
    /// Start the next episode automatically after one finishes (mpv only).
    pub auto_play_next: bool,
    pub panic_key: String,
    pub update_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player: PlayerChoice::Mpv,
            mpv_path: String::new(),
            vlc_path: String::new(),
            default_dub: false,
            default_quality: "best".to_string(),
            auto_play_next: true,
            panic_key: "F12".to_string(),
            update_url: "https://raw.githubusercontent.com/AidenColla/AniGUI/main/update-info.json".to_string(),
        }
    }
}

fn is_valid_quality(q: &str) -> bool {
    if q == "best" || q == "worst" {
        return true;
    }
    // e.g. "720p" or "1080p"
    (4..=5).contains(&q.len())
        && q.ends_with('p')
        && q[..q.len() - 1].chars().all(|c| c.is_ascii_digit())
}

impl Settings {
    /// Clean up user input (stray quotes/whitespace in paths, unknown quality).
    pub fn sanitized(mut self) -> Self {
        self.mpv_path = self.mpv_path.trim().trim_matches('"').trim().to_string();
        self.vlc_path = self.vlc_path.trim().trim_matches('"').trim().to_string();
        let quality = self.default_quality.trim().to_ascii_lowercase();
        if self.panic_key.trim().is_empty() { self.panic_key = "F12".to_string(); } else { self.panic_key = self.panic_key.trim().to_uppercase(); }
        self.update_url = self.update_url.trim().to_string();
        self.default_quality = if is_valid_quality(&quality) {
            quality
        } else {
            "best".to_string()
        };
        self
    }

    pub fn custom_path(&self, choice: PlayerChoice) -> Option<&str> {
        let path = match choice {
            PlayerChoice::Mpv => &self.mpv_path,
            PlayerChoice::Vlc => &self.vlc_path,
        };
        if path.is_empty() {
            None
        } else {
            Some(path.as_str())
        }
    }
}

/// One entry per anime: the last episode played.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub anime_id: String,
    pub anime_title: String,
    /// Cover image URL, if known. Older history files don't have it.
    #[serde(default)]
    pub poster: Option<String>,
    pub episode_id: String,
    pub episode_number: String,
    /// True once the player reached the end of that episode.
    pub completed: bool,
    /// Unix seconds.
    pub updated_at: u64,
}

/// Record that an episode started: the anime moves to the front, replacing its old entry.
pub fn record_started(entries: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    entries.retain(|e| e.anime_id != entry.anime_id);
    entries.insert(0, entry);
    entries.truncate(MAX_HISTORY);
}

/// Mark the stored episode as finished. Does nothing if the user has since moved
/// on to a different episode of the same anime. Returns whether anything changed.
pub fn mark_completed(
    entries: &mut [HistoryEntry],
    anime_id: &str,
    episode_id: &str,
    now: u64,
) -> bool {
    match entries
        .iter_mut()
        .find(|e| e.anime_id == anime_id && e.episode_id == episode_id)
    {
        Some(entry) => {
            entry.completed = true;
            entry.updated_at = now;
            true
        }
        None => false,
    }
}

pub fn remove(entries: &mut Vec<HistoryEntry>, anime_id: &str) {
    entries.retain(|e| e.anime_id != anime_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(anime: &str, ep: &str, t: u64) -> HistoryEntry {
        HistoryEntry {
            anime_id: anime.into(),
            anime_title: format!("Title {anime}"),
            poster: None,
            episode_id: format!("id-{ep}"),
            episode_number: ep.into(),
            completed: false,
            updated_at: t,
        }
    }

    #[test]
    fn started_moves_anime_to_front_and_replaces_old_entry() {
        let mut h = vec![];
        record_started(&mut h, entry("a", "1", 1));
        record_started(&mut h, entry("b", "1", 2));
        record_started(&mut h, entry("a", "2", 3));
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].anime_id, "a");
        assert_eq!(h[0].episode_number, "2");
        assert_eq!(h[1].anime_id, "b");
    }

    #[test]
    fn history_is_capped() {
        let mut h = vec![];
        for i in 0..(MAX_HISTORY + 20) {
            record_started(&mut h, entry(&format!("a{i}"), "1", i as u64));
        }
        assert_eq!(h.len(), MAX_HISTORY);
        assert_eq!(h[0].anime_id, format!("a{}", MAX_HISTORY + 19));
    }

    #[test]
    fn completed_only_applies_to_the_stored_episode() {
        let mut h = vec![entry("a", "2", 1)];
        assert!(!mark_completed(&mut h, "a", "id-1", 9)); // user already moved to ep 2
        assert!(!h[0].completed);
        assert!(mark_completed(&mut h, "a", "id-2", 9));
        assert!(h[0].completed);
        assert_eq!(h[0].updated_at, 9);
        assert!(!mark_completed(&mut h, "zzz", "id-2", 9));
    }

    #[test]
    fn replaying_a_finished_episode_resets_completed() {
        let mut h = vec![];
        record_started(&mut h, entry("a", "1", 1));
        mark_completed(&mut h, "a", "id-1", 2);
        record_started(&mut h, entry("a", "1", 3));
        assert!(!h[0].completed);
    }

    #[test]
    fn remove_deletes_by_anime() {
        let mut h = vec![entry("a", "1", 1), entry("b", "1", 2)];
        remove(&mut h, "a");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].anime_id, "b");
    }

    #[test]
    fn defaults_use_mpv() {
        let s = Settings::default();
        assert_eq!(s.player, PlayerChoice::Mpv);
        assert_eq!(s.default_quality, "best");
        assert!(!s.default_dub);
        assert!(s.auto_play_next);
    }

    #[test]
    fn sanitize_cleans_paths_and_quality() {
        let s = Settings {
            mpv_path: "  \"C:\\Tools\\mpv.exe\"  ".into(),
            default_quality: " 720P ".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.mpv_path, "C:\\Tools\\mpv.exe");
        assert_eq!(s.default_quality, "720p");
        let bad = Settings {
            default_quality: "ultra hd".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(bad.default_quality, "best");
        let s2 = Settings {
            default_quality: "1080p".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s2.default_quality, "1080p");
        assert!(!is_valid_quality("p"));
        assert!(!is_valid_quality("12345p"));
    }

    #[test]
    fn custom_path_is_per_player() {
        let s = Settings {
            vlc_path: "D:/vlc.exe".into(),
            ..Settings::default()
        };
        assert_eq!(s.custom_path(PlayerChoice::Vlc), Some("D:/vlc.exe"));
        assert_eq!(s.custom_path(PlayerChoice::Mpv), None);
    }
}


