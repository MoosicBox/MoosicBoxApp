use std::{
    collections::HashMap,
    env,
    sync::{Arc, OnceLock},
};

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use log::info;
use moosicbox_app_ws::{WebsocketSender as _, WsClient, WsHandle, WsMessage};
use moosicbox_audio_output::AudioOutputScannerError;
use moosicbox_core::sqlite::models::{ApiSource, Id};
use moosicbox_player::{
    local::LocalPlayer, Playback, PlaybackRetryOptions, PlaybackType, Player, PlayerError,
    PlayerSource, Track,
};
use moosicbox_session::models::{
    ApiPlayer, ApiUpdateSession, RegisterPlayer, UpdateSession, UpdateSessionPlaylistTrack,
};
use moosicbox_ws::models::{InboundPayload, OutboundPayload, UpdateSessionPayload};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString};
use tauri::{async_runtime::RwLock, AppHandle};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[cfg(all(feature = "cpal", feature = "android"))]
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: jni::JavaVM, res: *mut std::os::raw::c_void) -> jni::sys::jint {
    let vm = vm.get_java_vm_pointer() as *mut std::ffi::c_void;
    unsafe {
        ndk_context::initialize_android_context(vm, res);
    }
    jni::JNIVersion::V6.into()
}

#[derive(Debug, Error, Serialize)]
pub enum TauriPlayerError {
    #[error("Unknown({0})")]
    Unknown(String),
}

impl From<PlayerError> for TauriPlayerError {
    fn from(err: PlayerError) -> Self {
        TauriPlayerError::Unknown(err.to_string())
    }
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static LOG_LAYER: OnceLock<moosicbox_logging::free_log_client::FreeLogLayer> = OnceLock::new();

static API_URL: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static CONNECTION_ID: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static SIGNATURE_TOKEN: Lazy<Arc<RwLock<Option<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));
static CLIENT_ID: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static API_TOKEN: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));
static WS_TOKEN: Lazy<Arc<RwLock<Option<CancellationToken>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));
static WS_HANDLE: Lazy<Arc<RwLock<Option<WsHandle>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

lazy_static! {
    static ref PLAYERS: AsyncOnce<Arc<RwLock<HashMap<String, LocalPlayer>>>> =
        AsyncOnce::new(async { Arc::new(RwLock::new(HashMap::new())) });
    static ref SESSION_ACTIVE_PLAYERS: AsyncOnce<Arc<RwLock<HashMap<u64, LocalPlayer>>>> =
        AsyncOnce::new(async { Arc::new(RwLock::new(HashMap::new())) });
}

const DEFAULT_PLAYBACK_RETRY_OPTIONS: PlaybackRetryOptions = PlaybackRetryOptions {
    max_attempts: 10,
    retry_delay: std::time::Duration::from_millis(1000),
};

async fn new_player(audio_output_id: &str) -> Result<LocalPlayer, TauriPlayerError> {
    let headers = if API_TOKEN.read().await.is_some() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            API_TOKEN.read().await.clone().unwrap().to_string(),
        );
        Some(headers)
    } else {
        None
    };

    let query = if CLIENT_ID.read().await.is_some() && SIGNATURE_TOKEN.read().await.is_some() {
        let mut query = HashMap::new();
        query.insert(
            "clientId".to_string(),
            CLIENT_ID.read().await.clone().unwrap().to_string(),
        );
        query.insert(
            "signature".to_string(),
            SIGNATURE_TOKEN.read().await.clone().unwrap().to_string(),
        );
        Some(query)
    } else {
        None
    };

    let output = moosicbox_audio_output::output_factories()
        .await
        .into_iter()
        .find(|x| x.id.as_str() == audio_output_id)
        .ok_or_else(|| TauriPlayerError::Unknown("No outputs available".into()))?;

    let player = LocalPlayer::new(
        PlayerSource::Remote {
            host: API_URL
                .read()
                .await
                .clone()
                .ok_or(TauriPlayerError::Unknown("API_URL not set".to_string()))?
                .to_string(),
            headers,
            query,
        },
        Some(PlaybackType::Stream),
    )
    .await
    .map_err(|e| {
        TauriPlayerError::Unknown(format!("Failed to initialize new local player: {e:?}"))
    })?
    .with_output(output);

    Ok(player)
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[cfg(not(all(target_os = "android")))]
#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    use tauri::Manager as _;

    window.get_webview_window("main").unwrap().show().unwrap();
}

