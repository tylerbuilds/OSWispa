# MorpheOS Voice rebrand: current-state audit

- **Audit date:** 18 July 2026
- **Source branch:** `master`
- **Source commit:** `f3879ee06fdb2df5b0728130cfcbc47221590334`
- **Working tree at audit start:** clean
- **Current public release:** `v0.4.2`
- **Legacy public name:** OSWispa / OS Whisper
- **Target public name:** MorpheOS Voice

This is the required read-only baseline. No application or website files were changed before this inventory was completed. It covers the tracked repository, current GitHub repository and release metadata, the live MorpheOS website, and the current `/voice` route.

## Classification key

- **A — User-facing, safe to rename now.** Display copy or assets with no persistence or update identity.
- **B — Internal, safe to rename now.** Private implementation labels that do not form a stored or external contract.
- **C — Compatibility-sensitive; migration required.** A rename needs a tested alias, copy or upgrade path.
- **D — External identifier retained for this release.** Changing it now could split installs, permissions, packages, links or state.
- **E — Historical reference retained deliberately.** Release history and evidence must continue to use the name that was true at the time.

## Executive assessment

The product is a Rust push-to-talk dictation application with a working CLI-centred runtime, a Linux GTK/tray surface, alpha macOS and Windows packages, and an unreleased Tauri shell foundation. Local Whisper.cpp transcription is the default. An explicit remote, OpenAI-compatible endpoint is also supported and receives audio when selected.

The rebrand can safely change display names, copy, visual assets, source comments, current documentation and the unreleased desktop-shell presentation. It must not blindly rename the existing `oswispa` package, executable, configuration/data locations, environment variables, Unix socket, services, bundle identifier, GitHub repository or published release assets. Those identifiers are part of installed or external contracts. The safest first MorpheOS Voice release retains them as documented compatibility identifiers while introducing the new public name.

The repository does not contain an action/agent-command mode. It should therefore remain a dictation product and must not publish “One key to write. One key to act.”

## Architecture and languages

| Area | Current implementation | Evidence | Class |
|---|---|---|---|
| Core application | Rust 2021 crate, `oswispa` v0.4.2 | `Cargo.toml:1-6` | D package name; A description |
| Speech engine | `whisper-rs` bindings to Whisper.cpp, CPU by default with optional CUDA, HIPBLAS and Metal features | `Cargo.toml:14-16`, `Cargo.toml:76-80`, `src/transcribe/mod.rs` | E technical engine name |
| Recording | Linux `arecord`; macOS and Windows CPAL native backends; temporary WAV hand-off | `src/audio/` | B backend labels |
| Hotkeys | Linux `evdev`; macOS and Windows `rdev` | `src/hotkey/` | B backend labels; C shortcut state |
| Text delivery | Platform clipboard plus simulated insertion | `src/input/` | B |
| Linux UI | GTK4 settings and KSNI tray | `src/settings/`, `src/tray/linux.rs` | A display copy |
| Desktop foundation | Tauri 2 Rust shell with static HTML/CSS/JavaScript UI | `desktop/src-tauri/`, `desktop/ui/` | A display copy; D identity |
| Public website | Static HTML, CSS and JavaScript with Python claim/link validation | `website/`, `scripts/check_website.py` | A |
| Automation | YAML GitHub Actions; Bash and PowerShell packaging/smoke scripts | `.github/workflows/`, `install.sh`, `scripts/` | B display names; D contracts |

## Verified platform status

| Platform | Released behaviour | Current proof boundary | Rebrand treatment |
|---|---|---|---|
| Linux x86-64 | Primary alpha; Debian, RPM and tarball; tray and GTK settings; local/remote transcription; clipboard and insertion | CI/package smoke proof exists. This Fedora host can build and test. Physical microphone/hotkey/focus proof is separate. | Rename display surfaces. Retain CLI/package/service/path contracts initially. |
| macOS arm64/x86-64 | Alpha DMG and ZIP; CoreAudio capture, global hotkey, clipboard/insertion; Terminal-hosted app | v0.4.2 package smoke passed on hosted macOS runners. Unsigned/unnotarised; physical permission, microphone, hotkey and focused insertion proof remains outstanding. | Rename bundle display copy when the next package is prepared; retain current bundle ID until signed upgrade/permission proof. |
| Windows x86-64 | Alpha ZIP; WASAPI, global hotkey, clipboard/insertion; console-hosted app | v0.4.2 installed ZIP smoke passed on hosted Windows. Unsigned; no installer/tray in the released package; physical proof remains outstanding. | Rename visible copy and future outer package only with legacy executable compatibility. |

