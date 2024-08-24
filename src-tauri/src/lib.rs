use std::{
    collections::HashMap,
    env,
    sync::{Arc, LazyLock, OnceLock},
};

use async_recursion::async_recursion;
use log::info;
use moosicbox_app_ws::{WebsocketSendError, WebsocketSender as _, WsClient, WsHandle, WsMessage};
use moosicbox_audio_output::{AudioOutputError, AudioOutputFactory, AudioOutputScannerError};
use moosicbox_audio_zone::models::{ApiAudioZoneWithSession, ApiPlayer};
use moosicbox_core::{
    sqlite::models::{ApiSource, Id},
    types::PlaybackQuality,
};
use moosicbox_music_api::{FromId, MusicApi, MusicApisError, SourceToMusicApi};
use moosicbox_paging::Page;
use moosicbox_player::{
    local::LocalPlayer, Playback, PlaybackHandler, PlaybackRetryOptions, PlaybackType, PlayerError,
    PlayerSource, Track,
};
use moosicbox_remote_library::RemoteLibraryMusicApi;
use moosicbox_session::models::{
    ApiConnection, ApiPlaybackTarget, ApiSession, ApiUpdateSession, ApiUpdateSessionPlaylist,
    PlaybackTarget, RegisterPlayer, UpdateSession, UpdateSessionPlaylistTrack,
};
use moosicbox_upnp::{
    listener::Handle, player::UpnpAvTransportService, Device, Service, UpnpDeviceScannerError,
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

type ApiPlayersMap = HashMap<u64, Vec<(ApiPlayer, PlayerType, AudioOutputFactory)>>;

struct PlaybackTargetSessionPlayer {
    playback_target: ApiPlaybackTarget,
    session_id: u64,
    player: PlaybackHandler,
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
static ACTIVE_PLAYERS: LazyLock<Arc<RwLock<Vec<PlaybackTargetSessionPlayer>>>> =
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
static CURRENT_SESSION_ID: LazyLock<Arc<RwLock<Option<u64>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));
static CURRENT_AUDIO_ZONES: LazyLock<Arc<RwLock<Vec<ApiAudioZoneWithSession>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
#[allow(clippy::type_complexity)]
static CURRENT_PLAYERS: LazyLock<Arc<RwLock<Vec<(ApiPlayer, PlayerType, AudioOutputFactory)>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(vec![])));
static UPNP_AV_TRANSPORT_SERVICES: LazyLock<
    tokio::sync::RwLock<Vec<moosicbox_upnp::player::UpnpAvTransportService>>,
> = LazyLock::new(|| tokio::sync::RwLock::new(vec![]));

const DEFAULT_PLAYBACK_RETRY_OPTIONS: PlaybackRetryOptions = PlaybackRetryOptions {
    max_attempts: 10,
    retry_delay: std::time::Duration::from_millis(1000),
};

#[derive(Clone)]
enum PlayerType {
    Local,
    Upnp {
        source_to_music_api: Arc<Box<dyn SourceToMusicApi + Send + Sync>>,
        device: Device,
        service: Service,
        handle: Handle,
    },
}

