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
    return new Promise(async (resolve, reject) => {
        let cancelled = false;

        signal?.addEventListener("abort", () => {
            cancelled = true;
            reject();
        });

        const data = await invoke<T>("api_proxy", {
            url: `${Api.apiUrl()}/${url}${query ? `?${query}` : ""}`,
        });

        if (!cancelled) {
            resolve(data);
        }
    });
}

const apiOverride: ApiType = {
    async getArtist(artistId, signal) {
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiFetch("artist", query, signal);
    },
    async getArtistAlbums(artistId, signal) {
        const query = new URLSearchParams({
            artistId: `${artistId}`,
        });

        return apiFetch("artist/albums", query, signal);
    },
    getArtistCover(artist) {
        if (artist?.containsCover) {
            return `${Api.apiUrl()}/artists/${artist.artistId}/750x750`;
        }
        return "/img/album.svg";
    },
    async getAlbum(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch("album", query, signal);
    },
    async getAlbums(request, signal) {
        const query = new URLSearchParams({
            playerId: "none",
        });
        if (request?.sources) query.set("sources", request.sources.join(","));
        if (request?.sort) query.set("sort", request.sort);
        if (request?.filters?.search)
            query.set("search", request.filters.search);

        return apiFetch("albums", query, signal);
    },
    async getAlbumTracks(albumId, signal) {
        const query = new URLSearchParams({
            albumId: `${albumId}`,
        });

        return apiFetch("album/tracks", query, signal);
    },
    getAlbumArtwork(album) {
        if (album?.containsArtwork) {
            return `${Api.apiUrl()}/albums/${album.albumId}/300x300`;
        }
        return "/img/album.svg";
    },
    async getArtists(request, signal) {
        const query = new URLSearchParams();
        if (request?.sources) query.set("sources", request.sources.join(","));
        if (request?.sort) query.set("sort", request.sort);
        if (request?.filters?.search)
            query.set("search", request.filters.search);

        return apiFetch("artists", query, signal);
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
