#!/bin/bash
set -e

# OSWispa Installation Script
# Voice-to-text with Whisper - hold Ctrl+Super to record
# Supports: Ubuntu/Debian, Fedora/RHEL, Arch/Manjaro

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$HOME/.local/share/oswispa"
CONFIG_DIR="$HOME/.config/oswispa"
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

# Detect Linux distribution family
detect_distro() {
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
        print_status "Detected distro: $PRETTY_NAME ($DISTRO_FAMILY family)"
    else
        DISTRO_FAMILY="unknown"
    fi

    if [ "$DISTRO_FAMILY" = "unknown" ]; then
        print_error "Unsupported distribution. Please install dependencies manually."
        print_error "See README.md for the list of required packages."
        exit 1
    fi
}

detect_distro

# 1. Install system dependencies
print_status "Installing system dependencies..."
case "$DISTRO_FAMILY" in
    debian)
        sudo apt update
        sudo apt install -y \
            build-essential cmake pkg-config \
            libasound2-dev libpulse-dev libdbus-1-dev libappindicator3-dev \
            wl-clipboard ydotool netcat-openbsd xclip xdotool \
            curl git
        ;;
    fedora)
        sudo dnf install -y \
            gcc gcc-c++ cmake pkgconf-pkg-config \
            alsa-lib-devel pulseaudio-libs-devel dbus-devel \
            libappindicator-gtk3-devel openssl-devel \
            alsa-plugins-pulseaudio \
            alsa-utils ydotool nmap-ncat wl-clipboard xclip xdotool \
            curl git
        ;;
    arch)
        sudo pacman -S --needed --noconfirm \
            base-devel cmake pkg-config \
            alsa-lib libpulse dbus libappindicator-gtk3 \
            alsa-utils ydotool openbsd-netcat wl-clipboard xclip xdotool \
            curl git
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

# 4. Download Whisper model
MODEL_FILE="$MODEL_DIR/ggml-base.en.bin"
if [ ! -f "$MODEL_FILE" ]; then
    print_status "Downloading Whisper model (base.en, ~142MB)..."
    echo "This model provides a good balance of speed and accuracy for English."
    echo ""
    echo "Available models:"
    echo "  tiny.en   (~75MB)  - Fastest, lower accuracy"
    echo "  base.en   (~142MB) - Good balance (default)"
    echo "  small.en  (~466MB) - Better accuracy"
    echo "  medium.en (~1.5GB) - High accuracy, slower"
    echo ""

    curl -L -o "$MODEL_FILE" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"

    print_status "Model downloaded successfully!"
else
    print_status "Whisper model already exists at $MODEL_FILE"
fi

# 5. Setup input group permissions for evdev
print_status "Setting up input permissions..."
if ! groups | grep -q '\binput\b'; then
    print_warning "Adding user to 'input' group for keyboard access..."
    sudo usermod -aG input "$USER"
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
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF

# Note: ydotoold typically needs root, but we'll try user mode first
# If it fails, user needs to run: sudo ydotoold &

print_warning "ydotoold may require root permissions."
echo "If typing doesn't work, run: sudo ydotoold &"
echo "Or set up a udev rule for /dev/uinput access."
echo ""

# 7. Create udev rule for uinput (for ydotool without sudo)
print_status "Creating udev rule for uinput access..."
sudo tee /etc/udev/rules.d/60-uinput.rules > /dev/null << EOF
KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"
EOF

sudo udevadm control --reload-rules
sudo udevadm trigger

# 8. Detect GPU and build the application
print_status "Detecting GPU acceleration..."
cd "$SCRIPT_DIR"

GPU_FEATURES=""
GPU_TYPE="cpu"

