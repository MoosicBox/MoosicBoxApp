// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    sync::{Arc, RwLock},
};

use log::info;
use moosicbox_core::sqlite::models::Album;
use moosicbox_player::player::{
    AudioFormat, Playback, PlaybackQuality, PlaybackRetryOptions, PlaybackStatus, PlaybackType,
    Player, PlayerError, TrackOrId,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use tauri::Manager;
use tauri_plugin_log::LogTarget;

#[derive(Serialize)]
pub struct TauriPlayerError {
    message: String,
}

impl From<PlayerError> for TauriPlayerError {
    fn from(err: PlayerError) -> Self {
        TauriPlayerError {
            message: err.to_string(),
        }
    }
}

static API_URL: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static PLAYER: Lazy<Arc<RwLock<Player>>> = Lazy::new(|| {
    Arc::new(RwLock::new(Player::new(
        Some(
            API_URL
                .read()
                .unwrap()
                .clone()
                .expect("API_URL not set")
                .to_string(),
        ),
        Some(PlaybackType::Stream),
    )))
});
const DEFAULT_PLAYBACK_RETRY_OPTIONS: PlaybackRetryOptions = PlaybackRetryOptions {
    max_retry_count: 10,
    retry_delay: std::time::Duration::from_millis(1000),
};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    window.get_window("main").unwrap().show().unwrap(); // replace "main" by the name of your window
}

fn stop_player() -> Result<(), PlayerError> {
    if let Err(err) = PLAYER.read().unwrap().stop(None) {
        match err {
            PlayerError::NoPlayersPlaying => {}
            _ => return Err(err),
        }
    }
    Ok(())
}

#[tauri::command]
async fn set_api_url(api_url: String) -> Result<(), TauriPlayerError> {
    API_URL.write().unwrap().replace(api_url);
    let stopped = stop_player();
    *PLAYER.write().unwrap() = Player::new(
        Some(
            API_URL
                .read()
                .unwrap()
                .clone()
                .expect("API_URL not set")
                .to_string(),
        ),
        Some(PlaybackType::Stream),
    );
    Ok(stopped?)
}

#[tauri::command]
async fn get_albums() -> Vec<Album> {
    //moosicbox_core::sqlite::menu::get_albums()
    vec![Album {
        id: 121,
        title: "test alb".into(),
        artist: "test".into(),
        artist_id: 123,
        ..Default::default()
    }]
}

#[tauri::command]
async fn player_play(track_ids: Vec<i32>) -> Result<PlaybackStatus, TauriPlayerError> {
    stop_player()?;

    info!("Playing Symphonia Player: {track_ids:?}");

    let playback = Playback::new(
        track_ids.iter().map(|id| TrackOrId::Id(*id)).collect(),
        None,
        PlaybackQuality {
            format: AudioFormat::Source,
        },
        None,
    );

    Ok(PLAYER.read().unwrap().play_playback(
        playback,
        None,
        Some(DEFAULT_PLAYBACK_RETRY_OPTIONS),
    )?)
}

#[tauri::command]
async fn player_pause() -> Result<PlaybackStatus, TauriPlayerError> {
    Ok(PLAYER.read().unwrap().pause_playback(None)?)
}

#[tauri::command]
async fn player_resume() -> Result<PlaybackStatus, TauriPlayerError> {
    Ok(PLAYER
        .read()
        .unwrap()
        .resume_playback(None, Some(DEFAULT_PLAYBACK_RETRY_OPTIONS))?)
}

#[tauri::command]
async fn player_next_track() -> Result<PlaybackStatus, TauriPlayerError> {
    Ok(PLAYER
        .read()
        .unwrap()
        .next_track(None, None, Some(DEFAULT_PLAYBACK_RETRY_OPTIONS))?)
}

#[tauri::command]
async fn player_previous_track() -> Result<PlaybackStatus, TauriPlayerError> {
    Ok(PLAYER
        .read()
        .unwrap()
        .previous_track(None, None, Some(DEFAULT_PLAYBACK_RETRY_OPTIONS))?)
}

#[tauri::command]
fn player_update_playback(
    position: Option<u16>,
    seek: Option<f64>,
) -> Result<PlaybackStatus, TauriPlayerError> {
    Ok(PLAYER.read().unwrap().update_playback(
        None,
        position,
        seek,
        Some(DEFAULT_PLAYBACK_RETRY_OPTIONS),
    )?)
}

