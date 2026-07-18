# MorpheOS Voice launch guide

This is the maintainer checklist for a MorpheOS Voice release and the matching `morpheos.net/voice` product page. A rebrand, successful compilation or hosted-VM smoke test is not enough to release.

## Before the release pull request

1. Confirm the rebrand audit, migration map, test results and release-readiness report match the exact source commit.
2. Run format, strict Clippy, Rust tests, desktop UI tests, installer fixtures, website validation and dependency audit.
3. Confirm public surfaces say MorpheOS Voice while every retained `oswispa` identifier is documented and tested.
4. Prove a representative existing v0.4.2 configuration keeps its shortcut, model path, history and personal vocabulary.
5. Review `PRIVACY.md`, `THIRD_PARTY_NOTICES.md`, `TRADEMARK.md` and release notes against the artefacts to be built.

## Platform release gate

1. Build the Linux tarball, Debian package, RPM package, both macOS ZIP/DMG pairs and Windows x86-64 ZIP in the protected release workflow.
2. Install and execute those packages on clean hosted runners. Require version/platform, native backend, WAV contract and clipboard round-trip checks.
3. On physical target hardware, prove microphone permission, shortcut press/release, a real local transcription, focused-app insertion, copied-only fallback, cancellation and rapid retry.
4. Prove upgrade and rollback from v0.4.2 without deleting the established application-data directory.
5. Sign the Windows and macOS artefacts and notarise the macOS packages before presenting them as normal consumer downloads.

Hosted VMs do not prove physical microphones, permission prompts, global keypress delivery, focus-sensitive insertion, device hot-plug or GPU acceleration. Record those checks separately.

## Version, tag and publication

1. Update `Cargo.toml`, refresh `Cargo.lock` and move completed changelog entries out of Unreleased.
2. Confirm the release commit is the clean current tip of protected `master`.
3. Create `vX.Y.Z` matching the Cargo package version. The workflow rejects a tag that does not match or does not point at current `master`.
4. Confirm the complete asset set and `SHA256SUMS` before publication.
5. After the GitHub Release exists, dispatch `Public Release Smoke` for the new tag and let it download, checksum, install and rerun package checks against the public assets.
6. Re-run the physical workflow proof against the public candidate.

## Publish `morpheos.net/voice`

The repository's `website/` directory is a hand-off artefact for the parent MorpheOS site. Do not point DNS or publish it before the release exists.

Before deployment:

- make every download link resolve to the actual public release;
- keep current platform and signing limitations visible;
- set the canonical URL to `https://morpheos.net/voice`;
- serve the files under `/voice` without breaking relative assets;
- verify desktop and mobile layout, keyboard focus, reduced motion and all local links;
- confirm no analytics, form handler, remote font or other new third party was introduced without privacy review; and
- verify the product page links back to MorpheOS without adding a trial, account or freemium gate.

## Rollback

- Keep the previous release and checksum-covered assets available.
- Preserve the legacy state directory, executable and package paths.
- If a release fails, mark it clearly and restore product-page downloads to the last proven release.
- Fix forward with a new version; never move a published tag or silently replace an asset.
