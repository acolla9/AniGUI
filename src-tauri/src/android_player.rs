//! Android only: hands playback to the Kotlin PlayerPlugin (Media3 player).

use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{Emitter, Manager, Runtime};

use super::{ani_engine, data_dir, now_secs, persist, store, PlaybackEnded, PlaybackStarted};

pub struct PlayerHandle<R: Runtime>(pub PluginHandle<R>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePlayRequest {
    title: String,
    video_url: String,
    referer: String,
    audio_url: Option<String>,
    subtitle_url: Option<String>,
}

#[derive(Deserialize)]
struct NativePlayResult {
    #[serde(default)]
    completed: bool,
    #[serde(default)]
    failed: bool,
    #[serde(default)]
    message: Option<String>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("player")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.gravityanime.app", "PlayerPlugin")?;
            app.manage(PlayerHandle(handle));
            Ok(())
        })
        .build()
}

pub async fn play(
    app: tauri::AppHandle,
    anime_id: String,
    anime_title: String,
    anime_poster: Option<String>,
    episode: ani_engine::Episode,
    dub: bool,
    quality: String,
) -> Result<PlaybackStarted, String> {
    let dir = data_dir(&app)?;
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
    let subtitle_url = resolved
        .subtitles
        .iter()
        .find(|s| s.is_default)
        .map(|s| s.url.clone());
    let title = format!("{} Episode {}", anime_title, episode.number);

    let request = NativePlayRequest {
        title,
        video_url: source.url.clone(),
        referer: resolved.referer.clone(),
        audio_url: resolved.audio_url.clone(),
        subtitle_url: subtitle_url.clone(),
    };

    let _ = persist::update_history(&dir, |entries| {
        store::record_started(
            entries,
            store::HistoryEntry {
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

    let ended_anime_id = anime_id.clone();
    let ended_episode_id = episode.id.clone();
    let episode_number = episode.number.clone();
    let history_dir = dir.clone();
    let task_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let blocking_app = task_app.clone();
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            let handle = blocking_app.state::<PlayerHandle<tauri::Wry>>();
            handle.0.run_mobile_plugin::<NativePlayResult>("play", request)
        })
        .await;

        let (failed, completed, native_message) = match outcome {
            Ok(Ok(r)) => (r.failed, r.completed && !r.failed, r.message),
            Ok(Err(e)) => (true, false, Some(e.to_string())),
            Err(e) => (true, false, Some(e.to_string())),
        };
        if completed {
            let _ = persist::update_history(&history_dir, |entries| {
                store::mark_completed(entries, &ended_anime_id, &ended_episode_id, now_secs());
            });
        }
        let message = if failed {
            format!(
                "The player could not play episode {episode_number}: {}",
                native_message.unwrap_or_else(|| "unknown error".into())
            )
        } else if completed {
            format!("Finished episode {episode_number}.")
        } else {
            format!("Player closed during episode {episode_number}.")
        };
        let _ = task_app.emit(
            "playback-ended",
            PlaybackEnded {
                anime_id: ended_anime_id,
                episode_id: ended_episode_id,
                exit_code: None,
                failed,
                completed,
                message,
            },
        );
    });

    let mut message = format!(
        "Playing episode {} at {} in the built-in player.",
        episode.number, source.quality
    );
    if fell_back {
        message = format!(
            "{wanted} isn't available for this episode, so the best quality was used. {message}"
        );
    }
    Ok(PlaybackStarted {
        episode_id: episode.id,
        player: "Built-in player".to_string(),
        quality: source.quality,
        has_subtitles: subtitle_url.is_some(),
        has_separate_audio: resolved.audio_url.is_some(),
        message,
    })
}