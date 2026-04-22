import { useEffect, useRef, useState } from "react";

interface Props {
    value: string;
    onChange: (accelerator: string) => void;
    disabled?: boolean;
}

const MOD_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

function eventToAccelerator(e: KeyboardEvent): string {
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");

    if (!MOD_KEYS.has(e.key)) {
        let key = e.key;
        if (key === " ") key = "Space";
        else if (key.length === 1) key = key.toUpperCase();
        parts.push(key);
    }

    return parts.join("+");
}

function renderKeys(accelerator: string) {
    if (!accelerator) return <span className="hotkey-empty">Não definido</span>;
    const parts = accelerator.split("+");
    return (
        <span className="hotkey-keys">
            {parts.map((p, i) => (
                <span key={`${p}-${i}`} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    {i > 0 && <span className="plus">+</span>}
                    <kbd>{p}</kbd>
                </span>
            ))}
        </span>
    );
}

/** Captures a global-shortcut accelerator string from key presses.
 *  Click to focus, then press the desired combination. */
export function HotkeyRecorder({ value, onChange, disabled }: Props) {
    const [recording, setRecording] = useState(false);
    const [draft, setDraft] = useState(value);
    const ref = useRef<HTMLDivElement>(null);

    useEffect(() => setDraft(value), [value]);

    useEffect(() => {
        if (!recording) return;
        const handler = (e: KeyboardEvent) => {
            e.preventDefault();
            const acc = eventToAccelerator(e);
            setDraft(acc);
            if (e.key === "Escape") {
                setRecording(false);
                setDraft(value);
                return;
            }
            if (!MOD_KEYS.has(e.key)) {
                setRecording(false);
                onChange(acc);
            }
        };
        window.addEventListener("keydown", handler, true);
        return () => window.removeEventListener("keydown", handler, true);
    }, [recording, value, onChange]);

    return (
        <div
            ref={ref}
            className={`hotkey-recorder ${recording ? "recording" : ""}`}
            onClick={() => !disabled && setRecording(true)}
            role="button"
            tabIndex={0}
        >
            {renderKeys(draft)}
            <span className="hotkey-hint">
                {recording ? "Pressione a combinação… (Esc cancela)" : "Clique para alterar"}
            </span>
        </div>
    );
}

