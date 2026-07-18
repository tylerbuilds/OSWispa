# OSWispa Support

OSWispa is a public alpha. Clear, minimal reports help improve it without exposing the speech, text, credentials, or system details it is designed to protect.

## Choose the right route

- Use the [runtime bug form](https://github.com/tylerbuilds/OSWispa/issues/new?template=runtime_bug.yml) for recording, transcription, hotkey, clipboard, or text-insertion failures.
- Use the [installation problem form](https://github.com/tylerbuilds/OSWispa/issues/new?template=installation_problem.yml) for package, first-launch, permission, model-setup, or upgrade failures.
- Use the [feature proposal form](https://github.com/tylerbuilds/OSWispa/issues/new?template=feature_proposal.yml) for a concrete improvement with a defined user outcome.
- Use [GitHub Discussions](https://github.com/tylerbuilds/OSWispa/discussions) for open-ended questions, troubleshooting, and early ideas.
- Follow [SECURITY.md](SECURITY.md) to report a vulnerability privately.

Search existing Issues and Discussions before opening a new report.

## Safe diagnostics

Include only what is needed to reproduce the problem:

- OSWispa release or commit;
- operating system, version, architecture, and Linux desktop session where relevant;
- release package or installation route;
- local CPU, CUDA, ROCm, Metal, or optional remote transcription mode, without remote endpoint details;
- audio backend and general microphone type, without serial numbers;
- configured hotkey and whether press and release were detected;
- the shortest reproduction steps, expected behaviour, and actual behaviour; and
- at most a few relevant error lines after manually reviewing and redacting them.

Never upload or paste:

- transcript text, spoken content, audio, or video recordings;
- clipboard contents or screenshots containing private text;
- API keys, tokens, passwords, secrets, or remote endpoint details;
- OSWispa configuration, history, key files, or model paths containing usernames;
- complete logs, environment dumps, `env` or `printenv` output;
- unreviewed crash reports, core dumps, or diagnostic archives; or
- hostnames, usernames, private filesystem paths, device serials, or other identifying data.

Do not attach `~/.config/oswispa`, OSWispa's application-data directory, or an entire terminal session. If a maintainer needs a more specific diagnostic, they will ask for the smallest safe check.

## What release automation proves

The v0.4.2 release gates verify package checksums, installation and launch on clean hosted macOS and Windows virtual machines. They also exercise the packaged version/platform report, native backend wiring, WAV contract, and clipboard round-trip.

Those virtual-machine checks do **not** prove:

- physical microphone capture or operating-system permission prompts;
- physical global-hotkey delivery;
- focus-sensitive insertion into real desktop applications;
- Bluetooth or USB device hot-plug behaviour; or
- CUDA, ROCm, or Metal acceleration on user hardware.

Reports from real hardware should name the general hardware and backend path, without including device serial numbers or private recordings. See the [launch guide](website/LAUNCH_GUIDE.md) for the current release proof boundary.
