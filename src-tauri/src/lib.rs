mod ani_engine;
mod persist;
mod player;
mod store;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use player::PlayerKind;
use serde::Serialize;
use store::{HistoryEntry, PlayerChoice, Settings};
use tauri::{Emitter, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    app_name: &'static str,
    version: &'static str,
    engine_connected: bool,
    platform: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    query: String,
    results: Vec<ani_engine::AnimeResult>,
    engine_connected: bool,
    message: String,
}

#[tauri::command]
fn get_app_status() -> AppStatus {
    AppStatus {
        app_name: "AniGUI",
        version: env!("CARGO_PKG_VERSION"),
        // The native engine is now wired in. Provider requests can still fail
        // when the site changes, blocks requests, or the network is unavailable.
        engine_connected: true,
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "other"
        },
        message:
            "Native Rust search engine is connected. Live provider responses may change over time.",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodesResponse {
    anime_id: String,
    episodes: Vec<ani_engine::Episode>,
    message: String,
}

#[tauri::command]
async fn search_anime(query: String) -> Result<SearchResponse, String> {
    let cleaned = query.trim().to_string();
    if cleaned.is_empty() {
        return Err("Enter an anime title to search.".to_string());
    }

    let engine = ani_engine::AniEngine::new().map_err(|error| error.to_string())?;
    let results = engine
        .search(&cleaned)
        .await
        .map_err(|error| error.to_string())?;
    let count = results.len();
    let message = if count == 0 {
        format!("No results found for \"{cleaned}\".")
    } else {
        format!(
            "Found {count} result{} for \"{cleaned}\".",
            if count == 1 { "" } else { "s" }
        )
    };

    Ok(SearchResponse {
        query: cleaned,
        results,
        engine_connected: true,
        message,
    })
}

#[tauri::command]
async fn load_episodes(anime_id: String) -> Result<EpisodesResponse, String> {
    let cleaned = anime_id.trim().to_string();
    if cleaned.is_empty() {
        return Err("Choose an anime before loading episodes.".to_string());
    }

    let engine = ani_engine::AniEngine::new().map_err(|error| error.to_string())?;
    let episodes = engine
        .episodes(&cleaned)
        .await
        .map_err(|error| error.to_string())?;
    let count = episodes.len();

    Ok(EpisodesResponse {
        anime_id: cleaned,
        episodes,
        message: format!(
            "Loaded {count} episode{}.",
            if count == 1 { "" } else { "s" }
        ),
    })
}

// ---------------------------------------------------------------------------
// Settings, history and player detection
// ---------------------------------------------------------------------------

fn kind_of(choice: PlayerChoice) -> PlayerKind {
    match choice {
        PlayerChoice::Mpv => PlayerKind::Mpv,
        PlayerChoice::Vlc => PlayerKind::Vlc,
    }
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not locate the app data folder: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    Ok(persist::load_settings(&data_dir(&app)?))
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<Settings, String> {
    let dir = data_dir(&app)?;
    let cleaned = settings.sanitized();
    persist::save_settings(&dir, &cleaned).map_err(|e| format!("could not save settings: {e}"))?;
    Ok(cleaned)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayersInfo {
    selected: PlayerChoice,
    /// Where mpv / VLC were found, if they were.
    mpv: Option<String>,
    vlc: Option<String>,
    selected_found: bool,
    message: String,
}

#[tauri::command]
fn detect_players(app: tauri::AppHandle) -> Result<PlayersInfo, String> {
    let settings = persist::load_settings(&data_dir(&app)?);
    let resource_dir = app.path().resource_dir().ok();
    let find = |choice: PlayerChoice| {
        player::find_player_with_resources(
            kind_of(choice),
            settings.custom_path(choice),
            resource_dir.as_deref(),
        )
    };
    let mpv = find(PlayerChoice::Mpv);
    let vlc = find(PlayerChoice::Vlc);
    let chosen = match settings.player {
        PlayerChoice::Mpv => &mpv,
        PlayerChoice::Vlc => &vlc,
    };
    let message = match chosen {
        Some(p) => format!("Using {}", p.display_name()),
        None => player::missing_message(kind_of(settings.player)),
    };
    Ok(PlayersInfo {
        selected: settings.player,
        selected_found: chosen.is_some(),
        mpv: mpv.as_ref().map(|p| p.path.display().to_string()),
        vlc: vlc.as_ref().map(|p| p.path.display().to_string()),
        message,
    })
}

#[tauri::command]
fn get_history(app: tauri::AppHandle) -> Result<Vec<HistoryEntry>, String> {
    Ok(persist::load_history(&data_dir(&app)?))
}

#[tauri::command]
fn remove_history(app: tauri::AppHandle, anime_id: String) -> Result<Vec<HistoryEntry>, String> {
    let dir = data_dir(&app)?;
    persist::update_history(&dir, |entries| store::remove(entries, &anime_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(app: tauri::AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let dir = data_dir(&app)?;
    persist::update_history(&dir, |entries| entries.clear()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Playback
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStarted {
    episode_id: String,
    player: String,
    quality: String,
    has_subtitles: bool,
    has_separate_audio: bool,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackEnded {
    anime_id: String,
    episode_id: String,
    exit_code: Option<i32>,
    failed: bool,
    /// The episode played all the way to its end (detected for mpv only).
    completed: bool,
    message: String,
}

/// Resolve the episode's stream, pick a quality and open it in the player chosen in
/// Settings. Returns as soon as the player has started; a `playback-ended` event
/// follows when the player closes.
#[tauri::command]
async fn play_episode(
    app: tauri::AppHandle,
    anime_id: String,
    anime_title: String,
    anime_poster: Option<String>,
    episode: ani_engine::Episode,
    dub: bool,
    quality: String,
) -> Result<PlaybackStarted, String> {
    let dir = data_dir(&app)?;
    let settings = persist::load_settings(&dir);
    let kind = kind_of(settings.player);

    // Check for the player first so a missing install fails fast, before any network work.
    let resource_dir = app.path().resource_dir().ok();
    let found = player::find_player_with_resources(
        kind,
        settings.custom_path(settings.player),
        resource_dir.as_deref(),
    )
    .ok_or_else(|| player::missing_message(kind))?;

    let engine = ani_engine::AniEngine::new().map_err(|e| e.to_string())?;
    let resolved = engine
        .resolve_episode(&anime_id, &episode, dub)
        .await
        .map_err(|e| {
            format!(
                "Could not get a stream for episode {}: {e:#}",
                episode.number
            )
        })?;

    // Like ani-cli: fall back to the best quality if the requested one isn't offered.
    let wanted = if quality.trim().is_empty() {
        "best"
    } else {
        quality.trim()
    };
    let (source, fell_back) = match ani_engine::select_quality(&resolved.sources, wanted) {
        Ok(s) => (s, false),
        Err(_) => (
            ani_engine::select_quality(&resolved.sources, "best").map_err(|e| e.to_string())?,
            true,
        ),
    };

    // ani-cli uses the provider's default subtitle track, if there is one.
    let subtitle_url = resolved
        .subtitles
        .iter()
        .find(|s| s.is_default)
        .map(|s| s.url.clone());
    let title = format!("{} Episode {}", anime_title, episode.number);

    // mpv only: a tiny script tells us if the episode reached its end (vs. the user quitting).
    let marker: PathBuf = dir.join(format!(
        "playback-{}.eof",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&marker);
    let (script_arg, marker_arg) = if kind == PlayerKind::Mpv {
        match player::write_eof_script(&dir) {
            // Forward slashes work on Windows too and avoid mpv option-escaping surprises.
            Ok(script) => (
                Some(script.to_string_lossy().replace('\\', "/")),
                Some(marker.to_string_lossy().replace('\\', "/")),
            ),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    let request = player::PlayRequest {
        title: &title,
        video_url: &source.url,
        referer: &resolved.referer,
        audio_url: resolved.audio_url.as_deref(),
        subtitle_url: subtitle_url.as_deref(),
        eof_script: script_arg.as_deref(),
        eof_marker: marker_arg.as_deref(),
    };
    let mut child = player::spawn(&found, &request).map_err(|e| format!("{e:#}"))?;

    // Remember what is being watched (best effort).
    let _ = persist::update_history(&dir, |entries| {
        store::record_started(
            entries,
            HistoryEntry {
                anime_id: anime_id.clone(),
                anime_title: anime_title.clone(),
                poster: anime_poster.clone(),
                episode_id: episode.id.clone(),
                episode_number: episode.number.clone(),
                completed: false,
                updated_at: now_secs(),
            },
        )
    });

    // Reap the child, record completion, and tell the UI how it ended.
    let ended_anime_id = anime_id.clone();
    let ended_episode_id = episode.id.clone();
    let episode_number = episode.number.clone();
    let history_dir = dir.clone();
    let watch_marker = marker_arg.is_some();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|status| status.code());
        let failed = player::exit_code_is_failure(kind, code);
        let completed = !failed && watch_marker && marker.is_file();
        let _ = std::fs::remove_file(&marker);
        if completed {
            let _ = persist::update_history(&history_dir, |entries| {
                store::mark_completed(entries, &ended_anime_id, &ended_episode_id, now_secs());
            });
        }
        let message = if failed {
            let detail = if kind == PlayerKind::Mpv {
                player::mpv_log_tail(8)
                    .map(|t| format!("\n\nmpv said:\n{t}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            format!(
                "The player could not play episode {episode_number} (exit code {}). The stream may have expired or the provider may have changed; try again.{detail}",
                code.map(|c| c.to_string()).unwrap_or_else(|| "?".into())
            )
        } else if completed {
            format!("Finished episode {episode_number}.")
        } else {
            format!("Player closed during episode {episode_number}.")
        };
        let _ = app.emit(
            "playback-ended",
            PlaybackEnded {
                anime_id: ended_anime_id,
                episode_id: ended_episode_id,
                exit_code: code,
                failed,
                completed,
                message,
            },
        );
    });

    let mut message = format!(
        "Playing episode {} at {} in {}.",
        episode.number,
        source.quality,
        found.display_name()
    );
    if fell_back {
        message = format!(
            "{wanted} isn't available for this episode, so the best quality was used. {message}"
        );
    }
    Ok(PlaybackStarted {
        episode_id: episode.id,
        player: found.display_name(),
        quality: source.quality,
        has_subtitles: subtitle_url.is_some(),
        has_separate_audio: resolved.audio_url.is_some(),
        message,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            search_anime,
            load_episodes,
            get_settings,
            save_settings,
            detect_players,
            get_history,
            remove_history,
            clear_history,
            play_episode
        ])
        .run(tauri::generate_context!())
        .expect("error while running AniGUI");
}
