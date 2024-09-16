import './music-page.css';
import { createSignal, For, onMount, Show } from 'solid-js';
import { open } from '@tauri-apps/plugin-dialog';
import { onlyUnique } from '~/services/util';
import {
    Api,
    api,
    connections,
    getNewConnectionId,
    setConnection,
} from '~/services/api';

export default function musicPage() {
    const [folders, setFolders] = createSignal<string[]>([]);
    const [qobuzAuthSuccess, setQobuzAuthSuccess] = createSignal<boolean>();
    const [tidalAuthSuccess, setTidalAuthSuccess] = createSignal<boolean>();
    const [tidalDeviceAuthorization, setTidalDeviceAuthorization] =
        createSignal<Api.TidalDeviceAuthorizationResponse>();
    const [pollTimeout, setPollTimeout] = createSignal<NodeJS.Timeout>();

    let qobuzUsernameInput: HTMLInputElement;
    let qobuzPasswordInput: HTMLInputElement;

    async function addFolder() {
        const directories = await open({
            multiple: true,
            directory: true,
        });
        if (directories) {
            setFolders([...folders(), ...directories].filter(onlyUnique));
        }
    }

    async function authenticateTidal() {
        setTidalDeviceAuthorization(await api.tidalDeviceAuthorization());

        await pollTidalAuthorizationResponse();
    }

    async function pollTidalAuthorizationResponse() {
        console.debug('pollTidalAuthorizationResponse');
        if (pollTimeout()) {
            clearTimeout(pollTimeout());
        }

        const deviceAuthorization = tidalDeviceAuthorization();
        console.debug('pollTidalAuthorizationResponse:', deviceAuthorization);

        if (deviceAuthorization) {
            try {
                const response = await api.tidalDeviceAuthorizationToken(
                    deviceAuthorization.deviceCode,
                    true,
                );

                if (response.accessToken) {
                    setTidalAuthSuccess(true);
                    return;
                }
            } catch {
                console.debug('pollTidalAuthorizationResponse: not ready');
                setPollTimeout(
                    setTimeout(pollTidalAuthorizationResponse, 1000),
                );
            }
        }
    }

    async function authenticateQobuz() {
        const response = await api.authQobuz(
            qobuzUsernameInput.value,
            qobuzPasswordInput.value,
            true,
        );
        if (response.accessToken) {
            qobuzUsernameInput.value = '';
            qobuzPasswordInput.value = '';
            setQobuzAuthSuccess(true);
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

    async function scanQobuz() {
        await api.enableScanOrigin('QOBUZ');
        await api.startScan(['QOBUZ']);
    }

    async function scanTidal() {
        await api.enableScanOrigin('TIDAL');
        await api.startScan(['TIDAL']);
    }

    async function finish() {
        const requests = [];

        if (folders().length > 0) {
            requests.push(saveFolders());
        }

        if (qobuzAuthSuccess() === true) {
            requests.push(scanQobuz());
        }

        if (tidalAuthSuccess() === true) {
            requests.push(scanTidal());
        }

        await Promise.all(requests);
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
            <section class="setup-music-page-local-music">
                <h1>Local Music</h1>
                <p>Where do you store your music?</p>
                <button
                    onClick={addFolder}
                    type="button"
                    class="remove-button-styles moosicbox-button"
                >
                    Add Folder
                </button>
                <Show when={folders()}>
                    {(folders) => (
                        <For each={folders()}>
                            {(folder) => <p>{folder}</p>}
                        </For>
                    )}
                </Show>
                <button
                    onClick={saveFolders}
                    type="button"
                    class="remove-button-styles moosicbox-button"
                >
                    Save
                </button>
            </section>
            <hr />
            <section class="setup-music-page-tidal-music">
                <h1>Tidal</h1>
                <p>Sign in to your Tidal account (optional)</p>
                <button
                    onClick={authenticateTidal}
                    type="button"
                    class="remove-button-styles moosicbox-button"
                >
                    Start web authentication
                </button>
                <Show when={tidalDeviceAuthorization()}>
                    {(auth) => (
                        <div>
                            <span>
                                Follow this link to authenticate with Tidal:{' '}
                                <a href={auth().url} target="_blank">
                                    {auth().url}
                                </a>
                            </span>
                        </div>
                    )}
                </Show>
                <Show when={tidalAuthSuccess() === true}>
                    <p>Success!</p>
                </Show>
                <Show when={tidalAuthSuccess() === false}>
                    <p>Failed to authenticate!</p>
                </Show>
            </section>
            <hr />
            <section class="setup-music-page-qobuz-music">
                <h1>Qobuz</h1>
                <p>Sign in to your Qobuz account (optional)</p>
                <input
                    ref={qobuzUsernameInput!}
                    type="text"
                    onKeyUp={(e) => e.key === 'Enter' && authenticateQobuz()}
                />
                <input
                    ref={qobuzPasswordInput!}
                    type="password"
                    onKeyUp={(e) => e.key === 'Enter' && authenticateQobuz()}
                />
                <button
                    onClick={authenticateQobuz}
                    type="button"
                    class="remove-button-styles moosicbox-button"
                >
                    Login
                </button>
                <Show when={qobuzAuthSuccess() === true}>
                    <p>Success!</p>
                </Show>
                <Show when={qobuzAuthSuccess() === false}>
                    <p>Failed to authenticate!</p>
                </Show>
            </section>
            <button
                onClick={async () => {
                    await finish();
                    window.location.href = '/';
                }}
                type="button"
                class="remove-button-styles moosicbox-button"
            >
                Finish
            </button>
        </div>
    );
}
