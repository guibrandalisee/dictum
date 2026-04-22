import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { events } from "./lib/events";
import { api } from "./lib/api";
import { useAppStore } from "./stores/appStore";
import { StatusCard } from "./components/StatusCard";
import { SettingsPanel } from "./components/SettingsPanel";
import { HistoryPanel } from "./components/HistoryPanel";
import { ModelDownloadCard } from "./components/ModelDownloadCard";
import { Onboarding } from "./components/Onboarding";

type Tab = "status" | "settings" | "history";

const TABS: { id: Tab; label: string; sub: string; icon: JSX.Element }[] = [
    {
        id: "status",
        label: "Início",
        sub: "Veja o status da gravação e a última transcrição",
        icon: (
            <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                <line x1="12" y1="19" x2="12" y2="23" />
                <line x1="8" y1="23" x2="16" y2="23" />
            </svg>
        ),
    },
    {
        id: "settings",
        label: "Configurações",
        sub: "Atalhos, idioma, modelo e microfone",
        icon: (
            <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
        ),
    },
    {
        id: "history",
        label: "Histórico",
        sub: "Suas transcrições recentes",
        icon: (
            <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
                <polyline points="3 3 3 8 8 8" />
                <polyline points="12 7 12 12 15 14" />
            </svg>
        ),
    },
];

function statusKind(state: string, paused: boolean): {
    cls: string;
    label: string;
} {
    if (paused) return { cls: "paused", label: "Pausado" };
    switch (state) {
        case "recording": return { cls: "recording", label: "Gravando" };
        case "processing": return { cls: "processing", label: "Transcrevendo" };
        case "done": return { cls: "done", label: "Pronto" };
        case "error": return { cls: "error", label: "Erro" };
        default: return { cls: "idle", label: "Aguardando" };
    }
}

export default function App() {
    const {
        bootstrap,
        config,
        pipeline,
        paused,
        setPipeline,
        setDownloading,
        refreshHistory,
        refreshModel,
    } = useAppStore();
    const [tab, setTab] = useState<Tab>("status");
    const [bootError, setBootError] = useState<string | null>(null);
    const [showOnboarding, setShowOnboarding] = useState(false);

    const minimizeWindow = async () => {
        try {
            await getCurrentWindow().minimize();
        } catch {
            // ignored when not running inside Tauri runtime
        }
    };

    const hideToTray = async () => {
        try {
            await api.hideWindow();
        } catch {
            try {
                await getCurrentWindow().hide();
            } catch {
                // ignored when not running inside Tauri runtime
            }
        }
    };

    useEffect(() => {
        bootstrap()
            .then(() => {
                const cfg = useAppStore.getState().config;
                if (cfg && !cfg.onboarded) setShowOnboarding(true);
            })
            .catch((e) => setBootError(String(e)));
    }, [bootstrap]);

    useEffect(() => {
        const unlisten: Array<Promise<() => void>> = [];

        unlisten.push(events.onPipelineState((s) => setPipeline(s)));
        unlisten.push(
            events.onPipelineState(async (s) => {
                if (s.state === "done") {
                    await refreshHistory();
                }
            }),
        );
        unlisten.push(
            events.onModelDownload(async (p) => {
                setDownloading({ downloaded: p.downloaded, total: p.total });
                if (p.downloaded >= p.total && p.total > 0) {
                    setTimeout(() => {
                        setDownloading(null);
                        refreshModel();
                    }, 400);
                }
            }),
        );

        return () => {
            unlisten.forEach((p) => p.then((fn) => fn()).catch(() => { }));
        };
    }, [setPipeline, setDownloading, refreshHistory, refreshModel]);

    return (
        <div className="window-frame">
            <header className="window-chrome" data-tauri-drag-region>
                <div className="window-chrome-left" data-tauri-drag-region>
                    <span className="window-dot" aria-hidden />
                    <span className="window-title">Dictum</span>
                    <span className="window-subtitle">Ditado local com Whisper</span>
                </div>
                <div className="window-controls">
                    <button
                        className="window-btn"
                        title="Minimizar"
                        aria-label="Minimizar"
                        onClick={() => void minimizeWindow()}
                    >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                            <line x1="5" y1="12" x2="19" y2="12" />
                        </svg>
                    </button>
                    <button
                        className="window-btn close"
                        title="Fechar para bandeja"
                        aria-label="Fechar para bandeja"
                        onClick={() => void hideToTray()}
                    >
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                            <line x1="18" y1="6" x2="6" y2="18" />
                            <line x1="6" y1="6" x2="18" y2="18" />
                        </svg>
                    </button>
                </div>
            </header>

            {bootError ? (
                <main className="loading-screen">
                    <div className="banner error" style={{ maxWidth: 480 }}>
                        Falha ao iniciar o aplicativo:<br />
                        <code>{bootError}</code>
                    </div>
                </main>
            ) : !config ? (
                <main className="loading-screen">
                    <div className="loading-spinner" />
                    <p>Carregando…</p>
                </main>
            ) : (
                <>
                    <div className="app-shell">
                        <aside className="sidebar">
                            <div className="brand">
                                <div className="brand-mark">
                                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                                        <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
                                        <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                                        <line x1="12" y1="19" x2="12" y2="23" />
                                    </svg>
                                </div>
                                <div>
                                    <div className="brand-name">Dictum</div>
                                    <div className="brand-sub">Ditado local com Whisper</div>
                                </div>
                            </div>

                            <nav className="nav">
                                <div className="nav-section">Navegação</div>
                                {TABS.map((t) => (
                                    <button
                                        key={t.id}
                                        className={`nav-item ${tab === t.id ? "active" : ""}`}
                                        onClick={() => setTab(t.id)}
                                    >
                                        {t.icon}
                                        <span>{t.label}</span>
                                    </button>
                                ))}
                            </nav>

                            <div className="sidebar-footer">
                                <div className="status-pill" title={`Pipeline: ${pipeline.state}`}>
                                    <span className={`status-dot ${statusKind(pipeline.state, paused).cls}`} />
                                    <span>{statusKind(pipeline.state, paused).label}</span>
                                </div>
                                <div className="app-version">v0.1.0 · MVP</div>
                            </div>
                        </aside>

                        <main className="content">
                            <header className="page-header">
                                <div>
                                    <h1 className="page-title">{TABS.find((t) => t.id === tab)?.label}</h1>
                                    <p className="page-sub">{TABS.find((t) => t.id === tab)?.sub}</p>
                                </div>
                            </header>

                            {tab === "status" && (
                                <>
                                    <StatusCard />
                                    <ModelDownloadCard />
                                </>
                            )}
                            {tab === "settings" && (
                                <>
                                    <SettingsPanel />
                                    <ModelDownloadCard />
                                </>
                            )}
                            {tab === "history" && <HistoryPanel />}
                        </main>
                    </div>

                    {showOnboarding && <Onboarding onClose={() => setShowOnboarding(false)} />}
                </>
            )}
        </div>
    );
}

