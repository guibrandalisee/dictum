import { create } from "zustand";
import { api, AppConfig, AppPaths, HistoryEntry, ModelStatus, PipelineState } from "../lib/api";

interface AppStore {
    paths: AppPaths | null;
    config: AppConfig | null;
    modelStatus: ModelStatus | null;
    pipeline: PipelineState;
    history: HistoryEntry[];
    microphones: string[];
    paused: boolean;
    downloading: { downloaded: number; total: number } | null;
    errorBanner: string | null;

    bootstrap: () => Promise<void>;
    refreshConfig: () => Promise<void>;
    refreshModel: () => Promise<void>;
    refreshHistory: () => Promise<void>;
    refreshMicrophones: () => Promise<void>;
    setPipeline: (s: PipelineState) => void;
    setDownloading: (d: { downloaded: number; total: number } | null) => void;
    setErrorBanner: (msg: string | null) => void;
    togglePause: () => Promise<void>;
    saveConfig: (cfg: AppConfig) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
    paths: null,
    config: null,
    modelStatus: null,
    pipeline: { state: "idle" },
    history: [],
    microphones: [],
    paused: false,
    downloading: null,
    errorBanner: null,

    bootstrap: async () => {
        const [paths, config, modelStatus, history, mics, paused] = await Promise.all([
            api.getAppPaths(),
            api.getConfig(),
            api.getModelStatus(),
            api.getHistory(100),
            api.listMicrophones().catch(() => []),
            api.isPaused().catch(() => false),
        ]);
        set({ paths, config, modelStatus, history, microphones: mics, paused });
    },

    refreshConfig: async () => set({ config: await api.getConfig() }),
    refreshModel: async () => set({ modelStatus: await api.getModelStatus() }),
    refreshHistory: async () => set({ history: await api.getHistory(100) }),
    refreshMicrophones: async () =>
        set({ microphones: await api.listMicrophones().catch(() => []) }),

    setPipeline: (s) => set({ pipeline: s }),
    setDownloading: (d) => set({ downloading: d }),
    setErrorBanner: (msg) => set({ errorBanner: msg }),

    togglePause: async () => {
        const next = await api.togglePause();
        set({ paused: next });
    },

    saveConfig: async (cfg) => {
        try {
            const updated = await api.updateConfig(cfg);
            set({ config: updated, errorBanner: null });
            // Model may have changed -> refresh status
            await get().refreshModel();
        } catch (e) {
            set({ errorBanner: String(e) });
            throw e;
        }
    },
}));
