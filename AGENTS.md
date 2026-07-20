# MorpheOS Voice agent rules

## GitHub Actions cost policy

- Ordinary pull requests run the Linux CI and CodeQL gates only.
- macOS and Windows CI run after a change reaches `master`, or when a human explicitly dispatches the CI workflow for platform-specific proof.
- Release tags and explicitly requested public-release smoke tests retain full Linux, macOS, and Windows coverage.
- Do not add macOS, Windows, GPU, or larger hosted runners to routine pull-request workflows without Tyler's explicit approval.
- Keep workflow concurrency cancellation enabled for iterative CI so superseded runs stop promptly.
- Batch compatible Dependabot updates. Do not trigger repeated release or public-smoke workflows while a previous run is still useful evidence.
- Prefer the cheapest runner that proves the requested behaviour. Escalate runner cost for platform-specific evidence, not as a default quality signal.

Before changing workflow triggers or runner selection, state the expected run frequency and paid-runner impact in the pull request or commit message.
