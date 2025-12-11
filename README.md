# OSWispa 🎙️

**Open Source Whisper Assistant** - Lightning-fast voice-to-text for Linux/macOS/Windows

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A privacy-focused, locally-running voice transcription tool powered by [Whisper.cpp](https://github.com/ggerganov/whisper.cpp). Hold a hotkey, speak, release - your words appear instantly.

## ✨ Features

- **🎤 Push-to-Talk**: Hold `Ctrl+Super` to record, release to transcribe
- **🔒 100% Local**: No cloud APIs, no data leaves your machine
- **⚡ GPU Accelerated**: AMD ROCm, NVIDIA CUDA, or Apple Metal support
- **📋 Auto-Paste**: Text automatically copies to clipboard (Ctrl+V to paste)
- **🖥️ System Tray**: Status indicator shows recording state
- **🌍 Multilingual**: Supports 99 languages with the right model

## 🚀 Quick Start

### Linux (Ubuntu/Debian)

```bash
# Clone and install
git clone https://github.com/yourusername/oswispa.git
cd oswispa
./install.sh

# Run
oswispa
```

Then press **Ctrl+Super** → speak → release → **Ctrl+V** to paste!

## 📦 Installation

### Prerequisites (All Platforms)

- [Rust](https://rustup.rs/) (1.70+)
- CMake 3.16+
- A Whisper model (downloaded automatically or manually)

---

### 🐧 Linux

#### CPU Only (Works Everywhere)
```bash
# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential cmake pkg-config libssl-dev \
    libayatana-appindicator3-dev libasound2-dev

# Build
cargo build --release

# Install
sudo cp target/release/oswispa /usr/local/bin/
```

#### AMD GPU (ROCm) - Recommended for AMD users
```bash
# Install ROCm (6.0+)
# See: https://rocm.docs.amd.com/projects/install-on-linux/en/latest/

# Install dependencies
sudo apt install build-essential cmake pkg-config libssl-dev \
    libayatana-appindicator3-dev libasound2-dev \
    hipblas-dev rocblas-dev

# Edit Cargo.toml - change whisper-rs line to:
# whisper-rs = { version = "0.13", features = ["hipblas"] }

# Build with ROCm
export AMDGPU_TARGETS="gfx1100"  # Adjust for your GPU (gfx1030, gfx1100, etc.)
cargo build --release
```

**Finding your GPU architecture:**
```bash
rocminfo | grep "Name:" | grep gfx
```

#### NVIDIA GPU (CUDA)
```bash
# Install CUDA toolkit
# See: https://developer.nvidia.com/cuda-downloads

# Edit Cargo.toml - change whisper-rs line to:
# whisper-rs = { version = "0.13", features = ["cuda"] }

# Build with CUDA
cargo build --release
```

---

### 🍎 macOS

#### CPU Only
```bash
# Install dependencies
brew install cmake pkg-config

# Build
cargo build --release
```

#### Apple Silicon (Metal) - Recommended for M1/M2/M3
```bash
# Edit Cargo.toml - change whisper-rs line to:
# whisper-rs = { version = "0.13", features = ["metal"] }

# Build with Metal acceleration
cargo build --release
```

> ⚠️ **Note**: macOS requires accessibility permissions for auto-paste functionality.

---

### 🪟 Windows

#### Prerequisites
1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Install [CMake](https://cmake.org/download/)
3. Install [Rust](https://rustup.rs/)

#### CPU Only
```powershell
cargo build --release
```

#### NVIDIA GPU (CUDA)
```powershell
# Install CUDA Toolkit from NVIDIA

# Edit Cargo.toml - change whisper-rs line to:
# whisper-rs = { version = "0.13", features = ["cuda"] }

cargo build --release
```

> ⚠️ **Note**: Windows support is experimental. Wayland-specific features won't work.

---

## 📥 Downloading Models

OSWispa requires a Whisper model. Download from [Hugging Face](https://huggingface.co/ggerganov/whisper.cpp):

| Model | Size | Speed | Accuracy | Best For |
|-------|------|-------|----------|----------|
| `ggml-base.en.bin` | 142MB | ⚡⚡⚡ | Good | Fast dictation |
| `ggml-small.en.bin` | 488MB | ⚡⚡ | Better | General use |
| `ggml-medium.en.bin` | 1.5GB | ⚡ | Great | Recommended |
| `ggml-large-v3.bin` | 2.9GB | 🐢 | Best | Complex audio |

**Download:**
```bash
# Create models directory
mkdir -p ~/.local/share/oswispa/models
cd ~/.local/share/oswispa/models

# Download medium.en (recommended)
curl -LO https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin
```

**Configure model path** in `~/.config/oswispa/config.json`:
```json
{
  "model_path": "/path/to/your/model.bin"
}
```

---

## ⚙️ Configuration

Configuration file: `~/.config/oswispa/config.json`

```json
{
  "model_path": "~/.local/share/oswispa/models/ggml-medium.en.bin",
  "language": "en",
  "hotkey": {
    "ctrl": true,
    "alt": false,
    "shift": false,
    "super_key": true
  },
  "auto_paste": true,
  "audio_feedback": true,
  "max_history": 50
}
```

### Hotkey Options
- Default: `Ctrl+Super`
- Customize by editing the `hotkey` section

---

## 🎯 Usage

1. **Start**: Run `oswispa` (add to startup for auto-launch)
2. **Record**: Hold `Ctrl+Super`
3. **Speak**: Say your text clearly
4. **Release**: Let go of the keys
5. **Paste**: Press `Ctrl+V` in any application

### Tips
- Speak naturally - Whisper handles punctuation
- Short pauses are fine, long pauses may split sentences
- For best results, use `medium.en` or `large-v3` models

---

## 🔧 Troubleshooting

### No hotkey response?
- **Linux**: Ensure you're in the `input` group: `sudo usermod -aG input $USER`
- **Wayland**: The app uses evdev for global hotkeys

### No audio recording?
- Check microphone permissions
- Ensure `arecord` works: `arecord -d 2 test.wav`
- On PipeWire: Install `pipewire-alsa`

### No tray icon?
- Install GNOME extension: [AppIndicator Support](https://extensions.gnome.org/extension/615/appindicator-support/)
- Or run: `sudo apt install gnome-shell-extension-appindicator`

### GPU not detected?
- **AMD**: Check `rocminfo` shows your GPU
- **NVIDIA**: Check `nvidia-smi` works
- Ensure you built with correct feature flag

### Segfault on startup?
- Often a GPU driver mismatch
- Try CPU mode: rebuild without GPU features
- Check driver versions match (e.g., ROCm 6.2 libs with ROCm 6.2 hipcc)

---

## 🏗️ Architecture

```
oswispa/
├── src/
│   ├── main.rs          # Application entry, event loop
│   ├── audio/           # Recording via arecord
│   ├── transcribe/      # Whisper.cpp integration
│   ├── hotkey/          # Global hotkey detection (evdev)
│   ├── input/           # Clipboard & paste (wl-copy)
│   ├── tray/            # System tray (ksni)
│   └── feedback/        # Audio feedback sounds
├── install.sh           # One-line installer
└── Cargo.toml           # Rust dependencies
```

---

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Submit a pull request

---

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🙏 Acknowledgments

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) by Georgi Gerganov
- [whisper-rs](https://github.com/tazz4843/whisper-rs) Rust bindings
- OpenAI for the original Whisper model

---

**Made with ❤️ for the open source community**
