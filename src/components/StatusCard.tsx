import { useAppStore } from "../stores/appStore";

function formatDuration(ms: number): string {
    const safeMs = Number.isFinite(ms) ? Math.max(0, ms) : 0;
    if (safeMs < 1000) return `${Math.round(safeMs)}ms`;
    const seconds = safeMs / 1000;
    if (seconds < 10) {
        return `${seconds.toLocaleString("pt-BR", {
            minimumFractionDigits: 1,
            maximumFractionDigits: 1,
        })}s`;
    }
    return `${Math.round(seconds).toLocaleString("pt-BR")}s`;
}

function MicIcon({ kind }: { kind: string }) {
    if (kind === "done") {
        return (
            <svg className="mic-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <polyline points="20 6 9 17 4 12" />
            </svg>
        );
    }
    if (kind === "error") {
        return (
            <svg className="mic-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
        );
    }
    if (kind === "paused") {
        return (
            <svg className="mic-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <rect x="6" y="5" width="4" height="14" rx="1" />
                <rect x="14" y="5" width="4" height="14" rx="1" />
            </svg>
        );
    }
    if (kind === "processing") {
        return (
            <svg className="mic-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="6" cy="12" r="1.6" />
                <circle cx="12" cy="12" r="1.6" />
                <circle cx="18" cy="12" r="1.6" />
            </svg>
        );
    }
    return (
        <svg className="mic-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" y1="19" x2="12" y2="23" />
            <line x1="8" y1="23" x2="16" y2="23" />
        </svg>
    );
}

export function StatusCard() {
    const { pipeline, paused, togglePause, config } = useAppStore();

    let kind = "idle";
    let label = "Aguardando atalho";

    if (paused) {
        kind = "paused";
        label = "Pausado";
    } else if (pipeline.state === "recording") {
        kind = "recording";
        label = "Gravando…";
    } else if (pipeline.state === "processing") {
        kind = "processing";
        label = "Transcrevendo…";
    } else if (pipeline.state === "done") {
        kind = "done";
        label = pipeline.pasted ? "Texto colado" : "Texto copiado para o clipboard";
    } else if (pipeline.state === "error") {
        kind = "error";
        label = "Algo deu errado";
    }

    return (
        <section className="card">
            <div className="card-head">
                <div>
                    <h2 className="card-title">Status</h2>
                    <p className="card-sub">Pressione e segure o atalho para gravar</p>
                </div>
                <button className="btn btn-secondary btn-sm" onClick={togglePause}>
                    {paused ? "Retomar" : "Pausar"}
                </button>
            </div>

            <div className="status-hero">
                <div className={`mic-visual ${kind}`}>
                    <MicIcon kind={kind} />
                </div>
                <div className="status-info">
                    <div className="status-label">{label}</div>
                    <div className="status-meta">
                        Atalho: <kbd>{config?.hotkey ?? "—"}</kbd>
                        <br />
                        Segure, fale e solte para colar a transcrição no campo focado.
                    </div>
                </div>
            </div>

            {pipeline.state === "done" && pipeline.text && (
                <div className="transcript-block">
                    <div className="transcript-meta">
                        <span>Última transcrição · {pipeline.language}</span>
                        <span className="transcript-timings">
                            <span title="Duração da gravação">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                                    <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
                                    <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                                </svg>
                                Gravação {formatDuration(pipeline.recording_ms)}
                            </span>
                            <span className="transcript-timing-sep">·</span>
                            <span title="Tempo de transcrição">
                                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                                    <circle cx="12" cy="12" r="9" />
                                    <polyline points="12 7 12 12 15 14" />
                                </svg>
                                Transcrição {formatDuration(pipeline.processing_ms)}
                            </span>
                        </span>
                    </div>
                    <p>{pipeline.text}</p>
                </div>
            )}

            {pipeline.state === "error" && (
                <div className="banner error" style={{ marginTop: 16, marginBottom: 0 }}>
                    {pipeline.message}
                </div>
            )}
        </section>
    );
}

