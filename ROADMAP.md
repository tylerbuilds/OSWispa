# OSWispa Roadmap

OSWispa is a local-first, privacy-first voice dictation app inspired by Wispr Flow: push-to-talk, fast transcription, and frictionless text insertion.

This roadmap is intentionally versioned and biased toward shipping real artifacts people can install and run without building from source.

## v0.3.0 (Next) - Standalone Linux Desktop App

Goal: "Download -> install/run -> dictate" on Ubuntu/Debian.

Deliverables:
- Publish an `amd64` `.deb` on GitHub Releases.
- Keep the existing `amd64` `.tar.gz` as a fallback portable artifact.
- Install a desktop entry + icon so OSWispa shows up in the app launcher.
- Install `oswispa` and `oswispa-toggle` to the PATH.
- Update docs (README + website) to point to the latest release and recommend the `.deb`.

Engineering notes:
- Package using `cargo-deb` to keep packaging maintainable.
- Keep Linux-first behavior unchanged.
- Add buildability guardrails so `cargo check --no-default-features` works on macOS and Windows (prep work for v0.4.0/v0.5.0).

Known Linux/Wayland constraints (won't be fully solved in v0.3.0):
- Global hotkeys require `/dev/input` access (user must be in the `input` group).
- Auto-paste uses `ydotoold`/`uinput` and may require extra permissions depending on distro/session.

## v0.4.0 - macOS Desktop App

Goal: a native menu bar app with a configurable global hotkey, microphone capture, and paste into the focused app.

Deliverables:
- Distributable macOS app (`.dmg` or `.zip`) published on GitHub Releases.
- Global hotkey implementation for macOS with a Settings UI.
- Audio capture on macOS (CoreAudio via `cpal` or a native implementation).
- Text insertion via clipboard + Cmd+V (requires Accessibility permissions for key simulation).
- Keep model management and optional VPS backend behavior consistent with Linux.

Engineering milestones:
- Introduce a small platform abstraction layer for hotkeys, audio capture, tray/menubar, and text insertion.
- CI build checks on macOS.

## v0.5.0 - Windows Desktop App

Goal: a tray app with a configurable global hotkey, microphone capture, and paste into the focused app.

Deliverables:
- Installer (`.msi`/`.exe`) plus a portable `.zip` on GitHub Releases.
- Global hotkey via Windows APIs.
- Audio capture via WASAPI (via `cpal` or a native implementation).
- Text insertion via clipboard + key simulation (SendInput).
- CI build checks on Windows.

Engineering milestones:
- Platform abstraction layer completed across Linux/macOS/Windows.
- Replace Linux-only assumptions in runtime checks and error messages.

## Ongoing Quality Bar

Always true for every release:
- GitHub Release is the source of truth (assets + changelog).
- Website links and copy must reference the latest GitHub Release.
- Local-first and privacy-first defaults (remote/VPS is opt-in).