#[tauri::command]
async fn set_connection_id(connection_id: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting CONNECTION_ID: {connection_id}");

    CONNECTION_ID.write().await.replace(connection_id.clone());
    LOG_LAYER
        .get()
        .map(|x| x.set_property("connectionId", connection_id.into()));

    scan_outputs()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    Ok(())
}

#[tauri::command]
async fn set_connection_name(connection_name: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting CONNECTION_NAME: {connection_name}");

    LOG_LAYER
        .get()
        .map(|x| x.set_property("connectionName", connection_name.into()));

    Ok(())
}

#[tauri::command]
async fn set_client_id(client_id: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting CLIENT_ID: {client_id}");
    let existing = CLIENT_ID.read().await.as_ref().cloned();

    if existing.is_some_and(|x| x == client_id) {
        return Ok(());
    }

    CLIENT_ID.write().await.replace(client_id);

    init_ws_connection()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    scan_outputs()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    Ok(())
}

#[tauri::command]
async fn set_signature_token(signature_token: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting SIGNATURE_TOKEN: {signature_token}");
    let existing = SIGNATURE_TOKEN.read().await.as_ref().cloned();

    if existing.is_some_and(|x| x == signature_token) {
        return Ok(());
    }

    SIGNATURE_TOKEN.write().await.replace(signature_token);

    scan_outputs()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    Ok(())
}

#[tauri::command]
async fn set_api_token(api_token: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting API_TOKEN: {api_token}");
    let existing = API_TOKEN.read().await.as_ref().cloned();

    if existing.is_some_and(|x| x == api_token) {
        return Ok(());
    }

    API_TOKEN.write().await.replace(api_token);

    init_ws_connection()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    scan_outputs()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    Ok(())
}

#[tauri::command]
async fn set_api_url(api_url: String) -> Result<(), TauriPlayerError> {
    log::debug!("Setting API_URL: {api_url}");
    let existing = API_URL.read().await.as_ref().cloned();

    if existing.is_some_and(|x| x == api_url) {
        return Ok(());
    }

    API_URL.write().await.replace(api_url);

    init_ws_connection()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    scan_outputs()
        .await
        .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

    Ok(())
}

#[tauri::command]
async fn set_session_active_players(
    session_id: u64,
    players: Vec<ApiPlayer>,
) -> Result<(), TauriPlayerError> {
    log::debug!("Setting session active players: session_id={session_id} {players:?}");

    {
        let mut players_map = SESSION_ACTIVE_PLAYERS.get().await.write().await;
        for player in players.iter() {
            if let Some(existing) = players_map.get(&session_id) {
                if existing
                    .output
                    .as_ref()
                    .is_some_and(|output| output.lock().unwrap().id == player.audio_output_id)
                {
                    log::debug!(
                        "Skipping existing player for session_id={session_id} audio_output_id={}",
                        player.audio_output_id
                    );
                    continue;
                }
            }
            players_map.insert(session_id, new_player(&player.audio_output_id).await?);
        }
    }

    Ok(())
}

#[tauri::command]
async fn set_players(players: Vec<ApiPlayer>) -> Result<(), TauriPlayerError> {
    log::debug!("Setting players: {players:?}");

    {
        let mut players_map = PLAYERS.get().await.write().await;
        for player in players.iter() {
            players_map.insert(
                player.audio_output_id.clone(),
                new_player(&player.audio_output_id).await?,
            );
        }
    }

    Ok(())
}

async fn get_players(session_id: u64) -> Vec<LocalPlayer> {
    let players = SESSION_ACTIVE_PLAYERS.get().await.read().await;

    players
        .iter()
        .filter(|(sess, _)| **sess == session_id)
        .map(|(_, player)| player.clone())
        .collect()
}

#[derive(Copy, Debug, Serialize, Deserialize, EnumString, AsRefStr, PartialEq, Clone)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(untagged)]
pub enum TrackId {
    Library(u64),
    Tidal(u64),
    Qobuz(u64),
}

