# Third-party notices

The original MorpheOS Voice source code is MIT-licensed. Third-party dependencies, system libraries, tools and optional speech models retain their respective licences.

This inventory is based on the locked Cargo graph and tracked release inputs audited on 18 July 2026. `Cargo.lock` is the exact machine-readable dependency/version record for a build.

## Speech engine and models

| Component | How it is used | Licence/source |
|---|---|---|
| `whisper-rs` and `whisper-rs-sys` | Rust bindings and native build bridge | Unlicense; [upstream repository](https://github.com/tazz4843/whisper-rs) |
| Whisper.cpp | Local speech-recognition implementation compiled through the bindings | MIT; [upstream repository](https://github.com/ggerganov/whisper.cpp) |
| `ggerganov/whisper.cpp` model files | Optional GGML/GGUF models downloaded on first run or from Settings | Model repository declares MIT; [model repository](https://huggingface.co/ggerganov/whisper.cpp) |
| Custom imported models | User-supplied model files | Licence is determined by the model provider; MorpheOS Voice does not relicense them |

No speech model is committed to this repository. Managed models are downloaded separately from the URLs in `src/models/mod.rs` and stored on the user's computer.

## Direct Rust dependencies

| Dependency | Role | Declared licence |
|---|---|---|
| `anyhow` | Error handling | MIT OR Apache-2.0 |
| `arboard` | macOS/Windows clipboard | MIT OR Apache-2.0 |
| `chrono` | Transcript timestamps | MIT OR Apache-2.0 |
| `cpal` | macOS/Windows audio capture | Apache-2.0 |
| `crossbeam-channel` | Worker communication | MIT OR Apache-2.0 |
| `directories` | OS application directories | MIT OR Apache-2.0 |
| `enigo` | macOS/Windows text insertion | MIT |
| `evdev` | Linux global hotkey input | Apache-2.0 OR MIT |
| `futures-util` | Optional GUI networking support | MIT OR Apache-2.0 |
| `gtk4` Rust bindings | Linux Settings UI | MIT |
| `hound` | WAV reading/writing | Apache-2.0 |
| `indicatif` | First-run download progress | MIT |
| `ksni` | Linux StatusNotifier tray | Unlicense |
| `libc` | Platform system calls | MIT OR Apache-2.0 |
| `notify-rust` | Linux desktop notifications | MIT OR Apache-2.0 |
| `num_cpus` | Hardware probe | MIT OR Apache-2.0 |
| `rdev` | macOS/Windows global hotkeys | MIT |
| `regex` | Spoken punctuation/personalisation matching | MIT OR Apache-2.0 |
| `reqwest` | Model downloads and optional remote transcription | MIT OR Apache-2.0 |
| `serde`, `serde_json` | Configuration and protocol serialisation | MIT OR Apache-2.0 |
| `tempfile` | Private temporary recording/persistence files | MIT OR Apache-2.0 |
| `tokio` | Optional GTK/background support | MIT |
| `tracing`, `tracing-subscriber` | Content-redacted diagnostics | MIT |
| `wl-clipboard-rs` | Wayland clipboard integration | MIT OR Apache-2.0 |

## Desktop shell

| Dependency | Role | Declared licence |
|---|---|---|
| Tauri 2 / `tauri-build` | Native desktop shell and build support | Apache-2.0 OR MIT |
| `tauri-plugin-single-instance` | Prevents duplicate desktop-shell instances | Apache-2.0 OR MIT |

The Tauri updater, shell, HTTP, filesystem, process and opener plugins are not included. Updater artefacts are disabled.

## Platform libraries and commands

Release packages may dynamically use or depend on operating-system packages that are not copied into this repository, including:

- GTK, GLib, WebKitGTK and AppIndicator libraries on Linux;
- ALSA/`arecord`, `ydotool`, `wl-clipboard`, `xclip`, `xdotool` and netcat-compatible tools on Linux;
- macOS CoreAudio, Accessibility and clipboard frameworks;
- Windows WASAPI, clipboard and input APIs.

Those components are installed/provided by the operating system or package manager and retain their upstream licences. Review the package metadata for the target distribution when redistributing a complete image or appliance.

## Visual assets and fonts

- The MorpheOS Voice mark, favicon and interim app icons in this repository are project-authored for this rebrand.
- No font files are bundled. CSS requests IBM Plex where it is already available and falls back to platform fonts.
- No OpenAI, Whisper or third-party product logo is included.

## Complete transitive graph

Every package in the audited locked Cargo graph reported a licence expression. The graph includes MIT, Apache-2.0, BSD, MPL-2.0, Unicode, Zlib, Boost and other licences. Transitive versions and sources are recorded in `Cargo.lock`; use `cargo metadata --locked --format-version=1` to inspect the graph for a specific build.

This notice is a practical inventory, not a substitute for reviewing the licence files delivered by each dependency when preparing a new distribution.
