// @refresh reload
import { produce } from 'solid-js/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { appState, onStartup, onStartupFirst } from '~/services/app';
import {
    Api,
    ApiType,
    Track,
    api,
    apiUrl,
    clientId,
    toSessionPlaylistTrack,
    token,
    trackId,
} from '~/services/api';
import { trackEvent } from '@aptabase/tauri';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import { createPlayer as createSymphoniaPlayer } from '~/symphonia-player';
import {
    registerPlayer,
    setPlayerState,
    updateSessionPartial,
} from '~/services/player';
import * as player from '~/services/player';
import {
    InboundMessageType,
    connectionId,
    connectionName,
    onConnect,
    onConnectionNameChanged,
    onMessage,
    registerConnection,
    updateSession,
} from '~/services/ws';
import { PartialUpdateSession } from '~/services/types';
import { QueryParams } from '~/services/util';

const APTABASE_ENABLED = false;

(async () => {
    await listen('UPDATE_SESSION', async (event) => {
        console.debug('Received UPDATE_SESSION', event);
        const partialUpdate = event.payload as Api.UpdatePlaybackSession;

        const updatePlaybackSession: PartialUpdateSession = {
            ...partialUpdate,
            sessionId: partialUpdate.sessionId,
            playlist: undefined,
        };

        if (partialUpdate.playlist) {
            const libraryTracks = partialUpdate.playlist.tracks.filter(
                ({ type }) => type == 'LIBRARY',
            );

            const libraryIds = libraryTracks.map(({ id }) => id);

            const tidalTracks = partialUpdate.playlist.tracks.filter(
                ({ type }) => type == 'TIDAL',
            );

            const tracks: Track[] = (
                await Promise.all([
                    api.getTracks(libraryIds),
                    ...tidalTracks.map(({ id }) => api.getTidalTrack(id)),
                ])
            ).flat();

            updatePlaybackSession.playlist = {
                ...partialUpdate.playlist,
                sessionPlaylistId: partialUpdate.playlist.sessionPlaylistId,
                tracks: partialUpdate.playlist.tracks.map(
                    ({ id, type }) =>
                        tracks.find(
                            (track) =>
                                track.type === type && trackId(track) === id,
                        )!,
                ),
            };

            partialUpdate.playlist.tracks =
                updatePlaybackSession.playlist.tracks.map(
                    toSessionPlaylistTrack,
                );

            const matchingSession = player.playerState.playbackSessions.find(
                (s) => s.sessionId === updatePlaybackSession.sessionId,
            );

            if (!matchingSession) {
                throw new Error(
                    `Could not find matching session with id ${updatePlaybackSession.sessionId}`,
                );
            }

            updatePlaybackSession.playlist.sessionPlaylistId =
                matchingSession.playlist.sessionPlaylistId;
        } else {
            delete updatePlaybackSession.playlist;
        }

        setPlayerState(
            produce((state) => {
                updateSessionPartial(state, updatePlaybackSession);
            }),
        );
        updateSession(partialUpdate);
    });
})();

function updatePlayers() {
    const connection = appState.connections.find(
        (c) => c.connectionId === connectionId(),
    );

    connection?.players.forEach((player) => {
        const type = player.type as Api.PlayerType | AppPlayerType;
        switch (type) {
            case AppPlayerType.SYMPHONIA:
                registerPlayer(createSymphoniaPlayer(player.playerId));
                break;
            case Api.PlayerType.HOWLER:
                registerPlayer(createHowlerPlayer(player.playerId));
                break;
        }
    });
}

onMessage((data) => {
    switch (data.type) {
        case InboundMessageType.CONNECTIONS:
            updatePlayers();
            break;
    }
});

export enum AppPlayerType {
    SYMPHONIA = 'SYMPHONIA',
}

function updateConnection(connectionId: string, name: string) {
    registerConnection({
        connectionId,
        name,
        players: [
            {
                type: Api.PlayerType.HOWLER,
                name: 'Web Player',
            },
            {
                type: AppPlayerType.SYMPHONIA as unknown as Api.PlayerType,
                name: 'Symphonia Player',
            },
        ],
    });
}

onConnect(() => {
    updateConnection(connectionId()!, connectionName());
});
onConnectionNameChanged((name) => {
    updateConnection(connectionId()!, name);
});

// eslint-disable-next-line @typescript-eslint/no-unused-vars
function apiRequest<T>(
    method: 'get' | 'post',
    url: string,
    query?: QueryParams,
    signal?: AbortSignal,
): Promise<T> {
    // eslint-disable-next-line no-async-promise-executor
    return new Promise(async (resolve, reject) => {
        let cancelled = false;

        signal?.addEventListener('abort', () => {
            cancelled = true;
            reject();
        });

        const headers: Record<string, string> = {};

        const params = new QueryParams(query);
        const clientIdParam = clientId.get();

        if (clientIdParam) {
            params.set('clientId', clientIdParam);
        }

        if (params.size > 0) {
            if (url.indexOf('?') > 0) {
                url += '&';
            } else {
                url += '?';
            }

            url += params.toString();
        }

        const tokenParam = token.get();

        if (tokenParam) {
            headers.Authorization = tokenParam;
        }

        const args: { url: string; headers: Record<string, string> } = {
            url,
            headers,
        };

        const data = await invoke<T>(`api_proxy_${method}`, args);

        if (!cancelled) {
            resolve(data);
        }
    });
}

function circularStringify(obj: object): string {
    const getCircularReplacer = () => {
        const seen = new WeakSet();
        return (_key: string, value: unknown) => {
            if (typeof value === 'object' && value !== null) {
                if (seen.has(value)) {
                    return '[[circular]]';
                }
                seen.add(value);
            }
            return value;
        };
    };

    return JSON.stringify(obj, getCircularReplacer());
}

function objToStr(obj: unknown): string {
    if (typeof obj === 'string') {
        return obj;
    } else if (typeof obj === 'undefined') {
        return 'undefined';
    } else if (obj === null) {
        return 'null';
    } else if (typeof obj === 'object') {
        return circularStringify(obj);
    } else {
        return obj.toString();
    }
}

onStartup(() => {
    if (APTABASE_ENABLED) {
        trackEvent('onStartup');
    }
});

const apiOverride: Partial<ApiType> = {};

const originalApi = { ...api };

function updateApi(secure: boolean) {
    if (secure) {
        Object.assign(api, originalApi);
    } else {
        Object.assign(api, apiOverride);
    }
}

apiUrl.listen((url) => {
    updateApi(url.toLowerCase().startsWith('https://'));
});

onStartupFirst(async () => {
    await invoke('show_main_window');
    updateApi(apiUrl.get().toLowerCase().startsWith('https://'));
    await invoke('set_api_url', { apiUrl: apiUrl.get() });
    if (clientId.get()) {
        await invoke('set_client_id', { clientId: clientId.get() });
    }
    if (Api.signatureToken()) {
        await invoke('set_signature_token', {
            signatureToken: Api.signatureToken(),
        });
    }
    if (token.get()) {
        await invoke('set_api_token', { apiToken: token.get() });
    }

    clientId.listen(async (clientId) => {
        await invoke('set_client_id', { clientId });
    });
    Api.onSignatureTokenUpdated(async (token) => {
        await invoke('set_signature_token', { signatureToken: token });
    });
    token.listen(async (token) => {
        await invoke('set_api_token', { apiToken: token });
    });
    apiUrl.listen(async (url) => {
        await invoke('set_api_url', { apiUrl: url });
    });
});