Current `master` CI and CodeQL are green at the audited commit. That proves compilation, automated tests, package metadata and hosted smoke contracts; it does not prove the physical dictation loop.

## Name and identity inventory

### Packages, executables and application identities

| Contract | Current value | Location | Decision |
|---|---|---|---|
| Rust package/crate | `oswispa` | `Cargo.toml` | **D** — retain for the first rebrand release; changing the Linux package identity could break upgrades. |
| Desktop Rust package | `oswispa-desktop` | `desktop/src-tauri/Cargo.toml` | **D** build contract for now; it is not user-facing. |
| Primary executable | `oswispa` / `oswispa.exe` | Cargo default binary and release workflows | **D** — retain as a compatibility command. A later `morpheos-voice` alias may be added and proved. |
| Linux helper | `oswispa-toggle` | `scripts/oswispa-toggle.sh`, packages | **D** IPC/installation contract. |
| macOS app/launcher | `OSWispa.app`, executable `OSWispa`, embedded binary `oswispa` | `.github/workflows/release.yml`, `packaging/macos/` | **C/D** — public app name can change only with an upgrade/permission plan; current v0.4.2 asset remains historical. |
| Tauri product name | `OSWispa` | `desktop/src-tauri/tauri.conf.json` | **A** display name. |
| Tauri/release bundle ID | `com.tylerbuilds.oswispa` | Tauri config and macOS plist template | **D** — retain until signing, permissions and upgrade continuity are proved. |
| GTK settings app ID | `com.oswispa.settings.<pid>` | `src/settings/dialog.rs` | **D** internal desktop identity; changing it adds no user value in this pass. |
| macOS launch agent | `com.oswispa.agent` | `install.sh` | **D** — installed auto-start contract. |
| Linux user service | `oswispa.service` | `install.sh` | **D** — installed service/upgrade contract. |
| Linux desktop file/icon | `oswispa.desktop`, `oswispa.svg`, `Exec=oswispa`, `Icon=oswispa` | `packaging/linux/` and `install.sh` | **C/D** — display `Name` is A; filenames and Exec/Icon contracts remain. |
| Unix IPC socket | `oswispa.sock`, fallback `/tmp/oswispa-<uid>/oswispa.sock` | `src/runtime.rs`, `scripts/oswispa-toggle.sh` | **D** — live helper/daemon contract. |

### Release and installer names

The published v0.4.2 release contains these legacy assets and `SHA256SUMS`:

- `oswispa-linux-amd64.tar.gz`
- `oswispa_amd64.deb`
- `oswispa_x86_64.rpm`
- `oswispa-macos-arm64.zip` and `.dmg`
- `oswispa-macos-x86_64.zip` and `.dmg`
- `oswispa-windows-x86_64.zip`

The release workflow, public-release smoke workflow, README, website and validator all depend on those exact names. They are **D** for v0.4.2 and **E** in historical release records. Future MorpheOS Voice assets require aliases or a coordinated migration, not deletion of legacy download names.

## User-facing surface inventory

| Surface | Current legacy references | Current capability/truth | Class |
|---|---|---|---|
| Linux tray | Titles, status labels, tooltip, Help URL and settings entry say OSWispa | Real runtime state; history menu exposes transcript previews; settings are Linux-only | A copy; D tray ID |
| Tauri tray | “Open OSWispa”, tooltip and tray ID | Start/stop/cancel are wired to the engine; Settings and History pages are preview contracts, not persisted native controls | A copy; D tray ID |
| Tauri windows | OSWispa settings, history/recovery and Signal titles | Lifecycle Signal is real; most settings/history content is synthetic preview data | A copy; do not imply production wiring |
| GTK settings | “OSWispa Settings”, model/dictionary import and export labels | Real Linux configuration editor | A copy; D app ID |
| First-run onboarding | Terminal banner, device probe, auto model selection/download and success copy say OSWispa | Real CLI flow. Embedded desktop mode deliberately fails to Needs Attention rather than showing a hidden terminal wizard | A copy; UX remains PARTIAL |
| Permission copy | macOS microphone descriptions say OSWispa; docs explain Accessibility and Linux input-group access | Tauri plist contains microphone text, but Tauri bundling is disabled; released plist has separate Terminal/Apple Events wording | A copy; D bundle identity |
| Errors/recovery | CLI, tray, installer and package errors say OSWispa; Tauri lifecycle reports inserted/copied/needs-attention | Clipboard-only fallback is real. Text history is real in the runtime/Linux tray. Tauri history is synthetic and audio is not recoverable after a crash | A copy; behaviour must remain accurately qualified |
| About | No production About screen. Tauri settings contains a development-foundation panel | Must not be described as a complete About/licence surface | A copy; UX gap |
| Notifications | Linux notification/tray status strings use OSWispa and truthful lifecycle labels | Real Linux behaviour | A |
| CLI/logging | Startup and error text uses OSWispa; smoke marker is `OSWISPA_PLATFORM_SMOKE_OK` | Logs intentionally avoid transcript contents | A display text; D smoke marker |