impl From<TrackId> for Id {
    fn from(value: TrackId) -> Self {
        match value {
            TrackId::Library(id) => Id::Number(id),
            TrackId::Tidal(id) => Id::Number(id),
            TrackId::Qobuz(id) => Id::Number(id),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackIdWithApiSource {
    id: TrackId,
    source: ApiSource,
}

impl From<TrackIdWithApiSource> for UpdateSessionPlaylistTrack {
    fn from(value: TrackIdWithApiSource) -> Self {
        Self {
            id: value.id.as_ref().to_string(),
            r#type: value.source,
            data: None,
        }
    }
}

#[tauri::command]
async fn api_proxy_get(url: String, headers: Option<serde_json::Value>) -> serde_json::Value {
    let url = format!(
        "{}/{url}",
        API_URL.read().await.clone().expect("API_URL not set")
    );
    info!("Fetching url from proxy: {url}");
    let client = reqwest::Client::new();

    let mut builder = client.get(url);

    if let Some(headers) = headers {
        for header in headers.as_object().unwrap() {
            builder = builder.header(header.0, header.1.as_str().unwrap().to_string());
        }
    }

    let resp = builder.send().await.expect("Failed to get response");

    match resp.json().await {
        Ok(json) => json,
        Err(err) => {
            panic!("Json failed: {err:?}");
        }
    }
}

#[tauri::command]
async fn api_proxy_post(
    url: String,
    body: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
) -> serde_json::Value {
    let url = format!(
        "{}/{url}",
        API_URL
            .read()
            .await
            .clone()
            .unwrap_or_else(|| panic!("API_URL not set ({url})"))
    );
    info!("Posting url from proxy: {url}");
    let client = reqwest::Client::new();

    let mut builder = client.post(url);

    if let Some(headers) = headers {
        for header in headers.as_object().unwrap() {
            builder = builder.header(header.0, header.1.as_str().unwrap().to_string());
        }
    }

    if let Some(body) = body {
        builder = builder.json(&body);
    }

    match builder.send().await {
        Ok(resp) => resp.json().await.unwrap(),
        Err(e) => {
            log::error!("Failed to send request: {e:?}");
            serde_json::json!({"success": false})
        }
    }
}

pub fn on_playback_event(update: &UpdateSession, _current: &Playback) {
    let update = update.to_owned();

    tauri::async_runtime::spawn(async move {
        if let Some(handle) = WS_HANDLE.read().await.as_ref() {
            log::debug!("on_playback_event: Sending update session: update={update:?}");
            let message =
                serde_json::to_string(&InboundPayload::UpdateSession(UpdateSessionPayload {
                    payload: update,
                }))
                .map_err(|e| e.to_string())?;
            handle.send(&message).await.map_err(|e| e.to_string())?;
        } else {
            log::debug!("on_playback_event: No WS_HANDLE to send update to");
        }

        Ok::<_, String>(())
    });
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

#[derive(Debug, Error)]
pub enum ScanOutputsError {
    #[error(transparent)]
    AudioOutputScanner(#[from] AudioOutputScannerError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    TauriPlayer(#[from] TauriPlayerError),
}

async fn scan_outputs() -> Result<(), ScanOutputsError> {
    log::debug!("scan_outputs: attempting to scan outputs");
    {
        if API_URL.as_ref().read().await.is_none() || CONNECTION_ID.as_ref().read().await.is_none()
        {
            log::debug!("scan_outputs: missing API_URL or CONNECTION_ID, not scanning");
            return Ok(());
        }
    }

    moosicbox_audio_output::scan_outputs().await?;
    let outputs = moosicbox_audio_output::output_factories().await;
    log::debug!("scan_outputs: scanned outputs={outputs:?}");

    let players = outputs
        .into_iter()
        .map(|x| RegisterPlayer {
            audio_output_id: x.id,
            name: x.name,
        })
        .collect::<Vec<_>>();

    let connection_id = CONNECTION_ID.read().await.clone().unwrap();

    api_proxy_post(
        format!("session/register-players?connectionId={connection_id}",),
        Some(serde_json::to_value(players)?),
        None,
    )
    .await;

    Ok(())
}

async fn handle_playback_update(update: ApiUpdateSession) -> Result<(), HandleWsMessageError> {
    log::debug!("handle_playback_update: {update:?}");
    for player in get_players(update.session_id).await {
        player
            .update_playback(
                true,
                update.play,
                update.stop,
                update.playing,
                update.position,
                update.seek,
                update.volume,
                update.playlist.clone().map(|x| {
                    x.tracks
                        .iter()
                        .map(|track| Track {
                            id: track.track_id(),
                            source: track.api_source(),
                            data: None,
                        })
                        .collect()
                }),
                update.quality,
                Some(update.session_id),
                None,
                false,
                Some(DEFAULT_PLAYBACK_RETRY_OPTIONS),
            )
            .await?;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum HandleWsMessageError {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Player(#[from] PlayerError),
}

async fn handle_ws_message(message: OutboundPayload) -> Result<(), HandleWsMessageError> {
    match message {
        OutboundPayload::SessionUpdated(payload) => handle_playback_update(payload.payload).await?,
        OutboundPayload::SetSeek(payload) => {
            handle_playback_update(ApiUpdateSession {
                session_id: payload.payload.session_id,
                play: None,
                stop: None,
                name: None,
                active: None,
                playing: None,
                position: None,
                seek: Some(payload.payload.seek as f64),
                volume: None,
                playlist: None,
                quality: None,
            })
            .await?
        }
        _ => {}
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum InitWsError {
    #[error(transparent)]
    AudioOutputScanner(#[from] AudioOutputScannerError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    TauriPlayer(#[from] TauriPlayerError),
}

async fn init_ws_connection() -> Result<(), InitWsError> {
    log::debug!("init_ws_connection: attempting to connect to ws");
    {
        if API_URL.as_ref().read().await.is_none() {
            log::debug!("init_ws_connection: missing API_URL");
            return Ok(());
        }
    }
    {
        if let Some(token) = WS_TOKEN.read().await.as_ref() {
            token.cancel();
        }
    }
    let token = {
        let token = CancellationToken::new();
        WS_TOKEN.write().await.replace(token.clone());
        token
    };

    let api_url = API_URL.read().await.clone().unwrap();

    let (client_id, signature_token) = {
        if let Some(client_id) = CLIENT_ID.read().await.clone() {
            if let Some(api_token) = API_TOKEN.as_ref().read().await.clone() {
                tokio::select! {
                    api_token = api_proxy_post(
                        format!("auth/signature-token?clientId={client_id}"),
                        None,
                        Some(serde_json::json!({"Authorization": format!("bearer {api_token}")})),
                    ) => (Some(client_id), api_token.get("token").and_then(|x| x.as_str()).map(|x| x.to_string())),
                    _ = token.cancelled() => {
                        log::debug!("init_ws_connection: cancelled");
                        return Ok(());
                    }
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    };

    let ws_url = format!("ws{}/ws", &api_url[4..]);
    let (client, handle) = WsClient::new(ws_url);

    WS_HANDLE.write().await.replace(handle.clone());

    let mut client = client.with_cancellation_token(token.clone());

    moosicbox_task::spawn("moosicbox_app: ws", async move {
        let mut rx = client.start(client_id, signature_token);

        while let Some(m) = tokio::select! {
            resp = rx.recv() => {
                resp
            }
            _ = token.cancelled() => {
                None
            }
        } {
            match m {
                WsMessage::Message(bytes) => match String::from_utf8(bytes.into()) {
                    Ok(message) => {
                        if let Ok(message) = serde_json::from_str::<OutboundPayload>(&message) {
                            if let Err(e) = handle_ws_message(message).await {
                                log::error!("Failed to handle_ws_message: {e:?}");
                            }
                        } else {
                            log::error!("got invalid message: {message}");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read ws message: {e:?}");
                    }
                },
                WsMessage::Ping => {
                    log::debug!("got ping");
                }
            }
        }
        log::debug!("Exiting ws message loop");
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::var("TOKIO_CONSOLE") == Ok("1".to_string()) {
        console_subscriber::init();
    } else {
        let layer =
            moosicbox_logging::init("moosicbox_app.log").expect("Failed to initialize FreeLog");
        LOG_LAYER.set(layer).expect("Failed to set LOG_LAYER");
    }

    moosicbox_player::on_playback_event(crate::on_playback_event);

    let app_builder = tauri::Builder::default()
        .setup(|app| {
            APP.get_or_init(|| app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            #[cfg(not(all(target_os = "android")))]
            show_main_window,
            set_connection_id,
            set_connection_name,
            set_client_id,
            set_signature_token,
            set_api_token,
            set_api_url,
            set_players,
            set_session_active_players,
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
