import { InvokeArgs, invoke } from '@tauri-apps/api/tauri';
import { Api } from './services/api';
import { PlayerType, currentPlaybackSessionId } from './services/player';
import * as player from './services/player';
import { orderedEntries } from './services/util';

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

type UpdatePlayback = {
    sessionId: number;
    play?: boolean;
    stop?: boolean;
    playing?: boolean;
    quality?: Api.PlaybackQuality;
    position?: number;
    seek?: number;
    volume?: number;
    tracks?: number[];
};

async function invokePlayer(
    action: PlayerAction,
    args?: InvokeArgs,
): Promise<PlaybackStatus> {
    console.debug('invokePlayer', action, args);
    return (await invoke(action, args)) as PlaybackStatus;
}

async function updatePlayback(update: player.PlaybackUpdate): Promise<void> {
    console.debug('Received updatePlayback', update);

    let actions = {
        update: false,
    };

    const handler = {
        set<T = UpdatePlayback>(
            target: T,
            prop: keyof T,
            value: T[typeof prop],
        ): boolean {
            const existing = target[prop];

            if (existing !== value) {
                target[prop] = value;
                actions.update = true;
            }

            return true;
        },
    };

    const updatePlayback = new Proxy<UpdatePlayback>(
        {
            sessionId: update.sessionId,
        },
        handler,
    );

    const updates = orderedEntries(update, [
        'stop',
        'play',
        'tracks',
        'position',
        'volume',
        'seek',
        'playing',
        'quality',
    ]);

    for (const [key, value] of updates) {
        if (typeof value === 'undefined') continue;

        switch (key) {
            case 'stop':
                updatePlayback.stop = value;
                break;
            case 'play':
                updatePlayback.play = value;
                break;
            case 'tracks':
                updatePlayback.tracks = value.map(({ trackId }) => trackId);
                break;
            case 'position':
                updatePlayback.position = value;
                break;
            case 'volume':
                updatePlayback.volume = value / 100;
                break;
            case 'seek':
                if (!updatePlayback.play) continue;
                updatePlayback.seek = value;
                break;
            case 'playing':
                updatePlayback.playing = value;
                break;
            case 'quality':
                updatePlayback.quality = value;
                break;
            case 'sessionId':
                break;
            default:
                key satisfies never;
        }
    }

    if (actions.update) {
        const playbackStatus = await invokePlayer(
            PlayerAction.UPDATE_PLAYBACK,
            updatePlayback,
        );

        playbackId = playbackStatus.playbackId;
    }
}

export function createPlayer(id: number): PlayerType {
    return {
        id,
        async activate() {
            const update: UpdatePlayback = {
                tracks: player.playlist().map(({ trackId }) => trackId),
                position: player.playlistPosition(),
                seek: player.currentSeek(),
                volume: player.volume() / 100,
                sessionId: currentPlaybackSessionId()!,
                quality: player.playbackQuality(),
            };
            await invokePlayer(PlayerAction.UPDATE_PLAYBACK, update);
        },
        updatePlayback,
    };
}
