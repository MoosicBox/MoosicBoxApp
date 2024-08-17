import { listen, UnlistenFn } from '@tauri-apps/api/event';
import {
    InboundMessage,
    onConnectListener,
    onMessageListener,
    OutboundMessage,
    wsService,
} from './services/ws';
import { invoke } from '@tauri-apps/api/core';

let wsMessageSubscription: UnlistenFn | undefined;
let wsConnectSubscription: UnlistenFn | undefined;

export function override() {
    console.debug('Overriding ws service');

    wsService.reconnect = async function (): Promise<void> {};
    wsService.attemptConnection = async function (): Promise<void> {};
    wsService.newClient = async function (): Promise<void> {};

    wsService.send = function <T extends OutboundMessage>(value: T) {
        (async () => {
            console.debug('Propagating ws message to backend', value);
            await invoke('propagate_ws_message', { message: value });
            console.debug('Propagated ws message to backend', value);
        })();
    };

    (async () => {
        wsMessageSubscription?.();
        wsMessageSubscription = await listen<InboundMessage>(
            'ws-message',
            (message) => {
                console.debug('Received ws message from backend', message);
                onMessageListener.trigger(message.payload);
            },
        );
    })();

    (async () => {
        wsConnectSubscription?.();
        wsConnectSubscription = await listen<string>(
            'ws-connect',
            (message) => {
                console.debug('Received ws connect from backend', message);
                onConnectListener.trigger(message.payload);
            },
        );
    })();
}
