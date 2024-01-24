// @refresh reload
import { Routes } from '@solidjs/router';
import { Suspense } from 'solid-js';
import { produce } from 'solid-js/store';
import {
    Body,
    FileRoutes,
    Head,
    Html,
    Meta,
    Scripts,
    Title,
} from 'solid-start';
import { ErrorBoundary } from 'solid-start/error-boundary';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { appState, onStartup, onStartupFirst } from './services/app';
import { Api, ApiType, Track, api, trackId } from './services/api';
import { attachConsole, debug, error, info, warn } from 'tauri-plugin-log-api';
import { trackEvent } from '@aptabase/tauri';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import { createPlayer as createSymphoniaPlayer } from '~/symphonia-player';
import {
    registerPlayer,
    setPlayerState,
    updateSessionPartial,
} from './services/player';
import * as player from './services/player';
import {
    InboundMessageType,
    connectionId,
    connectionName,
    onConnect,
    onConnectionNameChanged,
    onMessage,
    registerConnection,
    updateSession,
} from './services/ws';
import { PartialUpdateSession } from './services/types';
import { QueryParams } from './services/util';

const APTABASE_ENABLED = false;

(async () => {
    await listen('UPDATE_SESSION', async (event) => {
        console.debug('Received UPDATE_SESSION', event);
        const partialUpdate = event.payload as Api.UpdatePlaybackSession;

        updateSession(partialUpdate);

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
        const clientId = Api.clientId();

        if (clientId) {
            params.set('clientId', clientId);
        }

        if (params.size > 0) {
            if (url.indexOf('?') > 0) {
                url += '&';
            } else {
                url += '?';
            }

            url += params.toString();
        }

        const token = Api.token();

        if (token) {
            headers.Authorization = token;
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

attachConsole();

function circularStringify(obj: object): string {
    const getCircularReplacer = () => {
        const seen = new WeakSet();
        return (_key: string, value: any) => {
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

console.debug = (...args) => {
    debug(args.map(objToStr).join(' '));
};

console.log = (...args) => {
    info(args.map(objToStr).join(' '));
};

console.warn = (...args) => {
    if (APTABASE_ENABLED) {
        trackEvent('warn', { args: args.map(circularStringify).join(', ') });
    }
    warn(args.map(objToStr).join(' '));
};

console.error = (...args) => {
    if (APTABASE_ENABLED) {
        trackEvent('error', { args: args.map(circularStringify).join(', ') });
    }
    error(args.map(objToStr).join(' '));
};

onStartup(() => {
    if (APTABASE_ENABLED) {
        trackEvent('onStartup');
    }
});

const apiOverride: Partial<ApiType> = {
    async getArtist(artistId, signal) {
        const query = new QueryParams({
            artistId: `${artistId}`,
        });

        return apiRequest('get', 'artist', query, signal);
    },
    async getArtistAlbums(artistId, signal) {
        const query = new QueryParams({
            artistId: `${artistId}`,
        });

        return apiRequest('get', 'artist/albums', query, signal);
    },
    async getAlbum(albumId, signal) {
        const query = new QueryParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album', query, signal);
    },
    async getAlbums(request, signal) {
        const query = new QueryParams();
        if (request?.sources) query.set('sources', request.sources.join(','));
        if (request?.sort) query.set('sort', request.sort);
        if (request?.filters?.search)
            query.set('search', request.filters.search);

        return apiRequest('get', 'albums', query, signal);
    },
    async getAlbumTracks(albumId, signal) {
        const query = new QueryParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album/tracks', query, signal);
    },
    async getAlbumVersions(albumId, signal) {
        const query = new QueryParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album/versions', query, signal);
    },
    async getTracks(trackIds, signal) {
        const query = new QueryParams({
            trackIds: `${trackIds.join(',')}`,
        });

        return apiRequest('get', 'tracks', query, signal);
    },
    async getArtists(request, signal) {
        const query = new QueryParams();
        if (request?.sources) query.set('sources', request.sources.join(','));
        if (request?.sort) query.set('sort', request.sort);
        if (request?.filters?.search)
            query.set('search', request.filters.search);

        return apiRequest('get', 'artists', query, signal);
    },
    async validateSignatureTokenAndClient(signature, signal) {
        const response = await apiRequest(
            'post',
            `auth/validate-signature-token?signature=${signature}`,
            new QueryParams(),
            signal,
        );

        return (
            typeof response === 'object' &&
            response !== null &&
            'valid' in response &&
            response.valid === true
        );
    },
    fetchSignatureToken: function (signal): Promise<string | undefined> {
        return apiRequest(
            'post',
            'auth/signature-token',
            new QueryParams(),
            signal,
        );
    },
    magicToken: function (
        magicToken: string,
        signal,
    ): Promise<{ clientId: string; accessToken: string }> {
        return apiRequest(
            'post',
            'auth/magic-token',
            new QueryParams({ magicToken }),
            signal,
        );
    },
};

const originalApi = { ...api };

function updateApi(secure: boolean) {
    if (secure) {
        Object.assign(api, originalApi);
    } else {
        Object.assign(api, apiOverride);
    }
}

Api.onApiUrlUpdated((url) => {
    updateApi(url.toLowerCase().startsWith('https://'));
});

export default function Root() {
    onStartupFirst(async () => {
        await invoke('show_main_window');
        updateApi(Api.apiUrl().toLowerCase().startsWith('https://'));
        await invoke('set_api_url', { apiUrl: Api.apiUrl() });
        if (Api.clientId()) {
            await invoke('set_client_id', { clientId: Api.clientId() });
        }
        if (Api.signatureToken()) {
            await invoke('set_signature_token', {
                signatureToken: Api.signatureToken(),
            });
        }
        if (Api.token()) {
            await invoke('set_api_token', { apiToken: Api.token() });
        }

        Api.onClientIdUpdated(async (clientId) => {
            await invoke('set_client_id', { clientId });
        });
        Api.onSignatureTokenUpdated(async (token) => {
            await invoke('set_signature_token', { signatureToken: token });
        });
        Api.onTokenUpdated(async (token) => {
            await invoke('set_api_token', { apiToken: token });
        });
        Api.onApiUrlUpdated(async (url) => {
            await invoke('set_api_url', { apiUrl: url });
        });
    });
    return (
        <Html lang="en">
            <Head>
                <Title>MoosicBox</Title>
                <Meta charset="utf-8" />
                <Meta
                    name="viewport"
                    content="width=device-width, initial-scale=1"
                />
            </Head>
            <Body>
                <Suspense>
                    <ErrorBoundary>
                        <Routes>
                            <FileRoutes />
                        </Routes>
                    </ErrorBoundary>
                </Suspense>
                <Scripts />
            </Body>
        </Html>
    );
}
