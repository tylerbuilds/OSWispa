# Open-source status

MorpheOS Voice is developed in the open. The original application source in this repository is available under the [MIT licence](../LICENSE).

MIT allows people to use, study, modify, redistribute and sell copies of the source, provided the copyright and licence notice are retained. Contributions accepted into this repository are licensed on the same basis.

Third-party libraries and optional speech models are not relicensed by MorpheOS Voice. See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and the locked dependency graph for their terms.

## Official builds and forks

Official source and downloads are linked from [morpheos.net/voice](https://morpheos.net/voice). During the transition, the official repository and release filenames still use the legacy OSWispa name so existing links and installations continue to work.

Forks are welcome under MIT. They should use their own name and visual identity and must not imply that they are an official MorpheOS release. See [TRADEMARK.md](../TRADEMARK.md).

## Reproducibility and release evidence

- `Cargo.lock` pins Rust dependency versions.
- GitHub Actions are pinned to commit SHAs.
- Release workflows build on clean hosted Linux, macOS and Windows runners.
- Published assets are covered by `SHA256SUMS`.
- Public package smoke checks redownload and launch release assets.

These checks do not make builds bit-for-bit reproducible and do not replace code signing. Current macOS and Windows packages are unsigned.

## Security and privacy

Please report vulnerabilities privately through [SECURITY.md](../SECURITY.md). Never include transcripts, recordings, credentials, complete logs or private paths in an issue.

The local speech path and optional remote endpoint have different data boundaries. See [PRIVACY.md](../PRIVACY.md) and [VOICE_DATA_FLOW.md](privacy/VOICE_DATA_FLOW.md).
