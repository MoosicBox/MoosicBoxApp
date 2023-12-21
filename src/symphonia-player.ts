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
        return false;
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
                position: playlistPosition(),
                seek: player.currentSeek(),
                volume: player.volume() / 100,
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

onVolumeChanged((value) => {
    (async () => {
        const sessionId = currentPlaybackSessionId();
        const playbackStatus = await invokePlayer(
            PlayerAction.UPDATE_PLAYBACK,
            {
                volume: value / 100,
                sessionId,
            },
        );

        playbackId = playbackStatus.playbackId;
    })();
});

async function updatePlayback(play: boolean) {
    console.debug('Updating playback', play);

    const sessionId = currentPlaybackSessionId();
    const playbackStatus = await invokePlayer(PlayerAction.UPDATE_PLAYBACK, {
        play,
        position: playlistPosition(),
        seek: player.currentSeek(),
        volume: player.volume() / 100,
        tracks: playlist().map((p) => p.trackId),
        sessionId,
        quality: player.playbackQuality(),
    });

    playbackId = playbackStatus.playbackId;
}

function seek(seek: number) {
    console.debug('Track seeked', seek);
    if (typeof seek === 'number') {
        setCurrentSeek(seek, false);
        console.debug(`Setting seek to ${seek}`);
    }
}

function pause() {
    console.debug('Pausing');
    setPlaying(false);
    (async () => {
        await invokePlayer(PlayerAction.PAUSE);
    })();
}

function previousTrack(): boolean {
    console.debug('Previous track');
    (async () => {
        await invokePlayer(PlayerAction.PREVIOUS_TRACK);
    })();
    return false;
}

function nextTrack(): boolean {
    console.debug('Next track');
    (async () => {
        await invokePlayer(PlayerAction.NEXT_TRACK);
    })();
    return false;
}

function stop() {
    console.debug('Stopping');
    playbackId = undefined;
    setCurrentSeek(undefined);
    setPlayerState({ currentTrack: undefined });
    setCurrentTrackLength(0);
    (async () => {
        await invokePlayer(PlayerAction.STOP_TRACK);
    })();
    console.debug('Track stopped');
}

async function playAlbum(album: Api.Album | Api.Track): Promise<boolean> {
    const versions = await api.getAlbumVersions(album.albumId);
    const tracks = versions[0].tracks;

    return playPlaylist(tracks);
}

function playPlaylist(tracks: Api.Track[]): boolean {
    console.debug('playPlaylist', tracks);
    playbackId = undefined;
    const firstTrack = tracks[0];
    setCurrentAlbum(firstTrack);
    setPlaylistPosition(0);
    setPlaylist(tracks);
    setCurrentSeek(0);
    (async () => {
        await updatePlayback(true);
        setPlaying(true);
    })();
    return true;
}

async function addAlbumToQueue(album: Api.Album | Api.Track) {
    const versions = await api.getAlbumVersions(album.albumId);
    const tracks = versions[0].tracks;

    addTracksToQueue(tracks);
}

async function addTracksToQueue(tracks: Api.Track[]) {
    setPlaylist([...playlist()!, ...tracks], false);
    (async () => {
        const sessionId = currentPlaybackSessionId();
        const playbackStatus = await invokePlayer(
            PlayerAction.UPDATE_PLAYBACK,
            {
                tracks: playlist().map((p) => p.trackId),
                position: playlistPosition(),
                sessionId,
            },
        );

        playbackId = playbackStatus.playbackId;
    })();
}

function removeTrackFromPlaylist(index: number) {
    console.debug('Removing track from playlist', index);
    if (index < playlistPosition()!) {
        setPlaylistPosition(playlistPosition()! - 1, false);
    }
    setPlaylist([...playlist()!.filter((_, i) => i !== index)], false);
    (async () => {
        await invokePlayer(PlayerAction.UPDATE_PLAYBACK, {
            position: playlistPosition(),
            tracks: playlist().map((p) => p.trackId),
        });
    })();
}

function playFromPlaylistPosition(index: number) {
    console.debug('Playing from playlist position', index);
    (async () => {
        await invokePlayer(PlayerAction.UPDATE_PLAYBACK, {
            play: true,
            position: index,
            seek: 0,
            tracks: playlist().map((p) => p.trackId),
        });
    })();
}

function onPositionUpdated(position: number) {
    console.debug('onPositionUpdated', position);
    if (isPlayerActive()) {
        setPlaylistPosition(position, false);
        setCurrentSeek(0, false);
        updatePlayback(true);
    }
}

function onSeekUpdated(seek: number) {
    console.debug('onSeekUpdated', seek);
    if (isPlayerActive()) {
        setCurrentSeek(seek, false);
    }
}

function onPlayingUpdated(updatedPlaying: boolean) {
    console.debug('onPlayingUpdated', updatedPlaying);
    if (isPlayerActive()) {
        if (updatedPlaying) {
            play();
        } else {
            pause();
        }
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
        addTracksToQueue,
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
