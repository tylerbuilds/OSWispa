#!/bin/bash
set -e

# OSWispa Installation Script
# Voice-to-text with Whisper - hold a hotkey to record
# Supports: Ubuntu/Debian, Fedora/RHEL, Arch/Manjaro, macOS

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM="$(uname -s)"
HELPERS="$SCRIPT_DIR/scripts/install_helpers.sh"

if [ ! -r "$HELPERS" ]; then
    echo "Missing installer helpers: $HELPERS" >&2
    exit 1
fi
source "$HELPERS"

# Platform-appropriate directories
if [ "$PLATFORM" = "Darwin" ]; then
    # Matches directories::ProjectDirs::from("com", "oswispa", "OSWispa").
    DATA_DIR="$HOME/Library/Application Support/com.oswispa.OSWispa"
    CONFIG_DIR="$DATA_DIR"
else
    DATA_DIR="$HOME/.local/share/oswispa"
    CONFIG_DIR="$HOME/.config/oswispa"
fi
MODEL_DIR="$DATA_DIR/models"

echo "================================"
echo "  OSWispa Installation Script"
echo "================================"
echo ""

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[+]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[!]${NC} $1"
}

# Detect platform and Linux distribution family
detect_platform() {
    if [ "$PLATFORM" = "Darwin" ]; then
        DISTRO_FAMILY="macos"
        print_status "Detected platform: macOS $(sw_vers -productVersion 2>/dev/null || echo '')"
        return
    fi

    if [ "$PLATFORM" != "Linux" ]; then
        print_error "Unsupported platform: $PLATFORM"
        exit 1
    fi

    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case "$ID" in
            ubuntu|debian|pop|linuxmint|elementary|zorin)
                DISTRO_FAMILY="debian"
                ;;
            fedora|rhel|centos|rocky|alma)
                DISTRO_FAMILY="fedora"
                ;;
            arch|manjaro|endeavouros|garuda)
                DISTRO_FAMILY="arch"
                ;;
            *)
                # Check ID_LIKE as fallback
                case "$ID_LIKE" in
                    *debian*|*ubuntu*) DISTRO_FAMILY="debian" ;;
                    *fedora*|*rhel*)   DISTRO_FAMILY="fedora" ;;
                    *arch*)            DISTRO_FAMILY="arch" ;;
                    *)                 DISTRO_FAMILY="unknown" ;;
                esac
                ;;
        esac
        print_status "Detected distro: ${PRETTY_NAME:-$ID} ($DISTRO_FAMILY family)"
    else
        DISTRO_FAMILY="unknown"
    fi

    if [ "$DISTRO_FAMILY" = "unknown" ]; then
        print_error "Unsupported distribution. Please install dependencies manually."
        print_error "See README.md for the list of required packages."
        exit 1
    fi
}

detect_platform
NEEDS_RELOGIN=0

# 1. Install system dependencies
print_status "Installing system dependencies..."
case "$DISTRO_FAMILY" in
    macos)
        # Xcode Command Line Tools (provides clang, make, etc.)
        if ! xcode-select -p &>/dev/null; then
            print_status "Installing Xcode Command Line Tools..."
            xcode-select --install
            echo ""
            print_warning "Xcode CLT is installing. Re-run this script when it finishes."
            exit 0
        fi
        # Homebrew
        if ! command -v brew &>/dev/null; then
            print_status "Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        fi
        brew install cmake
        ;;
    debian)
        sudo apt update
        sudo apt install -y \
            build-essential cmake pkg-config \
            libasound2-dev libpulse-dev libdbus-1-dev libappindicator3-dev \
            libssl-dev libgtk-4-dev \
            alsa-utils wl-clipboard ydotool netcat-openbsd xclip xdotool \
            curl git
        ;;
    fedora)
        sudo dnf install -y \
            gcc gcc-c++ cmake pkgconf-pkg-config \
            alsa-lib-devel pulseaudio-libs-devel dbus-devel \
            libappindicator-gtk3-devel openssl-devel gtk4-devel \
            alsa-plugins-pulseaudio \
            alsa-utils ydotool nmap-ncat wl-clipboard xclip xdotool \
            curl git
        ;;
    arch)
        sudo pacman -S --needed --noconfirm \
            base-devel cmake pkg-config \
            alsa-lib libpulse dbus libappindicator-gtk3 openssl gtk4 \
            alsa-utils ydotool openbsd-netcat wl-clipboard xclip xdotool \
            curl git
        ;;
    *)
        print_error "Internal error: unexpected DISTRO_FAMILY='$DISTRO_FAMILY'"
        exit 1
        ;;
