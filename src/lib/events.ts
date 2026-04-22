import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { DownloadProgress, PipelineState } from "./api";

export const events = {
    onPipelineState: (cb: (s: PipelineState) => void): Promise<UnlistenFn> =>
        listen<PipelineState>("pipeline://state", (e) => cb(e.payload)),
    onTranscript: (cb: (text: string) => void): Promise<UnlistenFn> =>
        listen<string>("pipeline://transcript", (e) => cb(e.payload)),
    onHotkeyPress: (cb: () => void): Promise<UnlistenFn> =>
        listen("hotkey://press", () => cb()),
    onHotkeyRelease: (cb: () => void): Promise<UnlistenFn> =>
        listen("hotkey://release", () => cb()),
    onModelDownload: (cb: (p: DownloadProgress) => void): Promise<UnlistenFn> =>
        listen<DownloadProgress>("model://download", (e) => cb(e.payload)),
};
