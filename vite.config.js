var _a, _b;
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
// Tauri expects a fixed port, fail if not available
var host = (_b = (_a = globalThis
    .process) === null || _a === void 0 ? void 0 : _a.env) === null || _b === void 0 ? void 0 : _b.TAURI_DEV_HOST;
export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                protocol: "ws",
                host: host,
                port: 1421,
            }
            : undefined,
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
    envPrefix: ["VITE_", "TAURI_ENV_*"],
    build: {
        target: "es2021",
        minify: "esbuild",
        sourcemap: false,
    },
});
