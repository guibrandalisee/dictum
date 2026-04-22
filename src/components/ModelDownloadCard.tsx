import { useState } from "react";
import { api, WhisperModel } from "../lib/api";
import { useAppStore } from "../stores/appStore";

const MODEL_LABELS: Record<WhisperModel, string> = {
    tiny: "tiny · ~75 MB · mais rápido",
    base: "base · ~142 MB · equilibrado (recomendado)",
    small: "small · ~466 MB · mais preciso",
};

export function ModelDownloadCard() {
    const { modelStatus, downloading, refreshModel, config } = useAppStore();
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    if (!modelStatus || !config) return null;

    const start = async () => {
        setBusy(true);
        setError(null);
        try {
            await api.downloadModel();
            await refreshModel();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
        }
    };

    const pct = downloading
        ? Math.min(100, Math.round((downloading.downloaded / downloading.total) * 100))
        : 0;
    const mb = (n: number) => (n / 1024 / 1024).toFixed(1);

    return (
        <section className="card">
            <div className="card-head">
                <div>
                    <h2 className="card-title">Modelo Whisper</h2>
                    <p className="card-sub">{MODEL_LABELS[modelStatus.model]}</p>
                </div>
                {modelStatus.installed ? (
                    <span className="badge ok">Instalado</span>
                ) : (
                    <span className="badge warn">Não baixado</span>
                )}
            </div>

            {downloading && (
                <div className="progress">
                    <div className="progress-bar-wrap">
                        <div className="progress-bar" style={{ width: `${pct}%` }} />
                    </div>
                    <div className="progress-meta">
                        <span>{pct}%</span>
                        <span>{mb(downloading.downloaded)} / {mb(downloading.total)} MB</span>
                    </div>
                </div>
            )}

            {!downloading && (
                <div className="row end" style={{ marginTop: 12 }}>
                    <button
                        className={modelStatus.installed ? "btn btn-secondary" : "btn"}
                        onClick={start}
                        disabled={busy}
                    >
                        {busy
                            ? "Iniciando…"
                            : modelStatus.installed
                                ? "Baixar novamente"
                                : "Baixar agora"}
                    </button>
                </div>
            )}

            {error && <div className="banner error" style={{ marginTop: 12, marginBottom: 0 }}>{error}</div>}
        </section>
    );
}

