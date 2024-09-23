import './server-page.css';
import { createSignal, For, onCleanup, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { connections, getNewConnectionId, setConnection } from '~/services/api';

type Server = {
    id: string;
    name: string;
    host: string;
    dns: string;
};

export default function serverPage() {
    const [intervalHandle, setIntervalHandle] = createSignal<NodeJS.Timeout>();
    const [servers, setServers] = createSignal<Server[]>([]);

    onMount(async () => {
        setIntervalHandle(
            setInterval(async () => {
                const servers = await invoke<Server[]>(
                    'fetch_moosicbox_servers',
                );
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

    async function selectServer(server: Server) {
        const existing = connections
            .get()
            .find((x) => x.apiUrl === server.host);

        if (existing) {
            setConnection(existing.id, existing);
            return;
        }

        const con = await setConnection(getNewConnectionId(), {
            name: server.name,
            apiUrl: server.host,
        });

        if (!con?.profiles || con.profiles.length === 0) {
            window.location.href = './profiles';
        } else {
            localStorage.removeItem('settingUp');
            window.location.href = '/';
        }
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
                                onClick={async () => await selectServer(server)}
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
