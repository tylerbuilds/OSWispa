# MorpheOS Voice rebrand test plan

## Purpose

Prove that the public-name change is accurate, preserves existing user state and does not hide core-loop or release defects. Static inspection and mocks may support a result but cannot earn PASS for hardware-dependent behaviour.

Statuses used in results and readiness: **PASS**, **PARTIAL**, **BLOCKED**, **FAIL**.

## Test layers

| Layer | Required evidence | Pass rule |
|---|---|---|
| Public identity | Source/UI/package/site assertions and old-name scan | MorpheOS Voice appears on intended surfaces; remaining legacy names map to documented compatibility/history classes |
| Existing-user state | Unit round trip plus isolated legacy-path launch | Settings, shortcut, model path, history and personalisation remain readable; no copy/delete/repeated migration |
| Core Rust | Format, strict Clippy and tests | No failure or warning under the CI lint command |
| Desktop boundary | UI contract tests, JavaScript parse, shell tests and native source compile | Transcript-free lifecycle remains bounded; new product name renders; source compiles with real target dependencies |
| Website | Link/claim validator and browser QA | No broken local links, unsupported claims, console errors or horizontal overflow; CTA and mobile navigation work |
| Installer/package | Shell fixtures, metadata inspection, extraction and packaged smoke | Intended public name is visible; legacy package contract remains; packaged binary reports correct backends |
| Linux controlled end to end | Real recorder, controlled speech audio, local model, clipboard and focused field | Recording starts/stops, non-empty transcript is produced, persisted text equals visibly inserted text |
| Linux physical input | Physical microphone and configured global shortcut | Permission/access, press/release and focused-app delivery work on hardware |
| macOS/Windows | Native build/package plus physical workflow | Current rebrand candidate passes clean install, permission, shortcut, audio, transcription, insertion, fallback and rollback |
| Local/offline | Local model with network unavailable | Dictation completes after the model is installed and UI makes the boundary clear |
| Remote processing | Explicit test endpoint with safe test audio | UI says remote; request fields, timeout, credential redaction, failure and local fallback match documentation |
| Privacy/security/licensing | Data-flow inspection, RustSec scan, dependency licences and artefact contents | Claims match code; no unreviewed secrets/bundled models; exceptions are explicit |
| Upgrade/rollback | Existing v0.4.2 install to candidate and back | Data and permissions survive without deleting the legacy state directory |

## Controlled Linux scenario

1. Build the rebranded compatibility executable.
2. Use isolated XDG config/data/runtime directories containing a representative legacy `oswispa/config.json` and an existing valid local model path.
3. Load a temporary PipeWire null sink; use its monitor as the configured microphone source.
4. Open a real GTK text field inside Xvfb and keep it focused.
5. Start recording through the normal command/socket boundary.
6. Play a generated speech WAV into the temporary source.
7. Stop capture and let normal Whisper.cpp transcription, clipboard verification, auto-insertion and history persistence run.
8. Confirm the persisted transcript and text copied from the visible field are identical and non-empty.
9. Capture a screenshot and unload the temporary audio device/processes.

This proves the Linux recorder/transcriber/delivery path without claiming a physical microphone or physical global-hotkey proof.

## Required release-only checks

- Native macOS Apple Silicon and Intel builds.
- Native Windows x86-64 build.
- Signed/notarised candidate packages and stable publisher identities.
- Physical microphone, shortcut, permission and focused-app proof on all advertised platforms.
- Clean install, v0.4.2 upgrade, uninstall and rollback.
- Public GitHub artefact/download smoke and the deployed `morpheos.net/voice` page.

Those checks require external workflows, target hardware, signing credentials or publication approval and are not performed from this local rebrand worktree.
