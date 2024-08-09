// @refresh reload
import { init, setProperty } from '@free-log/node-client';
import { invoke } from '@tauri-apps/api/core';
import { appState, onStartupFirst } from '~/services/app';
import { Api, ApiType, api, connection } from '~/services/api';
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

onCurrentAudioZoneIdChanged(async (audioZoneId) => {
    try {
        await invoke('set_current_audio_zone_id', { audioZoneId });
    } catch {}
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

onStartupFirst(async () => {
    try {
        await invoke('show_main_window');
    } catch {}

    setProperty('connectionId', connectionId.get());
    setProperty('connectionName', connectionName.get());

    let setCurrentAudioZoneId = Promise.resolve();

    if (typeof playerState.currentAudioZone?.id === 'number') {
        try {
            setCurrentAudioZoneId = invoke<void>('set_current_audio_zone_id', {
                audioZoneId: playerState.currentAudioZone.id,
            });
        } catch {}
    }

    try {
        await Promise.all([
            invoke('set_connection_id', { connectionId: connectionId.get() }),
            invoke('set_connection_name', {
                connectionName: connectionName.get(),
            }),
        ]);
    } catch {}

    const con = connection.get();
    if (con) {
        updateApi(con.apiUrl.toLowerCase().startsWith('https://'));
        try {
            await invoke('set_api_url', { apiUrl: con.apiUrl });
        } catch (e) {
            console.error('Failed to set_api_url:', e);
        }
        if (con.clientId) {
            try {
                await invoke('set_client_id', { clientId: con.clientId });
            } catch (e) {
                console.error('Failed to set_client_id:', e);
            }
        }
        if (Api.signatureToken()) {
            try {
                await invoke('set_signature_token', {
                    signatureToken: Api.signatureToken(),
                });
            } catch (e) {
                console.error('Failed to set_signature_token:', e);
            }
        }
        if (con.token) {
            try {
                await invoke('set_api_token', { apiToken: con.token });
            } catch (e) {
                console.error('Failed to set_api_token:', e);
            }
        }
    }

    connection.listen(async (con) => {
        if (!con) return;
        try {
            await invoke('set_api_url', { apiUrl: con.apiUrl });
        } catch (e) {
            console.error('Failed to set_api_url:', e);
        }
        try {
            await invoke('set_api_token', { apiToken: con.token });
        } catch (e) {
            console.error('Failed to set_api_token:', e);
        }
        try {
            await invoke('set_client_id', { clientId: con.clientId });
        } catch (e) {
            console.error('Failed to set_client_id:', e);
        }
    });
    connectionId.listen(async (connectionId) => {
        setProperty('connectionId', connectionId);
        try {
            await invoke('set_connection_id', { connectionId });
        } catch (e) {
            console.error('Failed to set_connection_id:', e);
        }
    });
    connectionName.listen(async (connectionName) => {
        setProperty('connectionName', connectionName);
        try {
            await invoke('set_connection_name', { connectionName });
        } catch (e) {
            console.error('Failed to set_connection_name:', e);
        }
    });
    Api.onSignatureTokenUpdated(async (token) => {
        try {
            await invoke('set_signature_token', { signatureToken: token });
        } catch (e) {
            console.error('Failed to set_signature_token:', e);
        }
    });
});
