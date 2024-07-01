// @refresh reload
import { init, setProperty } from '@free-log/node-client';
import { produce } from 'solid-js/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { appState, onStartupFirst } from '~/services/app';
import {
    Api,
    ApiType,
    Track,
    api,
    connection,
    toSessionPlaylistTrack,
    trackId,
} from '~/services/api';
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
    onMessage,
    registerConnection,
    updateSession,
} from '~/services/ws';
import { PartialUpdateSession } from '~/services/types';

init({
    logWriterApiUrl: 'https://logs.moosicbox.com',
    shimConsole: true,
    logLevel: 'WARN',
});

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
        (c) => c.connectionId === connectionId.get(),
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
    updateConnection(connectionId.get()!, connectionName.get());
});
connectionName.listen((name) => {
    updateConnection(connectionId.get()!, name);
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

connection.listen((con, prev) => {
    if (!con) return;

    if (con.apiUrl !== prev?.apiUrl) {
        updateApi(con.apiUrl.toLowerCase().startsWith('https://'));
    }
});

onStartupFirst(async () => {
    try {
        await invoke('show_main_window');
    } catch {}

    setProperty('connectionId', connectionId.get());
    setProperty('connectionName', connectionName.get());

    await Promise.all([
        invoke('set_connection_id', { connectionId: connectionId.get() }),
        invoke('set_connection_name', { connectionName: connectionName.get() }),
    ]);

    const con = connection.get();
    if (con) {
        updateApi(con.apiUrl.toLowerCase().startsWith('https://'));
        await invoke('set_api_url', { apiUrl: con.apiUrl });
        if (con.clientId) {
            await invoke('set_client_id', { clientId: con.clientId });
        }
        if (Api.signatureToken()) {
            await invoke('set_signature_token', {
                signatureToken: Api.signatureToken(),
            });
        }
        if (con.token) {
            await invoke('set_api_token', { apiToken: con.token });
        }
    }

    connection.listen(async (con) => {
        if (!con) return;
        await invoke('set_api_url', { apiUrl: con.apiUrl });
        await invoke('set_api_token', { apiToken: con.token });
        await invoke('set_client_id', { clientId: con.clientId });
    });
    connectionId.listen(async (connectionId) => {
        setProperty('connectionId', connectionId);
        await invoke('set_connection_id', { connectionId });
    });
    connectionName.listen(async (connectionName) => {
        setProperty('connectionName', connectionName);
        await invoke('set_connection_name', { connectionName });
    });
    Api.onSignatureTokenUpdated(async (token) => {
        await invoke('set_signature_token', { signatureToken: token });
    });
});
