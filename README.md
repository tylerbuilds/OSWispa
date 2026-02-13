# OSWispa 🎙️

**Open Source Whisper Assistant** - Lightning-fast voice-to-text for your desktop.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-green.svg)](#)
[![Status: Release Ready](https://img.shields.io/badge/Status-Release--Ready-blue.svg)](#)

A privacy-focused, locally-running voice transcription tool powered by [Whisper.cpp](https://github.com/ggerganov/whisper.cpp). Hold a hotkey, speak, release - your words appear instantly.

---

## 🚧 Project Status & Transparency

**Current State:** v0.1 (Alpha)

OSWispa is a **Linux-first** project, currently optimized for **Ubuntu/Debian** systems.

| Platform | Status | Notes |
|----------|--------|-------|
| **Ubuntu/Debian** | ✅ **Supported** | Primary dev environment. Automated installer available. |
| **Fedora/Arch** | ⚠️ **Manual** | Works, but `install.sh` uses `apt`. Manual dependency install required. |
| **macOS** | 🧪 **Experimental** | Theoretically supported via Metal, but untested. **Help wanted!** |
| **Windows** | 🧪 **Experimental** | Theoretically supported, but untested. **Help wanted!** |

### 🛑 Known Limitations
1.  **Installer**: The provided `install.sh` is strictly for **Ubuntu/Debian** (uses `apt`). Users on other distros must install dependencies manually (see below).
2.  **Auto-Paste Friction**: On Wayland/Linux, we use `ydotool` to simulate typing. This requires a background daemon (`ydotoold`) running, often as root. If text doesn't appear, this is usually why.
3.  **Global Hotkeys**: Wayland security model makes global hotkeys hard. We read directly from `/dev/input`, which requires your user to be in the `input` group.

---

## ✨ Features

- **🎤 Push-to-Talk**: Hold `Ctrl+Super` to record, release to transcribe
- **🔒 100% Local**: No cloud APIs, no data leaves your machine
- **⚡ GPU Accelerated**: AMD ROCm, NVIDIA CUDA, or Apple Metal support
- **📋 Auto-Paste**: Text is typed directly into your active window
- **🌍 Multilingual**: Supports 99 languages with the right model

---

## 🤝 Call for Contributors

We need your help to make OSWispa truly cross-platform! We are actively looking for contributions to:

- [ ] Create installation scripts for **Fedora**, **Arch**, and **macOS** (Homebrew).
- [ ] Test and debug the **Windows** build process.
- [ ] Improve the specific **Wayland** integration without requiring root/input group hacks.
- [ ] Add a proper GUI settings menu (currently experimental).

If you can help, please fork the repo and submit a PR! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 🚀 Installation (Ubuntu/Debian)

The easiest way to get started on Ubuntu 22.04/24.04:

```bash
# Clone and install
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
./install.sh

# Run
oswispa
```

**After install:**
1.  Log out and back in (to refresh user groups).
2.  Ensure `ydotoold` is running if you want auto-paste (`sudo ydotoold &`).
3.  Press **Ctrl+Super**, speak, and release!

---

## 🛠️ Manual Installation (Other Distros/OS)

### 1. Prerequisites (All Platforms)
*   [Rust](https://rustup.rs/) (1.70+)
*   CMake 3.16+
*   `libssl-dev`, `pkg-config`, `libasound2-dev` (Linux)

### 2. GPU Acceleration (Optional but Recommended)

#### AMD GPU (ROCm)
Required for fast transcription on AMD cards.
1.  Install ROCm (6.0+).
2.  Build with ROCm feature: `AMDGPU_TARGETS="gfx1100" cargo build --release --features gpu-hipblas` (adjust `gfx...` for your card).

#### NVIDIA GPU (CUDA)
1.  Install CUDA Toolkit.
2.  Build: `cargo build --release --features gpu-cuda`.

#### macOS (Metal)
1.  Build: `cargo build --release --features gpu-metal`.

### 3. Build & Run
```bash
cargo build --release
./target/release/oswispa
```

---

## 📥 Models

OSWispa needs a model file to work.

| Model | Size | Speed | Recommendation |
|-------|------|-------|----------------|
| `base.en` | 142MB | ⚡⚡⚡ | Fast dictation |
| `medium.en` | 1.5GB | ⚡ | **Good balance** |
| `distil-large-v3` | 1.5GB | ⚡ | **Best performance/size** |
| `large-v3` | 2.9GB | 🐢 | High accuracy, multilingual |

**Manual Download:**
Save models to `~/.local/share/oswispa/models/`.
Download links: [Hugging Face ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp/tree/main)

---

## 🔧 Troubleshooting

**"It says recording but nothing happens."**
*   Check if `ydotoold` is running: `pgrep ydotoold`.
*   Try manual paste: The text is also copied to your clipboard. Press `Ctrl+V`.

**"Permission denied accessing /dev/input/..."**
*   You need to be in the `input` group.
    *   Run: `sudo usermod -aG input $USER`
    *   **Log out and log back in.**

**"The app crashes with a 'segmentation fault'."**
*   Usually a GPU driver mismatch. Try building without GPU features (default `Cargo.toml`) to test CPU mode first.

---

## 📜 License

MIT License - Copyright (c) 2026 Tyler Casey. See [LICENSE](LICENSE) for details.

---

**Made with ❤️ for the open source community.**
