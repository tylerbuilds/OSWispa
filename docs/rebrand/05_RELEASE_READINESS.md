# MorpheOS Voice release readiness

## Verdict

**DO NOT SHIP a new public cross-platform release yet.**

The rebrand, compatibility contract, Linux core loop, local website and Linux package structure are reviewable. Release-critical native onboarding, target-platform proof, signing and upgrade/rollback evidence are still incomplete.

## Gate table

| Gate | Status | Release meaning |
|---|---|---|
| Public brand/copy | PASS | MorpheOS Voice naming, positioning and limitations are consistent across source, UI, package metadata, README and site |
| Legacy state preservation | PASS | One established ProjectDirs identity remains; representative settings, shortcut and model path survive without copy/delete |
| Core Rust quality | PASS | Format, strict Clippy and automated tests pass |
| Linux controlled dictation | PASS | Real capture pipeline, local model, clipboard, focused-field insertion and history equality proved |
| Linux physical workflow | BLOCKED | Physical microphone and physical global-hotkey proof not recorded |
| Native desktop shell | PARTIAL | Source/API type-check and boundary tests pass; native dependency/runtime proof does not |
| Native first-run onboarding | FAIL | Embedded desktop mode still cannot complete model/permission onboarding; it reports Needs Attention |
| Linux package structure | PARTIAL | Local `.deb`/`.rpm` generation and extracted smoke pass; protected compatibility-floor workflow has not run on this branch |
| macOS current candidate | BLOCKED | Native build, real permissions, microphone, hotkey, insertion, signing and notarisation not proved |
| Windows current candidate | BLOCKED | Native build, real permissions, microphone, hotkey, insertion, installer and signing not proved |
| Existing v0.4.2 upgrade and rollback | PARTIAL | Same identity makes in-place compatibility likely and unit proof passes; installed-package upgrade/rollback is not exercised |
| Local privacy claims | PASS | Local-mode data flow and no-telemetry claim match inspected code and controlled run |
| Remote privacy claims | PARTIAL | Code/docs align; no live provider proof |
| Crash/audio recovery | FAIL | In-progress recording/transcription cannot be recovered after a process/OS crash; public copy now says so |
| Dependency security | PARTIAL | No unignored RustSec vulnerability after documented exceptions; 22 warnings remain debt |
| Open-source/licence | PASS | Original source MIT, third-party/model boundaries and official-brand policy are documented |
| Website hand-off | PASS | Local `/voice` artefact and browser QA pass |
| Production website | BLOCKED | `morpheos.net/voice` is not deployed and must not point at unreleased downloads |
| Release publication | BLOCKED | No approved version bump, tag, external PR/merge, signing-credential use or GitHub Release |

## Conditions before SHIP WITH CONDITIONS can be considered

1. Complete the native first-run model and permission flow or explicitly release only the proven CLI/Linux tray path.
2. Build the exact candidate through protected Linux, macOS and Windows workflows.
3. Record physical microphone, global shortcut, cancellation, rapid retry, focused insertion and copied-only fallback on every advertised platform.
4. Sign Windows/macOS artefacts and notarise macOS, or deliberately narrow the release audience and obtain Tyler's explicit risk acceptance.
5. Exercise clean install, v0.4.2 upgrade, uninstall and rollback without deleting legacy state.
6. Re-run dependency/licence review against the exact artefacts.
7. Approve the release version and final mark/publisher identities.
8. Merge through review, publish the GitHub Release, pass public-download smoke, then deploy `/voice` with matching downloads.

## Immediate safe next state

Open a review pull request for the rebrand branch without tagging or deploying. Treat macOS, Windows, native onboarding and signing as release-gate follow-ups rather than soft launch notes.
