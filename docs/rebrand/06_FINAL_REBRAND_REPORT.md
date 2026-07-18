# MorpheOS Voice final rebrand report

## Recommendation

**DO NOT SHIP a new public cross-platform release yet.** The branch is ready for review, not publication.

## What changed

- Public product name changed to **MorpheOS Voice** with the headline **“Talk instead of type — in any app.”**
- Product role defined as the genuinely free, MIT-licensed/open-source attractor at `morpheos.net/voice`, without a trial, account, paywall or lead-capture gate.
- MorpheOS-family palette, typography, `M` + voice/cursor icon, product mark and documentation lockup added.
- Runtime, Linux tray/settings, installer copy, package display metadata, macOS display metadata, Tauri titles/tray, desktop UI and tests updated.
- README rewritten for users first, with accurate platform, processing, recovery and installation boundaries.
- Static product site rebuilt for `/voice`; contact now uses GitHub/email links rather than a third-party form handler.
- Brand, visual, migration, compatibility, privacy, data-flow, building, releasing, open-source, third-party notice, trademark, conduct, test and readiness documents added or rewritten.
- Compatibility tests and brand-contract tests added.
- Linux `.deb`/`.rpm` structure, extracted binaries and a controlled real local dictation/insertion path tested.

## What deliberately did not change

The transition keeps these identifiers because they own existing installs, state, automation or published URLs:

- Cargo package/crate: `oswispa`
- executable and Windows binary: `oswispa`, `oswispa.exe`
- helper, service and desktop entry: `oswispa-toggle`, `oswispa.service`, `oswispa.desktop`
- bundle/app identifiers: `com.tylerbuilds.oswispa`, `com.oswispa.agent`
- macOS internal launcher executable: `OSWispa`
- tray/IPC IDs and socket: `oswispa`, `oswispa.sock`
- environment and smoke prefixes: `OSWISPA_*`, `OSWISPA_PLATFORM_SMOKE_OK`
- ProjectDirs identity and state/model locations: `ProjectDirs::from("com", "oswispa", "OSWispa")`
- current release asset filenames and official repository URL: `tylerbuilds/OSWispa`
- historical changelog, audit, tags and release names.

The Tauri updater remains disabled. No repository rename, version bump, tag, release, DNS change or production deployment was made.

## Existing-user data

Existing users retain settings, shortcuts, provider configuration, downloaded models, completed transcript history, personal vocabulary and stored remote token because the application continues to use the same established identity and file locations.

There is intentionally no copy/delete migration in this release. Tests prove one legacy identity and a representative settings/shortcut/model-path round trip. Installed-package upgrade and rollback still need target-platform proof before release.

## Platforms actually tested

- **Fedora Linux x86-64:** format/lint/tests, debug/release builds, platform smoke, package generation/metadata, extracted Debian/RPM smoke, browser/UI checks and controlled local dictation into a focused GTK field.
- **macOS:** no current-branch native build or hardware test in this worktree. Historical v0.4.2 hosted-VM evidence is not treated as proof of the rebrand candidate.
- **Windows:** no current-branch native build or hardware test in this worktree. Historical v0.4.2 hosted-VM evidence is not treated as proof of the rebrand candidate.

The Linux controlled run used a temporary PipeWire source and socket start/stop. It did not prove a physical microphone or physical global shortcut.

## Transcription paths actually tested

- **Local Whisper.cpp:** PASS on Linux with the installed `base.en` model. Controlled speech produced a non-empty transcript, and persisted history exactly matched the text visibly inserted into a GTK field.
- **Remote OpenAI-compatible endpoint:** unit/static validation only; no live request. PARTIAL.
- **Offline:** local code path inspected and no remote call is made in local mode, but network isolation was not forced during the E2E run. PARTIAL.

## Privacy claims

The qualified local claim is supported: local mode records to an owner-only temporary WAV, processes with an installed local model, copies/inserts the transcript and drops the temporary file on normal/handled paths. The first model download still uses the network.

The product is not described as universally private/local. Remote mode sends audio and request fields to the chosen provider. The desktop app has no product telemetry. Completed transcript history is local by default. In-progress audio is not recoverable after a crash, and no cross-platform “clear everything” control exists yet.

## Licence findings

- `LICENSE` is valid MIT text with the repository-established holder Tyler Casey, 2026.
- Original MorpheOS Voice source is described as MIT, not the entire distribution.
- Whisper.cpp is MIT; `whisper-rs` bindings are Unlicense; optional model files are separately downloaded and retain upstream/provider terms.
- Every locked Cargo package reported a licence expression.
- Third-party dependencies, system tools, models, fonts/icons and Tauri components are separated in `THIRD_PARTY_NOTICES.md`.
- `TRADEMARK.md` keeps official MorpheOS project identity distinct from MIT fork rights.

## Security and privacy concerns

- Two `quick-xml` RustSec advisories remain explicitly ignored under the existing assessed dependency contexts.
- The current lockfile reports 22 additional unmaintained/unsound warnings, largely in legacy GTK/Tauri/transitive lines; migration remains due.
- macOS/Windows packages are unsigned and macOS is not notarised.
- Linux global shortcuts require low-level input access; Wayland insertion normally relies on `ydotoold`.
- Tauri native onboarding, production Settings and production History are incomplete.
- In-progress audio/transcription has no crash recovery.
- Optional remote credentials use an environment variable or private file, not a platform keychain.

## Open blockers

1. Complete or narrow the native desktop/onboarding release scope.
2. Native current-candidate builds on Linux with WebKitGTK dependencies, macOS and Windows.
3. Physical microphone, shortcut, permission and focused-insertion proof on all advertised platforms.
4. Signed Windows/macOS artefacts and macOS notarisation.
5. Installed v0.4.2 upgrade/uninstall/rollback proof.
6. Protected CI/package/public-download evidence for the exact release commit.
7. Approved version, publisher identity, final mark and repository/site publication plan.
8. Deployment of `morpheos.net/voice` only after the release download set exists.

## Exact next release steps

1. Review this branch and decide whether the next release is Linux-first or must include macOS/Windows consumer readiness.
2. Resolve native onboarding and compile the Tauri runtime against real platform dependencies.
3. Run protected CI for the review commit; fix any target-only regression.
4. Perform physical workflow and upgrade/rollback matrices; attach receipts to the readiness report.
5. Approve signing publisher/team identities and produce signed/notarised candidates.
6. Choose the next pre-1.0 minor version, update Cargo/lock/changelog and rerun the complete test gate.
7. Merge through a reviewed PR. Tag only the current protected `master` commit.
8. Confirm checksum-covered assets, run public-download smoke, then deploy `/voice` with those exact links.

## Decisions requiring Tyler

- Approve or revise the interim `M` + voice/cursor mark.
- Choose Linux-first versus cross-platform release scope.
- Approve the next version and accepted RustSec/dependency debt.
- Confirm macOS/Windows signing identities and whether unsigned builds may remain available as explicitly technical artefacts.
- Decide whether/when to rename or transfer `tylerbuilds/OSWispa` to `morpheos-voice` after redirect and update-channel proof.
- Choose the quiet MorpheOS cross-sell destination on `/voice`.
- Approve external actions: PR, merge, version/tag, GitHub Release and production `/voice` deployment.
