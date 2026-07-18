# MorpheOS Voice privacy notice

MorpheOS Voice supports local and remote transcription. The active choice changes where recorded audio is processed.

## Local — processed on this computer

Local mode is the default. MorpheOS Voice records into a unique owner-only temporary WAV, processes it with an installed speech model on the same computer and deletes the temporary file after the attempt is released by the application. The first model download needs a network connection.

Local mode does not intentionally upload microphone audio or transcripts.

## Remote — sent to the endpoint you selected

Remote mode is optional. When selected, MorpheOS Voice sends the recorded WAV, remote model name, optional language/task fields and bearer credential to the configured OpenAI-compatible endpoint. HTTPS is required unless insecure HTTP is explicitly enabled. The endpoint operator's privacy and retention terms apply.

If remote processing fails and a local model is available, the app may fall back to local processing.

## Data stored on the computer

- `config.json`: shortcut, model path, processing mode, endpoint and preferences.
- `history.json`: bounded text-only transcript history, default maximum 50 entries.
- `personalisation.json`: phrase replacements explicitly created by the user.
- `models/`: downloaded or imported speech models.
- `secrets/remote_api_key`: optional fallback token file. On Unix it is owner-only; the app does not currently use the OS keychain.

For compatibility with existing OS Whisper/OSWispa installations, the first MorpheOS Voice transition release continues to use the legacy application directories. No rebrand code copies or deletes that data.

Enabled personal-dictionary spellings are supplied as a bounded prompt only to the local model. Dictionary entries are not sent to the optional remote endpoint and are not learned by monitoring edits, keystrokes or foreground applications.

## Clipboard, insertion and other applications

Completed text is copied to the system clipboard. If automatic insertion is enabled, MorpheOS Voice asks the operating system to insert it into the focused application. The clipboard manager and target application may retain data under their own policies.

If insertion cannot be confirmed but clipboard copy succeeded, the app reports **Copied**. If clipboard copy fails, it does not paste stale clipboard content.

## Permissions

- Microphone permission is required to capture speech.
- macOS Accessibility permission is required for global hotkeys and text insertion.
- Linux global hotkeys currently require membership in the `input` group, which gives broad read access to input devices for that user session.
- Windows must allow microphone access for desktop applications.

## History, temporary audio and crashes

Audio is not kept as recovery history. Normal completion, cancellation and handled errors delete the temporary WAV through the application's temporary-file lifecycle. A process or operating-system crash may leave a temporary-file remnant; the current product does not provide crash recovery for in-progress audio.

The Linux runtime exposes recent text recovery. The current Tauri History screen is a development preview and cannot clear production files. A single verified cross-platform “clear all data” control is still required.

## Diagnostics and telemetry

The desktop application has no account, product analytics or telemetry. Diagnostics report states, sizes and redacted failures rather than transcript or remote response content. Operating-system service/console logs follow the retention settings of that system.

Do not send transcripts, audio, credentials, configuration files or complete logs in a support request.

## Website and contact

The project website uses local assets and has no analytics or contact-form processor. Contact links open GitHub or the user's email client. Do not submit API keys, private transcripts or recorded audio. A future `morpheos.net/voice` deployment must document any parent-site analytics before publication.

See [the detailed voice data flow](docs/privacy/VOICE_DATA_FLOW.md). Privacy questions can be sent to [hello@morpheos.net](mailto:hello@morpheos.net).
