import { InvokeArgs, invoke } from '@tauri-apps/api/tauri';
import { Api, api } from './services/api';
import {
    PlayerType,
    playing,
    playlist,
    playlistPosition,
    setCurrentAlbum,
    setCurrentSeek,
    setCurrentTrackLength,
    setPlaying,
    setPlaylist,
    setPlaylistPosition,
    onVolumeChanged,
    setPlayerState,
    currentPlaybackSessionId,
    isPlayerActive,
} from './services/player';
import * as player from './services/player';

onVolumeChanged((_value) => {});

let playbackId: number | undefined;

type PlaybackStatus = { playbackId: number };

enum PlayerAction {
    PLAY = 'player_play',
    PAUSE = 'player_pause',
    RESUME = 'player_resume',
    STOP_TRACK = 'player_stop_track',
    NEXT_TRACK = 'player_next_track',
    PREVIOUS_TRACK = 'player_previous_track',
    UPDATE_PLAYBACK = 'player_update_playback',
}

async function invokePlayer(
    action: PlayerAction,
    args?: InvokeArgs,
): Promise<PlaybackStatus> {
    return (await invoke(action, args)) as PlaybackStatus;
}

function play(): boolean {
    if (playing()) {
        console.debug('Already playing');
        (async () => {
            await updatePlayback();
        })();
        return true;
    }
    if (typeof playbackId !== 'undefined') {
        console.debug('Resuming playback');
        (async () => {
            const playbackStatus = await invokePlayer(PlayerAction.RESUME);

            setPlaying(true, false);
            playbackId = playbackStatus.playbackId;
            console.debug('Playing', playbackStatus);
        })();
    } else {
        console.debug('Starting playback');
        const sessionId = currentPlaybackSessionId();
        if (!sessionId) {
            throw new Error('Failed to get current playback sessions id');
        }
        (async () => {
            const playbackStatus = await invokePlayer(PlayerAction.PLAY, {
                trackIds: playlist()?.map((t) => t.trackId) || [],
                sessionId,
                quality: player.playbackQuality(),
            });

            playbackId = playbackStatus.playbackId;

            setPlaying(true, false);
            console.debug('Playing', playbackStatus);
        })();
    }
    return true;
}

async function updatePlayback() {
    const playbackStatus = await invokePlayer(PlayerAction.UPDATE_PLAYBACK, {
        position: playlistPosition(),
        seek: 0,
        tracks: playlist().map((p) => p.trackId),
    });

    playbackId = playbackStatus.playbackId;
}

function seek(seek: number) {
    console.debug('Track seeked');
    if (typeof seek === 'number') {
        setCurrentSeek(seek, false);
        console.debug(`Setting seek to ${seek}`);
    }
}

function pause() {
    setPlaying(false);
    (async () => {
        await invokePlayer(PlayerAction.PAUSE);
    })();
}

function previousTrack(): boolean {
    (async () => {
        await invokePlayer(PlayerAction.PREVIOUS_TRACK);
    })();
    return false;
}

function nextTrack(): boolean {
    (async () => {
        await invokePlayer(PlayerAction.NEXT_TRACK);
    })();
    return false;
}

function stop() {
    playbackId = undefined;
    setCurrentSeek(undefined);
    setPlayerState({ currentTrack: undefined });
    setCurrentTrackLength(0);
    (async () => {
        await invokePlayer(PlayerAction.STOP_TRACK);
    })();
    console.debug('Track stopped');
    console.trace();
}

async function playAlbum(album: Api.Album | Api.Track): Promise<boolean> {
    const tracks = await api.getAlbumTracks(album.albumId);
    return playPlaylist(tracks);
}

function playPlaylist(tracks: Api.Track[]): boolean {
    playbackId = undefined;
    const firstTrack = tracks[0];
    setCurrentAlbum(firstTrack);
    setPlaylistPosition(0);
    setPlaylist(tracks);
    setCurrentSeek(0);
    return player.play()!;
}

async function addAlbumToQueue(album: Api.Album | Api.Track) {
    const tracks = await api.getAlbumTracks(album.albumId);

    setPlaylist([...playlist()!, ...tracks]);
}

function removeTrackFromPlaylist(index: number) {
    console.debug('Removing track from playlist', index);
    if (index < playlistPosition()!) {
        setPlaylistPosition(playlistPosition()! - 1);
    }
    setPlaylist([...playlist()!.filter((_, i) => i !== index)]);
}

function playFromPlaylistPosition(index: number) {
    console.debug('Playing from playlist position', index);
    (async () => {
        await invokePlayer(PlayerAction.UPDATE_PLAYBACK, {
            position: index,
            seek: 0,
            tracks: playlist().map((p) => p.trackId),
        });
    })();
}

function onPositionUpdated(position: number) {
    setPlaylistPosition(position, false);
    setCurrentSeek(0, false);
}

function onSeekUpdated(seek: number) {
    if (isPlayerActive()) {
        setCurrentSeek(seek, false);
    }
}

function onPlayingUpdated(updatedPlaying: boolean) {
    if (isPlayerActive()) {
        setPlaying(updatedPlaying);
    }
}

export function createPlayer(id: number): PlayerType {
    return {
        id,
        play,
        playAlbum,
        playPlaylist,
        playFromPlaylistPosition,
        addAlbumToQueue,
        removeTrackFromPlaylist,
        pause,
        stop,
        seek,
        previousTrack,
        nextTrack,
        onPositionUpdated,
        onSeekUpdated,
        onPlayingUpdated,
    };
}
