use std::{
    collections::HashMap,
    env,
    sync::{Arc, LazyLock, OnceLock},
};

use log::info;
use moosicbox_app_ws::{WebsocketSendError, WebsocketSender as _, WsClient, WsHandle, WsMessage};
use moosicbox_audio_output::AudioOutputScannerError;
use moosicbox_audio_zone::models::{ApiAudioZoneWithSession, ApiPlayer};
use moosicbox_core::{
    sqlite::models::{ApiSource, Id},
    types::PlaybackQuality,
};
use moosicbox_paging::Page;
use moosicbox_player::{
    local::LocalPlayer, Playback, PlaybackRetryOptions, PlaybackType, Player, PlayerError,
    PlayerSource, Track,
};
use moosicbox_session::models::{
    ApiConnection, ApiPlaybackTarget, ApiSession, ApiUpdateSession, ApiUpdateSessionPlaylist,
    PlaybackTarget, RegisterPlayer, UpdateSession, UpdateSessionPlaylistTrack,
};
use moosicbox_ws::models::{
    EmptyPayload, InboundPayload, OutboundPayload, SessionUpdatedPayload, UpdateSessionPayload,
};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString};
use tauri::{async_runtime::RwLock, AppHandle, Emitter};
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

#[derive(Debug, Error, Serialize, Deserialize)]
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

type ApiPlayersMap = HashMap<u64, Vec<ApiPlayer>>;

struct PlaybackTargetSessionPlayer {
    playback_target: ApiPlaybackTarget,
    session_id: u64,
    player: LocalPlayer,
}

static API_URL: LazyLock<Arc<RwLock<Option<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static CONNECTION_ID: LazyLock<Arc<RwLock<Option<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static SIGNATURE_TOKEN: LazyLock<Arc<RwLock<Option<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static CLIENT_ID: LazyLock<Arc<RwLock<Option<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static API_TOKEN: LazyLock<Arc<RwLock<Option<String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static WS_TOKEN: LazyLock<Arc<RwLock<Option<CancellationToken>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static WS_HANDLE: LazyLock<Arc<RwLock<Option<WsHandle>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static AUDIO_ZONE_ACTIVE_API_PLAYERS: LazyLock<Arc<RwLock<ApiPlayersMap>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
static AUDIO_ZONE_ACTIVE_PLAYERS: LazyLock<Arc<RwLock<Vec<PlaybackTargetSessionPlayer>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static PLAYBACK_QUALITY: LazyLock<Arc<RwLock<Option<PlaybackQuality>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static WS_MESSAGE_BUFFER: LazyLock<Arc<RwLock<Vec<InboundPayload>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static CURRENT_PLAYBACK_TARGET: LazyLock<Arc<RwLock<Option<PlaybackTarget>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static CURRENT_CONNECTIONS: LazyLock<Arc<RwLock<Vec<ApiConnection>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static PENDING_PLAYER_SESSIONS: LazyLock<Arc<RwLock<HashMap<u64, u64>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
static CURRENT_SESSIONS: LazyLock<Arc<RwLock<Vec<ApiSession>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static CURRENT_AUDIO_ZONES: LazyLock<Arc<RwLock<Vec<ApiAudioZoneWithSession>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static CURRENT_PLAYERS: LazyLock<Arc<RwLock<Vec<ApiPlayer>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));

const DEFAULT_PLAYBACK_RETRY_OPTIONS: PlaybackRetryOptions = PlaybackRetryOptions {
    max_attempts: 10,
    retry_delay: std::time::Duration::from_millis(1000),
};

