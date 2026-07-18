# OSWispa Tauri shell foundation

This package is an additive Tauri 2 shell around the existing `oswispa` library. It is excluded
from the workspace's default members, so existing CLI builds, Linux packages and release jobs keep
their current behaviour until native parity is proved.

## Implemented boundary

- One process owns one embedded engine, guarded by Tauri's official single-instance plugin.
- The native tray owns Open OSWispa, History, Show Signal, Start, Stop, Cancel and Quit.
- The engine runs on its own worker and exposes only `{ "state": "..." }` lifecycle payloads.
- Signal is non-focusable and transcript-free. Settings and History are local development previews;
  they do not read or mutate configuration, history, files or the clipboard.
- The only webview permission is lifecycle listen/unlisten for the Signal window. No shell, HTTP,
  filesystem, process, opener or updater plugin is present, and all frontend assets are local.
- macOS metadata contains a microphone usage description and the audio-input entitlement. No
  signing identity, updater key or credential is stored here.

## Deliberate limitations

This is not yet a release package. The existing release workflows remain authoritative. A native
run still needs signed macOS TCC proof, physical microphone and shortcut proof on macOS/Windows,
and installer smoke tests. Embedded mode also refuses to open the terminal model wizard; the next
onboarding slice must provision or select a valid local model and restart the engine.

The Tauri host can be compiled explicitly with:

```sh
cargo check --locked -p oswispa-desktop
```

Linux compilation additionally requires WebKitGTK 4.1, Ayatana AppIndicator and librsvg development
packages. Contract-only tests do not need a WebView runtime:

```sh
cargo test --locked -p oswispa-desktop --no-default-features
```
