# OSWispa 🎙️

**Open Source Whisper Assistant** - Lightning-fast voice-to-text for your desktop.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform: Linux, macOS & Windows](https://img.shields.io/badge/Platform-Linux%2C%20macOS%20%26%20Windows-green.svg)](#)
[![Status: Alpha](https://img.shields.io/badge/Status-Alpha-orange.svg)](#)

A privacy-focused, local-first voice transcription tool powered by [Whisper.cpp](https://github.com/ggerganov/whisper.cpp). Hold a hotkey, speak, release - your words appear instantly.

[Support](SUPPORT.md) · [Privacy](PRIVACY.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [July 2026 audit](docs/AUDIT-2026-07-18.md)

---

## 🚧 Project Status & Transparency

**Current State:** v0.4.2 (Alpha)

The [v0.4.2 release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.4.2) contains the July security and reliability audit fixes, lower-floor Linux packages, VM-tested macOS packages, and the first functional Windows package.

| Platform | Status | Notes |
|----------|--------|-------|
| **Ubuntu/Debian** | ✅ **Supported** | Primary dev environment. Automated installer with GPU auto-detection. |
| **macOS** | ✅ **Supported** | Audio, hotkeys, clipboard all working. Metal GPU untested. |
| **Fedora/Arch** | ✅ **Supported** | Automated installer with `dnf`/`pacman` support. |
| **Windows** | ✅ **Supported (Alpha)** | WASAPI audio, global hotkeys, clipboard and text insertion. VM-tested x86-64 ZIP. |

### 🛑 Known Limitations
1.  **Installer**: The source installer supports Ubuntu/Debian, Fedora/RHEL, Arch/Manjaro, and macOS. Windows uses the release ZIP.
2.  **Auto-Paste (Linux)**: Uses `ydotool` to simulate typing. The source installer enables its user service.
3.  **Global Hotkeys (Linux)**: Reads `/dev/input` directly, requires `input` group membership.
4.  **Global Hotkeys (macOS)**: Requires Accessibility permission in System Settings.
5.  **macOS Packaging**: The current app bundle is not signed or notarised.
6.  **Windows Packaging**: The ZIP is not code-signed and has no tray UI yet; keep its console window open.

---

## ✨ Features

- **🎤 Push-to-Talk**: Configurable modifiers + optional trigger key
- **🔒 Local-First**: Runs fully local by default with optional VPS backend
- **⚡ GPU Accelerated**: AMD ROCm, NVIDIA CUDA, or Apple Metal support
- **📋 Auto-Paste**: Text is typed directly into your active window
- **🌍 Multilingual**: Supports 99 languages with the right model
- **🧩 Model Flexibility**: Import custom `.bin` / `.gguf` models and switch quickly
- **✍️ Spoken Formatting**: Say commands like `quotation mark` and `new line`
- **📖 Personal Dictionary**: Keep explicit names and phrases spelt correctly, entirely locally

---

## 🤝 Call for Contributors

- [x] ~~Create installation scripts for **Fedora**, **Arch**, and **macOS**~~ (done).
- [ ] Add a signed **Windows MSIX/MSI installer** and native tray UI.
- [ ] Improve **Wayland** integration without requiring root/input group hacks.
- [ ] Add a proper GUI settings menu (currently experimental).
- [ ] Test **Metal GPU** acceleration on macOS Apple Silicon.

If you can help, please fork the repo and submit a PR! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 🚀 Installation (Linux)

### Option A: Install the `.deb`

The v0.4.2 Linux artefacts are built on Ubuntu 22.04, lowering the compatibility floor to GLIBC 2.35.

1. Download the latest `amd64` `.deb` from GitHub Releases.
2. Install it with `apt` (preferred, pulls dependencies):

```bash
curl -LO https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa_amd64.deb
sudo apt install ./oswispa_amd64.deb
oswispa
```

> **Note:** The `.deb` ships a CPU-only binary. For GPU acceleration, build from source (see below).

### Option B: Install the `.rpm`

```bash
curl -LO https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa_x86_64.rpm
sudo dnf install ./oswispa_x86_64.rpm
oswispa
```

The release page includes `SHA256SUMS` covering every downloadable asset.

### Option C (Recommended for GPU builds): Build From Source

The install script automatically detects your GPU and builds with the right features.

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
./install.sh
```

The installer will:
- Detect every AMD ROCm architecture or NVIDIA CUDA and build with GPU acceleration
- Select the ROCm device with the most VRAM instead of assuming GPU 0
- Validate model downloads before atomically installing them
- Fall back to CPU-only if no GPU toolkit is found
- Create, enable, and start a systemd user service with the correct GPU environment variables

**After install:**
1.  If the installer added you to the `input` group, log out and back in.
2.  Confirm auto-paste is ready with `systemctl --user status ydotoold`.
3.  Press your configured hotkey (default **Ctrl+Super**), speak, and release!

---

## 🍎 Installation (macOS)

### Option A (Recommended): Download the App Package

1. Download the matching macOS DMG from GitHub Releases: `oswispa-macos-arm64.dmg` for Apple Silicon or `oswispa-macos-x86_64.dmg` for Intel.
2. Open the `.dmg`.
3. Drag `OSWispa.app` into `Applications`.
4. Open `OSWispa` from `Applications`.
5. If macOS blocks the first launch, Control-click `OSWispa.app` and choose `Open`.

> **Note:** The packaged macOS app launches OSWispa in Terminal so you can see setup prompts and status output. Apple Silicon builds use Metal; Intel builds stay on the CPU path.

### Option B: Install Script

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
./install.sh
```

The installer will:
- Install Xcode CLT and Homebrew (if needed) + cmake
- Auto-detect Apple Silicon and build with Metal GPU acceleration
- Download and validate the base English Whisper model
- Create a LaunchAgent for auto-start on login

### Option C: Manual Build

```bash
brew install cmake
cargo build --release --no-default-features
./target/release/oswispa
```

> **Note:** `--no-default-features` disables the GTK4 GUI (Linux-only). All core features work without it.

### macOS Permissions

OSWispa needs two permissions on macOS:

1. **Microphone**: Granted automatically on first recording attempt.
2. **Accessibility**: Required for global hotkeys. Go to **System Settings > Privacy & Security > Accessibility** and add `oswispa`.

### macOS Limitations (v0.4.2)

- No native menu bar/tray UI yet; the packaged app launches OSWispa in Terminal
- No signed/notarized installer yet, so first launch may require Control-click > Open
- Apple Silicon packages use Metal; Intel packages use CPU transcription
- First launch auto-selects a model, but power users can still override it with `OSWISPA_SETUP_MANUAL=1`

---

## 🪟 Installation (Windows)

1. Download [`oswispa-windows-x86_64.zip`](https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa-windows-x86_64.zip).
2. Extract the complete ZIP into a permanent folder.
3. Run `oswispa.exe` and keep its console window open.
4. Allow desktop-app microphone access if Windows prompts for it.
5. Hold **Ctrl+Windows**, speak, then release the keys.

PowerShell installation:

```powershell
Invoke-WebRequest https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa-windows-x86_64.zip -OutFile oswispa-windows-x86_64.zip
Expand-Archive .\oswispa-windows-x86_64.zip -DestinationPath .\OSWispa
Set-Location .\OSWispa
.\oswispa.exe
```

> **Note:** This alpha package is not code-signed. Windows SmartScreen may require **More info → Run anyway**. There is no native tray UI yet.

---

## 🛠️ Manual Installation (Other Distros/OS)

### 1. Prerequisites (All Platforms)
*   Current stable [Rust](https://rustup.rs/)
*   CMake 3.16+
*   `libssl-dev`, `pkg-config`, `libasound2-dev` (Linux)

### 2. GPU Acceleration (Optional but Recommended)

#### AMD GPU (ROCm)

```bash
# Ensure ROCm is installed (6.0+) and hipcc is in PATH
export PATH="/opt/rocm/bin:$PATH"

# Compile for every architecture visible to ROCm (including names such as gfx90a)
GFX_ARCHES=$(rocminfo | grep -oE 'gfx[0-9a-f]+' | sort -u | paste -sd ';' -)

# Build with HIPBlas
AMDGPU_TARGETS="$GFX_ARCHES" cargo build --release --features gpu-hipblas
```

**Common GPU architectures:** `gfx1100` (RX 7900), `gfx1030` (RX 6800), `gfx900` (Vega), `gfx906` (MI50).

**Multi-GPU systems:** Set `ROCR_VISIBLE_DEVICES` to the intended ROCm index if needed. Do not set `HSA_OVERRIDE_GFX_VERSION` automatically; it can disguise an incompatible build and should only be used when your ROCm/driver guidance explicitly requires it.

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

OSWispa needs a model file to work. Managed models live in `~/.local/share/oswispa/models/` on Linux and `~/Library/Application Support/com.oswispa.OSWispa/models/` on macOS.

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
- Remote mode sends recorded audio to the configured service; review its privacy and retention policy first.

---

## 📖 Personal Dictionary

OSWispa can replace phrases that Whisper commonly mishears, such as `os whisper` → `OSWispa`.
Replacements are literal, run locally before spoken punctuation commands, and never learn from or
monitor other applications. Enabled preferred spellings also form a small, bounded prompt for the
local Whisper model; dictionary contents are not sent to the optional remote transcription backend.

On Linux, open **Settings → Dictionary** to add, edit, enable, disable, delete, import, or export
entries. Other platforms can edit `personalisation.json` in the OSWispa data directory and restart
OSWispa. On Linux the path is normally `~/.local/share/oswispa/personalisation.json`:

```json
{
  "schema_version": 1,
  "dictionary": [
    {
      "spoken": "os whisper",
      "written": "OSWispa",
      "enabled": true,
      "case_sensitive": false
    }
  ]
}
```

OSWispa validates the version, entry count, lengths, control characters, and duplicate spoken
phrases before using the file. If validation fails, it preserves the file, disables the dictionary
for that run, and continues normal dictation.

---

## 🔧 Troubleshooting

**"It says recording but nothing happens."**
*   Check the managed daemon: `systemctl --user status ydotoold`.
*   Try manual paste: The text is also copied to your clipboard. Press `Ctrl+V`.

**"It inserts `[BLANK_AUDIO]` or reports no speech."**
*   OSWispa follows the system default PipeWire/PulseAudio input unless a source is set in Settings.
*   Check the current source with `pactl get-default-source` and list alternatives with `pactl list short sources`.
*   Select the working microphone with `pactl set-default-source SOURCE_NAME`, or paste its source name into **Settings → General → Linux microphone source** to override the system default for OSWispa only.

**"The hotkey stops working after I close the terminal."**
*   Run OSWispa through its user service: `systemctl --user enable --now oswispa`.
*   Check startup and hotkey logs with `journalctl --user -u oswispa -n 100`.

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