async fn new_player(
    session_id: u64,
    playback_target: ApiPlaybackTarget,
    audio_output_id: &str,
) -> Result<LocalPlayer, TauriPlayerError> {
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

    let session = CURRENT_SESSIONS
        .read()
        .await
        .iter()
        .find(|x| x.session_id == session_id)
        .cloned();

    if let Some(session) = session {
        log::debug!("new_player: init_from_api_session session={session:?}");
        if let Err(e) = player.init_from_api_session(session).await {
            log::error!("Failed to init player from api session: {e:?}");
        }
    } else {
        log::debug!("new_player: No session info available for player yet");
        PENDING_PLAYER_SESSIONS
            .write()
            .await
            .insert(player.id as u64, session_id);
    }

    player
        .update_playback(
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            *PLAYBACK_QUALITY.read().await,
            Some(session_id),
            Some(playback_target.into()),
            false,
            None,
        )
        .await?;

    Ok(player)
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[cfg(not(all(target_os = "android")))]
#[tauri::command]
async fn show_main_window(window: tauri::Window) {
    use tauri::Manager as _;

    window.get_webview_window("main").unwrap().show().unwrap();
}

#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    connection_id: Option<String>,
    connection_name: Option<String>,
    api_url: Option<String>,
    client_id: Option<String>,
    signature_token: Option<String>,
    api_token: Option<String>,
    playback_target: Option<PlaybackTarget>,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

#[tauri::command]
async fn set_state(state: AppState) -> Result<(), TauriPlayerError> {
    log::debug!("set_state: state={state:?}");

    let mut updated_connection_details = false;

    {
        if let Some(connection_id) = &state.connection_id {
            LOG_LAYER
                .get()
                .map(|x| x.set_property("connectionId", connection_id.to_owned().into()));
        } else {
            LOG_LAYER.get().map(|x| x.remove_property("connectionId"));
        }

        let mut connection_id = CONNECTION_ID.write().await;

        if connection_id.as_ref() != state.connection_id.as_ref() {
            updated_connection_details = true;
        }

        *connection_id = state.connection_id;
    }

    {
        if let Some(connection_name) = &state.connection_name {
            LOG_LAYER
                .get()
                .map(|x| x.set_property("connectionName", connection_name.to_owned().into()));
        } else {
            LOG_LAYER.get().map(|x| x.remove_property("connectionName"));
        }
    }

    {
        if let Some(client_id) = &state.client_id {
            LOG_LAYER
                .get()
                .map(|x| x.set_property("clientId", client_id.to_owned().into()));
        } else {
            LOG_LAYER.get().map(|x| x.remove_property("clientId"));
        }

        let mut client_id = CLIENT_ID.write().await;

        if client_id.as_ref() != state.client_id.as_ref() {
            updated_connection_details = true;
        }

        *client_id = state.client_id;
    }

    {
        let mut signature_token = SIGNATURE_TOKEN.write().await;

        if signature_token.as_ref() != state.signature_token.as_ref() {
            updated_connection_details = true;
        }

        *signature_token = state.signature_token;
    }

    {
        let mut api_token = API_TOKEN.write().await;

        if api_token.as_ref() != state.api_token.as_ref() {
            updated_connection_details = true;
        }

        *api_token = state.api_token;
    }

    {
        if let Some(api_url) = &state.api_url {
            LOG_LAYER
                .get()
                .map(|x| x.set_property("apiUrl", api_url.to_owned().into()));
        } else {
            LOG_LAYER.get().map(|x| x.remove_property("apiUrl"));
        }

        let mut api_url = API_URL.write().await;

        if api_url.as_ref() != state.api_url.as_ref() {
            updated_connection_details = true;
        }

        *api_url = state.api_url;
    }

    {
        *CURRENT_PLAYBACK_TARGET.write().await = state.playback_target;
    }

    if updated_connection_details {
        scan_outputs()
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

        reinit_players().await?;

        fetch_audio_zones()
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

        init_ws_connection()
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;
    }

    Ok(())
}

async fn reinit_players() -> Result<(), TauriPlayerError> {
    let mut players_map = AUDIO_ZONE_ACTIVE_PLAYERS.write().await;
    let ids = {
        players_map
            .iter()
            .map(|x| (x.playback_target.clone(), x.session_id, x.player.clone()))
            .collect::<Vec<_>>()
    };

    for (i, (playback_target, session_id, player)) in ids.into_iter().enumerate() {
        let output_id = player.output.as_ref().unwrap().lock().unwrap().id.clone();
        log::debug!("reinit_players: playback_target={playback_target:?} session_id={session_id} output_id={output_id}");
        let created_player = new_player(session_id, playback_target.clone(), &output_id).await?;

        let playback = player.active_playback.read().unwrap().as_ref().cloned();

        if let Some(playback) = playback {
            created_player
                .update_playback(
                    false,
                    None,
                    None,
                    Some(playback.playing),
                    Some(playback.position),
                    Some(playback.progress),
                    Some(playback.volume.load(std::sync::atomic::Ordering::SeqCst)),
                    Some(playback.tracks.clone()),
                    Some(playback.quality),
                    playback.session_id,
                    Some(playback_target.clone().into()),
                    false,
                    None,
                )
                .await?;
        }

        players_map[i] = PlaybackTargetSessionPlayer {
            playback_target,
            session_id,
            player: created_player,
        };
    }

    Ok(())
}

#[tauri::command]
async fn set_audio_zone_active_players(
    session_id: u64,
    audio_zone_id: u64,
    players: Vec<ApiPlayer>,
) -> Result<(), TauriPlayerError> {
    log::debug!("Setting audio_zone active players: session_id={session_id} audio_zone_id={audio_zone_id} {players:?}");

    let mut api_players_map = AUDIO_ZONE_ACTIVE_API_PLAYERS.write().await;
    api_players_map.insert(audio_zone_id, players.clone());

    {
        let mut players_map = AUDIO_ZONE_ACTIVE_PLAYERS.write().await;
        for player in players.iter() {
            if let Some(existing) = players_map.iter().find(|x| match x.playback_target {
                ApiPlaybackTarget::AudioZone { audio_zone_id: id } => id == audio_zone_id,
                _ => false,
            }) {
                let different_session = {
                    !existing
                        .player
                        .active_playback
                        .read()
                        .unwrap()
                        .as_ref()
                        .is_some_and(|p| p.session_id.is_some_and(|s| s == session_id))
                };

                let same_output =
                    {
                        existing.player.output.as_ref().is_some_and(|output| {
                            output.lock().unwrap().id == player.audio_output_id
                        })
                    };

                if !different_session && same_output {
                    log::debug!(
                        "Skipping existing player for audio_zone_id={audio_zone_id} audio_output_id={}",
                        player.audio_output_id
                    );
                    continue;
                }
            }

            let playback_target = ApiPlaybackTarget::AudioZone { audio_zone_id };
            let player =
                new_player(session_id, playback_target.clone(), &player.audio_output_id).await?;
            log::debug!(
                "set_audio_zone_active_players: audio_zone_id={audio_zone_id} session_id={session_id:?}"
            );
            let playback_target_session_player = PlaybackTargetSessionPlayer {
                playback_target,
                session_id,
                player,
            };
            if let Some((i, _)) =
                players_map
                    .iter()
                    .enumerate()
                    .find(|(_, x)| match x.playback_target {
                        ApiPlaybackTarget::AudioZone { audio_zone_id: id } => {
                            id == audio_zone_id && x.session_id == session_id
                        }
                        _ => false,
                    })
            {
                players_map[i] = playback_target_session_player;
            } else {
                players_map.push(playback_target_session_player);
            }
        }
    }

    Ok(())
}

async fn update_audio_zones() -> Result<(), TauriPlayerError> {
    let audio_zones_binding = CURRENT_AUDIO_ZONES.read().await;
    let audio_zones: &[ApiAudioZoneWithSession] = audio_zones_binding.as_ref();
    let players_binding = CURRENT_PLAYERS.read().await;
    let players: &[ApiPlayer] = players_binding.as_ref();

    log::debug!(
        "\
        Updating audio zones\n\t\
        audio_zones={audio_zones:?}\n\t\
        players={players:?}\n\t\
        "
    );

    for audio_zone in audio_zones {
        let players = audio_zone
            .players
            .clone()
            .into_iter()
            .filter(|x| players.iter().any(|p| p.player_id == x.player_id))
            .collect::<Vec<_>>();

        if !players.is_empty() {
            set_audio_zone_active_players(audio_zone.session_id, audio_zone.id, players).await?;
        }
    }
    Ok(())
}

async fn get_players(session_id: u64, playback_target: &ApiPlaybackTarget) -> Vec<LocalPlayer> {
    let players = AUDIO_ZONE_ACTIVE_PLAYERS.read().await;

    players
        .iter()
        .filter(|x| {
            let target = &x.playback_target;
            log::trace!(
                "get_players: Checking if player is in session: target={target:?} session_id={session_id} player_zone_id={playback_target:?}",
            );
            x.player.active_playback
                .read()
                .unwrap()
                .as_ref()
                .is_some_and(|p| p.session_id.is_some_and(|s| {
                    log::trace!(
                        "get_players: player playback.session_id={s} target session_id={session_id}",
                    );
                    s == session_id
                }))
        })
        .filter(|x| {
            let target = &x.playback_target;
            log::trace!(
                "get_players: Checking if player is in zone: target={target:?} session_id={session_id} player_zone_id={playback_target:?}",
            );
            target == playback_target
        })
        .map(|x| x.player.clone())
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
async fn set_playback_quality(quality: PlaybackQuality) -> Result<(), TauriPlayerError> {
    log::debug!("Setting playback quality: {quality:?}");

    PLAYBACK_QUALITY.write().await.replace(quality);

    let binding = AUDIO_ZONE_ACTIVE_PLAYERS.read().await;
    let players = binding.iter();

    for x in players {
        x.player
            .update_playback(
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                *PLAYBACK_QUALITY.read().await,
                Some(x.session_id),
                Some(x.playback_target.clone().into()),
                false,
                None,
            )
            .await?;
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum SendWsMessageError {
    #[error(transparent)]
    WebsocketSend(#[from] WebsocketSendError),
    #[error(transparent)]
    HandleWsMessage(#[from] HandleWsMessageError),
}

async fn send_ws_message(
    handle: &WsHandle,
    message: InboundPayload,
    handle_update: bool,
) -> Result<(), SendWsMessageError> {
    if handle_update {
        match &message {
            InboundPayload::UpdateSession(payload) => {
                handle_playback_update(&payload.payload.clone().into()).await?;
            }
            InboundPayload::SetSeek(payload) => {
                handle_playback_update(&ApiUpdateSession {
                    session_id: payload.payload.session_id,
                    playback_target: payload.payload.playback_target.clone(),
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
                .await?;
            }
            _ => {}
        }
    }

    handle
        .send(&serde_json::to_string(&message).unwrap())
        .await?;

    Ok(())
}

async fn flush_ws_message_buffer() -> Result<(), SendWsMessageError> {
    if let Some(handle) = WS_HANDLE.read().await.as_ref() {
        let mut binding = WS_MESSAGE_BUFFER.write().await;
        log::debug!(
            "flush_ws_message_buffer: Flushing {} ws messages from buffer",
            binding.len()
        );

        let messages = binding.drain(..);

        for message in messages {
            send_ws_message(handle, message, true).await?;
        }
    } else {
        log::debug!("flush_ws_message_buffer: No WS_HANDLE");
    }

    Ok(())
}

#[tauri::command]
async fn propagate_ws_message(message: InboundPayload) -> Result<(), TauriPlayerError> {
    log::debug!("Received ws message from frontend: {message}");

    if let Some(handle) = WS_HANDLE.read().await.as_ref() {
        send_ws_message(handle, message, true)
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;
    } else {
        WS_MESSAGE_BUFFER.write().await.push(message);
    }

    Ok(())
}

async fn send_request_builder(
    builder: RequestBuilder,
) -> Result<serde_json::Value, TauriPlayerError> {
    log::debug!("send_request_builder: Sending request");
    match builder.send().await {
        Ok(resp) => {
            log::debug!("send_request_builder: status_code={}", resp.status());
            let success = resp.status().is_success();
            match resp.text().await {
                Ok(text) => {
                    if success {
                        match serde_json::from_str(&text) {
                            Ok(resp) => {
                                log::debug!("Got post response: {resp:?}");
                                Ok(resp)
                            }
                            Err(e) => {
                                log::error!("Failed to parse request response: {e:?} ({text:?})");
                                Err(TauriPlayerError::Unknown(format!("Json failed: {e:?}")))
                            }
                        }
                    } else {
                        log::error!("Failure response: ({text:?})");
                        Err(TauriPlayerError::Unknown(format!(
                            "Request failed: {text:?}"
                        )))
                    }
                }
                Err(e) => {
                    log::error!("Failed to parse request response: {e:?}");
                    Err(TauriPlayerError::Unknown(format!("Json failed: {e:?}")))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to send request: {e:?}");
            Err(TauriPlayerError::Unknown(format!("Json failed: {e:?}")))
        }
    }
}

#[tauri::command]
async fn api_proxy_get(
    url: String,
    headers: Option<serde_json::Value>,
) -> Result<serde_json::Value, TauriPlayerError> {
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

    send_request_builder(builder).await
}

#[tauri::command]
async fn api_proxy_post(
    url: String,
    body: Option<serde_json::Value>,
    headers: Option<serde_json::Value>,
) -> Result<serde_json::Value, TauriPlayerError> {
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

    send_request_builder(builder).await
}

pub fn on_playback_event(update: &UpdateSession, _current: &Playback) {
    log::debug!("on_playback_event: received update, spawning task to handle update={update:?}");

    let update = update.to_owned();

    moosicbox_task::spawn("moosicbox_app: on_playback_event", async move {
        if let Some(handle) = WS_HANDLE.read().await.as_ref() {
            log::debug!("on_playback_event: Sending update session: update={update:?}");

            APP.get()
                .unwrap()
                .emit(
                    "ws-message",
                    OutboundPayload::SessionUpdated(SessionUpdatedPayload {
                        payload: update.clone().into(),
                    }),
                )
                .map_err(|e| e.to_string())?;

            send_ws_message(
                handle,
                InboundPayload::UpdateSession(UpdateSessionPayload { payload: update }),
                false,
            )
            .await
            .map_err(|e| e.to_string())?;
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

    if moosicbox_audio_output::output_factories().await.is_empty() {
        moosicbox_audio_output::scan_outputs().await?;
    }

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
    let api_token = API_TOKEN.read().await.clone();
    let client_id = CLIENT_ID
        .read()
        .await
        .clone()
        .map(|x| format!("&clientId={x}"))
        .unwrap_or_default();

    let response = api_proxy_post(
        format!("session/register-players?connectionId={connection_id}{client_id}",),
        Some(serde_json::to_value(players)?),
        api_token.map(|token| serde_json::json!({"Authorization": format!("bearer {token}")})),
    )
    .await?;

    let players: Vec<ApiPlayer> = serde_json::from_value(response)?;

    log::debug!("scan_outputs: players={players:?}");

    *CURRENT_PLAYERS.write().await = players;

    update_audio_zones().await?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum FetchAudioZonesError {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    TauriPlayer(#[from] TauriPlayerError),
}

async fn fetch_audio_zones() -> Result<(), FetchAudioZonesError> {
    let api_token = API_TOKEN.read().await.clone();
    let client_id = CLIENT_ID
        .read()
        .await
        .clone()
        .filter(|x| !x.is_empty())
        .map(|x| format!("?clientId={x}"))
        .unwrap_or_default();

    let response = api_proxy_get(
        format!("audio-zone/with-session{client_id}",),
        api_token.map(|token| serde_json::json!({"Authorization": format!("bearer {token}")})),
    )
    .await?;

    log::debug!("fetch_audio_zones: audio_zones={response}");

    let zones: Page<ApiAudioZoneWithSession> = serde_json::from_value(response)?;

    *CURRENT_AUDIO_ZONES.write().await = zones.items();

    update_audio_zones().await?;

    Ok(())
}

async fn get_session_playback_for_player(
    mut update: ApiUpdateSession,
    player: &LocalPlayer,
) -> ApiUpdateSession {
    let session_id = {
        player
            .active_playback
            .read()
            .unwrap()
            .as_ref()
            .and_then(|x| x.session_id)
    };

    if let Some(session_id) = session_id {
        if session_id != update.session_id {
            let session = {
                CURRENT_SESSIONS
                    .read()
                    .await
                    .iter()
                    .find(|s| s.session_id == session_id)
                    .cloned()
            };

            if let Some(session) = session {
                update.session_id = session_id;

                if update.position.is_none() {
                    update.position = session.position;
                }
                if update.seek.is_none() {
                    update.seek = session.seek.map(|x| x as f64);
                }
                if update.volume.is_none() {
                    update.volume = session.volume;
                }
                if update.playlist.is_none() {
                    update.playlist = Some(ApiUpdateSessionPlaylist {
                        session_playlist_id: session.playlist.session_playlist_id,
                        tracks: session.playlist.tracks.clone(),
                    });
                }
            }
        }
    }

    update
}

async fn handle_playback_update(update: &ApiUpdateSession) -> Result<(), HandleWsMessageError> {
    log::debug!("handle_playback_update: {update:?}");
    let mut players = get_players(update.session_id, &update.playback_target).await;

    if players.is_empty() {
        match &update.playback_target {
            ApiPlaybackTarget::AudioZone { audio_zone_id } => {
                let audio_zone = {
                    let binding = CURRENT_AUDIO_ZONES.read().await;
                    let audio_zones: &[ApiAudioZoneWithSession] = binding.as_ref();
                    audio_zones.iter().find(|x| x.id == *audio_zone_id).cloned()
                };
                if let Some(audio_zone) = audio_zone {
                    let audio_zone_with_session = ApiAudioZoneWithSession {
                        id: *audio_zone_id,
                        session_id: update.session_id,
                        name: audio_zone.name,
                        players: audio_zone.players,
                    };
                    CURRENT_AUDIO_ZONES
                        .write()
                        .await
                        .push(audio_zone_with_session);
                    update_audio_zones().await?;
                    players = get_players(update.session_id, &update.playback_target).await;
                }
            }
            ApiPlaybackTarget::ConnectionOutput { .. } => unimplemented!(),
        }
    }

    for player in players {
        let update = get_session_playback_for_player(update.to_owned(), &player).await;

        log::debug!("handle_playback_update: player={}", player.id);

        if let Some(quality) = update.quality {
            PLAYBACK_QUALITY.write().await.replace(quality);
        }

        player
            .update_playback(
                true,
                update.play,
                update.stop,
                update.playing,
                update.position,
                update.seek,
                update.volume,
                update.playlist.map(|x| {
                    x.tracks
                        .iter()
                        .map(|track| Track {
                            id: track.track_id(),
                            source: track.api_source(),
                            data: track.data(),
                        })
                        .collect()
                }),
                update.quality,
                Some(update.session_id),
                Some(update.playback_target.into()),
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
    #[error(transparent)]
    Emit(#[from] tauri::Error),
    #[error(transparent)]
    Tauri(#[from] TauriPlayerError),
}

async fn handle_ws_message(message: OutboundPayload) -> Result<(), HandleWsMessageError> {
    match &message {
        OutboundPayload::SessionUpdated(payload) => {
            handle_playback_update(&payload.payload).await?
        }
        OutboundPayload::SetSeek(payload) => {
            handle_playback_update(&ApiUpdateSession {
                session_id: payload.payload.session_id,
                playback_target: payload.payload.playback_target.clone(),
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
        OutboundPayload::ConnectionId(payload) => {
            APP.get()
                .unwrap()
                .emit("on-connect", payload.connection_id.to_owned())?;
        }
        OutboundPayload::Connections(payload) => {
            *CURRENT_CONNECTIONS.write().await = payload.payload.clone();

            update_audio_zones().await?;
        }
        OutboundPayload::Sessions(payload) => {
            let player_ids = {
                let mut player_ids = vec![];
                let player_sessions = PENDING_PLAYER_SESSIONS
                    .read()
                    .await
                    .iter()
                    .map(|(x, y)| (*x, *y))
                    .collect::<Vec<_>>();

                for (player_id, session_id) in player_sessions {
                    if let Some(session) =
                        payload.payload.iter().find(|x| x.session_id == session_id)
                    {
                        if let Some(player) = AUDIO_ZONE_ACTIVE_PLAYERS
                            .read()
                            .await
                            .iter()
                            .find(|x| x.player.id as u64 == player_id)
                            .map(|x| &x.player)
                        {
                            log::debug!(
                                "handle_ws_message: init_from_api_session session={session:?}"
                            );
                            if let Err(e) = player.init_from_api_session(session.clone()).await {
                                log::error!("Failed to init player from api session: {e:?}");
                            }
                            player_ids.push(player_id);
                        }
                    }
                }

                player_ids
            };
            {
                PENDING_PLAYER_SESSIONS
                    .write()
                    .await
                    .retain(|id, _| !player_ids.contains(id));
            }
            *CURRENT_SESSIONS.write().await = payload.payload.clone();

            update_audio_zones().await?;
        }

        OutboundPayload::AudioZoneWithSessions(payload) => {
            *CURRENT_AUDIO_ZONES.write().await = payload.payload.clone();

            update_audio_zones().await?;
        }
        _ => {}
    }

    APP.get().unwrap().emit("ws-message", message)?;

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

    let client_id = CLIENT_ID.read().await.clone();
    let signature_token = SIGNATURE_TOKEN.read().await.clone();

    let ws_url = format!("ws{}/ws", &api_url[4..]);
    let (client, handle) = WsClient::new(ws_url);

    WS_HANDLE.write().await.replace(handle.clone());

    let mut client = client.with_cancellation_token(token.clone());

    moosicbox_task::spawn("moosicbox_app: ws", async move {
        let mut rx = client.start(client_id, signature_token, {
            let handle = handle.clone();
            move || {
                tauri::async_runtime::spawn({
                    let handle = handle.clone();
                    async move {
                        log::debug!("Sending GetConnectionId");
                        if let Err(e) = send_ws_message(
                            &handle,
                            InboundPayload::GetConnectionId(EmptyPayload {}),
                            true,
                        )
                        .await
                        {
                            log::error!("Failed to send GetConnectionId WS message: {e:?}");
                        }
                        if let Err(e) = flush_ws_message_buffer().await {
                            log::error!("Failed to flush WS message buffer: {e:?}");
                        }
                    }
                });
            }
        });

        while let Some(m) = tokio::select! {
            resp = rx.recv() => {
                resp
            }
            _ = token.cancelled() => {
                None
            }
        } {
            match m {
                WsMessage::TextMessage(message) => {
                    if let Ok(message) = serde_json::from_str::<OutboundPayload>(&message) {
                        if let Err(e) = handle_ws_message(message).await {
                            log::error!("Failed to handle_ws_message: {e:?}");
                        }
                    } else {
                        log::error!("got invalid message: {message}");
                    }
                }
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
            set_playback_quality,
            set_audio_zone_active_players,
            set_state,
            propagate_ws_message,
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
