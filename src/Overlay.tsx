import { useEffect, useRef, useState } from "react";
import {
    currentMonitor,
    cursorPosition,
    getCurrentWindow,
    monitorFromPoint,
    PhysicalPosition,
} from "@tauri-apps/api/window";
import { events } from "./lib/events";
import type { PipelineState } from "./lib/api";

type Phase = "idle" | "recording" | "processing" | "done" | "error";

const HIDE_DELAY_MS = 2600;

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

async function positionOverlay() {
    try {
        const win = getCurrentWindow();
        const cursor = await cursorPosition().catch(() => null);
        const monitorByCursor = cursor
            ? await monitorFromPoint(cursor.x, cursor.y).catch(() => null)
            : null;
        const monitor = monitorByCursor ?? (await currentMonitor());
        if (!monitor) return;

        const innerSize = await win.innerSize();
        const margin = 32;
        const x = monitor.position.x + Math.max(0, monitor.size.width - innerSize.width - margin);
        const y = monitor.position.y + Math.max(0, monitor.size.height - innerSize.height - margin * 2);
        await win.setPosition(new PhysicalPosition(x, y));
    } catch {
        // ignore
    }
}

async function showOverlay() {
    try {
        const win = getCurrentWindow();
        await positionOverlay();
        await win.show();
        await win.setAlwaysOnTop(true);
    } catch {
        // ignore
    }
}

async function hideOverlay() {
    try {
        await getCurrentWindow().hide();
    } catch {
        // ignore
    }
}

export default function Overlay() {
    const [phase, setPhase] = useState<Phase>("idle");
    const [recordingMs, setRecordingMs] = useState(0);
    const [processingMs, setProcessingMs] = useState(0);
    const [errorMsg, setErrorMsg] = useState<string | null>(null);

    const recordingStartRef = useRef<number | null>(null);
    const processingStartRef = useRef<number | null>(null);
    const tickRef = useRef<number | null>(null);
    const hideTimerRef = useRef<number | null>(null);

    useEffect(() => {
        const promise = events.onPipelineState((state: PipelineState) => {
            if (hideTimerRef.current) {
                window.clearTimeout(hideTimerRef.current);
                hideTimerRef.current = null;
            }

            switch (state.state) {
                case "recording": {
                    setErrorMsg(null);
                    setPhase("recording");
                    recordingStartRef.current = performance.now();
                    setRecordingMs(0);
                    setProcessingMs(0);
                    showOverlay();
                    break;
                }
                case "processing": {
                    setPhase("processing");
                    processingStartRef.current = performance.now();
                    if (recordingStartRef.current != null) {
                        setRecordingMs(performance.now() - recordingStartRef.current);
                    }
                    setProcessingMs(0);
                    showOverlay();
                    break;
                }
                case "done": {
                    setPhase("done");
                    setRecordingMs(state.recording_ms);
                    setProcessingMs(state.processing_ms);
                    showOverlay();
                    hideTimerRef.current = window.setTimeout(() => {
                        hideOverlay();
                        setPhase("idle");
                    }, HIDE_DELAY_MS);
                    break;
                }
                case "error": {
                    setPhase("error");
                    setErrorMsg(state.message);
                    showOverlay();
                    hideTimerRef.current = window.setTimeout(() => {
                        hideOverlay();
                        setPhase("idle");
                    }, HIDE_DELAY_MS + 800);
                    break;
                }
                case "idle": {
                    setPhase("idle");
                    hideOverlay();
                    break;
                }
            }
        });
        return () => {
            promise.then((un) => un()).catch(() => { });
        };
    }, []);

    // Live ticker for the recording timer.
    useEffect(() => {
        if (phase === "recording") {
            const tick = () => {
                if (recordingStartRef.current != null) {
                    setRecordingMs(performance.now() - recordingStartRef.current);
                }
                tickRef.current = window.setTimeout(tick, 100);
            };
            tickRef.current = window.setTimeout(tick, 100);
            return () => {
                if (tickRef.current) window.clearTimeout(tickRef.current);
            };
        }
        if (phase === "processing") {
            const tick = () => {
                if (processingStartRef.current != null) {
                    setProcessingMs(performance.now() - processingStartRef.current);
                }
                tickRef.current = window.setTimeout(tick, 100);
            };
            tickRef.current = window.setTimeout(tick, 100);
            return () => {
                if (tickRef.current) window.clearTimeout(tickRef.current);
            };
        }
    }, [phase]);

    const label = (() => {
        switch (phase) {
            case "recording": return "Ouvindo";
            case "processing": return "Transcrevendo";
            case "done": return "Pronto";
            case "error": return "Erro";
            default: return "Dictum";
        }
    })();

    const timeText = (() => {
        switch (phase) {
            case "recording": return formatDuration(recordingMs);
            case "processing": return formatDuration(processingMs);
            case "done": return `${formatDuration(recordingMs)} · ${formatDuration(processingMs)}`;
            case "error": return errorMsg ?? "Falhou";
            default: return "";
        }
    })();

    return (
        <div className={`ovl ovl-${phase}`} data-tauri-drag-region>
            <div className="ovl-orb">
                {phase === "recording" && (
                    <div className="ovl-bars">
                        <span /><span /><span /><span /><span />
                    </div>
                )}
                {phase === "processing" && <div className="ovl-spin" />}
                {phase === "done" && (
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                    </svg>
                )}
                {phase === "error" && (
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                )}
                {phase === "idle" && <div className="ovl-dot" />}
            </div>
            <div className="ovl-text">
                <div className="ovl-label">{label}</div>
                {timeText && <div className="ovl-time">{timeText}</div>}
            </div>
        </div>
    );
}
