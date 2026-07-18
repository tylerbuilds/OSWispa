# MorpheOS Voice support

MorpheOS Voice is a public alpha. A small, carefully redacted report is more useful than a full log containing private speech or system details.

## Choose the right route

- Use the [runtime bug form](https://github.com/tylerbuilds/OSWispa/issues/new?template=runtime_bug.yml) for recording, transcription, shortcut, clipboard or insertion failures.
- Use the [installation problem form](https://github.com/tylerbuilds/OSWispa/issues/new?template=installation_problem.yml) for package, first-launch, permission, model, upgrade or uninstall failures.
- Use the [feature proposal form](https://github.com/tylerbuilds/OSWispa/issues/new?template=feature_proposal.yml) for a bounded improvement with an observable user outcome.
- Use [GitHub Discussions](https://github.com/tylerbuilds/OSWispa/discussions) for questions, troubleshooting and early ideas.
- Follow [Security](SECURITY.md) for a private vulnerability report.

Search existing Issues and Discussions before opening a new report.

## Safe diagnostics

Include only what is needed:

- MorpheOS Voice release or commit;
- operating system, version, architecture and Linux desktop session where relevant;
- release package or installation route;
- local CPU, CUDA, ROCm, Metal or optional remote mode, without endpoint details;
- audio backend and general microphone type, without serial numbers;
- configured shortcut and whether press/release was detected;
- shortest reproduction steps, expected behaviour and actual behaviour; and
- a few manually reviewed, redacted error lines if essential.

Never upload or paste:

- transcript text, spoken content, audio or video recordings;
- clipboard contents or screenshots containing private text;
- API keys, tokens, passwords, remote endpoints or other secrets;
- configuration, history, personal-vocabulary or key files;
- complete logs, environment dumps, crash reports, core dumps or diagnostic archives; or
- hostnames, usernames, private paths, device serials or identifying data.

Do not attach `~/.config/oswispa`, the legacy application-data directory or an entire terminal session. Those paths remain active compatibility locations for MorpheOS Voice.

## What release automation proves

The v0.4.2 gates verify checksums, package installation and launch on clean hosted macOS and Windows virtual machines. They also exercise the packaged version/platform report, native backend wiring, WAV contract and clipboard round-trip.

They do not prove physical microphone capture, permission prompts, physical global shortcuts, focus-sensitive insertion, device hot-plug or GPU acceleration. Real-hardware reports should name only the general hardware and backend path.
