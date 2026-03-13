# Changelog

## Unreleased

### Docs
- Added realistic model guidance for Intel Macs, Apple Silicon Macs, and discrete GPU systems.
- Documented that OSWispa now auto-tests the machine on first launch and picks a model that aims to stay responsive.
- Clarified that `large-v3` is an accuracy-first manual choice, not the normal speed-first default.

## v0.4.1 - 2026-03-12

### Added
- Added `distil-large-v3` to the curated local model list.
- Added a stable Linux checkpoint tag for this release cycle: `checkpoint-2026-03-12-stable-dictation`.
- Added a packaged macOS app download: `OSWispa.app` inside a drag-to-Applications `.dmg`, with a bundled plain-English readme.

### Changed
- Linux dictation now prewarms and reuses local Whisper contexts instead of rebuilding them for every utterance.
- Wayland auto-insert now types text directly into the focused app, while still copying the transcript to the clipboard.
- The Linux hotkey listener now ignores the `ydotoold` virtual keyboard so text insertion doesn't retrigger recording.
- Local GPU transcription now keeps a larger free-VRAM reserve before using the GPU path.
- The active Linux dictation path now favors the faster release-to-text flow for short dictation bursts.

### Fixed
- Fixed Linux hotkey/tray sessions that felt clunky because the transcription path was recreating model state repeatedly.
- Fixed repeated self-trigger loops caused by simulated input devices being treated as real keyboards.
- Fixed Wayland insertion flows that were tripping app-specific clipboard and image-paste handlers.
- Fixed ROCm usage on the primary Linux path by rebuilding and validating against the correct AMD target during bring-up.