esac

# 2. Install Rust if not present
if ! command -v cargo &> /dev/null; then
    print_status "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    print_status "Rust already installed: $(cargo --version)"
fi

# 3. Create directories
print_status "Creating data directories..."
mkdir -p "$MODEL_DIR"
mkdir -p "$CONFIG_DIR"
chmod 700 "$DATA_DIR" "$CONFIG_DIR"

# 4. Download Whisper model
MODEL_FILE="$MODEL_DIR/ggml-base.en.bin"
if ! oswispa_validate_model_file "$MODEL_FILE" "$OSWISPA_BASE_MODEL_MIN_BYTES"; then
    if [ -e "$MODEL_FILE" ]; then
        print_warning "Existing model is incomplete or invalid; replacing it safely."
    fi
    print_status "Downloading Whisper model (base.en, ~142MB)..."
    echo "This model provides a good balance of speed and accuracy for English."
    echo ""
    echo "Available models:"
    echo "  tiny.en   (~75MB)  - Fastest, lower accuracy"
    echo "  base.en   (~142MB) - Good balance (default)"
    echo "  small.en  (~466MB) - Better accuracy"
    echo "  medium.en (~1.5GB) - High accuracy, slower"
    echo ""

    MODEL_TEMP="$MODEL_FILE.downloading"
    rm -f "$MODEL_TEMP"
    if ! curl --fail --location --retry 3 --retry-delay 2 --connect-timeout 20 \
        --output "$MODEL_TEMP" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"; then
        rm -f "$MODEL_TEMP"
        print_error "Model download failed; the existing model was left untouched."
        exit 1
    fi

    if ! oswispa_validate_model_file "$MODEL_TEMP" "$OSWISPA_BASE_MODEL_MIN_BYTES"; then
        rm -f "$MODEL_TEMP"
        print_error "Downloaded payload is not a complete Whisper model."
        exit 1
    fi

    mv -f "$MODEL_TEMP" "$MODEL_FILE"

    print_status "Model downloaded successfully!"
else
    print_status "Whisper model already exists at $MODEL_FILE"
fi

# 5-7: Linux-only setup (input group, ydotool, udev)
if [ "$PLATFORM" != "Darwin" ]; then
    # 5. Setup input group permissions for evdev
    print_status "Setting up input permissions..."
    if ! id -nG | tr ' ' '\n' | grep -qx input; then
        print_warning "Adding user to 'input' group for keyboard access..."
        sudo usermod -aG input "$USER"
        NEEDS_RELOGIN=1
        echo ""
        print_warning "IMPORTANT: You need to log out and back in for group changes to take effect!"
        echo ""
    fi

    # 6. Setup ydotool daemon
    print_status "Setting up ydotool daemon..."

    # Create systemd user service for ydotoold
    mkdir -p "$HOME/.config/systemd/user"
    cat > "$HOME/.config/systemd/user/ydotoold.service" << 'EOF'
[Unit]
Description=ydotool daemon
Documentation=man:ydotool(1)

[Service]
ExecStart=/usr/bin/ydotoold
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

    # 7. Create udev rule for uinput (for ydotool without sudo)
    print_status "Creating udev rule for uinput access..."
    sudo tee /etc/udev/rules.d/60-uinput.rules > /dev/null << EOF
KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"
EOF

    sudo udevadm control --reload-rules
    sudo udevadm trigger
fi

# 8. Detect GPU and build the application
print_status "Detecting GPU acceleration..."
cd "$SCRIPT_DIR"

GPU_FEATURES=""
GPU_TYPE="cpu"
BUILD_FLAGS=""

if [ "$PLATFORM" = "Darwin" ]; then
    # macOS: disable GTK4 GUI (Linux-only)
    BUILD_FLAGS="--no-default-features"
    # Check for Apple Silicon (Metal GPU)
    if [ "$(uname -m)" = "arm64" ]; then
        print_status "Apple Silicon detected — Metal GPU available"
        GPU_FEATURES="--features gpu-metal"
        GPU_TYPE="metal"
    fi
