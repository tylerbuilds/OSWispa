# MorpheOS Voice user guide

MorpheOS Voice is for short dictation: hold a shortcut, speak, release and put the completed text into the application you are already using. It is not a meeting recorder or live-captioning service.

## First dictation

1. Install the package for your operating system.
2. Complete the first-run device check and local model download.
3. Grant the requested platform permissions.
4. Focus a normal text field.
5. Hold the shortcut, speak a short phrase and release the keys.
6. Wait for the inserted or copied status.

Default shortcuts:

- Linux: **Ctrl + Super**
- macOS: **Ctrl + Cmd**
- Windows: **Ctrl + Windows**

The transcript is copied to the clipboard before supported auto-insertion. If insertion is blocked by the focused application or operating-system permissions, paste the clipboard contents manually.

## Understand the states

- **Ready** — waiting for the shortcut.
- **Arming** — starting the microphone capture path.
- **Listening** — the audio backend has confirmed capture.
- **Processing** — recording has stopped and transcription is running.
- **Delivering** — the completed text is being copied and inserted.
- **Inserted** — clipboard delivery and automatic insertion succeeded.
- **Copied** — text is on the clipboard but automatic insertion did not complete.
- **Failed / Needs attention** — the current step did not complete; follow the displayed recovery action.

Press **Escape** while recording to cancel. A very quick shortcut tap is also treated as a cancellation rather than useful speech.

## Local and remote processing

### Local — processed on this computer

Local mode is the default. MorpheOS Voice records to an owner-only temporary WAV, transcribes it with the selected local Whisper.cpp model, then deletes the WAV on success, cancellation and handled failure paths. The first model download needs a network connection; local dictation can work offline after that.

### Remote — sent to the selected provider for processing

Remote mode is optional. It sends the WAV and configured request fields to the OpenAI-compatible endpoint you choose. That provider can receive and retain the request under its own terms. Personal dictionary entries are not sent as a remote vocabulary prompt.

See [Privacy](../PRIVACY.md) and the [voice data-flow map](../docs/privacy/VOICE_DATA_FLOW.md).

## First-launch model choice

The first-run check looks at:

- whether the current build can use Metal, CUDA, HIPBLAS or CPU only;
- available memory; and
- a short local CPU sample.

It then favours a model that should remain responsive rather than choosing the largest model automatically. Start with that choice. Move up for accuracy or down for responsiveness only after trying real dictation.

Typical choices:

- `base.en` — fast English replies and prompts on modest hardware;
- `small.en` — stronger English accuracy with moderate memory use;
- `medium.en` — a heavier English option for capable computers;
- `distil-large-v3` — high-end English speed/accuracy balance; and
- `large-v3` — multilingual and accuracy-first, with the highest local resource cost in the curated list.

Model size does not guarantee a specific speed or accuracy. Audio quality, accent, language, hardware and backend all matter. See the [model guide](models.html).

## Settings and personal vocabulary

The current tray and graphical settings are Linux-only. Open the MorpheOS Voice tray menu to:

- change the shortcut;
- choose a Linux PipeWire/PulseAudio microphone source or follow the system default;
- choose or import a model;
- enable or disable audio feedback and auto-insertion;
- manage local personal-vocabulary entries; and
- configure the optional remote backend.

The macOS and Windows alpha packages do not yet have a complete native settings interface. They use the legacy configuration file in each operating system's application configuration directory.

Use the personal vocabulary for names and phrases that transcription repeatedly spells incorrectly. Linux users can add, edit, enable, disable, delete, import and export entries through **Settings → Dictionary**. Changes apply immediately.

MorpheOS Voice applies enabled replacements literally before spoken punctuation. Longer phrases win when entries overlap, replacements do not cascade and matching does not replace text inside a larger word.

macOS and Windows users can edit `personalisation.json` in the established application-data directory, then restart the app:

```json
{
  "schema_version": 1,
  "dictionary": [
    {
      "spoken": "morph e os voice",
      "written": "MorpheOS Voice",
      "enabled": true,
      "case_sensitive": false
    }
  ]
}
```

The dictionary stays local, does not monitor other applications and is not sent to the optional remote backend. If the document is invalid, MorpheOS Voice preserves it, disables the dictionary for that run and continues normal dictation.

## Platform notes

### Linux

The global shortcut reads Linux input devices directly. If the installer added you to the `input` group, log out and back in. Wayland insertion normally uses the `ydotoold` user service.

```bash
systemctl --user status oswispa
systemctl --user status ydotoold
```

If transcription returns `[BLANK_AUDIO]` or reports no speech, confirm the PipeWire/PulseAudio source:

```bash
pactl get-default-source
pactl list short sources
```

Set the working source system-wide with `pactl set-default-source SOURCE_NAME`, or choose a source in **Settings → General → Linux microphone source**.

### macOS

The current package is unsigned and unnotarised. It launches through Terminal, which must stay open, and needs:

1. **Microphone** access for audio capture.
2. **Accessibility** access for the global shortcut and text insertion.

If the existing published app is still named OSWispa, grant permission to that compatibility bundle rather than creating a second data location.

### Windows

Extract the complete ZIP, run `oswispa.exe` and keep the console window open. Enable microphone access for desktop applications under **Settings → Privacy & security → Microphone**. The unsigned alpha may require a reviewed **More info → Run anyway** SmartScreen override.

## Recovery and limits

- Completed text is copied to the clipboard and may also be present in bounded local history.
- A failed insertion should leave the completed text available to paste manually.
- Audio being captured at the moment of a process or operating-system crash is not recoverable.
- The app does not automatically learn from corrections.
- The app does not execute actions or control the computer beyond inserting the text it produced.

If a failure repeats, follow [Support](../SUPPORT.md) and remove private content before sharing diagnostics.
