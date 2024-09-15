import './music-page.css';
import { createSignal, For, onMount, Show } from 'solid-js';
import { open } from '@tauri-apps/plugin-dialog';
import { onlyUnique } from '~/services/util';
import {
    api,
    connections,
    getNewConnectionId,
    setConnection,
} from '~/services/api';

export default function musicPage() {
    const [folders, setFolders] = createSignal<string[]>([]);

    async function addFolder() {
        const directories = await open({
            multiple: true,
            directory: true,
        });
        if (directories) {
            setFolders([...folders(), ...directories].filter(onlyUnique));
        }
    }

    async function saveFolders() {
        await api.enableScanOrigin('LOCAL');
        await Promise.all(
            folders().map((folder) => {
                return api.addScanPath(folder);
            }),
        );
        await api.startScan(['LOCAL']);
    }

    onMount(async () => {
        if (connections.get().length === 0) {
            setConnection(getNewConnectionId(), {
                name: 'Bundled',
                apiUrl: 'http://localhost:8016',
            });
        }
    });

    return (
        <div>
            <p>Where do you store your music?</p>
            <button
                onClick={addFolder}
                type="button"
                class="remove-button-styles add-music-folder-button"
            >
                Add Folder
            </button>
            <Show when={folders()}>
                {(folders) => (
                    <For each={folders()}>{(folder) => <p>{folder}</p>}</For>
                )}
            </Show>
            <button
                onClick={async () => {
                    await saveFolders();
                    window.location.href = '/';
                }}
                type="button"
                class="remove-button-styles finish-button"
            >
                Finish
            </button>
        </div>
    );
}
