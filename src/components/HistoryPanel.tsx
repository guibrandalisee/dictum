import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/api";
import { useAppStore } from "../stores/appStore";

const CopyIcon = () => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <rect x="9" y="9" width="13" height="13" rx="2" />
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
);

const TrashIcon = () => (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="3 6 5 6 21 6" />
        <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
        <path d="M10 11v6M14 11v6" />
        <path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
    </svg>
);

const HistoryIcon = () => (
    <svg className="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
        <polyline points="3 3 3 8 8 8" />
        <polyline points="12 7 12 12 15 14" />
    </svg>
);

export function HistoryPanel() {
    const { history, refreshHistory } = useAppStore();

    const clear = async () => {
        if (!confirm("Apagar todo o histórico?")) return;
        await api.clearHistory();
        await refreshHistory();
    };

    const deleteOne = async (entry: (typeof history)[number]) => {
        const ok = await api.deleteHistoryEntry(entry);
        if (!ok) {
            alert("Não foi possível excluir este item (ele pode já ter sido removido).");
        }
        await refreshHistory();
    };

    if (history.length === 0) {
        return (
            <section className="card">
                <div className="card-head">
                    <div>
                        <h2 className="card-title">Histórico</h2>
                        <p className="card-sub">Suas transcrições recentes ficam aqui</p>
                    </div>
                </div>
                <div className="empty">
                    <HistoryIcon />
                    <div className="empty-title">Nenhuma transcrição ainda</div>
                    <div className="empty-sub">
                        Use o atalho push-to-talk para gravar e ver suas transcrições aparecerem por aqui.
                    </div>
                </div>
            </section>
        );
    }

    return (
        <section className="card">
            <div className="card-head">
                <div>
                    <h2 className="card-title">Histórico</h2>
                    <p className="card-sub">{history.length} transcriç{history.length === 1 ? "ão" : "ões"}</p>
                </div>
                <button className="btn btn-ghost btn-sm" onClick={clear}>
                    Limpar tudo
                </button>
            </div>
            <ul className="history-list">
                {[...history].reverse().map((entry, i) => (
                    <li key={`${entry.timestamp}-${i}`} className="history-item">
                        <div className="history-meta">
                            <div className="history-meta-left">
                                <span>{new Date(entry.timestamp).toLocaleString()}</span>
                                {entry.language && (
                                    <>
                                        <span className="dot" />
                                        <span>{entry.language}</span>
                                    </>
                                )}
                                {entry.duration_ms != null && (
                                    <>
                                        <span className="dot" />
                                        <span>{(entry.duration_ms / 1000).toFixed(1)}s</span>
                                    </>
                                )}
                            </div>
                            <div className="history-actions">
                                <button
                                    className="btn-icon"
                                    onClick={() => writeText(entry.text)}
                                    title="Copiar"
                                >
                                    <CopyIcon />
                                </button>
                                <button
                                    className="btn-icon danger"
                                    onClick={() => void deleteOne(entry)}
                                    title="Excluir"
                                >
                                    <TrashIcon />
                                </button>
                            </div>
                        </div>
                        <p className="history-text">{entry.text}</p>
                    </li>
                ))}
            </ul>
        </section>
    );
}

