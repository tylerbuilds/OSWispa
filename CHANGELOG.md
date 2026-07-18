# Changelog

Scope window: project inception on 2025-12-11 through v0.5.0 on 2026-07-18.

This changelog is reconstructed from Git history, tags, GitHub Releases, merged pull requests, and the current source tree. A tag link is used where no GitHub Release was published.

## 0.5.0 - 2026-07-18

### Rebrand

- Adopted **MorpheOS Voice** as the public product name, with the positioning “Talk instead of type — in any app.”
- Added a MorpheOS-family visual system, users-first README and `/voice` product-page hand-off for `morpheos.net/voice`.
- Retained the `oswispa` executable, package filenames, application IDs, state directories, environment-variable prefix and official repository URL for upgrade, data and rollback compatibility.
- Added migration, compatibility, privacy, open-source, third-party licence and release-readiness documentation for the transition.

### Added

- Added privacy-safe runtime, installation, and feature-proposal forms, with Q&A and early ideas routed to GitHub Discussions ([PR #33](https://github.com/tylerbuilds/OSWispa/pull/33)).
- Added a deterministic, user-curated personal dictionary with literal longest-first phrase replacement, a bounded local Whisper vocabulary prompt, and a Linux editor ([PR #38](https://github.com/tylerbuilds/OSWispa/pull/38)).
- Exposed the existing runtime through a reusable engine library with typed non-blocking commands, transcript-redacted lifecycle events, and joined shutdown ([PR #39](https://github.com/tylerbuilds/OSWispa/pull/39)).
- Added an original local-only desktop UI foundation for Ready Check, Settings, the compact Signal, and bounded recovery history. This remains a development host contract rather than a native shell ([PR #40](https://github.com/tylerbuilds/OSWispa/pull/40)).

### Changed

- Reworked the public website around OSWispa's own voice-to-cursor story, truthful platform boundaries, local assets, and automated claim/link validation ([PR #35](https://github.com/tylerbuilds/OSWispa/pull/35)).
- Replaced ambiguous recording notifications with truthful Arming, Listening, Processing, Delivering, Inserted, Copied, and Failed lifecycle states, without transcript previews ([PR #37](https://github.com/tylerbuilds/OSWispa/pull/37)).
- Required release tags to match the current `master` commit, bounded every workflow job, and expanded installed public-package smoke proof across Linux formats ([PR #36](https://github.com/tylerbuilds/OSWispa/pull/36)).

### Fixed

- Isolated Windows recorder sessions so stop, cancellation, error, panic, and immediate restart paths cannot overlap or leak stale state ([PR #34](https://github.com/tylerbuilds/OSWispa/pull/34)).
- Hardened macOS CoreAudio capture with per-session cleanup, native sample-format conversion, exact mono 16 kHz output, and streaming anti-alias filtering before downsampling ([PR #41](https://github.com/tylerbuilds/OSWispa/pull/41)).

### Security

- Stored personalisation in a versioned private file with bounded input, control-character and duplicate rejection, symlink-safe canonical persistence, and no app monitoring or automatic learning.
- Kept dictionary entries local even when the optional remote transcription backend is enabled.
- Kept support intake privacy-safe by explicitly excluding transcripts, recordings, keys, clipboard contents, and raw environment or configuration dumps ([PR #33](https://github.com/tylerbuilds/OSWispa/pull/33)).

## 0.4.2 - 2026-07-18

### Security

- Removed transcript text and remote response bodies from application logs.
- Added atomic owner-only persistence for configuration, clipboard history, and stored remote API keys, with symlink and ownership checks.
- Made microphone recordings owner-only temporary files that are automatically deleted on failed or cancelled paths.
- Restricted remote endpoints to HTTPS unless insecure HTTP is explicitly enabled, and capped remote responses at 2 MiB.
- Moved fallback IPC into an owner-only directory, made socket permissions fail closed, and bounded IPC commands.
- Updated `rust-openssl` to 0.10.80, closing eight indexed memory-safety alerts in the remote HTTPS dependency path.
- Enabled GitHub vulnerability alerts, automated security-update pull requests, private vulnerability reporting, secret scanning, push protection, pinned Rust CodeQL analysis, and protected `master` with required PR checks.

### Fixed

- Clipboard verification now fails truthfully instead of allowing stale clipboard contents to be auto-pasted.
- Recording state now clears after command and transcription failures; the tray displays the configured hotkey.
- Corrupt configuration now produces an actionable error and remains intact for recovery.
- Model downloads reject HTTP errors, incomplete payloads, path traversal, and files without a valid GGML/GGUF header.
- Startup and Settings validate existing models so interrupted downloads from older releases can be repaired.
- Corrected the curated `large-v3` filename and consolidated first-run model downloads onto the validated path.
- The source installer now downloads models safely, uses the application's real macOS data directory, and delays service startup when a new Linux group login is required.
- Multi-GPU ROCm builds include every detected architecture, select the largest-VRAM device, and preserve that choice during runtime VRAM checks.
- RPM runtime requirements and Linux desktop categories now match the application.
- Windows now has working WASAPI recording, global push-to-talk hotkeys, verified clipboard delivery, and text insertion instead of compile-only stubs.

### Changed

- The Linux source installer manages `ydotoold` and OSWispa as user services and removes the duplicate desktop autostart path.
- Compatible dependency updates repair the audited `anyhow` and `memmap2` versions and move Wayland clipboard support off its future-incompatible release line.
- CI now enforces formatting, strict Clippy, shell fixtures, package smoke tests, desktop validation, and RustSec auditing.
- Release automation pins every action, validates tag/version/master ancestry, builds Linux on Ubuntu 22.04, verifies package requirements, and publishes one checksummed asset set.
- Release candidates now install and execute both macOS DMGs and the Windows ZIP on hosted VMs before artefacts can be published.

### Documentation

- Added a privacy notice, security reporting policy, and [full July 2026 audit](docs/AUDIT-2026-07-18.md).
- Clarified package compatibility, installer service behaviour, model locations, and the boundary between released and unreleased fixes.
- Added a Windows package readme covering microphone permission, first-run setup, SmartScreen, and the default Ctrl+Windows push-to-talk shortcut.

### Evidence

- Runtime truth and transcript privacy: [PR #17](https://github.com/tylerbuilds/OSWispa/pull/17).
- Model and source-installer integrity: [PR #18](https://github.com/tylerbuilds/OSWispa/pull/18).
- Temporary-audio privacy and GPU selection: [PR #19](https://github.com/tylerbuilds/OSWispa/pull/19).
- CI, package and release hardening: [PR #20](https://github.com/tylerbuilds/OSWispa/pull/20), [PR #26](https://github.com/tylerbuilds/OSWispa/pull/26), and [PR #28](https://github.com/tylerbuilds/OSWispa/pull/28).
- Windows runtime and installed-package VM gates: [PR #29](https://github.com/tylerbuilds/OSWispa/pull/29) and [release rehearsal 29654348248](https://github.com/tylerbuilds/OSWispa/actions/runs/29654348248).

## Version Timeline

| Version | Date | Kind | Evidence |
| --- | --- | --- | --- |
| v0.5.0 | 2026-07-18 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.5.0) |
| v0.4.2 | 2026-07-18 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.4.2) |
| v0.4.1 | 2026-03-13 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.4.1) |
| v0.4.0 | 2026-02-26 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.4.0) |
| v0.3.3 | 2026-02-19 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.3.3) |
| v0.3.2 | 2026-02-15 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.3.2) |
| v0.3.1 | 2026-02-15 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.3.1) |
| v0.3.0 | 2026-02-15 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.3.0) |
| v0.2.2 | 2026-02-13 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.2.2) |
| v0.2.1 | 2026-02-13 | Tag only | [Git tag](https://github.com/tylerbuilds/OSWispa/tree/v0.2.1) |
| v0.2.0 | 2026-02-13 | Tag only | [Git tag](https://github.com/tylerbuilds/OSWispa/tree/v0.2.0) |
| v0.1.1 | 2026-02-13 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.1.1) |
| v0.1.0-alpha.1 | 2025-12-28 | Release | [GitHub Release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.1.0-alpha.1) |

## Capability History

### Wave 1 - Local dictation foundation (2025-12-11 to 2025-12-28)

#### Delivered capability

- Built a local-first Rust dictation application around Whisper inference, microphone capture, configurable hotkeys, transcript history, clipboard delivery, and tray controls.
- Added AMD ROCm acceleration with lazy model loading and CPU fallback when GPU allocation fails.
- Published the first public alpha with Linux install guidance, model downloads, a user guide, and a launch guide.

#### Closed workstreams

- Initial local dictation loop and configuration surface.
- Linux hardware acceleration and resilient fallback.
- First public packaging and documentation pass for [v0.1.0-alpha.1](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.1.0-alpha.1).

#### Representative commits

- [Initial OSWispa implementation](https://github.com/tylerbuilds/OSWispa/commit/2a28797e684e36231d2656d49fd20c97e3b0b515).
- [Lazy GPU loading and VRAM-aware fallback](https://github.com/tylerbuilds/OSWispa/commit/33f5f5b05ae1619db6fa810665b6b2f5d1b2bde0).

### Wave 2 - Configurable backends and models (2026-02-13)

#### Delivered capability

- Added local, remote, and automatic backend modes, configurable trigger keys, custom model import, and expanded settings controls.
- Hardened the early IPC and paste paths, made GPU backends opt-in for portable CI, and added CI/release automation.
- Introduced Debian packaging and iterated rapidly through v0.2.0, v0.2.1, and the published v0.2.2 release.

#### Closed workstreams

- Backend selection and remote transcription configuration.
- Hotkey v2 and custom model management.
- Debian packaging and portable build automation, culminating in [v0.2.2](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.2.2).

#### Representative commits

- [Backend modes, trigger keys, and model import primitives](https://github.com/tylerbuilds/OSWispa/commit/cee0868dfae691cf9ae15d6cd761e9e9e2a325dc).
- [Expanded GUI settings for the new controls](https://github.com/tylerbuilds/OSWispa/commit/d4e15df063b51d6353116e2e12cb16c835183f44).
- [Debian package support](https://github.com/tylerbuilds/OSWispa/commit/762f2298b9bbf569f4e270df689d4cb7a3eb0bd3).

### Wave 3 - Cross-platform delivery and reliable insertion (2026-02-15 to 2026-03-13)

#### Delivered capability

- Improved Ubuntu clipboard handling, Wayland/X11 insertion, WAV correctness, and direct typing through `ydotool`.
- Added NVIDIA and AMD GPU auto-detection, macOS support, a hardware-aware first-run setup wizard, and a packaged macOS app bundle.
- Added persistent local Whisper contexts, protected the hotkey listener from simulated-input loops, and expanded the curated model list through v0.4.1.

#### Closed workstreams

- Cross-platform build compatibility in [PR #10](https://github.com/tylerbuilds/OSWispa/pull/10).
- Ubuntu paste reliability in [PR #14](https://github.com/tylerbuilds/OSWispa/pull/14).
- v0.4.1 release packaging in [PR #15](https://github.com/tylerbuilds/OSWispa/pull/15).

#### Representative commits

- [Reliable Linux paste and WAV repair](https://github.com/tylerbuilds/OSWispa/commit/a98f5ed8295bea912612dd0406908316b8007dc1).
- [GPU auto-detection and macOS support](https://github.com/tylerbuilds/OSWispa/commit/d3a9c25293e44084c4bc1ba754675f97e3ce8643).
- [Packaged macOS app bundle](https://github.com/tylerbuilds/OSWispa/commit/0031b1c9cecda0792330727e10635dbd8400a33f).

### Wave 4 - Post-release reliability, privacy, and release engineering (2026-07-13 to 2026-07-18)

#### Delivered capability

- Made Linux microphone discovery and startup reliable, including hot-plug recovery and better device diagnostics.
- Corrected false-success clipboard behaviour, recording-state recovery, transcript logging, remote response handling, configuration persistence, and IPC permissions.
- Hardened model acquisition, installer service management, temporary audio lifecycle, multi-GPU runtime selection, dependencies, CI, packaging, and release publication.
- Replaced the Windows compile-only stubs with native recording, hotkey, clipboard and text-insertion paths, then added a downloadable Windows ZIP.
- Added installed-app release gates on Apple Silicon macOS, Intel macOS and Windows hosted VMs.

#### Closed workstreams

- Linux audio startup reliability in [PR #16](https://github.com/tylerbuilds/OSWispa/pull/16).
- Runtime truth and transcript privacy in [PR #17](https://github.com/tylerbuilds/OSWispa/pull/17).
- Model and installer integrity in [PR #18](https://github.com/tylerbuilds/OSWispa/pull/18).
- Temporary-audio privacy and GPU correctness in [PR #19](https://github.com/tylerbuilds/OSWispa/pull/19).
- CI, release and security automation in [PR #20](https://github.com/tylerbuilds/OSWispa/pull/20), [PR #26](https://github.com/tylerbuilds/OSWispa/pull/26), and [PR #28](https://github.com/tylerbuilds/OSWispa/pull/28).
- Windows runtime and VM-tested desktop packages in [PR #29](https://github.com/tylerbuilds/OSWispa/pull/29).

#### Representative commits

- [Reliable Linux microphone selection](https://github.com/tylerbuilds/OSWispa/commit/3ece99203afab9e60f016f92b29d1e038e8cfd03).
- [Runtime truth and privacy fixes](https://github.com/tylerbuilds/OSWispa/commit/3984a298eafbec12d2e5b33997fe0e6fc18421ac).
- [Model and source-installer integrity](https://github.com/tylerbuilds/OSWispa/commit/e05d4f1bbbc4e37ccffd899a3a61844d53854dee).
- [Temporary-audio and GPU selection hardening](https://github.com/tylerbuilds/OSWispa/commit/679f21e4850a94645b4f7a4288d734182e12a99e).
- [Windows runtime and VM-tested desktop packages](https://github.com/tylerbuilds/OSWispa/commit/32fdfc4665b678dbe29e03f12e880d5f69663aad).

## Notes for Agents

- Do not describe `Unreleased` entries as shipped until a matching tag and GitHub Release exist.
- Keep release dates aligned with GitHub's publication timestamp; use tag links for versions without a GitHub Release.
- Link fixes to a pull request or full commit URL so future audits can trace the claim to repository evidence.
