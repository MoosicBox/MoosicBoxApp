// @refresh reload
import { init, setProperty } from '@free-log/node-client';
import { invoke } from '@tauri-apps/api/core';
import { appState, onStartupFirst } from '~/services/app';
import { Api, ApiType, api, connection } from '~/services/api';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import { registerPlayer } from '~/services/player';
import {
    InboundMessageType,
    connectionId,
    connectionName,
    onConnect,
    onMessage,
    wsService,
} from '~/services/ws';
import { override } from './ws';

init({
    logWriterApiUrl: 'https://logs.moosicbox.com',
    shimConsole: true,
    logLevel: 'WARN',
});

override();

async function updatePlayers() {
    const connection = appState.connections.find(
        (c) => c.connectionId === connectionId.get(),
    );

    if (connection?.players) {
        connection.players
            .filter((player) => player.audioOutputId === 'HOWLER')
            .forEach((player) => {
                registerPlayer(createHowlerPlayer(player.playerId));
            });
    }
}

onMessage(async (data) => {
    switch (data.type) {
        case InboundMessageType.CONNECTIONS: {
            await updatePlayers();
            break;
        }
    }
});

function updateConnection(connectionId: string, name: string) {
    wsService.registerConnection({
        connectionId,
        name,
        players: [
            {
                audioOutputId: 'HOWLER',
                name: 'Web Player',
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
