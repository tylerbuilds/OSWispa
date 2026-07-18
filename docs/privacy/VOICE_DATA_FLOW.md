# MorpheOS Voice data flow

## Summary

MorpheOS Voice has two transcription paths. **Local** processes a temporary recording on the same computer. **Remote** sends the recording to the endpoint selected by the user. The active mode must be visible wherever the user chooses or starts processing.

## Core flow

1. The user presses the configured global shortcut.
2. The runtime enters **Arming** while it asks the audio backend to start.
3. Only after the backend confirms capture does the app enter **Listening**.
4. Releasing the shortcut stops capture. Escape/cancel discards the attempt.
5. The recorder finalises a unique owner-only temporary WAV.
6. Local or remote transcription produces text.
7. Explicit personal-dictionary replacements and optional spoken punctuation run locally.
8. The app verifies that the text reached the clipboard.
9. If auto-insert is enabled, it attempts insertion into the focused application.
10. The app reports **Inserted**, **Copied** or **Needs attention** and stores bounded text history.
11. The temporary WAV is deleted when its owning temporary path is dropped, including handled error paths.

## Data by stage

| Stage | Data | Location/recipient | Retention |
|---|---|---|---|
| Capture | Raw microphone samples | Process memory and an owner-only temporary WAV | For the active attempt; deleted after processing/drop |
| Local transcription | WAV plus local model | Same computer | WAV temporary; model retained until user removes it |
| Remote transcription | WAV bytes, selected remote model, optional language/task and bearer token | User-configured endpoint | MorpheOS Voice does not control provider retention |
| Personalisation | Explicit phrase pairs and bounded local model prompt | `personalisation.json`; local Whisper context | Until user edits/removes the file; not sent to remote endpoint |
| Clipboard delivery | Completed transcript | System clipboard | Controlled by the operating system/clipboard manager |
| Text insertion | Completed transcript and simulated paste/input event | Focused application | Controlled by that application |
| Recovery history | Completed transcript and timestamp | `history.json` in legacy compatibility data directory | Bounded by `max_history`, default 50 |
| Configuration | Shortcut, model path, mode, endpoint and preferences | `config.json` | Until edited/removed |
| Optional token | Remote bearer token | Configured environment variable or private `secrets/remote_api_key` file | Until environment/file is cleared |
| Diagnostics | State, lengths, status and redacted errors | stderr/journal/console depending on launch method | Controlled by OS/service log policy; transcript content is not intentionally logged |

## Local mode

Label: **Local — Processed on this computer.**

The app loads a model from local storage and runs Whisper.cpp on the same computer. Local mode does not upload the recording or transcript. The initial model download uses the network and obtains the selected model from the configured Hugging Face source.

## Remote mode

Label: **Remote — Sent to the endpoint you selected for processing.**

Remote mode is opt-in. It posts a multipart request to the configured OpenAI-compatible endpoint. HTTPS is required unless the user explicitly enables insecure HTTP. A remote failure may fall back to an installed local model.

The endpoint operator can receive the audio, selected model name, optional language/task fields, network metadata and bearer credential. Its security, storage and retention policy applies.

## Keys and secrets

The app does not use a platform keychain. It reads a token from the environment-variable name configured by the user or from a private fallback file. On Unix, application directories are owner-only and the secret file is written with owner-only permissions. Secrets must never be included in issues or logs.

## History and deletion

- Completed transcripts are stored by default in bounded local history.
- Audio is not stored in recovery history.
- The Linux runtime/tray exposes recent transcript recovery. The current Tauri History page contains synthetic preview rows and cannot clear production files.
- There is not yet one verified cross-platform control that clears all history, models and cached state.
- Temporary audio should disappear after normal completion, cancellation and handled errors. A process/OS crash may leave operating-system temporary-file remnants; there is no user-facing crash-recovery or stale-audio cleanup receipt yet.

## Telemetry and website

The desktop application has no product analytics or telemetry. Lifecycle events passed to the Tauri Signal contain only a bounded state name and no transcript.

The repository website uses local assets and has no product analytics or contact-form processor. Contact routes open GitHub or the user's email client. The future MorpheOS `/voice` host may have parent-site analytics only after its data handling is documented and approved.

## Failure and recovery boundaries

- If clipboard verification fails, auto-insert is skipped to avoid pasting stale clipboard content.
- If insertion fails after a successful copy, the result is **Copied** and remains available on the clipboard/history.
- If both copy and insertion fail, recovery is not guaranteed.
- An in-progress recording or transcription is not recoverable after a crash.
- “Inserted” reports that the insertion API returned without error; physical focused-app verification is still a release test.
