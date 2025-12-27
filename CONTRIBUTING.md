# Contributing to OSWispa

First off, thank you for considering contributing to OSWispa! It's people like you that make it a great tool.

## How Can I Contribute?

### Reporting Bugs
- Use GitHub Issues to report bugs.
- Include OS version, hardware details (especially GPU/ROCm version), and steps to reproduce.

### Suggesting Enhancements
- Enhancement suggestions are tracked as GitHub issues.
- Provide a clear description and use case for the feature.

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

## License
By contributing, you agree that your contributions will be licensed under the MIT License.
