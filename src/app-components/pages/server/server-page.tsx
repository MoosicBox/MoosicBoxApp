import { config } from '~/config';
import './server-page.css';
import { createSignal, For, onCleanup, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { connections, getNewConnectionId, setConnection } from '~/services/api';

export default function serverPage() {
    if (config.bundled) return bundledAppServerPage();
    if (config.app) return appServerPage();
    throw new Error(`Invalid configuration: ${JSON.stringify(config)}`);
}

type Server = {
    id: string;
    name: string;
    host: string;
    dns: string;
};

export function appServerPage() {
    const [intervalHandle, setIntervalHandle] = createSignal<NodeJS.Timeout>();
    const [servers, setServers] = createSignal<Server[]>([]);

    onMount(async () => {
        setIntervalHandle(
            setInterval(async () => {
                const servers = await invoke<Server[]>(
                    'fetch_moosicbox_servers',
                );
                console.log(servers);
                setServers(servers);
            }, 1000),
        );
    });

    onCleanup(async () => {
        const handle = intervalHandle();

        if (handle) {
            clearInterval(handle);
        }
    });

    function selectServer(server: Server) {
        const existing = connections
            .get()
            .find((x) => x.apiUrl === server.host);

        if (existing) {
            setConnection(existing.id, existing);
            return;
        }

        const id = getNewConnectionId();
        setConnection(id, {
            id,
            name: server.name,
            apiUrl: server.host,
            clientId: '',
            token: '',
            staticToken: '',
        });
    }

    return (
        <div>
            {servers().length === 0 && (
                <p>Searching for compatible MoosicBox servers...</p>
            )}
            {servers().length > 0 && <p>Select your MoosicBox server</p>}
            <For each={servers()}>
                {(server) => (
                    <div class="server-page-server">
                        <div>
                            {server.name} - {server.host}
                        </div>
                        <div>
                            <button
                                onClick={async () => {
                                    selectServer(server);
                                    window.location.href = '/';
                                }}
                                type="button"
                                class="remove-button-styles select-button"
                            >
                                Select
                            </button>
                        </div>
                    </div>
                )}
            </For>
        </div>
    );
}

export function bundledAppServerPage() {
    return <div>todo</div>;
}
