# OSWispa 🎙️
**Open Source Whisper Assistant**

A lightweight, privacy-focused voice-to-text assistant for Linux, powered by Whisper.cpp.

## Features
- **Global Hotkey**: Press `Ctrl+Super` (default) to toggle recording anywhere.
- **Local Processing**: Audio transcribed locally using Whisper (no cloud APIs).
- **Auto-Paste**: Transcribed text is automatically typed into your active window.
- **System Tray**: Quick access to settings and status indicator.
- **Wayland Support**: Works on GNOME Wayland via custom shortcut integration.

## Installation

### Dependencies
Ensure you have the following installed:
- `libappindicator3-dev` (for tray icon)
- `libssl-dev`
- `pkg-config`
- `cmake` (for building Whisper)
- `xdotool` or `ydotool` (optional, for improved typing support)

### Building from Source
```bash
cargo build --release
sudo cp target/release/oswispa /usr/local/bin/oswispa
```

### Setup (Wayland/GNOME)
For global hotkeys to work on Wayland, you must register a system shortcut that communicates with OSWispa.

**Automatic Setup:**
Run the included script:
```bash
./scripts/setup_gnome_shortcut.sh
```

**Manual Setup:**
1. Open **Settings > Keyboard > View and Customize Shortcuts > Custom Shortcuts**.
2. Add a new shortcut:
   - **Name**: `OSWispa Toggle`
   - **Command**: `/usr/local/bin/oswispa-toggle`
   - **Shortcut**: `Ctrl+Super` (or your preferred key)

## Usage
1. **Start the App**: Run `oswispa` (or add to Startup Applications).
2. **Toggle Recording**: 
   - Press `Ctrl+Super`. You will hear a start sound (if enabled).
   - Speak your text.
   - Press `Ctrl+Super` again to stop.
3. **Transcription**: The text will be transcribed and automatically pasted into your active window.

## Troubleshooting
- **No Hotkey Response?** Verify the custom shortcut is set in GNOME settings. Check `/tmp/oswispa.sock` exists when app is running.
- **No Tray Icon?** Ensure you have the [AppIndicator and KStatusNotifierItem Support](https://extensions.gnome.org/extension/615/appindicator-support/) extension installed.
- **Permission Errors?** Ensure your user is in the `input` group: `sudo usermod -aG input $USER`.

## License
MIT
