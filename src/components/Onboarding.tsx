import { useState } from "react";
import { api } from "../lib/api";
import { useAppStore } from "../stores/appStore";

type Step = "welcome" | "model" | "mic" | "hotkey" | "done";

const STEPS: Step[] = ["welcome", "model", "mic", "hotkey", "done"];

function Stepper({ current }: { current: Step }) {
    const idx = STEPS.indexOf(current);
    return (
        <div className="stepper" aria-hidden>
            {STEPS.map((s, i) => (
                <span
                    key={s}
                    className={`stepper-dot ${i === idx ? "active" : i < idx ? "done" : ""}`}
                />
            ))}
        </div>
    );
}

export function Onboarding({ onClose }: { onClose: () => void }) {
    const { config, modelStatus, downloading, refreshModel, saveConfig, refreshMicrophones } =
        useAppStore();
    const [step, setStep] = useState<Step>("welcome");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const finish = async () => {
        if (!config) return;
        setBusy(true);
        try {
            await saveConfig({ ...config, onboarded: true });
            onClose();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
        }
    };

    const downloadAndContinue = async () => {
        setBusy(true);
        setError(null);
        try {
            await api.downloadModel();
            await refreshModel();
            setStep("mic");
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
        }
    };

    return (
        <div className="modal-backdrop">
            <div className="modal">
                <Stepper current={step} />

                {step === "welcome" && (
                    <>
                        <h2>Bem-vindo ao Dictum</h2>
                        <p>
                            Esse app fica rodando em segundo plano. Quando você segura o atalho,
                            ele grava o que você fala; quando solta, transcreve com o Whisper{" "}
                            <strong>rodando localmente</strong> e cola no campo focado.
                        </p>
                        <p style={{ color: "var(--text-3)", fontSize: "0.85rem" }}>
                            Tudo offline depois do download inicial. Nada é enviado para a internet.
                        </p>
                        <div className="row">
                            <button className="btn" onClick={() => setStep("model")}>Vamos lá</button>
                        </div>
                    </>
                )}

                {step === "model" && (
                    <>
                        <h2>1. Baixar o modelo</h2>
                        <p>
                            Modelo selecionado: <strong>{modelStatus?.model}</strong>. Tamanho aproximado:{" "}
                            <strong>{((modelStatus?.size_bytes ?? 0) / 1024 / 1024).toFixed(0)} MB</strong>.
                        </p>
                        {downloading && (
                            <div className="progress">
                                <div className="progress-bar-wrap">
                                    <div
                                        className="progress-bar"
                                        style={{
                                            width: `${Math.round(
                                                (downloading.downloaded / downloading.total) * 100,
                                            )}%`,
                                        }}
                                    />
                                </div>
                            </div>
                        )}
                        {modelStatus?.installed ? (
                            <>
                                <p><span className="badge ok">Já instalado</span></p>
                                <div className="row">
                                    <button className="btn" onClick={() => setStep("mic")}>Continuar</button>
                                </div>
                            </>
                        ) : (
                            <div className="row">
                                <button className="btn btn-ghost" onClick={() => setStep("mic")}>
                                    Pular
                                </button>
                                <button className="btn" onClick={downloadAndContinue} disabled={busy}>
                                    {busy ? "Baixando…" : "Baixar agora"}
                                </button>
                            </div>
                        )}
                        {error && <div className="banner error" style={{ marginTop: 12, marginBottom: 0 }}>{error}</div>}
                    </>
                )}

                {step === "mic" && (
                    <>
                        <h2>2. Microfone</h2>
                        <p>
                            Vamos usar o microfone padrão do sistema. Você pode alterar
                            depois nas Configurações.
                        </p>
                        <div className="row">
                            <button
                                className="btn btn-ghost"
                                onClick={async () => {
                                    await refreshMicrophones();
                                }}
                            >
                                Listar microfones
                            </button>
                            <button className="btn" onClick={() => setStep("hotkey")}>Continuar</button>
                        </div>
                    </>
                )}

                {step === "hotkey" && (
                    <>
                        <h2>3. Atalho</h2>
                        <p>
                            O atalho atual é <kbd>{config?.hotkey}</kbd>. Você pode alterar a
                            qualquer momento na aba <strong>Configurações</strong>.
                        </p>
                        <div className="row">
                            <button className="btn" onClick={() => setStep("done")}>Continuar</button>
                        </div>
                    </>
                )}

                {step === "done" && (
                    <>
                        <h2>Tudo pronto 🎉</h2>
                        <p>
                            Segure <kbd>{config?.hotkey}</kbd>, fale algo e solte. O texto
                            será colado no campo focado.
                        </p>
                        <p style={{ color: "var(--text-3)", fontSize: "0.85rem" }}>
                            Esta janela pode ser fechada — o app continua rodando na bandeja.
                        </p>
                        <div className="row">
                            <button className="btn" onClick={finish} disabled={busy}>
                                Concluir
                            </button>
                        </div>
                    </>
                )}
            </div>
        </div>
    );
}

