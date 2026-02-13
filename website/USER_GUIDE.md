# OSWispa User Guide 🎙️

Welcome to OSWispa! This guide will help you get the most out of your new voice-to-text assistant.

## How to Record

1.  **Hotkey**: Press and hold your configured hotkey (**Ctrl + Super** by default).
    - You'll hear a subtle "beep" (if audio feedback is enabled) and see a "Recording..." notification.
2.  **Speak**: Talk naturally. OSWispa uses Whisper models locally on your machine by default (or an optional VPS backend if enabled).
3.  **Release**: Let go of the keys.
    - OSWispa will transcribe your speech and automatically paste it into your active window.

## Tips for Best Accuracy

- **Silence**: Try to record in a quiet environment.
- **Punctuation**: OSWispa automatically attempts to punctuate your speech. If you prefer manual control, you can toggle this in settings.
- **Models**: Use the `distil-large-v3` model for the best balance of speed and accuracy.

## Settings

Right-click the OSWispa icon in your system tray to:
- Change hotkey modifiers and optional trigger key.
- Select or import a different Whisper model.
- Enable/Disable audio feedback.
- Toggle auto-paste.
- Configure optional remote VPS backend.
