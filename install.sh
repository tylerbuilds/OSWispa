#!/bin/bash
set -e

# OSWispa Installation Script for Ubuntu
# Voice-to-text with Whisper - hold Ctrl+Super to record

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$HOME/.local/share/oswispa"
CONFIG_DIR="$HOME/.config/oswispa"
MODEL_DIR="$DATA_DIR/models"

echo "================================"
echo "  OSWispa Installation Script"
echo "================================"
echo ""

# Check if running on Ubuntu/Debian
if ! command -v apt &> /dev/null; then
    echo "[WARNING] apt not found. This script is designed for Ubuntu/Debian."
    echo "You may need to install dependencies manually."
fi

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

# 1. Install system dependencies
print_status "Installing system dependencies..."
sudo apt update
sudo apt install -y \
    build-essential \
    cmake \
    pkg-config \
    libasound2-dev \
    libpulse-dev \
    libdbus-1-dev \
    libappindicator3-dev \
    wl-clipboard \
    ydotool \
    netcat-openbsd \
    curl \
    git

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

# 8. Build the application
print_status "Building OSWispa..."
cd "$SCRIPT_DIR"
cargo build --release

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

# 12. Create config file
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
