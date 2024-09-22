import './profile-page.css';
import { createSignal, onMount, Show } from 'solid-js';
import {
    connection,
    connections,
    getNewConnectionId,
    refreshConnectionProfiles,
    setConnection,
} from '~/services/api';
import { htmx } from '~/middleware/htmx';

export default function profilePage() {
    let root: HTMLDivElement;

    const [errorMessage, setErrorMessage] = createSignal<string>();

    onMount(async () => {
        htmx.process(root);

        if (connections.get().length === 0) {
            await setConnection(getNewConnectionId(), {
                name: 'Bundled',
                apiUrl: 'http://localhost:8016',
            });
        } else {
            document.body.dispatchEvent(new Event('load-new-profile'));
        }

        root.addEventListener('create-moosicbox-profile', async (e) => {
            if (!('detail' in e))
                throw new Error(`Invalid create-moosicbox-profile event`);

            setErrorMessage(undefined);

            type CreateMoosicBoxProfile = {
                success: boolean;
                message: string;
                profile?: string | undefined;
            };

            const attempt = e.detail as CreateMoosicBoxProfile;

            if (!attempt.success) {
                setErrorMessage(attempt.message);
                return;
            }

            if (attempt.profile) {
                const con = connection.get();

                if (con) {
                    const updated = await setConnection(con.id, {
                        profile: attempt.profile,
                    });
                    await refreshConnectionProfiles(updated);

                    window.location.href = './music';
                }
            }
        });
    });

    return (
        <div ref={root!}>
            <section class="setup-profile-page-local-profile">
                <h1>Setup your profile</h1>
                <div
                    hx-get={`/admin/profiles/new`}
                    hx-trigger="connection-updated from:body, load-new-profile from:body"
                >
                    loading...
                </div>
                <Show when={errorMessage()}>
                    {(errorMessage) => <p>{errorMessage()}</p>}
                </Show>
            </section>
        </div>
    );
}
