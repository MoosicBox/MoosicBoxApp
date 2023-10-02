// @refresh reload
import { Routes } from "@solidjs/router";
import { Suspense } from "solid-js";
import {
    Body,
    FileRoutes,
    Head,
    Html,
    Meta,
    Scripts,
    Title,
} from "solid-start";
import { ErrorBoundary } from "solid-start/error-boundary";
import { invoke } from "@tauri-apps/api/tauri";
import { onStartup } from "./services/app";
import { Api, ApiType, api } from "./services/api";

function apiFetch<T>(
    url: string,
    query?: URLSearchParams,
    signal?: AbortSignal,
): Promise<T> {
    return new Promise((resolve, reject) => {
        let cancelled = false;

        signal?.addEventListener("abort", () => {
            cancelled = true;
            reject();
        });

        invoke<T>("api_proxy", {
            url: `${Api.apiUrl()}/${url}${query ? `?${query}` : ""}`,
        }).then((data) => {
            if (!cancelled) {
                resolve(data);
            }
        });
    });
}

const apiOverride: ApiType = {
    async getAlbum(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch("album", query, signal);
    },
    async getAlbums(request, signal): Promise<Api.Album[]> {
        const query = new URLSearchParams({
            playerId: "none",
        });
        if (request?.sources) query.set("sources", request.sources.join(","));
        if (request?.sort) query.set("sort", request.sort);
        if (request?.filters?.search)
            query.set("search", request.filters.search);

        return apiFetch("albums", query, signal);
    },
    async getAlbumTracks(albumId, signal): Promise<Api.Track[]> {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch("album/tracks", query, signal);
    },
    getAlbumArtwork(album): string {
        if (album?.containsArtwork) {
            return `${Api.apiUrl()}/albums/${album.albumId}/300x300`;
        }
        return "/img/album.svg";
    },
};

Object.entries(apiOverride).forEach(([key, value]) => {
    api[key as keyof typeof api] = value;
});

export default function Root() {
    onStartup(async () => {
        await invoke("show_main_window");
    });
    return (
        <Html lang="en">
            <Head>
                <Title>MoosicBox</Title>
                <Meta charset="utf-8" />
                <Meta
                    name="viewport"
                    content="width=device-width, initial-scale=1"
                />
            </Head>
            <Body>
                <Suspense>
                    <ErrorBoundary>
                        <Routes>
                            <FileRoutes />
                        </Routes>
                    </ErrorBoundary>
                </Suspense>
                <Scripts />
            </Body>
        </Html>
    );
}
