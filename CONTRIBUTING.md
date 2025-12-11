# Contributing to OSWispa

Thank you for your interest in contributing! Here's how you can help.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/oswispa.git`
3. Create a branch: `git checkout -b feature/your-feature-name`

## Development Setup

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential cmake pkg-config libssl-dev \
    libayatana-appindicator3-dev libasound2-dev

# Build in debug mode
cargo build

# Run with logging
RUST_LOG=debug cargo run
```

## Code Style

- Follow Rust conventions (`cargo fmt` before committing)
- Run `cargo clippy` and address warnings
- Add tests for new functionality

## Pull Request Process

1. Update the README.md if needed
2. Ensure all tests pass: `cargo test`
3. Update CHANGELOG.md (if exists)
4. Submit PR with clear description

## Reporting Issues

When reporting bugs, include:
- OS and version
- GPU type (if relevant)
- Steps to reproduce
- Error messages or logs

## Feature Requests

Open an issue describing:
- The problem you're trying to solve
- Your proposed solution
- Alternative approaches considered

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
