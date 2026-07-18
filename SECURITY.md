# MorpheOS Voice security policy

## Supported version

Security fixes target the current `master` branch and latest GitHub release. Move to the newest tagged release when one is available.

## Report a vulnerability privately

Use [GitHub private vulnerability reporting](https://github.com/tylerbuilds/OSWispa/security/advisories/new) or email [hello@morpheos.net](mailto:hello@morpheos.net). Include:

- affected version or commit;
- platform and installation method;
- minimal reproduction steps and impact; and
- any suggested mitigation.

Do not include credentials, private transcripts, recorded audio, clipboard contents or complete environment/configuration dumps. Avoid a public issue until the report has been assessed.

## Security boundaries

MorpheOS Voice treats microphone audio, transcripts, clipboard history, configuration, personal vocabulary and remote API credentials as sensitive data.

Local processing keeps transcription on the computer running the app after the model download. Remote processing is an explicit trust decision: audio is sent to the selected endpoint and becomes subject to that provider's security and retention policy.

The project does not claim that unsigned packages, low-level Linux input access or third-party model downloads are risk-free. Current accepted risks and audit limits are tracked in the [July 2026 audit](docs/AUDIT-2026-07-18.md), [Privacy](PRIVACY.md) and [release-readiness report](docs/rebrand/05_RELEASE_READINESS.md).
