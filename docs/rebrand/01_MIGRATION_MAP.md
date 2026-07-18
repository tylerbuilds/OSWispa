# MorpheOS Voice migration map

## Migration principle

The first MorpheOS Voice release changes the public product name without changing the installed identity that currently owns user state. This is an in-place compatibility migration: the new display name continues to read and write the proven OS Whisper/OSWispa locations and contracts.

That choice preserves settings, shortcuts, downloaded models, history, personal dictionary entries and optional remote credentials without copying or deleting them. It also leaves a direct rollback path to v0.4.2 because both versions use the same state.

No legacy user data will be deleted automatically.

## Surface map

| Surface | Legacy value | First MorpheOS Voice release | Later stable-identity option | Proof required before later change |
|---|---|---|---|---|
| Public name | OSWispa / OS Whisper | MorpheOS Voice | — | User-facing name tests |
| Company | OSWispa / TylerBuilds copy | MorpheOS | — | Brand review |
| Canonical product URL | GitHub Pages / legacy host | `https://morpheos.net/voice` in metadata and docs | Deploy at `/voice` | Production site approval and smoke test |
| GitHub repository | `tylerbuilds/OSWispa` | Retained | MorpheOS-owned `morpheos-voice` repository | Redirects, clone/pull, Issues, Actions, Pages and release-link proof |
| Cargo package | `oswispa` | Retained | `morpheos-voice` with package-manager replaces/provides rules | Debian/RPM clean install, upgrade, rollback and uninstall |
| CLI binary | `oswispa` | Retained and documented as compatibility command | Add `morpheos-voice`; retain `oswispa` alias for at least one stable cycle | Script/service/hotkey and package path tests |
| Windows binary | `oswispa.exe` | Retained | Add `morpheos-voice.exe` with legacy alias | Installer upgrade and app-path proof |
| Linux helper | `oswispa-toggle` | Retained | Add branded alias if useful | IPC/service tests |
| Linux desktop file/icon path | `oswispa.desktop`, `oswispa.svg` | Paths retained; visible Name/Comment rebranded | New file IDs with aliases | Desktop database, upgrade/uninstall proof |
| Linux service | `oswispa.service` | Retained; Description rebranded | New alias unit only after package transition | Enable/restart/upgrade/rollback proof |
| macOS launch agent | `com.oswispa.agent` | Retained; display/help copy rebranded | New label after signed app transition | launchd upgrade/unload/rollback proof |
| Bundle ID | `com.tylerbuilds.oswispa` | Retained | MorpheOS signing identity, likely `net.morpheos.voice` | Signing, notarisation, permissions and upgrade continuity |
| Tauri product name | OSWispa | MorpheOS Voice | — | Build/window/title tests |
| Unix socket | `oswispa.sock` | Retained | Dual-listen/alias if ever renamed | Running upgrade and helper compatibility |
| Environment prefix | `OSWISPA_` | Retained | Add `MORPHEOS_VOICE_` aliases with precedence rules | Existing shell/service config tests |
| Smoke marker | `OSWISPA_PLATFORM_SMOKE_OK` | Retained | Add versioned neutral marker alongside it | Current release/public-smoke compatibility |
| Release asset filenames | `oswispa-*` | Retained for the transition release | Publish branded names plus legacy aliases | Public-download checksums and old-link proof |
| App bundle display name | OSWispa | MorpheOS Voice in new source builds | — | Packaged app install/launch proof |
| App bundle executable | OSWispa | Retained internally | Rename only with launcher fallback | Installed app/automation proof |

## State map

| State | Current location/contract | First-release action | Result for existing users |
|---|---|---|---|
| General settings | Legacy ProjectDirs config location, `config.json` | Read and write in place | Settings and backend choice survive |
| Keyboard shortcut | `Config.hotkey` in legacy `config.json` | Read and write in place | Shortcut survives |
| Model selection | Absolute/relative path in `Config.model_path` and `fallback_model_path` | Preserve exact path | Existing model remains selected |
| Downloaded models | Legacy data directory `models/` | Continue using directory | No re-download required |
| Transcript history | Legacy data directory `history.json` | Continue using file | Existing bounded history survives |
| Personal dictionary | Legacy data directory `personalisation.json` | Continue using file | Entries survive and remain local |
| Remote provider settings | `Config.remote_backend` | Continue using configuration | Endpoint/model/timeout survive |
| Optional remote token | Legacy config `secrets/remote_api_key` or configured environment variable | Continue using source; never print/copy token in migration logs | Credential continues to work; no secret migration |
| Snippets/templates | Not implemented | No action | No state to migrate |
| Database | Not implemented | No action | No state to migrate |
| In-progress audio | Owner-only temporary WAV | Never migrated or retained | Crash recovery remains unavailable |

## In-place migration sequence

1. Start the rebranded application using the retained installed identity.
2. Resolve the same legacy ProjectDirs state locations used by v0.4.2.
3. Load and validate existing configuration, including shortcut and model paths.
4. Load history and personal dictionary from the same data directory.
5. Continue atomic/private writes to the same files.
6. Leave filenames, paths and state untouched for rollback.
7. Never repeat a copy step because no copy is performed in this release.

This is intentionally simpler and safer than creating two partially synchronised state trees.

## Future copy migration, if stable identity changes

If a later signed release adopts new MorpheOS-owned paths, implement and prove this exact order:

1. If new state exists, use it and do not overwrite it.
2. If only legacy state exists, copy each recognised file to a staging directory under the new location.
3. Reject symlinks and preserve owner-only permissions.
4. Validate every copied JSON document and referenced model file.
5. Atomically promote the staging directory or individual files.
6. Write a versioned migration receipt containing paths and checksums only—never transcripts, dictionary phrases, endpoints or tokens.
7. Reopen and verify the new state before recording migration success.
8. Preserve the legacy directory unchanged for rollback.
9. Do not rerun after a valid receipt/new state is present.
10. Offer legacy-data removal only as a separate, explicit user action after a successful stable release cycle.

## Test map

| Required behaviour | Planned evidence in this rebrand |
|---|---|
| New public name appears | Static UI/website/native-shell contract tests plus Rust display-copy tests |
| Legacy configuration is detected | Rust tests keep the legacy ProjectDirs tuple and load a representative existing config |
| Existing settings survive | Config round-trip test with non-default values |
| Existing shortcuts survive | Config round-trip assertion for all modifiers and trigger key |
| Existing downloaded models remain available | Test preserves an existing model path instead of rewriting it |
| No duplicate migration | In-place strategy: no copy/migration routine exists; tests assert one canonical path |
| No user data is deleted | Persistence tests and source inspection; no rebrand deletion code is added |
| History/dictionary survive | Existing persistence/personalisation tests plus compatibility-path assertions |
| Credentials remain private | No token content in logs/tests; retained private file/environment contract |
| Rollback remains possible | v0.4.2 and rebranded source resolve the same state identity |

## External migration not performed here

- GitHub repository rename or transfer.
- GitHub repository description/homepage update.
- DNS, Cloudflare, MorpheOS production repository or `/voice` deployment.
- Signing identity, notarisation or Windows publisher creation.
- Package-registry publication.
- Release tag or GitHub Release publication.

These are externally visible operations and require a final approved execution step after local and platform proof.
