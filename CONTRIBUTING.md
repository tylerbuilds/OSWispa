# Contributing to MorpheOS Voice

MorpheOS Voice is a focused voice-typing tool. Contributions should make the shortcut → speech → text → delivery loop clearer, safer or more dependable without turning the project into a meeting recorder, chatbot or desktop agent.

## Before opening work

- Search existing Issues and Discussions.
- Use Discussions for open-ended ideas and questions.
- Open an issue before a large platform, packaging, privacy or architecture change.
- Keep changes small enough to review and prove.
- Do not copy another product's code, assets, wording or distinctive interface.

## Reporting problems

Read [Support](SUPPORT.md), then choose the runtime-bug or installation-problem form. Include the MorpheOS Voice release or commit, operating system, package, relevant processing path and the shortest safe reproduction steps.

Never attach transcript text, recordings, clipboard contents, API keys, configuration files, environment dumps, credentials or unreviewed crash/core dumps.

## Pull requests

1. Explain the user problem and the intended result.
2. Keep compatibility-sensitive identifiers unchanged unless the migration and rollback path is part of the change.
3. Add or update tests for the behaviour and important failure paths.
4. Run the relevant checks from [Building](docs/BUILDING.md).
5. State exactly which platforms and real hardware paths were tested.
6. Update privacy, third-party notices and release documentation when data flow or dependencies change.

Passing a mocked or hosted-VM check does not prove microphone permissions, global hotkeys, focused-app insertion or GPU acceleration on physical hardware.

## Development setup

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
cargo build --no-default-features
cargo test --no-default-features
```

The legacy repository directory and Cargo package name are retained during the MorpheOS Voice transition. Linux GUI development also needs GTK4, libappindicator and the audio/input dependencies documented in [Building](docs/BUILDING.md).

If a filesystem does not support Cargo's locking, use a local target directory:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/morpheos-voice-target cargo build --no-default-features
```

## Code and copy standards

- Run `cargo fmt --check` and strict Clippy for Rust changes.
- Use British English in public copy.
- Prefer plain, evidence-backed language over broad claims.
- Keep microphone audio, transcript text, dictionary entries, clipboard contents and credentials out of logs, fixtures and issue reports.
- Preserve the MIT boundary: third-party libraries, tools and optional models keep their own licences.

## Licence

By contributing, you agree that your original contribution is licensed under the project's [MIT License](LICENSE). Do not submit code or assets you do not have the right to contribute.
