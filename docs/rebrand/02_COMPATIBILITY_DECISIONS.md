# MorpheOS Voice compatibility decisions

## Decision summary

The rebrand is display-first and state-preserving. MorpheOS Voice becomes the public product name while existing installed identifiers stay stable for the first transition release.

This is not an incomplete search-and-replace. It is a deliberate decision to avoid breaking existing users before signing identities and cross-platform installers are stable.

## Decisions

### 1. Keep the Rust package and primary command as `oswispa`

**Decision:** Retain `Cargo.toml` package name, crate import, `oswispa`/`oswispa.exe` binary and package asset paths.

**Reason:** Debian, RPM, source installs, shell scripts, services, hotkeys and release workflows depend on these names. Renaming them without replaces/provides rules and installer proof could leave two installations or break upgrades.

**Public treatment:** Documentation calls `oswispa` the transition compatibility command. A future release may add `morpheos-voice` as the preferred alias while continuing to ship `oswispa`.

### 2. Keep the legacy application data identity

**Decision:** Continue using `ProjectDirs::from("com", "oswispa", "OSWispa")` for configuration and data.

**Reason:** This automatically preserves settings, shortcuts, history, dictionary, models and stored remote credentials. It also gives v0.4.2 a rollback path.

**Security:** Existing atomic writes, owner-only Unix permissions and symlink rejection remain in force. No user data is copied, logged or deleted.

### 3. Keep bundle and service identities

**Decision:** Retain `com.tylerbuilds.oswispa`, `com.oswispa.agent`, `oswispa.service`, `oswispa.desktop`, the `oswispa` icon ID and `oswispa.sock`.

**Reason:** These identifiers own permissions, startup behaviour, package files and IPC. The future MorpheOS-owned bundle ID should be selected alongside real signing/publisher identities, not in a cosmetic pass.

### 4. Rename all safe display surfaces

**Decision:** Change current UI, tray, window, onboarding, permission, notification, installer, README, website, support and issue-form copy to MorpheOS Voice.

**Exception:** Historical changelog/audit/release text retains OSWispa where that was the correct released name. Compatibility instructions retain exact legacy commands and paths.

### 5. Keep current release asset names during transition

**Decision:** Do not remove or rename the v0.4.2 assets. The next transition rehearsal may keep the legacy outer filenames while the contained app/display copy says MorpheOS Voice.

**Reason:** `releases/latest/download/...` links, checksums and public-smoke automation use exact names. Branded assets can later be added alongside legacy aliases after installation and rollback proof.

### 6. Use `morpheos.net/voice` as canonical, but do not deploy it in this change

**Decision:** Set canonical/metadata/documentation to `https://morpheos.net/voice` and keep site assets path-relative. Do not alter production DNS, Cloudflare or the MorpheOS site repository.

**Reason:** The route currently returns 404 and the OSWispa repository's Pages workflow cannot safely claim ownership of the parent MorpheOS domain. The local output is a hand-off artefact until Tyler approves production publication.

### 7. Preserve GitHub URLs until repository migration is approved

**Decision:** Current source, issue, discussion, security and release links continue to use `https://github.com/tylerbuilds/OSWispa`.

**Reason:** It is the live official repository. A speculative future URL would create broken support and download paths.

### 8. Do not introduce Act mode

**Decision:** Rebrand the current product as voice typing only.

**Reason:** The engine can start, stop and cancel dictation but cannot approval-gate or prove consequential external actions.

### 9. Do not globally claim private/local processing

**Decision:** Show Local and Remote modes explicitly. Use local/private claims only when they name the local backend boundary.

**Reason:** Remote mode uploads audio and request metadata to a user-configured endpoint and can use a bearer credential.

### 10. Keep the source licence holder as Tyler Casey

**Decision:** Retain `Copyright (c) 2026 Tyler Casey` in the MIT licence.

**Reason:** This is the only legal ownership evidence in the repository. “MorpheOS” is the company/brand name supplied for product copy, but the audit found no basis to replace the legal copyright holder.

### 11. Separate source licensing from brand identity

**Decision:** State that original source is MIT and dependencies/models retain their own licences; add a short practical trademark policy.

**Reason:** MIT permits forks and commercial use. It does not require official MorpheOS branding to be licensed as if it were code.

### 12. Mark native product gaps instead of hiding them

**Decision:** Rebrand the Tauri shell and its truthful lifecycle, but continue to label unwired Settings/History data as preview/development behaviour. Do not call first-run native onboarding complete.

**Reason:** The desktop shell intentionally exposes no generic filesystem/config commands and currently fails fast when embedded setup needs a model. Renaming it does not make the integration complete.

## Compatibility guarantees for the transition release

- Existing `config.json` remains readable.
- Configured hotkeys remain unchanged.
- Existing model paths and downloaded models remain in use.
- Existing `history.json` and `personalisation.json` remain in use.
- Existing optional remote token lookup remains unchanged.
- Existing `oswispa` command, helper, service and socket remain valid.
- Existing release URLs and checksums remain valid.
- The rebrand adds no data-deletion path.
- Rolling back to v0.4.2 does not require a reverse state migration.

## Known compatibility limitations

- A future bundle-ID change may reset macOS permission grants; it must be paired with signing and explicit upgrade UX.
- Changing package names later will need Debian/RPM replaces/provides/conflicts metadata and uninstall proof.
- Running two differently named binaries simultaneously is unsafe until the singleton/IPC contract is deliberately shared.
- The current Windows ZIP has no installer-managed upgrade path.
- The current macOS app is unsigned and Terminal-hosted.
- Existing model paths can point outside the managed data directory; a future migration must preserve rather than normalise them blindly.

## Release gate

These compatibility decisions support a local rebrand implementation and release rehearsal. They do not by themselves make a release shippable. The final readiness report must remain PARTIAL or BLOCKED until native first-run, signing, physical dictation and installer-upgrade proof are complete.