# Check for AMD ROCm
if [ -f "/opt/rocm/bin/hipcc" ] || compgen -G "/opt/rocm-*/bin/hipcc" >/dev/null 2>&1; then
    ROCM_PATH=$(dirname "$(dirname "$(ls /opt/rocm*/bin/hipcc 2>/dev/null | head -1)")" 2>/dev/null)
    if [ -n "$ROCM_PATH" ] && [ -d "$ROCM_PATH" ]; then
        print_status "AMD ROCm detected at $ROCM_PATH"
        GPU_FEATURES="--features gpu-hipblas"
        GPU_TYPE="amd"
        export PATH="$ROCM_PATH/bin:$PATH"
        export HIP_PATH="$ROCM_PATH"
        # Auto-detect GPU architecture
        if command -v rocminfo &>/dev/null; then
            GFX_ARCH=$(rocminfo 2>/dev/null | grep -oP 'gfx\d+' | head -1)
            if [ -n "$GFX_ARCH" ]; then
                export AMDGPU_TARGETS="$GFX_ARCH"
                print_status "Detected AMD GPU architecture: $GFX_ARCH"
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

if [ -n "$GPU_FEATURES" ]; then
    print_status "Building OSWispa with GPU acceleration ($GPU_TYPE)..."
    cargo build --release $GPU_FEATURES
else
    print_status "No GPU toolkit found, building CPU-only..."
    cargo build --release
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
Categories=Utility;Audio;
Keywords=voice;speech;transcription;whisper;
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

# 11. Create autostart entry
print_status "Creating autostart entry..."
mkdir -p "$HOME/.config/autostart"
cp "$HOME/.local/share/applications/oswispa.desktop" "$HOME/.config/autostart/"

# 12. Create systemd user service for OSWispa
print_status "Creating systemd user service..."
mkdir -p "$HOME/.config/systemd/user"

# Build environment block for GPU
GPU_ENV=""
if [ "$GPU_TYPE" = "amd" ]; then
    GPU_ENV="Environment=HIP_VISIBLE_DEVICES=0
Environment=ROCR_VISIBLE_DEVICES=0"
    # Derive HSA_OVERRIDE_GFX_VERSION from detected architecture (e.g. gfx1100 -> 11.0.0)
    if [ -n "$GFX_ARCH" ]; then
        HSA_DIGITS="${GFX_ARCH#gfx}"
        HSA_MAJOR="${HSA_DIGITS:0:2}"
        HSA_MINOR="${HSA_DIGITS:2:1}"
        HSA_PATCH="${HSA_DIGITS:3:1}"
        HSA_VERSION="${HSA_MAJOR}.${HSA_MINOR}.${HSA_PATCH}"
        GPU_ENV="$GPU_ENV
Environment=HSA_OVERRIDE_GFX_VERSION=$HSA_VERSION"
        print_status "HSA_OVERRIDE_GFX_VERSION set to $HSA_VERSION"
    fi
    if [ -n "$ROCM_PATH" ]; then
        GPU_ENV="$GPU_ENV
Environment=LD_LIBRARY_PATH=$ROCM_PATH/lib:$ROCM_PATH/hip/lib"
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

print_status "Enable with: systemctl --user enable --now oswispa"

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

echo ""
echo "================================"
echo "  Installation Complete!"
echo "================================"
echo ""
echo "IMPORTANT STEPS:"
echo ""
echo "1. Log out and back in (for 'input' group permissions)"
echo ""
echo "2. Start the ydotool daemon (for text injection):"
echo "   sudo ydotoold &"
echo "   (Or use systemd: systemctl --user enable --now ydotoold)"
echo ""
echo "3. Install GNOME AppIndicator extension (for system tray):"
echo "   - Open 'Extensions' app or visit extensions.gnome.org"
echo "   - Search for 'AppIndicator' or 'KStatusNotifierItem'"
echo "   - Enable the extension"
echo ""
echo "4. Run OSWispa:"
echo "   oswispa"
echo ""
echo "USAGE:"
echo "  - Hold Ctrl+Super to start recording"
echo "  - Release to stop and transcribe"
echo "  - Text is copied to clipboard AND pasted automatically"
echo ""
echo "Troubleshooting:"
echo "  - No tray icon? Install AppIndicator GNOME extension"
echo "  - Text not typing? Run: sudo ydotoold &"
echo "  - Permission denied? Log out/in for input group"
echo ""
print_status "Enjoy OSWispa!"
