# MorpheOS Voice roadmap

MorpheOS Voice is an open voice-typing layer for the sentence, not the meeting.

**Talk instead of type — in any app.**

This roadmap separates what people can download today from the proof still required before the next product layer ships.

## Current public alpha — v0.5.0

The [v0.5.0 release](https://github.com/tylerbuilds/OSWispa/releases/tag/v0.5.0) is the current public alpha under the MorpheOS Voice name. It retains the legacy OSWispa repository, command, package and state identifiers for upgrade and rollback safety.

- Linux ships as a Debian package, RPM, and portable tarball, with a desktop entry, configurable push-to-talk, local Whisper, and clipboard/text delivery.
- macOS ships Intel and Apple Silicon DMG/ZIP packages with CoreAudio capture, global hotkeys, clipboard insertion, and optional Metal on Apple Silicon. The current package launches through Terminal and is not signed or notarised.
- Windows ships an x86-64 portable ZIP with native WASAPI capture, a Ctrl+Windows global hotkey, clipboard verification, and text insertion. The package passed installed-app hosted-VM proof but remains unsigned and has no native tray or installer.
- Local transcription remains the default, and the remote backend is explicit and opt-in.

## v0.5.0 product foundation

The current public alpha includes the following work:

- Truthful lifecycle states from capture acknowledgement through delivery outcome.
- A private deterministic personal dictionary and Linux editor; dictionary contents stay out of the optional remote backend.
- A reusable, transcript-redacted engine boundary for an embedding desktop host.
- An original local-only UI contract for Ready Check, Settings, the compact Signal, and bounded recovery history. Its development adapter does not claim native microphone, hotkey, model, or insertion behaviour.
- Per-session Windows and macOS capture hardening, including streaming macOS anti-alias conversion.
- The MorpheOS Voice public name, MorpheOS-family visual direction, users-first documentation and compatibility-safe product/site migration.

Track the merged detail in [Unreleased](CHANGELOG.md#unreleased).

## Next productisation gates

These gates are ordered. Passing a build or VM smoke test is not a substitute for the native and physical proof below.

1. **Native shell and onboarding** — host the existing engine behind real Ready Check, Settings, Signal, tray/menu, first-run model and permission flows while preserving the current `oswispa` CLI as a compatibility path.
2. **Signing and notarisation** — establish stable application identity, sign Windows and macOS artefacts, notarise macOS packages, and verify permission persistence across upgrades.
3. **Physical workflow proof** — on supported hardware, exercise microphone permission, rapid press/release hotkeys, cancellation/restart, focused-app insertion, copied-only fallback, and recovery from device or worker failure.
4. **Installer parity** — retain verified Linux formats, replace the Windows ZIP-only path with a signed installer plus portable option, and prove clean install, upgrade, uninstall, and rollback on each platform.
5. **Updater** — add update checks and signed update delivery only after stable identity, signed artefacts, installer parity, and rollback proof exist.

## Ongoing quality bar

- A GitHub Release, its checksummed assets, and its changelog are the release source of truth.
- Website download links and platform copy must match that release, not `master`.
- Linux, macOS, Windows, CodeQL, package, and public-download checks must pass at the protected release commit.
- Local-first and privacy-first behaviour remains the default; remote transcription stays explicit and opt-in.
- Transcript text, microphone audio, clipboard contents, dictionary entries, credentials, and raw remote bodies stay out of routine logs and lifecycle events.
- Hardware-dependent capabilities are described as proven only when a signed release candidate has a recorded physical test receipt.
