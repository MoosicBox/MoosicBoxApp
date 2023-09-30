import { defineConfig, searchForWorkspaceRoot } from "vite";
import solid from "solid-start/vite";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
    plugins: [
        solid({
            // appRoot: "node_modules/@moosicbox/moosicbox-ui/src",
            // routesDir: "routes",
            // clientEntry: "node_modules/@moosicbox/moosicbox-ui/src/entry-client",
            // serverEntry: "node_modules/@moosicbox/moosicbox-ui/src/entry-server",
            // router: import('./node_modules/@moosicbox/moosicbox-ui/node_modules/solid-start/fs-router/router.js').Router
            //appRoot: "src",
            //routesDir: "../node_modules/@moosicbox/moosicbox-ui/src/routes",
            // routesDir: "../MoosicBoxUI/src/routes",
            // clientEntry: "src/entry-client",
            // serverEntry: "src/entry-server",
            // router: Router// import('./node_modules/solid-start/fs-router/router.js').Router,
            // router: import('solid-start/fs-router/router').Router,
            ssr: false,
        }),
    ],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        fs: {
            allow: [
                searchForWorkspaceRoot(process.cwd()),
                //"node_modules/@moosicbox/moosicbox-ui/src",
                //"../MoosicBoxUI/src",
            ],
        },
    },
    //publicDir: "node_modules/@moosicbox/moosicbox-ui/public",
    // publicDir: "MoosicBoxUI/public",
    // 3. to make use of `TAURI_DEBUG` and other env variables
    // https://tauri.studio/v1/api/config#buildconfig.beforedevcommand
    envPrefix: ["VITE_", "TAURI_"],
    build: {
        // Tauri supports es2021
        target: ["es2021", "chrome100", "safari13"],
        // don't minify for debug builds
        minify: !process.env.TAURI_DEBUG ? ("esbuild" as const) : false,
        // produce sourcemaps for debug builds
        sourcemap: !!process.env.TAURI_DEBUG,
    },
}));