## Documentation and public project surfaces

| Surface | State | Class/action |
|---|---|---|
| README | User and developer information mixed together; opens with “Open Source Whisper Assistant” and several enthusiastic/overbroad claims | **A** rewrite users first; keep historical release URLs **D/E** |
| Website | GitHub Pages site names OSWispa and uses an unrelated dark/neon visual system | **A** replace with MorpheOS family treatment and canonical `/voice` metadata |
| MorpheOS website | `https://morpheos.net/` is live and uses IBM Plex Sans/Mono, a simple `M` mark, strong black/white contrast and practical product copy | Brand evidence; `/voice` currently returns 404 |
| Legacy deployment | GitHub Pages deploys `website/`; optional nginx config targets `oswispa.tylerbuilds.com` | **D** do not silently deploy or delete; prepare `/voice` hand-off separately |
| Support/security/contributing/privacy | Present but branded OSWispa | **A** display rename; paths and old release links remain where required |
| Changelog and July audit | Detailed OSWispa release history | **E** retain historical names and add a rebrand boundary rather than rewriting history |
| Roadmap | Current productisation/release truth under OSWispa | **A** rename current framing; **E** retain released history |
| Missing governance docs | No `CODE_OF_CONDUCT.md`, `THIRD_PARTY_NOTICES.md`, `TRADEMARK.md`, `docs/OPEN_SOURCE.md`, `docs/BUILDING.md` or `docs/RELEASING.md` | Documentation gap |
| GitHub templates | Runtime, installation and feature forms say OSWispa; contact links use current repository URL | **A** display rename; **D** repository links |
| Repository metadata | GitHub description is “Open source hotkey to record, copy and paste text anywhere, developed on Ubuntu”; homepage is unset | External update required later; do not change in this local pass |

## Visual assets, screenshots and demos

Tracked visual files are:

- `website/favicon.svg` — black rounded square with white “OS”.
- `packaging/linux/oswispa.svg` — same legacy “OS” treatment.
- `desktop/src-tauri/icons/icon.svg`, `icon.png`, `icon.ico` — original dark Signal rings/compass mark.

There are no tracked screenshots, demo recordings, product photographs, webfonts, font binaries or social cards. The website workflow illustration is HTML/CSS and explicitly labelled as an illustration. No current-build screenshot can therefore be reused or falsely presented as live proof.

The live MorpheOS parent site is the only verified connected brand reference found. A conservative interim Voice treatment should inherit its typography, `M` parent mark, black/white contrast and practical tone, while retaining a distinct small waveform/cursor signal. Unresolved trademark/logo refinements should be marked `[NEEDS TYLER]`.

## Persistence and compatibility inventory

### Stored state

`directories::ProjectDirs::from("com", "oswispa", "OSWispa")` determines application directories. Confirmed/documented paths include:

- Linux configuration: `~/.config/oswispa/config.json`
- Linux data: `~/.local/share/oswispa/`
- macOS data/model path: `~/Library/Application Support/com.oswispa.OSWispa/`
- Platform-specific Windows directories resolved by the `directories` crate from the same legacy tuple
- `history.json` — bounded text transcript history
- `personalisation.json` — explicit personal dictionary
- `models/` — downloaded/imported Whisper model files
- `secrets/remote_api_key` — optional remote token, owner-only on Unix
- `config.json` — shortcut, model path, backend and preference state
- `~/Library/Logs/oswispa.log` for the source-installed macOS launch agent

There is no database, keychain integration, template/snippet store, deep-link scheme or updater database. Configuration, history, dictionary and secret writes are atomic/private on Unix and reject symlinks. The token is stored in a `0600` file on Unix, not an operating-system keychain.

### Environment and protocol contracts

- `OSWISPA_SETUP_MANUAL`
- `OSWISPA_REMOTE_API_KEY` (documented example; the configured variable name is user-selectable)
- `OSWISPA_BASE_MODEL_MIN_BYTES` (installer helper)
- `OSWISPA_PLATFORM_SMOKE_OK` (CI/package proof marker)
- `oswispa.sock` plus `oswispa-toggle` text commands