else
    # Linux: Check for AMD ROCm
    if [ -f "/opt/rocm/bin/hipcc" ] || compgen -G "/opt/rocm-*/bin/hipcc" >/dev/null 2>&1; then
        ROCM_PATH=$(dirname "$(dirname "$(ls /opt/rocm*/bin/hipcc 2>/dev/null | head -1)")" 2>/dev/null)
        if [ -n "$ROCM_PATH" ] && [ -d "$ROCM_PATH" ]; then
            print_status "AMD ROCm detected at $ROCM_PATH"
            GPU_FEATURES="--features gpu-hipblas"
            GPU_TYPE="amd"
            export PATH="$ROCM_PATH/bin:$PATH"
            export HIP_PATH="$ROCM_PATH"
            # Compile for every detected ROCm architecture. Multi-GPU systems
            # often expose an integrated GPU before the discrete card.
            if command -v rocminfo &>/dev/null; then
                GFX_ARCHES=$(rocminfo 2>/dev/null | oswispa_rocm_targets_from_stream)
                if [ -n "$GFX_ARCHES" ]; then
                    export AMDGPU_TARGETS="$GFX_ARCHES"
                    print_status "Detected AMD GPU architectures: $GFX_ARCHES"
                fi
            fi

            # Prefer the GPU with the most VRAM at runtime instead of assuming
            # that ROCm device zero is the discrete card.
            if command -v rocm-smi &>/dev/null; then
                ROCM_DEVICE_INDEX=$(rocm-smi --showmeminfo vram 2>/dev/null \
                    | oswispa_largest_rocm_gpu_from_stream)
                if [ -n "$ROCM_DEVICE_INDEX" ]; then
                    print_status "Selected ROCm GPU $ROCM_DEVICE_INDEX (largest VRAM)"
                fi
            fi
        fi
    # Check for NVIDIA CUDA
    elif command -v nvcc &>/dev/null || [ -d "/usr/local/cuda" ]; then
        print_status "NVIDIA CUDA detected"
        GPU_FEATURES="--features gpu-cuda"
        GPU_TYPE="nvidia"
        if [ -d "/usr/local/cuda" ]; then
            export PATH="/usr/local/cuda/bin:$PATH"
        fi
    fi
fi

if [ -n "$GPU_FEATURES" ]; then
    print_status "Building OSWispa with GPU acceleration ($GPU_TYPE)..."
    cargo build --release $BUILD_FLAGS $GPU_FEATURES
else
    print_status "No GPU toolkit found, building CPU-only..."
    cargo build --release $BUILD_FLAGS
fi

# 9. Install binary
print_status "Installing binary..."
sudo cp target/release/oswispa /usr/local/bin/
sudo chmod +x /usr/local/bin/oswispa

# 9b. Install IPC toggle helper
if [ -f "$SCRIPT_DIR/scripts/oswispa-toggle.sh" ]; then
    print_status "Installing oswispa-toggle helper..."
    sudo cp "$SCRIPT_DIR/scripts/oswispa-toggle.sh" /usr/local/bin/oswispa-toggle
    sudo chmod +x /usr/local/bin/oswispa-toggle
fi

# 10-12: Platform-specific service and autostart setup
if [ "$PLATFORM" = "Darwin" ]; then
    # macOS: Create LaunchAgent for auto-start
    print_status "Creating LaunchAgent for auto-start..."
    mkdir -p "$HOME/Library/LaunchAgents"
    cat > "$HOME/Library/LaunchAgents/com.oswispa.agent.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.oswispa.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/oswispa</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>StandardOutPath</key>
    <string>$HOME/Library/Logs/oswispa.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/Library/Logs/oswispa.log</string>
</dict>
</plist>
EOF
    print_status "Enable with: launchctl load ~/Library/LaunchAgents/com.oswispa.agent.plist"

else
    # Linux: Desktop entry, autostart, and systemd service

    # 10. Create desktop entry
    print_status "Creating desktop entry..."
    mkdir -p "$HOME/.local/share/applications"
    cat > "$HOME/.local/share/applications/oswispa.desktop" << EOF
