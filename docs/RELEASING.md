# Releasing MorpheOS Voice

Do not publish a release because the source name and website changed. A release requires green automated checks plus explicit native, signing and upgrade evidence.

## Current identity boundary

- Public product: MorpheOS Voice
- Compatibility package/command: `oswispa`
- Current live release: v0.5.0 public alpha under the MorpheOS Voice name
- Canonical future page: `https://morpheos.net/voice`
- Current official repository: `https://github.com/tylerbuilds/OSWispa`

The transition release keeps legacy outer asset filenames so existing `releases/latest/download` URLs continue to work. Historical assets are never replaced in place.

## Pre-release gate

1. Confirm the release-readiness report has no unaccepted FAIL/BLOCKED item.
2. Confirm the branch is clean and based on current protected `master`.
3. Run format, strict Clippy, all non-hardware tests, UI/site validators and dependency audit.
4. Build Linux Debian/RPM/tar packages and the macOS/Windows packages through the protected workflow.
5. Confirm app/window/installer display copy says MorpheOS Voice while compatibility identifiers remain intentional.
6. On real target hardware, prove microphone permission, hotkey press/release, transcription, focused insertion, copied-only fallback, rapid retry and cancellation.
7. Prove an existing v0.4.2 install retains settings, shortcut, models, history and dictionary.
8. Prove clean install, upgrade, uninstall and rollback for every installer format.
9. Sign Windows/macOS artefacts and notarise macOS packages using approved publisher identities.
10. Review `THIRD_PARTY_NOTICES.md`, `PRIVACY.md` and release notes against the exact artefacts.
11. Obtain Tyler's approval for the version, external repository/site changes and publication.

## Version and tag contract

The project is pre-1.0. A product rebrand with new desktop behaviour is a minor release, not a patch. The exact version remains `[NEEDS TYLER]` until the release gates pass.

The release workflow requires:

- tag `vX.Y.Z` to match `Cargo.toml`;
- tag commit to be the current `master` commit;
- tag commit to be contained in protected `master`;
- the complete expected asset set to exist before publication.

Never move a published tag. If a release candidate is wrong, fix it and use a new version.

## Publication sequence

1. Merge reviewed changes only after required checks pass.
2. Update all workspace versions and `Cargo.lock`.
3. Move release entries out of `Unreleased` in `CHANGELOG.md`.
4. Re-run the complete test gate.
5. Create the annotated version tag from current protected `master` and push it.
6. Watch every release job to completion.
7. Confirm release notes and all checksum-covered assets.
8. Dispatch `Public Release Smoke` for the new tag.
9. Verify the public downloads again on real hardware.
10. Publish `morpheos.net/voice` only when its downloads and platform copy match the release.
11. Record signing, package, hardware and public-URL evidence in the release-readiness report.

## Rollback

- Do not delete the previous GitHub Release or its assets.
- Preserve the legacy state directory and compatibility executable.
- If the new release fails, mark it clearly, restore website download links to the last proved release and publish a new fixed version.
- Never ask users to delete their configuration or model directory as a routine rollback step.

## v0.5.0 publication decision

Tyler explicitly approved the v0.5.0 public-alpha publication, GitHub Release and `/voice` deployment on 18 July 2026. That acceptance does not turn the remaining native onboarding, physical-platform, signing or upgrade/rollback gaps into PASS results. Keep the alpha/platform limitations visible, retain the previous release for rollback and fix forward with a new version if a release defect is found.
