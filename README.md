# Dictum

**Dictum** is a lightweight, push-to-talk dictation application for Windows, powered by local Whisper transcription. It allows users to seamlessly transcribe speech to text across any application using a customizable global hotkey, processing the audio entirely on your local machine for maximum privacy and performance.

---

> **Note on AI Usage:** 
> The development and source code of this project were highly assisted and generated utilizing Artificial Intelligence (AI) tools. While the architecture and specific requirements correspond to the project's goals, much of the underlying logic and structure was autonomously written by AI.

## ✨ Features

- **Push-to-Talk Dictation:** Press and hold a customizable global hotkey to start recording; release to instantly transcribe.
- **Local AI Processing:** Powered by local Whisper models, ensuring that your voice data never leaves your computer.
- **Auto-Paste Functionality:** Transcribed text can be automatically pasted into your active window or saved to your clipboard.
- **Model Management:** Download and manage different hardware-accelerated Whisper models directly from the UI.
- **System Tray Integration:** Runs quietly in the background (system tray) with minimal resource usage.
- **History Panel:** View and manage previously transcribed audio and text.

## 🛠️ Tech Stack

This project is built using modern, performant frameworks and languages:
- **Frontend:** [React](https://reactjs.org/) + [TypeScript](https://www.typescriptlang.org/), styled with custom CSS and bundled via [Vite](https://vitejs.dev/).
- **State Management:** [Zustand](https://github.com/pmndrs/zustand).
- **Backend/Desktop Integration:** [Tauri](https://tauri.app/) (Rust 🦀), leveraging its plugins for global shortcuts, clipboard management, autostart, and system tray functionality.

## 🚀 Getting Started

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- Required C++ build tools for Windows (via Visual Studio Installer).

### Installation & Build

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/dictum.git
   cd dictum
   ```

2. **Install frontend dependencies:**
   ```bash
   npm install
   ```

3. **Run in development mode:**
   This command starts the Vite dev server and the Tauri application concurrently.
   ```bash
   npm run tauri:dev
   ```

4. **Build for production:**
   To compile the TypeScript project, build the web assets, and bundle the executables:
   ```bash
   npm run tauri:build
   ```
   The generated executable will be located in `src-tauri/target/release/bundle/`.

## ⚙️ Configuration

- Upon launching for the first time, Dictum will prompt you to download the base Whisper model.
- You can access the **Settings Panel** through the system tray right-click menu or the main app window to configure hotkeys, audio sources, and model preferences.