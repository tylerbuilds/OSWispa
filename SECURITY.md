# Security Policy

## Supported version

Security fixes target the current `master` branch and the latest GitHub release. Users should move to the newest tagged release when one is available.

## Reporting a vulnerability

Please use [GitHub private vulnerability reporting](https://github.com/tylerbuilds/OSWispa/security/advisories/new) or email [tc@tylerbuilds.com](mailto:tc@tylerbuilds.com). Include:

- the affected version or commit;
- platform and installation method;
- reproduction steps and impact;
- any suggested mitigation.

Do not include credentials, private transcripts, or recorded audio. Please avoid opening a public issue until the report has been assessed.

## Security boundaries

OSWispa treats microphone audio, transcripts, clipboard history, configuration, and remote API credentials as sensitive local data. Remote transcription is an explicit trust decision: audio leaves the device and is governed by the configured service's security and retention policy.

Known accepted risks and audit limitations are tracked in [the July 2026 audit](docs/AUDIT-2026-07-18.md).
