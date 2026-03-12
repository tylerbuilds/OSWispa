# Changelog

## v0.4.0 - 2026-03-12

### Added
- Added `distil-large-v3` to the curated local model list.
- Added a stable Linux checkpoint tag for this release cycle: `checkpoint-2026-03-12-stable-dictation`.

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