async fn new_player(
    session_id: u64,
    playback_target: ApiPlaybackTarget,
    output: AudioOutputFactory,
    player_type: PlayerType,
) -> Result<PlaybackHandler, TauriPlayerError> {
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

    let host = API_URL
        .read()
        .await
        .clone()
        .ok_or_else(|| TauriPlayerError::Unknown("API_URL not set".to_string()))?;

    let player_source = PlayerSource::Remote {
        host: host.clone(),
        headers,
        query,
    };

    let mut player = match player_type {
        PlayerType::Local => {
            let local_player = LocalPlayer::new(player_source, Some(PlaybackType::Stream))
                .await
                .map_err(|e| {
                    TauriPlayerError::Unknown(format!(
                        "Failed to initialize new local player: {e:?}"
                    ))
                })?
                .with_output(output.clone());

            let playback = local_player.playback.clone();
            let receiver = local_player.receiver.clone();

            let handler = PlaybackHandler::new(local_player.clone())
                .with_playback(playback)
                .with_output(Some(Arc::new(std::sync::Mutex::new(output))))
                .with_receiver(receiver);

            local_player
                .playback_handler
                .write()
                .unwrap()
                .replace(handler.clone());

            handler
        }
        PlayerType::Upnp {
            source_to_music_api,
            device,
            service,
            handle,
        } => {
            let upnp_player = moosicbox_upnp::player::UpnpPlayer::new(
                source_to_music_api,
                device,
                service,
                player_source,
                handle,
            );

            let playback = upnp_player.playback.clone();
            let receiver = upnp_player.receiver.clone();

            let handler = PlaybackHandler::new(upnp_player.clone())
                .with_playback(playback)
                .with_output(Some(Arc::new(std::sync::Mutex::new(output))))
                .with_receiver(receiver);

            upnp_player
                .playback_handler
                .write()
                .unwrap()
                .replace(handler.clone());

            handler
        }
    };

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
    current_session_id: Option<u64>,
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

    {
        *CURRENT_SESSION_ID.write().await = state.current_session_id;
    }

    if state.current_session_id.is_some() {
        update_playlist()
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;
    }

    if updated_connection_details {
        scan_outputs()
            .await
            .map_err(|e| TauriPlayerError::Unknown(e.to_string()))?;

        init_upnp_players()
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
    let mut players_map = ACTIVE_PLAYERS.write().await;
    let ids = {
        players_map
            .iter()
            .map(|x| (x.playback_target.clone(), x.session_id, x.player.clone()))
            .collect::<Vec<_>>()
    };

    for (i, (playback_target, session_id, player)) in ids.into_iter().enumerate() {
        let output = player.output.as_ref().unwrap().lock().unwrap().clone();
        log::debug!("reinit_players: playback_target={playback_target:?} session_id={session_id} output={output:?}");
        let mut created_player = new_player(
            session_id,
            playback_target.clone(),
            output,
            PlayerType::Local,
        )
        .await?;

        let playback = player.playback.read().unwrap().clone();

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

async fn set_audio_zone_active_players(
    session_id: u64,
    audio_zone_id: u64,
    players: Vec<(ApiPlayer, PlayerType, AudioOutputFactory)>,
) -> Result<(), TauriPlayerError> {
    log::debug!("Setting audio_zone active players: session_id={session_id} audio_zone_id={audio_zone_id} {:?}", players.iter().map(|(x, _, _)| x).collect::<Vec<_>>());

    let mut api_players_map = AUDIO_ZONE_ACTIVE_API_PLAYERS.write().await;
    api_players_map.insert(audio_zone_id, players.clone());

    {
        let mut players_map = ACTIVE_PLAYERS.write().await;
        for (player, ptype, output) in players.iter() {
            if let Some(existing) = players_map.iter().find(|x| match x.playback_target {
                ApiPlaybackTarget::AudioZone { audio_zone_id: id } => id == audio_zone_id,
                _ => false,
            }) {
                let different_session = {
                    !existing
                        .player
                        .playback
                        .read()
                        .unwrap()
                        .as_ref()
                        .is_some_and(|p| p.session_id.is_some_and(|s| s == session_id))
                };

                let same_output = existing
                    .player
                    .output
                    .as_ref()
                    .is_some_and(|output| output.lock().unwrap().id == player.audio_output_id);

                if !different_session && same_output {
                    log::debug!(
                        "Skipping existing player for audio_zone_id={audio_zone_id} audio_output_id={}",
                        player.audio_output_id
                    );
                    continue;
                }
            }

            let playback_target = ApiPlaybackTarget::AudioZone { audio_zone_id };
            let player = new_player(
                session_id,
                playback_target.clone(),
                output.clone(),
                ptype.clone(),
            )
            .await?;
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
    let players: &[(ApiPlayer, PlayerType, AudioOutputFactory)] = players_binding.as_ref();

    log::debug!(
        "\
        Updating audio zones\n\t\
        audio_zones={audio_zones:?}\n\t\
        players={:?}\n\t\
        ",
        players.iter().map(|(x, _, _)| x).collect::<Vec<_>>()
    );

    for audio_zone in audio_zones {
        let players = audio_zone
            .players
            .clone()
            .into_iter()
            .filter_map(|x| {
                players
                    .iter()
                    .find(|(p, _, _)| p.player_id == x.player_id)
                    .map(|(_, ptype, output)| (x, ptype.clone(), output.clone()))
            })
            .collect::<Vec<_>>();

        if !players.is_empty() {
            set_audio_zone_active_players(audio_zone.session_id, audio_zone.id, players).await?;
        }
    }
    Ok(())
}

#[async_recursion]
async fn get_players(
    session_id: u64,
    playback_target: &ApiPlaybackTarget,
    recursed: bool,
) -> Result<Vec<PlaybackHandler>, TauriPlayerError> {
    let players = {
        let mut playback_handlers = vec![];
        let active_players = ACTIVE_PLAYERS.read().await;

        for player in active_players.iter() {
            let target = &player.playback_target;
            log::trace!(
                "get_players: Checking if player is in session: target={target:?} session_id={session_id} player_zone_id={playback_target:?}",
            );
            if !player.player.playback
                .read()
                .unwrap()
                .as_ref()
                .is_some_and(|p| p.session_id.is_some_and(|s| {
                    log::trace!(
                        "get_players: player playback.session_id={s} target session_id={session_id}",
                    );
                    s == session_id
                })) {
                continue;
            }
            log::trace!(
                "get_players: Checking if player is in zone: target={target:?} session_id={session_id} player_zone_id={playback_target:?}",
            );
            if target != playback_target {
                continue;
            }

            playback_handlers.push(player.player.clone());
        }
        playback_handlers
    };

    if recursed || !players.is_empty() {
        return Ok(players);
    }

    match playback_target {
        ApiPlaybackTarget::AudioZone { audio_zone_id } => {
            log::debug!("get_players: ApiPlaybackTarget::AudioZone audio_zone_id={audio_zone_id}");
            let audio_zone = {
                let binding = CURRENT_AUDIO_ZONES.read().await;
                let audio_zones: &[ApiAudioZoneWithSession] = binding.as_ref();
                audio_zones.iter().find(|x| x.id == *audio_zone_id).cloned()
            };
            if let Some(audio_zone) = audio_zone {
                let audio_zone_with_session = ApiAudioZoneWithSession {
                    id: *audio_zone_id,
                    session_id,
                    name: audio_zone.name,
                    players: audio_zone.players,
                };
                CURRENT_AUDIO_ZONES
                    .write()
                    .await
                    .push(audio_zone_with_session);
                update_audio_zones().await?;
                return get_players(session_id, playback_target, true).await;
            }
        }
        ApiPlaybackTarget::ConnectionOutput {
            connection_id,
            output_id,
        } => {
            log::debug!("get_players: ApiPlaybackTarget::ConnectionOutput connection_id={connection_id} output_id={output_id}");

            let same_connection = {
                CONNECTION_ID
                    .read()
                    .await
                    .as_ref()
                    .is_some_and(|x| x == connection_id)
            };

            if same_connection {
                log::debug!("get_players: ApiPlaybackTarget::ConnectionOutput same connection");

                let binding = CURRENT_PLAYERS.read().await;
                let current_players: &[(ApiPlayer, PlayerType, AudioOutputFactory)] =
                    binding.as_ref();

                if let Some((_, ptype, output)) = current_players
                    .iter()
                    .find(|(x, _, _)| {
                        log::trace!("get_players: ApiPlaybackTarget::ConnectionOutput checking '{}' == '{output_id}'", x.audio_output_id);
                        &x.audio_output_id == output_id
                    })
                {
                    log::debug!("get_players: ApiPlaybackTarget::ConnectionOutput creating player for output_id={output_id} session_id={session_id} playback_target={playback_target:?}");

                    let player = new_player(
                        session_id,
                        playback_target.clone(),
                        output.clone(),
                        ptype.clone(),
                    )
                    .await?;
                    log::debug!(
                        "get_players: ApiPlaybackTarget::ConnectionOutput created new player={}",
                        player.id
                    );
                    ACTIVE_PLAYERS
                        .write()
                        .await
                        .push(PlaybackTargetSessionPlayer {
                            playback_target: playback_target.clone(),
                            session_id,
                            player: player.clone(),
                        });
                    return Ok(vec![player]);
                }
            }
        }
    }

    Ok(players)
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

    let mut binding = ACTIVE_PLAYERS.write().await;
    let players = binding.iter_mut();

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
    log::debug!("send_ws_message: handle_update={handle_update} message={message:?}");

    if handle_update {
        let message = message.clone();
        moosicbox_task::spawn("send_ws_message: handle_update", async move {
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

            Ok::<_, HandleWsMessageError>(())
        });
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
    moosicbox_logging::debug_or_trace!(
        ("propagate_ws_message: received ws message from frontend: {message}"),
        ("propagate_ws_message: received ws message from frontend: {message:?}")
    );

    moosicbox_task::spawn("propagate_ws_message", async move {
        let handle = { WS_HANDLE.read().await.clone() };

        if let Some(handle) = handle {
            send_ws_message(&handle, message, true).await?;
        } else {
            moosicbox_logging::debug_or_trace!(
                ("propagate_ws_message: pushing message to buffer: {message}"),
                ("propagate_ws_message: pushing message to buffer: {message:?}")
            );
            WS_MESSAGE_BUFFER.write().await.push(message);
        }

        Ok::<_, SendWsMessageError>(())
    });

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
        API_URL
            .read()
            .await
            .clone()
            .ok_or_else(|| TauriPlayerError::Unknown(format!("API_URL not set ({url})")))?
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
            .ok_or_else(|| TauriPlayerError::Unknown(format!("API_URL not set ({url})")))?
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
    #[error(transparent)]
    RegisterPlayers(#[from] RegisterPlayersError),
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
        .iter()
        .map(|x| RegisterPlayer {
            audio_output_id: x.id.clone(),
            name: x.name.clone(),
        })
        .collect::<Vec<_>>();

    let players = register_players(&players).await?;

    log::debug!("scan_outputs: players={players:?}");

    let players = players
        .into_iter()
        .filter_map(|p| {
            outputs
                .iter()
                .find(|output| output.id == p.audio_output_id)
                .map(|output| (p, PlayerType::Local, output.clone()))
        })
        .collect::<Vec<_>>();

    add_players_to_current_players(players).await;

    update_audio_zones().await?;

    Ok(())
}

async fn add_players_to_current_players(players: Vec<(ApiPlayer, PlayerType, AudioOutputFactory)>) {
    let mut existing_players = CURRENT_PLAYERS.write().await;

    let new_players = players
        .into_iter()
        .filter(|(p, _, _)| {
            !existing_players
                .iter()
                .any(|(existing, _, _)| existing.player_id == p.player_id)
        })
        .collect::<Vec<_>>();

    log::debug!(
        "add_players_to_current_players: Adding new_players={:?}",
        new_players.iter().map(|(x, _, _)| x).collect::<Vec<_>>()
    );

    existing_players.extend(new_players);
}

#[derive(Debug, Error)]
pub enum RegisterPlayersError {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    TauriPlayer(#[from] TauriPlayerError),
}

async fn register_players(
    players: &[RegisterPlayer],
) -> Result<Vec<ApiPlayer>, RegisterPlayersError> {
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

    Ok(serde_json::from_value(response)?)
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
    player: &PlaybackHandler,
) -> ApiUpdateSession {
    let session_id = {
        player
            .playback
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

    let current_session_id = { *CURRENT_SESSION_ID.read().await };

    if current_session_id.is_some_and(|id| update.session_id == id) {
        if let Some((url, query)) = get_url_and_query().await {
            use tauri_plugin_player::PlayerExt;

            if let Err(e) =
                APP.get()
                    .unwrap()
                    .player()
                    .update_state(tauri_plugin_player::UpdateState {
                        playing: update.playing,
                        position: update.position,
                        seek: update.seek,
                        volume: update.volume,
                        playlist: update
                            .playlist
                            .as_ref()
                            .map(|x| tauri_plugin_player::Playlist {
                                tracks: x
                                    .tracks
                                    .iter()
                                    .filter_map(|x| convert_track(x.clone(), &url, &query))
                                    .collect::<Vec<_>>(),
                            }),
                    })
            {
                log::debug!("Failed to update_state: {e:?}");
            }
        }
    }

    let players = get_players(update.session_id, &update.playback_target, false).await?;

    for mut player in players {
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

fn album_cover_url(album_id: &str, source: ApiSource, url: &str, query: &str) -> String {
    format!("{url}/files/albums/{album_id}/300x300?source={source}{query}")
}

fn convert_track(
    value: moosicbox_library::models::ApiTrack,
    url: &str,
    query: &str,
) -> Option<tauri_plugin_player::Track> {
    let api_source = value.api_source();

    match value {
        moosicbox_library::models::ApiTrack::Library { track_id, data } => {
            let album_cover = if data.contains_cover {
                Some(album_cover_url(&data.album_id.as_string(), api_source, url, query))
            } else {
                None
            };
            Some(tauri_plugin_player::Track {
                id: track_id.to_string(),
                title: data.title,
                album: data.album,
                album_cover,
                artist: data.artist,
                artist_cover: None,
                duration: data.duration
            })
        }
        _ => {
            value.data().map(|x| {
                let album_id = x
                    .get("albumId")
                    .and_then(|x| {
                        if x.is_string() {
                            x.as_str().map(|x| x.to_string())
                        } else if x.is_number() {
                            x.as_u64().map(|x| x.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let contains_cover = x
                    .get("containsCover")
                    .and_then(|x| x.as_bool())
                    .unwrap_or_default();

                let album_cover = if contains_cover {
                    Some(album_cover_url(&album_id, api_source, url, query))
                } else {
                    None
                };

                log::trace!("handle_ws_message: Converting track data={x} contains_cover={contains_cover} album_cover={album_cover:?}");

                tauri_plugin_player::Track {
                    id: value.track_id().to_string(),
                    title: x
                        .get("title")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    album: x
                        .get("album")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    album_cover,
                    artist: x
                        .get("artist")
                        .and_then(|x| x.as_str().map(|x| x.to_string()))
                        .unwrap_or_default(),
                    artist_cover: x
                        .get("artistCover")
                        .and_then(|x| x.as_str().map(|x| x.to_string())),
                    duration: x
                        .get("duration")
                        .and_then(|x| x.as_f64())
                        .unwrap_or_default(),
                }
            })
        }
    }
}

async fn get_url_and_query() -> Option<(String, String)> {
    let url_string = { API_URL.read().await.clone() };
    let Some(url) = url_string else {
        return None;
    };

    let mut query = String::new();
    if let Some(client_id) = CLIENT_ID.read().await.clone() {
        query.push_str(&format!("&clientId={client_id}"));
    }
    if let Some(signature_token) = SIGNATURE_TOKEN.read().await.clone() {
        query.push_str(&format!("&signature={signature_token}"));
    }

    Some((url, query))
}

async fn update_playlist() -> Result<(), HandleWsMessageError> {
    use tauri_plugin_player::PlayerExt;

    log::trace!("update_playlist");

    let current_session_id = { *CURRENT_SESSION_ID.read().await };
    let Some(current_session_id) = current_session_id else {
        return Ok(());
    };

    let Some(session) = ({
        let binding = CURRENT_SESSIONS.read().await;
        let sessions: &[ApiSession] = &binding;
        sessions
            .iter()
            .find(|x| x.session_id == current_session_id)
            .cloned()
    }) else {
        return Ok(());
    };

    log::debug!("update_playlist: session={session:?}");

    let Some((url, query)) = get_url_and_query().await else {
        return Ok(());
    };

    match APP
        .get()
        .unwrap()
        .player()
        .update_state(tauri_plugin_player::UpdateState {
            playing: Some(session.playing),
            position: session.position,
            seek: session.seek.map(|x| x as f64),
            volume: session.volume,
            playlist: Some(tauri_plugin_player::Playlist {
                tracks: session
                    .playlist
                    .tracks
                    .into_iter()
                    .filter_map(|x| convert_track(x, &url, &query))
                    .collect::<Vec<_>>(),
            }),
        }) {
        Ok(_resp) => {
            log::debug!("Successfully set state");
        }
        Err(e) => {
            log::error!("Failed to set state: {e:?}");
        }
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
    log::debug!("handle_ws_message: {message:?}");
    moosicbox_task::spawn("handle_ws_message", {
        let message = message.clone();
        async move {
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
                                if let Some(player) = ACTIVE_PLAYERS
                                    .write()
                                    .await
                                    .iter_mut()
                                    .find(|x| x.player.id as u64 == player_id)
                                    .map(|x| &mut x.player)
                                {
                                    log::debug!(
                                "handle_ws_message: init_from_api_session session={session:?}"
                            );
                                    if let Err(e) =
                                        player.init_from_api_session(session.clone()).await
                                    {
                                        log::error!(
                                            "Failed to init player from api session: {e:?}"
                                        );
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
                    update_playlist().await?;
                }

                OutboundPayload::AudioZoneWithSessions(payload) => {
                    *CURRENT_AUDIO_ZONES.write().await = payload.payload.clone();

                    update_audio_zones().await?;
                }
                _ => {}
            }

            Ok::<_, HandleWsMessageError>(())
        }
    });

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

pub struct SourceToRemoteLibrary {
    host: String,
}

impl SourceToMusicApi for SourceToRemoteLibrary {
    fn get(&self, source: ApiSource) -> Result<Arc<Box<dyn MusicApi>>, MusicApisError> {
        Ok(Arc::new(Box::new(RemoteLibraryMusicApi::new(
            self.host.to_owned(),
            source,
        ))))
    }
}

static UPNP_LISTENER_HANDLE: OnceLock<moosicbox_upnp::listener::Handle> = OnceLock::new();

#[derive(Debug, Error)]
pub enum InitUpnpError {
    #[error(transparent)]
    UpnpDeviceScanner(#[from] UpnpDeviceScannerError),
    #[error(transparent)]
    TauriPlayer(#[from] TauriPlayerError),
    #[error(transparent)]
    AudioOutput(#[from] AudioOutputError),
    #[error(transparent)]
    RegisterPlayers(#[from] RegisterPlayersError),
}

async fn init_upnp_players() -> Result<(), InitUpnpError> {
    moosicbox_upnp::scan_devices().await?;

    let services = {
        let mut av_transport_services = UPNP_AV_TRANSPORT_SERVICES.write().await;
        av_transport_services.clear();

        for device in moosicbox_upnp::devices().await {
            let service_id = "urn:upnp-org:serviceId:AVTransport";
            if let Ok((device, service)) =
                moosicbox_upnp::get_device_and_service(&device.udn, service_id)
            {
                av_transport_services.push(UpnpAvTransportService { device, service });
            }
        }

        av_transport_services.clone()
    };

    let mut outputs = Vec::with_capacity(services.len());
    services
        .iter()
        .map(|x| x.clone().try_into())
        .collect::<Result<Vec<AudioOutputFactory>, AudioOutputError>>()?;

    let url_string = { API_URL.read().await.clone() };
    let url = url_string.as_deref();

    let Some(url) = url else {
        return Ok(());
    };

    for service in services.into_iter() {
        let player_type = PlayerType::Upnp {
            source_to_music_api: Arc::new(Box::new(SourceToRemoteLibrary {
                host: url.to_owned(),
            })),
            device: service.device.clone(),
            service: service.service.clone(),
            handle: UPNP_LISTENER_HANDLE.get().unwrap().clone(),
        };
        let output: AudioOutputFactory = service.try_into()?;

        outputs.push((output, player_type));
    }

    let register_players_payload = outputs
        .iter()
        .map(|(x, _)| RegisterPlayer {
            audio_output_id: x.id.clone(),
            name: x.name.clone(),
        })
        .collect::<Vec<_>>();

    let api_players = register_players(&register_players_payload).await?;

    log::debug!("init_upnp_players: players={api_players:?}");

    let api_players = api_players
        .into_iter()
        .filter_map(|p| {
            if let Some((output, ptype)) = outputs
                .iter()
                .find(|(output, _ptype)| output.id == p.audio_output_id)
            {
                Some((p, ptype.clone(), output.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    add_players_to_current_players(api_players).await;

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

    let upnp_service =
        moosicbox_upnp::listener::Service::new(moosicbox_upnp::listener::UpnpContext::new());

    let join_upnp_service = tauri::async_runtime::spawn(async {
        let upnp_service_handle = upnp_service.handle();
        let join_upnp_service = upnp_service.start();

        UPNP_LISTENER_HANDLE
            .set(upnp_service_handle.clone())
            .unwrap_or_else(|_| panic!("Failed to set UPNP_LISTENER_HANDLE"));

        join_upnp_service.await
    });

    #[allow(unused_mut)]
    let mut app_builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_player::init())
        .setup(|app| {
            APP.get_or_init(|| app.handle().clone());

            #[cfg(target_os = "android")]
            {
                use tauri_plugin_notification::{NotificationExt as _, PermissionState};

                let state = app.notification().permission_state()?;

                if state != PermissionState::Denied && state != PermissionState::Granted {
                    app.notification().request_permission()?;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            #[cfg(not(all(target_os = "android")))]
            show_main_window,
            set_playback_quality,
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
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_handle, event| {
            log::trace!("event: {event:?}");
            match event {
                tauri::RunEvent::Exit { .. } => {}
                tauri::RunEvent::ExitRequested { .. } => {}
                tauri::RunEvent::WindowEvent { .. } => {}
                tauri::RunEvent::Ready => {}
                tauri::RunEvent::Resumed => {}
                tauri::RunEvent::MainEventsCleared => {}
                _ => {}
            }
        });

    if let Err(e) = tauri::async_runtime::block_on(join_upnp_service) {
        log::error!("Failed to join UPnP service: {e:?}");
    }
}
