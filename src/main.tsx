import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Overlay from "./Overlay";
import "./styles.css";

const isOverlay =
    window.location.hash === "#overlay" ||
    window.location.search.includes("overlay=1");

if (isOverlay) {
    // Load overlay-only stylesheet to avoid the main app's reset clobbering
    // the transparent body / pill layout.
    import("./overlay.css");
}

const Root = isOverlay ? Overlay : App;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Root />
    </React.StrictMode>,
);
