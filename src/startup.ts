// @refresh reload
import { init, setProperty } from '@free-log/node-client';
import { invoke, InvokeArgs } from '@tauri-apps/api/core';
import { appState, onStartupFirst } from '~/services/app';
import { Api, ApiType, Connection, api, connection } from '~/services/api';
import { createPlayer as createHowlerPlayer } from '~/services/howler-player';
import {
    onCurrentAudioZoneIdChanged,
    playerState,
    registerPlayer,
} from '~/services/player';
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

function tryInvoke(event: string, payload?: InvokeArgs) {
    (async () => {
        try {
            invoke(event, payload);
        } catch (e) {
            console.error(`Failed to invoke '${event}':`, e);
        }
    })();
}

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

onCurrentAudioZoneIdChanged((audioZoneId) => {
    updateStateForConnection(connection.get(), { audioZoneId });
});

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

type State = {
    connectionId?: string | undefined;
    connectionName?: string | undefined;
    apiUrl?: string | undefined;
    clientId?: string | undefined;
    signatureToken?: string | undefined;
    apiToken?: string | undefined;
    audioZoneId?: number | undefined;
};

function updateStateForConnection(con: Connection | null, overrides?: State) {
    if (con?.apiUrl) {
        updateApi(con.apiUrl.toLowerCase().startsWith('https://'));
    }

    const state: State = {
        connectionId: connectionId.get(),
        connectionName: con?.name,
        apiUrl: con?.apiUrl,
        clientId: con?.clientId,
        signatureToken: Api.signatureToken(),
        apiToken: con?.token,
        audioZoneId:
            typeof playerState.currentAudioZone?.id === 'number'
                ? playerState.currentAudioZone.id
                : undefined,
    };

    Object.assign(state, overrides);

    console.debug('Setting state', state);

    tryInvoke('set_state', { state });
}

onStartupFirst(async () => {
    tryInvoke('show_main_window');

    setProperty('connectionId', connectionId.get());
    setProperty('connectionName', connectionName.get());

    updateStateForConnection(connection.get());

    connection.listen(async (con) => {
        updateStateForConnection(con);
    });
    connectionId.listen(async (connectionId) => {
        setProperty('connectionId', connectionId);
    });
    connectionName.listen(async (connectionName) => {
        setProperty('connectionName', connectionName);
    });
    Api.onSignatureTokenUpdated(async () => {
        updateStateForConnection(connection.get());
    });
});
