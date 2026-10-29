mod ani_engine;
mod persist;
mod player;
mod store;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU32, Ordering};
static ACTIVE_PLAYER_PID: AtomicU32 = AtomicU32::new(0);
static PANIC_VK_CODE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x7B); // VK_F12 = 0x7B

fn parse_vk_code(key: &str) -> u32 {
    let k = key.trim().to_uppercase();
    match k.as_str() {
        "F1" => 0x70,
        "F2" => 0x71,
        "F3" => 0x72,
        "F4" => 0x73,
        "F5" => 0x74,
        "F6" => 0x75,
        "F7" => 0x76,
        "F8" => 0x77,
        "F9" => 0x78,
        "F10" => 0x79,
        "F11" => 0x7A,
        "F12" => 0x7B,
        "ESCAPE" | "ESC" => 0x1B,
        "SPACE" => 0x20,
        "INSERT" => 0x2D,
        "DELETE" => 0x2E,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "PAUSE" => 0x13,
        "SCROLLLOCK" => 0x91,
        "PRINTSCREEN" => 0x2C,
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            if ch.is_ascii_digit() || ch.is_ascii_uppercase() {
                ch as u32
            } else {
                0x7B
            }
        }
        _ => 0x7B,
    }
}

#[cfg(target_os = "windows")]
fn start_global_panic_listener(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let mut key_was_down = false;
        loop {
            let vk = PANIC_VK_CODE.load(Ordering::SeqCst) as i32;
            let state = unsafe { GetAsyncKeyState(vk) };
            let is_down = (state as u16 & 0x8000) != 0;

            if is_down && !key_was_down {
                trigger_panic(app_handle.clone());
                break;
            }
            key_was_down = is_down;
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    });
}


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
        app_name: "GravityAnime",
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
    PANIC_VK_CODE.store(parse_vk_code(&cleaned.panic_key), Ordering::SeqCst);
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
    let find =
        |choice: PlayerChoice| player::find_player(kind_of(choice), settings.custom_path(choice));
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
    let found = player::find_player(kind, settings.custom_path(settings.player))
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
    ACTIVE_PLAYER_PID.store(child.id(), Ordering::SeqCst);

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
        ACTIVE_PLAYER_PID.store(0, Ordering::SeqCst);
        let failed = player::exit_code_is_failure(kind, code);
        let completed = !failed && watch_marker && marker.is_file();
        let _ = std::fs::remove_file(&marker);
        if completed {
            let _ = persist::update_history(&history_dir, |entries| {
                store::mark_completed(entries, &ended_anime_id, &ended_episode_id, now_secs());
            });
        }
        let message = if failed {
            format!(
                "The player could not play episode {episode_number} (exit code {}). The stream may have expired or the provider may have changed; try again.",
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


#[derive(serde::Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct VersionItem {
    version: String,
    release_date: Option<String>,
    notes: Option<String>,
    download_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct UpdateManifest {
    current_version: VersionItem,
    all_previous_versions: Option<Vec<VersionItem>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckResult {
    has_update: bool,
    current_version: String,
    latest_version: String,
    release_date: Option<String>,
    notes: Option<String>,
    download_url: Option<String>,
    all_previous_versions: Vec<VersionItem>,
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateCheckResult, String> {
    let dir = data_dir(&app)?;
    let settings = persist::load_settings(&dir);
    let url = if settings.update_url.trim().is_empty() {
        "https://raw.githubusercontent.com/AidenColla/AniGUI/main/update-info.json"
    } else {
        settings.update_url.trim()
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("GravityAnime-Updater/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Could not check for updates: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Update server returned HTTP status {}", res.status()));
    }

    let manifest: UpdateManifest = res
        .json()
        .await
        .map_err(|e| format!("Invalid update-info.json format: {e}"))?;

    let current = env!("CARGO_PKG_VERSION");
    let current_semver = semver::Version::parse(current).unwrap_or_else(|_| semver::Version::new(0, 1, 0));
    let latest_clean = manifest.current_version.version.trim().trim_start_matches('v');
    let latest_semver = semver::Version::parse(latest_clean).unwrap_or_else(|_| semver::Version::new(0, 0, 0));

    let has_update = latest_semver > current_semver;

    Ok(UpdateCheckResult {
        has_update,
        current_version: current.to_string(),
        latest_version: manifest.current_version.version,
        release_date: manifest.current_version.release_date,
        notes: manifest.current_version.notes,
        download_url: manifest.current_version.download_url,
        all_previous_versions: manifest.all_previous_versions.unwrap_or_default(),
    })
}

#[tauri::command]
async fn install_update(download_url: String) -> Result<String, String> {
    let url = download_url.trim();
    if url.is_empty() {
        return Err("No download URL provided.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("GravityAnime-Updater/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download update installer: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read update stream: {e}"))?;

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join("GravityAnime_Update_Installer.exe");

    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("Could not save installer to temp folder: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new(&installer_path).arg("/S").spawn().map_err(|e| {
            format!("Could not execute installer {}: {e}", installer_path.display())
        })?;
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Err("Auto-execution of setup is currently supported on Windows.".to_string());
    }
}

#[tauri::command]
fn trigger_panic(app: tauri::AppHandle) {
    let pid = ACTIVE_PLAYER_PID.swap(0, Ordering::SeqCst);
    if pid != 0 {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .spawn();
        }
    }
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Ok(dir) = data_dir(&app.handle().clone()) {
                let settings = persist::load_settings(&dir);
                PANIC_VK_CODE.store(parse_vk_code(&settings.panic_key), Ordering::SeqCst);
            }
            #[cfg(target_os = "windows")]
            {
                start_global_panic_listener(app.handle().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            trigger_panic,
            check_for_updates,
            install_update,
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
        .expect("error while running GravityAnime");
}
