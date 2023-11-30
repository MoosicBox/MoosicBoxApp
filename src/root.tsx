// @refresh reload
import { Routes } from '@solidjs/router';
import { Suspense } from 'solid-js';
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
import { appState, onStartup } from './services/app';
import { Api, ApiType, api } from './services/api';
import { attachConsole, debug, error, info, warn } from 'tauri-plugin-log-api';
import { trackEvent } from '@aptabase/tauri';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import { PlayerType, player } from './services/player';
import {
    InboundMessageType,
    connectionId,
    connectionName,
    onConnect,
    onConnectionNameChanged,
    onMessage,
    registerConnection,
} from './services/ws';

let currentPlayer: PlayerType | undefined;

function updatePlayer() {
    const connection = appState.connections.find(
        (c) => c.connectionId === connectionId(),
    );

    const newPlayer = connection?.players[0];

    if (newPlayer && currentPlayer?.id !== newPlayer.playerId) {
        currentPlayer = createHowlerPlayer(newPlayer.playerId);
    }

    Object.assign(player, currentPlayer);

    console.debug('Set player to', currentPlayer);
}

onMessage((data) => {
    if (data.type === InboundMessageType.CONNECTIONS) {
        updatePlayer();
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

function apiFetch<T>(
    url: string,
    query?: URLSearchParams,
    signal?: AbortSignal,
): Promise<T> {
    return new Promise(async (resolve, reject) => {
        let cancelled = false;

        signal?.addEventListener('abort', () => {
            cancelled = true;
            reject();
        });

        const data = await invoke<T>('api_proxy_get', {
            url: `${url}${query ? `?${query}` : ''}`,
        });

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
    trackEvent('warn', { args: args.map(circularStringify).join(', ') });
    warn(args.map(objToStr).join(' '));
};

console.error = (...args) => {
    trackEvent('error', { args: args.map(circularStringify).join(', ') });
    error(args.map(objToStr).join(' '));
};

onStartup(() => {
    trackEvent('onStartup');
});

const apiOverride: ApiType = {
    async getArtist(artistId, signal) {
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiFetch('artist', query, signal);
    },
    async getArtistAlbums(artistId, signal) {
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiFetch('artist/albums', query, signal);
    },
    getArtistCover(artist) {
        if (artist?.containsCover) {
            return `${Api.apiUrl()}/artists/${artist.artistId}/750x750`;
        }
        return '/img/album.svg';
    },
    async getAlbum(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch('album', query, signal);
    },
    async getAlbums(request, signal) {
        const query = new URLSearchParams({
            playerId: 'none',
        });
        if (request?.sources) query.set('sources', request.sources.join(','));
        if (request?.sort) query.set('sort', request.sort);
        if (request?.filters?.search)
            query.set('search', request.filters.search);

        return apiFetch('albums', query, signal);
    },
    async getAlbumTracks(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch('album/tracks', query, signal);
    },
    getAlbumArtwork(album) {
        if (album?.containsArtwork) {
            return `${Api.apiUrl()}/albums/${album.albumId}/300x300`;
        }
        return '/img/album.svg';
    },
    async getArtists(request, signal) {
        const query = new URLSearchParams();
        if (request?.sources) query.set('sources', request.sources.join(','));
        if (request?.sort) query.set('sort', request.sort);
        if (request?.filters?.search)
            query.set('search', request.filters.search);

        return apiFetch('artists', query, signal);
    },
};

Object.assign(api, apiOverride);

export default function Root() {
    onStartup(async () => {
        await invoke('show_main_window');
        await invoke('set_api_url', { apiUrl: Api.apiUrl() });

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
