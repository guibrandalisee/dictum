import { useState } from "react";
import { AppConfig, Language, WhisperModel } from "../lib/api";
import { useAppStore } from "../stores/appStore";
import { HotkeyRecorder } from "./HotkeyRecorder";

function Toggle({
    checked,
    onChange,
}: {
    checked: boolean;
    onChange: (v: boolean) => void;
}) {
    return (
        <label className="toggle">
            <input
                type="checkbox"
                checked={checked}
                onChange={(e) => onChange(e.target.checked)}
            />
            <span className="toggle-slider" />
        </label>
    );
}

export function SettingsPanel() {
    const { config, microphones, refreshMicrophones, saveConfig, errorBanner } =
        useAppStore();
    const [draft, setDraft] = useState<AppConfig | null>(null);
    const [saving, setSaving] = useState(false);

    const current = draft ?? config;
    if (!current) return <p>Carregando…</p>;

    const update = (patch: Partial<AppConfig>) => {
        setDraft({ ...current, ...patch });
    };

    const save = async () => {
        if (!draft) return;
        setSaving(true);
        try {
            await saveConfig(draft);
            setDraft(null);
        } catch {
            // banner shows error
        } finally {
            setSaving(false);
        }
    };

    const dirty = draft !== null;

    return (
        <section className="card">
            <div className="card-head">
                <div>
                    <h2 className="card-title">Configurações</h2>
                    <p className="card-sub">Ajuste como o ditado se comporta</p>
                </div>
            </div>

            {errorBanner && <div className="banner error">{errorBanner}</div>}

            <div className="field">
                <label className="field-label">Atalho push-to-talk</label>
                <HotkeyRecorder
                    value={current.hotkey}
                    onChange={(acc) => update({ hotkey: acc })}
                />
                <small className="field-help">
                    O Windows nem sempre aceita combinações só de modificadores (ex.: Ctrl+Alt).
                    Se não funcionar, tente algo como <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Space</kbd>.
                </small>
            </div>

            <div className="field">
                <label className="field-label">Idioma</label>
                <select
                    value={current.language}
                    onChange={(e) => update({ language: e.target.value as Language })}
                >
                    <option value="auto">Detectar automaticamente</option>
                    <option value="pt">Português</option>
                    <option value="en">English</option>
                </select>
            </div>

            <div className="field">
                <label className="field-label">Modelo Whisper</label>
                <select
                    value={current.model}
                    onChange={(e) => update({ model: e.target.value as WhisperModel })}
                >
                    <option value="tiny">tiny — mais rápido, menos preciso</option>
                    <option value="base">base — equilibrado (recomendado)</option>
                    <option value="small">small — mais preciso, mais lento</option>
                </select>
            </div>

            <div className="field">
                <label className="field-label">Microfone</label>
                <div className="row-input">
                    <select
                        value={current.microphone ?? ""}
                        onChange={(e) =>
                            update({ microphone: e.target.value || null })
                        }
                    >
                        <option value="">Padrão do sistema</option>
                        {microphones.map((m) => (
                            <option key={m} value={m}>
                                {m}
                            </option>
                        ))}
                    </select>
                    <button className="btn btn-secondary btn-sm" onClick={refreshMicrophones}>
                        Atualizar
                    </button>
                </div>
            </div>

            <div className="field">
                <label className="field-label">
                    <span>Duração máxima por gravação</span>
                    <span className="field-value">{current.max_recording_seconds}s</span>
                </label>
                <input
                    type="range"
                    min={5}
                    max={120}
                    value={current.max_recording_seconds}
                    onChange={(e) =>
                        update({ max_recording_seconds: parseInt(e.target.value, 10) })
                    }
                />
            </div>

            <div className="field-row">
                <div className="field-row-text">
                    <span className="field-label">Iniciar com o Windows</span>
                    <span className="field-help">O app inicia em segundo plano e fica na bandeja.</span>
                </div>
                <Toggle
                    checked={current.auto_start}
                    onChange={(v) => update({ auto_start: v })}
                />
            </div>

            <div className="field-row">
                <div className="field-row-text">
                    <span className="field-label">Guardar histórico local</span>
                    <span className="field-help">As transcrições ficam salvas no seu computador.</span>
                </div>
                <Toggle
                    checked={current.keep_history}
                    onChange={(v) => update({ keep_history: v })}
                />
            </div>

            <div className="row end" style={{ marginTop: 18 }}>
                {dirty && (
                    <button className="btn btn-ghost" onClick={() => setDraft(null)}>
                        Descartar
                    </button>
                )}
                <button className="btn" onClick={save} disabled={!dirty || saving}>
                    {saving ? "Salvando…" : "Salvar alterações"}
                </button>
            </div>
        </section>
    );
}

