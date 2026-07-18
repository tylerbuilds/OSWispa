# Contributing to OSWispa

First off, thank you for considering contributing to OSWispa! It's people like you that make it a great tool.

## How Can I Contribute?

### Reporting Bugs
- Read [SUPPORT.md](SUPPORT.md), then use the runtime bug or installation problem form in GitHub Issues.
- Include the OSWispa version, operating system, package, relevant backend and the shortest safe reproduction steps.
- Never attach transcript text, recordings, API keys, configuration files, environment dumps, clipboard contents, secrets, or unreviewed crash/core dumps.

### Suggesting Enhancements
- Take open-ended ideas and questions to GitHub Discussions.
- Use the feature proposal form for a bounded improvement with a clear user outcome and measurable success criteria.

### Pull Requests
- We love pull requests! 
- **Platform Support**: One of the biggest goals is to expand OSWispa beyond Ubuntu/Wayland. If you can help port it to:
  - macOS (CoreAudio + CoreML/Metal)
  - Windows (WASAPI + DirectML/CUDA)
  - Other Linux distros (Arch, Fedora, NixOS)
- Please open an issue first to discuss major changes.
- Ensure your code is formatted with `cargo fmt`.

## Development Setup

1.  **Dependencies**: See `README.md` for the Ubuntu setup.
2.  **Build**: `cargo build`
3.  **Run**: `cargo run --features gui`

## Common Issues

### File Locking (External Drives / Network Mounts)
If you keep the repo on a filesystem that does not support file locking (for example exFAT or some network mounts), Cargo may fail with errors like:
`incremental compilation: could not create session directory lock file: Operation not supported (os error 45)`.

Workaround: run Cargo with incremental compilation disabled and a target directory on a local filesystem:
```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/oswispa-target cargo build
```

Git note: if you hit `.git/index.lock`, ensure no other git process is running, then remove the lock file.

## License
By contributing, you agree that your contributions will be licensed under the MIT License.