#[tauri::command]
async fn api_proxy_get(url: String) -> serde_json::Value {
    let url = format!(
        "{}/{url}",
        API_URL.read().unwrap().clone().expect("API_URL not set")
    );
    info!("Fetching url from proxy: {url}");
    reqwest::get(url).await.unwrap().json().await.unwrap()
}

#[tauri::command]
async fn api_proxy_post(url: String, body: Option<serde_json::Value>) -> serde_json::Value {
    let url = format!(
        "{}/{url}",
        API_URL.read().unwrap().clone().expect("API_URL not set")
    );
    info!("Posting url from proxy: {url}");
    let client = reqwest::Client::new();

    let mut builder = client.post(url);

    if let Some(body) = body {
        builder = builder.json(&body);
    }

    builder.send().await.unwrap().json().await.unwrap()
}

#[cfg(feature = "aptabase")]
fn track_event(handler: &tauri::AppHandle, name: &str, props: Option<serde_json::Value>) {
    use std::{
        collections::HashMap,
        sync::{Mutex, OnceLock},
        time::Duration,
    };

    use debounce::EventDebouncer;
    use log::{debug, trace};
    use tauri_plugin_aptabase::EventTracker;

    static DISABLED_EVENTS: [&str; 2] = ["app_main_events_cleared", "app_window_event"];
    static DEBOUNCER_COUNTS: OnceLock<Mutex<HashMap<String, u16>>> = OnceLock::new();
    static DEBOUNCER: OnceLock<EventDebouncer<String>> = OnceLock::new();

    DEBOUNCER.get_or_init(|| {
        let debounce_duration = Duration::from_millis(10);
        EventDebouncer::new(debounce_duration, move |data: String| {
            let counts = DEBOUNCER_COUNTS.get().unwrap();
            let count = *counts.lock().unwrap().get(&data).unwrap_or(&0);
            if count > 1 {
                trace!("{data} ({count} times)");
            } else {
                trace!("{}", data);
            }
            counts.lock().unwrap().remove(&data);
        })
    });
    DEBOUNCER_COUNTS.get_or_init(|| Mutex::new(HashMap::new()));

    if DISABLED_EVENTS.iter().any(|n| *n == name) {
        let message = format!("Not tracking disabled event {name}: {props:?}").to_string();

        DEBOUNCER.get().unwrap().put(message.clone());

        let counts = DEBOUNCER_COUNTS.get().unwrap();
        let count = *counts.lock().unwrap().get(&message).unwrap_or(&0);
        counts.lock().unwrap().insert(message.clone(), count + 1);
        return;
    }

    debug!("Tracking event {name}: {props:?}");
    handler.track_event(name, props);
}

fn main() {
    let app_builder = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    LogTarget::Stdout,
                    // LogTarget::Webview,
                    LogTarget::LogDir,
                ])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            show_main_window,
            set_api_url,
            get_albums,
            player_play,
            player_pause,
            player_resume,
            player_next_track,
            player_previous_track,
            player_update_playback,
            api_proxy_get,
            api_proxy_post,
        ]);

    #[cfg(feature = "aptabase")]
    {
        use serde_json::json;
        use tauri_plugin_aptabase::EventTracker;

        let aptabase_app_key = std::env!("APTABASE_APP_KEY");

        app_builder
            .plugin(tauri_plugin_aptabase::Builder::new(aptabase_app_key).build())
            .build(tauri::generate_context!())
            .expect("error while running tauri application")
            .run(|handler, event| match event {
                tauri::RunEvent::Exit { .. } => {
                    track_event(handler, "app_exited", None);
                    handler.flush_events_blocking();
                }
                tauri::RunEvent::ExitRequested { api, .. } => track_event(
                    handler,
                    "app_exit_requested",
                    Some(json!({"api": format!("{api:?}")})),
                ),
                tauri::RunEvent::WindowEvent { label, event, .. } => track_event(
                    handler,
                    "app_window_event",
                    Some(json!({"label": label, "event": format!("{event:?}")})),
                ),
                tauri::RunEvent::Ready => track_event(handler, "app_ready", None),
                tauri::RunEvent::Resumed => track_event(handler, "app_resumed", None),
                tauri::RunEvent::MainEventsCleared => {
                    track_event(handler, "app_main_events_cleared", None)
                }
                _ => {}
            });
    }

    #[cfg(not(feature = "aptabase"))]
    app_builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