All are **D** in the first rebrand release. New aliases can be additive later; removing or silently changing them would break existing automation or stored configuration.

## Privacy and data-flow findings

- Capture begins only after the recorder reports successful start; the lifecycle distinguishes Arming from Listening.
- Each attempt uses a unique owner-only temporary WAV. RAII cleanup removes it after transcription and on handled error/drop paths.
- Local mode uses a downloaded Whisper.cpp model on the same computer. Models are stored locally and are not uploaded by the app.
- Remote mode is opt-in. It sends the WAV bytes, remote model name, optional language/task fields and bearer token to the configured endpoint. The endpoint provider, retention and processing terms are outside MorpheOS Voice's control.
- Remote failure can fall back to an installed local model.
- Personal dictionary replacements are local and its bounded vocabulary prompt is only supplied to local Whisper.
- Text transcripts are stored in `history.json` up to `max_history`; audio is not retained as recovery history.
- There is no product telemetry or analytics in the application. The current project website loads no analytics or remote fonts; its optional contact form posts user-entered fields to FormSubmit.
- Users can clear history through existing runtime/tray behaviour, but the Tauri History page is only a synthetic fixture. There is no single, verified cross-platform “clear all cached audio and history” UI.
- A crash cannot recover in-progress audio or transcription. Completed transcript recovery is limited to clipboard/history state that was successfully persisted.

Current “local-first” language is supported only when paired with the explicit remote-mode qualification. “Nothing leaves your device” is not a valid global claim.

## Command and agent functionality

The reusable engine exposes `Start`, `Stop`, `Cancel` and `Shutdown` commands for dictation lifecycle control. No code path interprets transcripts as desktop actions, drives an autonomous agent or provides an approval/proof loop for consequential actions. This is Write mode only in product terminology. “Act mode” belongs in future-direction documentation only.

## Open-source and licence baseline

- The repository `LICENSE` is the standard MIT licence with `Copyright (c) 2026 Tyler Casey`; repository metadata also identifies MIT.
- Cargo metadata reports an SPDX licence expression for every locked Rust package. The dependency graph includes MIT, Apache-2.0, BSD, MPL-2.0, Unicode, Zlib, BSL and other permissive/copyleft-compatible components; it is not accurate to call every component MIT.
- `whisper-rs` and `whisper-rs-sys` declare the Unlicense.
- Whisper.cpp is MIT-licensed. The configured `ggerganov/whisper.cpp` model repository declares MIT and models are downloaded separately rather than committed to this repository.
- Tauri and the official single-instance plugin are MIT OR Apache-2.0.
- CPAL is Apache-2.0; platform clipboard/hotkey/UI stacks retain their own licences.
- System tools such as `ydotool`, `wl-clipboard`, `xclip`, `xdotool`, `alsa-utils` and netcat are package dependencies, not relicensed source.
- The icon SVGs appear project-authored; no external asset attribution is present. There are no bundled fonts.

The project lacks a third-party notices inventory. Public copy must say: “The original MorpheOS Voice source code is MIT-licensed. Third-party dependencies and optional speech models retain their respective licences.”

## Updates, signing and release automation

- Tauri updater artefacts are disabled and no update feed exists.
- GitHub Releases plus checksums are the current release source of truth.
- macOS packages are unsigned and unnotarised. The unreleased Tauri config requests hardened runtime and audio-input entitlement but bundling is disabled.
- Windows ZIPs are unsigned and there is no MSI/MSIX installer or stable publisher identity.
- CI builds/tests Linux, macOS and Windows, validates the website/UI contracts and runs dependency audit/CodeQL.
- Release automation validates tag/version/master ancestry, packages all three systems, runs hosted package smoke checks and publishes only from tags.
- Public-release smoke automation redownloads, checksum-verifies, installs/extracts and launches published assets.
- GitHub Pages publishes on `website/**` changes to `master`; it does not publish `morpheos.net/voice`.

## Tests and fixtures containing the legacy name

Legacy strings occur in Rust unit tests, Linux input fixtures, personal-dictionary examples, Tauri native-shell contract tests, UI contract tests, installer fixtures, website claim tests, CI smoke markers and release/public-smoke workflows. Display assertions may move to MorpheOS Voice. Compatibility assertions for package paths, bundle ID, executable names, environment variables and smoke markers must remain or be expanded to prove deliberate retention.

Existing automated tests do not prove:

