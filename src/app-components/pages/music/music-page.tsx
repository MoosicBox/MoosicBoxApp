import './music-page.css';
import { createSignal, For, Show } from 'solid-js';
import { open } from '@tauri-apps/plugin-dialog';
import { onlyUnique } from '~/services/util';

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
        </div>
    );
}