[Desktop Entry]
Type=Application
Name=OSWispa
Comment=Voice to text with Whisper - hold Ctrl+Super to record
Exec=/usr/local/bin/oswispa
Icon=audio-input-microphone
Terminal=false
Categories=Utility;AudioVideo;Audio;
Keywords=voice;speech;transcription;whisper;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

    # 11. Remove the legacy desktop autostart entry. The user service below
    # owns the process lifecycle so OSWispa cannot start twice at login.
    rm -f "$HOME/.config/autostart/oswispa.desktop"

    # 12. Create systemd user service for OSWispa
    print_status "Creating systemd user service..."
    mkdir -p "$HOME/.config/systemd/user"

    # Build environment block for GPU
    GPU_ENV=""
    if [ "$GPU_TYPE" = "amd" ]; then
        if [ -n "${ROCM_DEVICE_INDEX:-}" ]; then
            GPU_ENV="Environment=ROCR_VISIBLE_DEVICES=$ROCM_DEVICE_INDEX"
        fi
        if [ -n "$ROCM_PATH" ]; then
            if [ -n "$GPU_ENV" ]; then
                GPU_ENV="$GPU_ENV
Environment=LD_LIBRARY_PATH=$ROCM_PATH/lib:$ROCM_PATH/hip/lib"
            else
                GPU_ENV="Environment=LD_LIBRARY_PATH=$ROCM_PATH/lib:$ROCM_PATH/hip/lib"
            fi
        fi
    fi

    cat > "$HOME/.config/systemd/user/oswispa.service" << EOF
[Unit]
Description=OSWispa Voice-to-Text Service
Documentation=https://github.com/tylerbuilds/OSWispa
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/oswispa
Restart=on-failure
RestartSec=5
$GPU_ENV

[Install]
WantedBy=default.target
EOF

fi

# 13. Create config file
if [ ! -f "$CONFIG_DIR/config.json" ]; then
    print_status "Creating default config..."
    cat > "$CONFIG_DIR/config.json" << EOF
{
    "model_path": "$MODEL_FILE",
    "max_history": 50,
    "auto_paste": true,
    "notification_enabled": true
}
EOF
fi
chmod 600 "$CONFIG_DIR/config.json"

if [ "$PLATFORM" != "Darwin" ]; then
    print_status "Enabling OSWispa and ydotool user services..."
    systemctl --user daemon-reload
    systemctl --user enable ydotoold.service
    systemctl --user enable oswispa.service
    if [ "$NEEDS_RELOGIN" -eq 1 ]; then
        print_warning "Services are enabled and will start after you log out and back in."
    else
        systemctl --user restart ydotoold.service
        systemctl --user restart oswispa.service
    fi
fi

echo ""
echo "================================"
echo "  Installation Complete!"
echo "================================"
echo ""

if [ "$PLATFORM" = "Darwin" ]; then
    echo "IMPORTANT STEPS:"
    echo ""
    echo "1. Grant Accessibility permission:"
    echo "   System Settings > Privacy & Security > Accessibility"
    echo "   Add and enable 'oswispa'"
    echo ""
    echo "2. Run OSWispa:"
    echo "   oswispa"
    echo ""
    echo "3. (Optional) Auto-start on login:"
    echo "   launchctl load ~/Library/LaunchAgents/com.oswispa.agent.plist"
    echo ""
    echo "USAGE:"
    echo "  - Hold Ctrl+Super to start recording"
    echo "  - Release to stop and transcribe"
    echo "  - Text is copied to clipboard AND pasted automatically"
    echo ""
    echo "Troubleshooting:"
    echo "  - Hotkeys don't work? Grant Accessibility permission (step 1)"
    echo "  - Microphone permission is requested automatically on first use"
else
    echo "IMPORTANT STEPS:"
    echo ""
    echo "1. If the installer added you to the 'input' group, log out and back in."
    echo ""
    echo "2. Check the ydotool user service (for text injection):"
    echo "   systemctl --user status ydotoold"
    echo ""
    echo "3. Install GNOME AppIndicator extension (for system tray):"
    echo "   - Open 'Extensions' app or visit extensions.gnome.org"
    echo "   - Search for 'AppIndicator' or 'KStatusNotifierItem'"
    echo "   - Enable the extension"
    echo ""
    echo "4. OSWispa is running as a user service:"
    echo "   systemctl --user status oswispa"
    echo ""
    echo "USAGE:"
    echo "  - Hold Ctrl+Super to start recording"
    echo "  - Release to stop and transcribe"
    echo "  - Text is copied to clipboard AND pasted automatically"
    echo ""
    echo "Troubleshooting:"
    echo "  - No tray icon? Install AppIndicator GNOME extension"
    echo "  - Text not typing? Check: journalctl --user -u ydotoold -n 50"
    echo "  - Permission denied? Log out/in for input group"
    echo "  - [BLANK_AUDIO]? Check: pactl get-default-source"
fi

echo ""
print_status "Enjoy OSWispa!"