- native first-run completion inside the Tauri shell;
- real permission prompts;
- physical microphone/hotkey/focus behaviour;
- installer upgrade from OSWispa to MorpheOS Voice;
- cross-platform settings/history UI persistence;
- crash recovery for in-progress audio.

## Reference classification by repository area

| Area | Classification | Required treatment |
|---|---|---|
| Current app/tray/settings/onboarding/permission/error strings | A | Rename to MorpheOS Voice. |
| Website/README/current guides/forms/support copy | A | Rewrite in the approved voice and hierarchy. |
| Source comments, internal thread names and non-contract UI bridge errors | B | Rename where it improves clarity. |
| Stored settings, shortcuts, model paths, history, dictionary and secrets | C | Preserve through the legacy directory contract now; add migration only when the destination identity is stable. |
| Executable/package/service/socket/env/bundle/update/repository identifiers | D | Retain and document for this release. Do not delete aliases. |
| v0.4.2 assets, changelog entries, July audit, old PRs/tags/releases | E | Retain the historical name and explain the rebrand boundary. |
| Technical references to Whisper/Whisper.cpp | E | Retain only where describing the supported engine/model. Do not use as the product name. |

## Prioritised findings

### High — Native first-use experience is not complete

- **Location:** `desktop/src-tauri/src/lib.rs`, `desktop/ui/index.html`, `src/setup.rs`
- **Issue:** The Tauri shell can start the engine, but embedded first-run setup deliberately fails to Needs Attention when no model exists; the Settings and History pages remain preview-only.
- **Root cause:** Product UI was added as a bounded foundation without exposing file/config commands or a native onboarding workflow.
- **Required fix:** Rebrand the truthful surfaces but mark first-run/native Settings/History as partial until real bridge commands and permission/model flows are implemented and physically tested.

### High — Identity rename can split existing installations

- **Location:** `src/runtime.rs`, `install.sh`, `Cargo.toml`, `packaging/`, `.github/workflows/release.yml`
- **Issue:** A blind rename would orphan configuration, shortcuts, models, history, dictionary, services and package-manager state.
- **Root cause:** The legacy product name is embedded in installed identifiers as well as display copy.
- **Required fix:** Retain compatibility identifiers for the first rebrand release and explicitly test that legacy state remains the canonical read path. Introduce new aliases only with upgrade/uninstall/rollback proof.

### High — Current packages are not ready for a branded production release

- **Location:** macOS/Windows packaging and release workflow
- **Issue:** macOS is unsigned/unnotarised and Terminal-hosted; Windows is an unsigned ZIP with no installer. Physical dictation and permission flows are not proved on either platform.
- **Required fix:** Do not tag or publish a MorpheOS Voice release until signing identity, installers and native hardware proof are complete.

### Medium — Public privacy wording needs a visible mode boundary

- **Location:** `website/index.html`, `README.md`, `PRIVACY.md`, desktop UI
- **Issue:** “Private by architecture” and broad local-first headlines can be read as applying to remote mode; the active backend is not consistently visible on every primary surface.
- **Required fix:** Use explicit Local/Cloud labels, retain the remote data-flow disclosure and avoid global “nothing leaves” claims.

### Medium — Open-source distribution documentation is incomplete

- **Location:** repository root and `docs/`
- **Issue:** MIT source licensing is present, but third-party/model notices, code of conduct, trademark identity, build and release documents are missing.
- **Required fix:** Add the missing files and distinguish original MIT source from third-party licences.

### Medium — Canonical business route is not deployed

- **Location:** public web state and `.github/workflows/deploy-website.yml`
- **Issue:** `https://morpheos.net/voice` returns 404; the repository publishes to GitHub Pages and retains a legacy nginx target.
- **Required fix:** Prepare `/voice`-safe assets and canonical metadata locally. Publishing or changing deployment ownership requires Tyler's external-change approval.

### Low — Current visual treatment does not belong to the MorpheOS family

- **Location:** `website/styles.css`, `desktop/ui/styles.css`, legacy icons
- **Issue:** Star-field gradients, serif display type and the legacy OS mark diverge from the parent site's IBM Plex, restrained black/white system.
- **Required fix:** Adopt a conservative MorpheOS-aligned Voice treatment with accessible contrast and a simple speech/cursor motif.

## Audit gate decision

The audit is complete and implementation may proceed in reviewable batches. The first implementation must preserve the compatibility contracts identified as C/D, must not rewrite historical E references as if they never existed, and must not publish, tag, rename the GitHub repository or alter production deployment without a final external-change decision.
