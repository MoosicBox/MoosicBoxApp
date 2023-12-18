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
import { Api, ApiType, api } from './services/api';
import { attachConsole, debug, error, info, warn } from 'tauri-plugin-log-api';
import { trackEvent } from '@aptabase/tauri';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import { createPlayer as createSymphoniaPlayer } from '~/symphonia-player';
import {
    PlayerType,
    player,
    setPlayerState,
    updateSessionPartial,
} from './services/player';
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

const APTABASE_ENABLED = false;

let currentPlayer: PlayerType | undefined;

(async () => {
    await listen('UPDATE_SESSION', async (event) => {
        const partialUpdate = event.payload as PartialUpdateSession;

        if (partialUpdate.playlist) {
            const ids = partialUpdate.playlist.tracks as unknown as number[];
            partialUpdate.playlist.tracks = await api.getTracks(ids);
        }

        setPlayerState(
            produce((state) => {
                updateSessionPartial(state, partialUpdate);
            }),
        );
        updateSession(partialUpdate);
    });
})();

function updatePlayer(type: Api.PlayerType | AppPlayerType) {
    const connection = appState.connections.find(
        (c) => c.connectionId === connectionId(),
    );

    const newPlayer = connection?.players?.find((p) => p.type === type);

    if (newPlayer && currentPlayer?.id !== newPlayer.playerId) {
        switch (type) {
            case AppPlayerType.SYMPHONIA:
                currentPlayer = createSymphoniaPlayer(newPlayer.playerId);
                break;
            case Api.PlayerType.HOWLER:
                currentPlayer = createHowlerPlayer(newPlayer.playerId);
                break;
        }
    }

    Object.assign(player, currentPlayer);

    console.debug('Set player to', currentPlayer);
}

onMessage((data) => {
    switch (data.type) {
        case InboundMessageType.CONNECTIONS:
            updatePlayer(Api.PlayerType.HOWLER);
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
    query?: URLSearchParams,
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

        const params = new URLSearchParams(query);
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
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiRequest('get', 'artist', query, signal);
    },
    async getArtistAlbums(artistId, signal) {
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiRequest('get', 'artist/albums', query, signal);
    },
    getArtistCover(artist) {
        if (artist?.containsCover) {
            return Api.getPath(`artists/${artist.artistId}/750x750`);
        }
        return '/img/album.svg';
    },
    async getAlbum(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album', query, signal);
    },
    async getAlbums(request, signal) {
        const query = new URLSearchParams();
        if (request?.sources) query.set('sources', request.sources.join(','));
        if (request?.sort) query.set('sort', request.sort);
        if (request?.filters?.search)
            query.set('search', request.filters.search);

        return apiRequest('get', 'albums', query, signal);
    },
    async getAlbumTracks(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album/tracks', query, signal);
    },
    async getAlbumVersions(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiRequest('get', 'album/versions', query, signal);
    },
    async getTracks(trackIds, signal) {
        const query = new URLSearchParams({
            trackIds: `${trackIds.join(',')}`,
        });

        return apiRequest('get', 'tracks', query, signal);
    },
    getAlbumArtwork(album, width, height) {
        if (album?.containsArtwork) {
            return Api.getPath(`albums/${album.albumId}/${width}x${height}`);
        }
        return '/img/album.svg';
    },
    getAlbumSourceArtwork: function (
        album: { albumId: number; containsArtwork: boolean } | undefined,
    ): string {
        if (album?.containsArtwork) {
            return Api.getPath(`albums/${album.albumId}/source`);
        }
        return '/img/album.svg';
    },
    async getArtists(request, signal) {
        const query = new URLSearchParams();
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
            new URLSearchParams(),
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
            new URLSearchParams(),
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
            new URLSearchParams({ magicToken }),
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
