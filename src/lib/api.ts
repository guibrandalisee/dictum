import { invoke } from "@tauri-apps/api/core";

export interface AppPaths {
    root: string;
    config_file: string;
    models_dir: string;
    history_file: string;
    logs_dir: string;
    portable: boolean;
}

export type Language = "auto" | "pt" | "en";
export type WhisperModel = "tiny" | "base" | "small";

export interface AppConfig {
    hotkey: string;
    language: Language;
    model: WhisperModel;
    microphone: string | null;
    auto_start: boolean;
    keep_history: boolean;
    max_recording_seconds: number;
    onboarded: boolean;
    version: number;
}

export interface HistoryEntry {
    timestamp: string;
    text: string;
    language: string | null;
    duration_ms: number | null;
}

export interface ModelStatus {
    model: WhisperModel;
    installed: boolean;
    path: string;
    size_bytes: number;
}

export interface DownloadProgress {
    model: WhisperModel;
    downloaded: number;
    total: number;
}

export type PipelineState =
    | { state: "idle" }
    | { state: "recording" }
    | { state: "processing" }
    | {
        state: "done";
        text: string;
        language: string;
        pasted: boolean;
        recording_ms: number;
        processing_ms: number;
    }
    | { state: "error"; message: string };

export const api = {
    getAppPaths: () => invoke<AppPaths>("get_app_paths"),
    getConfig: () => invoke<AppConfig>("get_config"),
    updateConfig: (config: AppConfig) =>
        invoke<AppConfig>("update_config", { config }),
    setHotkey: (accelerator: string) =>
        invoke<AppConfig>("set_hotkey", { accelerator }),
    setAutoStart: (enabled: boolean) =>
        invoke<AppConfig>("set_auto_start", { enabled }),
    getHistory: (limit = 100) => invoke<HistoryEntry[]>("get_history", { limit }),
    deleteHistoryEntry: (entry: HistoryEntry) =>
        invoke<boolean>("delete_history_entry", { entry }),
    clearHistory: () => invoke<void>("clear_history"),
    showWindow: () => invoke<void>("show_window"),
    hideWindow: () => invoke<void>("hide_window"),
    listMicrophones: () => invoke<string[]>("list_microphones"),
    getModelStatus: () => invoke<ModelStatus>("get_model_status"),
    downloadModel: () => invoke<ModelStatus>("download_model"),
    togglePause: () => invoke<boolean>("toggle_pause"),
    isPaused: () => invoke<boolean>("is_paused"),
    testRecording: (seconds: number) =>
        invoke<void>("test_recording", { seconds }),
};
