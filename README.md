# OSWispa 🎙️

**Open Source Whisper Assistant** - Lightning-fast voice-to-text for your desktop.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform: Linux & macOS](https://img.shields.io/badge/Platform-Linux%20%26%20macOS-green.svg)](#)
[![Status: Release Ready](https://img.shields.io/badge/Status-Release--Ready-blue.svg)](#)

A privacy-focused, local-first voice transcription tool powered by [Whisper.cpp](https://github.com/ggerganov/whisper.cpp). Hold a hotkey, speak, release - your words appear instantly.

---

## 🚧 Project Status & Transparency

**Current State:** v0.4.0 (Alpha)

| Platform | Status | Notes |
|----------|--------|-------|
| **Ubuntu/Debian** | ✅ **Supported** | Primary dev environment. Automated installer with GPU auto-detection. |
| **macOS** | ✅ **Supported** | Audio, hotkeys, clipboard all working. Metal GPU untested. |
| **Fedora/Arch** | ⚠️ **Manual** | Works, but `install.sh` uses `apt`. Manual dependency install required. |
| **Windows** | 🧪 **Experimental** | Theoretically supported, but untested. **Help wanted!** |

### 🛑 Known Limitations
1.  **Installer**: The provided `install.sh` is for **Ubuntu/Debian** (uses `apt`). Other distros: install deps manually.
2.  **Auto-Paste (Linux)**: Uses `ydotool` to simulate typing. Requires `ydotoold` daemon running.
3.  **Global Hotkeys (Linux)**: Reads `/dev/input` directly, requires `input` group membership.
4.  **Global Hotkeys (macOS)**: Requires Accessibility permission in System Settings.

---

## ✨ Features

- **🎤 Push-to-Talk**: Configurable modifiers + optional trigger key
- **🔒 Local-First**: Runs fully local by default with optional VPS backend
- **⚡ GPU Accelerated**: AMD ROCm, NVIDIA CUDA, or Apple Metal support
- **📋 Auto-Paste**: Text is typed directly into your active window
- **🌍 Multilingual**: Supports 99 languages with the right model
- **🧩 Model Flexibility**: Import custom `.bin` / `.gguf` models and switch quickly
- **✍️ Spoken Formatting**: Say commands like `quotation mark` and `new line`

---

## 🤝 Call for Contributors

- [ ] Create installation scripts for **Fedora**, **Arch**, and **macOS** (Homebrew).
- [ ] Test and debug the **Windows** build process.
- [ ] Improve **Wayland** integration without requiring root/input group hacks.
- [ ] Add a proper GUI settings menu (currently experimental).
- [ ] Test **Metal GPU** acceleration on macOS Apple Silicon.

If you can help, please fork the repo and submit a PR! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 🚀 Installation (Ubuntu/Debian)

### Option A (Recommended): Install the `.deb`

1. Download the latest `amd64` `.deb` from GitHub Releases.
2. Install it with `apt` (preferred, pulls dependencies):

```bash
curl -LO https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa_amd64.deb
sudo apt install ./oswispa_amd64.deb
oswispa
```

> **Note:** The `.deb` ships a CPU-only binary. For GPU acceleration, build from source (see below).

### Option B: Build From Source (with GPU auto-detection)

The install script automatically detects your GPU and builds with the right features.

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
./install.sh
```

The installer will:
- Detect AMD ROCm or NVIDIA CUDA and build with GPU acceleration
- Fall back to CPU-only if no GPU toolkit is found
- Create a systemd service with the correct GPU environment variables

**After install:**
1.  Log out and back in (for `input` group permissions).
2.  Ensure `ydotoold` is running for auto-paste (`sudo ydotoold &`).
3.  Press your configured hotkey (default **Ctrl+Super**), speak, and release!

---

## 🍎 Installation (macOS)

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- CMake 3.16+ (`brew install cmake`)
- Xcode Command Line Tools (`xcode-select --install`)

### Build & Run

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
cargo build --release --no-default-features
./target/release/oswispa
```

> **Note:** `--no-default-features` disables the GTK4 GUI (Linux-only). All core features work without it.

### macOS Permissions

OSWispa needs two permissions on macOS:

1. **Microphone**: Granted automatically on first recording attempt.
2. **Accessibility**: Required for global hotkeys. Go to **System Settings > Privacy & Security > Accessibility** and add `oswispa`.

### macOS Limitations (v0.4.0)

- No system tray icon (runs in terminal only)
- No `.pkg` installer (manual build required)
- Metal GPU is available via `--features gpu-metal` but untested
- CPU transcription is the default

---

## 🛠️ Manual Installation (Other Distros/OS)

### 1. Prerequisites (All Platforms)
*   [Rust](https://rustup.rs/) (1.70+)
*   CMake 3.16+
*   `libssl-dev`, `pkg-config`, `libasound2-dev` (Linux)

### 2. GPU Acceleration (Optional but Recommended)

#### AMD GPU (ROCm)

```bash
# Ensure ROCm is installed (6.0+) and hipcc is in PATH
export PATH="/opt/rocm/bin:$PATH"

# Auto-detect GPU architecture
GFX_ARCH=$(rocminfo | grep -oP 'gfx\d+' | head -1)

# Build with HIPBlas
AMDGPU_TARGETS="$GFX_ARCH" cargo build --release --features gpu-hipblas
```

**Common GPU architectures:** `gfx1100` (RX 7900), `gfx1030` (RX 6800), `gfx900` (Vega), `gfx906` (MI50).

**Multi-GPU systems:** Set `HIP_VISIBLE_DEVICES=0` to select a specific GPU. Add `HSA_OVERRIDE_GFX_VERSION=11.0.0` if you get architecture mismatch errors.

#### NVIDIA GPU (CUDA)

```bash
# Ensure CUDA toolkit is installed and nvcc is in PATH
cargo build --release --features gpu-cuda
```

**Multi-GPU systems:** Set `CUDA_VISIBLE_DEVICES=0` to select a specific GPU.

#### macOS (Metal)

```bash
cargo build --release --features gpu-metal
```

### 3. Build & Run (CPU-only)
```bash
cargo build --release
./target/release/oswispa
```

---

## 📥 Models

OSWispa needs a model file to work. Save models to `~/.local/share/oswispa/models/`.

| Model | Size | Speed | GPU VRAM | Recommendation |
|-------|------|-------|----------|----------------|
| `tiny.en` | 75MB | ⚡⚡⚡⚡ | <1GB | Quick testing |
| `base.en` | 142MB | ⚡⚡⚡ | <1GB | Fast dictation |
| `small.en` | 466MB | ⚡⚡ | ~1GB | Good accuracy |
| `medium.en` | 1.5GB | ⚡ | ~2.5GB | **Good balance** |
| `large-v3-turbo` | 1.6GB | ⚡⚡ | ~3GB | **Best speed/accuracy** |
| `large-v3` | 2.9GB | 🐢 | ~5GB | Highest accuracy, multilingual |

**Download:** [Hugging Face ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp/tree/main)

**GPU model recommendations:**
- **<4GB VRAM**: `base.en` or `small.en`
- **4-8GB VRAM**: `medium.en` or `large-v3-turbo`
- **8GB+ VRAM**: `large-v3-turbo` (recommended) or `large-v3`

---

## 🌐 Optional VPS Backend

OSWispa is local-first by default, but you can optionally route transcription to a VPS.

- Set backend mode to `remote` in Settings.
- Use an HTTPS endpoint (HTTP is blocked unless explicitly allowed).
- Store API tokens via Settings (saved to a local `0600` secret file) or set an env var (e.g. `OSWISPA_REMOTE_API_KEY`).
- If remote transcription fails and local models exist, OSWispa falls back to local automatically.

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
*   Usually a GPU driver mismatch. Try building without GPU features to test CPU mode first.

**macOS: "Hotkeys don't work"**
*   Grant Accessibility permission: **System Settings > Privacy & Security > Accessibility**.

---

## 📜 License

MIT License - Copyright (c) 2026 Tyler Casey. See [LICENSE](LICENSE) for details.

---

**Made with ❤️ for the open source community.**
