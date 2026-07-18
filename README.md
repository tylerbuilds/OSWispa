# MorpheOS Voice

<p align="center">
  <img src="website/morpheos-voice-lockup.svg" alt="MorpheOS Voice" width="520">
</p>

<p align="center"><strong>Talk instead of type — in any app.</strong></p>

<p align="center">Free, open-source voice typing for your computer.</p>

<p align="center">
  <a href="LICENSE"><img alt="MIT licence" src="https://img.shields.io/badge/source-MIT-b9f27c"></a>
  <a href="https://github.com/tylerbuilds/OSWispa/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/tylerbuilds/OSWispa?label=release"></a>
  <img alt="Project status: alpha" src="https://img.shields.io/badge/status-alpha-d3a94f">
</p>

MorpheOS Voice turns a short spoken thought into text in the application you are already using. Linux is the primary alpha platform; macOS and Windows packages are also available with the limitations set out below.

[Download MorpheOS Voice](https://github.com/tylerbuilds/OSWispa/releases/latest) · [Product page](https://morpheos.net/voice) · [Privacy](PRIVACY.md) · [Support](SUPPORT.md)

> **Transition note:** the public product is now MorpheOS Voice. The current repository URL, command name (`oswispa`), package filenames and application-data locations retain the legacy OS Whisper/OSWispa identifiers for upgrade and rollback safety.

## What it does

```text
Hold the shortcut → speak → release → transcribe → copy → insert
```

Focus an email, document, message, prompt or text field. Hold the configured shortcut while you speak, then release it. MorpheOS Voice processes the recording, copies the completed text to the clipboard and, when supported, inserts it at the cursor.

This is push-to-talk dictation, not a meeting recorder, chatbot or desktop agent. Write mode produces text only; there is no Act mode in the current product.

## Why MorpheOS Voice

| Capability | Current behaviour |
| --- | --- |
| Free and open source | The original source is MIT-licensed; dependencies and optional models keep their own licences. |
| Works where you write | Completed text is inserted into the focused application when the platform insertion path succeeds. |
| Local processing | Local Whisper.cpp processing is the default after a model has been downloaded. |
| Provider choice | An optional OpenAI-compatible remote endpoint can be configured explicitly. Audio then leaves the computer. |
| Recoverable text | Completed transcripts are copied to the clipboard and kept in bounded local history. Failed insertion does not discard the clipboard copy. |
| Your shortcut | The default is Ctrl+Super on Linux, Ctrl+Cmd on macOS and Ctrl+Windows on Windows. |
| Personal vocabulary | A deterministic local dictionary can correct names and phrases without automatic learning or app monitoring. |

## Quick start on Linux

The quickest tested route is a release package. The filenames remain legacy compatibility identifiers for this transition release.

Ubuntu or Debian:

```bash
curl -LO https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa_amd64.deb
sudo apt install ./oswispa_amd64.deb
oswispa
```

Fedora or RHEL-compatible systems:

```bash
curl -LO https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa_x86_64.rpm
sudo dnf install ./oswispa_x86_64.rpm
oswispa
```

On first launch, MorpheOS Voice checks the computer and downloads a suitable speech model. If the installer adds your account to the Linux `input` group, log out and back in before using the global shortcut.

Then:

1. Focus a normal text field.
2. Hold **Ctrl+Super** and speak a short phrase.
3. Release the keys and wait for the delivery status.
4. If insertion is blocked, paste the clipboard contents manually.

## Installation

All current downloads are alpha software. Release packages are checksummed in `SHA256SUMS`.

| Platform | Package | Current proof and limitations |
| --- | --- | --- |
| Linux x86-64 | `.deb`, `.rpm`, tarball or source | Primary platform. Release packages are CPU-only. Global shortcuts require input-device access; Wayland insertion normally uses the `ydotoold` user service. |
| macOS 12+ | Apple Silicon or Intel DMG/ZIP | Package install and launch are VM-tested. The app is unsigned and unnotarised, launches through Terminal and has no menu-bar UI. Microphone, Accessibility and physical insertion still need hardware permission proof for each release candidate. |
| Windows x86-64 | Portable ZIP | Package extraction and launch are VM-tested. The app is unsigned, has no installer or tray and must keep its console window open. Physical microphone, hotkey and focused-app insertion need hardware proof for each release candidate. |

### macOS

1. Download the matching DMG from [GitHub Releases](https://github.com/tylerbuilds/OSWispa/releases/latest).
2. Drag **MorpheOS Voice.app** to Applications. Older published DMGs may still show **OSWispa.app**.
3. Control-click the app and choose **Open** if Gatekeeper blocks the unsigned build.
4. Grant Microphone and Accessibility access when prompted.
5. Keep the Terminal window open; hold **Ctrl+Cmd** to dictate.

### Windows

```powershell
Invoke-WebRequest https://github.com/tylerbuilds/OSWispa/releases/latest/download/oswispa-windows-x86_64.zip -OutFile oswispa-windows-x86_64.zip
Expand-Archive .\oswispa-windows-x86_64.zip -DestinationPath .\MorpheOS-Voice
Set-Location .\MorpheOS-Voice
.\oswispa.exe
```

Keep the console open. If SmartScreen appears, inspect the publisher warning and checksum before choosing **More info → Run anyway**. Hold **Ctrl+Windows** to dictate.

### Build from source

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
./install.sh
```

The source installer supports Ubuntu/Debian, Fedora/RHEL, Arch/Manjaro and macOS. It detects supported CUDA, ROCm or Metal build paths and otherwise builds for the CPU. See [Building](docs/BUILDING.md) for manual and GPU-specific commands.

## How processing works

### Local — processed on this computer

Local mode records to an owner-only temporary WAV, runs the selected Whisper.cpp model on the same computer, then deletes the temporary audio on success, cancellation and handled failure paths. The first model download requires a network connection; later local dictation can work offline.

### Remote — sent to the selected provider for processing

Remote mode sends the WAV plus the configured model/language/task fields to the OpenAI-compatible endpoint you choose. That provider can receive and retain the request under its own policy. Remote mode is opt-in and requires explicit configuration.

No product analytics or transcription telemetry is built into the desktop application. Completed transcript text is stored locally in bounded history unless history is disabled.

Read [Privacy](PRIVACY.md) and the [voice data-flow map](docs/privacy/VOICE_DATA_FLOW.md) before configuring a remote provider.

## Configuration

The basic experience is deliberately small: install, complete first-run model setup, grant platform permissions, then hold a shortcut and speak.

Linux exposes settings and personal vocabulary through its tray menu. The macOS and Windows alpha packages currently use the legacy JSON configuration file rather than a complete native settings interface.

Advanced controls include:

- local model selection and custom model import;
- local or remote processing;
- shortcut modifiers and an optional trigger key;
- language, spoken punctuation and formatting;
- history limits and auto-paste;
- Linux microphone-source override; and
- local personal vocabulary.

The first transition release continues to use the established OSWispa data directories so existing settings, shortcuts, models, history, dictionary and stored token remain available. See the [migration map](docs/rebrand/01_MIGRATION_MAP.md).

## Troubleshooting

### The shortcut does nothing on Linux

If installation added you to the `input` group, log out and back in. Confirm the helper and app services:

```bash
systemctl --user status ydotoold
systemctl --user status oswispa
```

### Recording uses the wrong Linux microphone

```bash
pactl list short sources
pactl get-default-source
```

Choose the correct source system-wide with `pactl set-default-source SOURCE_NAME`, or set the Linux microphone override in Settings.

### Text was copied but not inserted

Paste manually with Ctrl+V or Cmd+V. Insertion depends on the focused application, desktop session and platform permissions; clipboard delivery is the recovery path.

### The first launch cannot find a model

Reconnect for the initial model download, or import a valid Whisper.cpp `.bin` or `.gguf` model. Downloads are validated before installation and incomplete files are not accepted as models.

### macOS or Windows blocks the app

Current packages are not signed. Verify the release checksum, then use the documented Gatekeeper or SmartScreen override. Signed installers are a release blocker, not a hidden claim.

More help is in the [user guide](website/USER_GUIDE.md) and [Support](SUPPORT.md).

## Known limitations

- This is alpha software; do not rely on it as the only copy of important text.
- The current native desktop shell is still a development host. Its Settings and History screens are not yet a complete replacement for the proven runtime and Linux tray controls.
- macOS and Windows packages are unsigned. macOS uses a Terminal launcher; Windows uses a console and portable ZIP.
- Linux global hotkeys require low-level input access, and Wayland insertion normally requires `ydotoold`.
- Physical microphone, hotkey, permission, focus-sensitive insertion and GPU behaviour cannot be proven by hosted VM package tests.
- Audio captured immediately before an operating-system or process crash is not recoverable. Completed transcripts are the recoverable unit.
- MorpheOS Voice is not affiliated with OpenAI. Whisper.cpp and optional model files are third-party components.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --no-default-features
```

See [Building](docs/BUILDING.md), [Contributing](CONTRIBUTING.md), [Releasing](docs/RELEASING.md) and the [July 2026 audit](docs/AUDIT-2026-07-18.md).

## Licence and official project

The original MorpheOS Voice source code is licensed under the [MIT License](LICENSE). Third-party dependencies and optional speech models retain their respective licences; see [Third-party notices](THIRD_PARTY_NOTICES.md).

“MorpheOS” and “MorpheOS Voice” identify the official project. MIT permits forks, but unofficial builds must not impersonate official releases. See [Trademark](TRADEMARK.md).

Official product information lives at [morpheos.net/voice](https://morpheos.net/voice). During this compatibility transition, the official source and downloads remain at [github.com/tylerbuilds/OSWispa](https://github.com/tylerbuilds/OSWispa).
