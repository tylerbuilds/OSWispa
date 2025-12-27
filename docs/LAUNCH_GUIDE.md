# OSWispa GitHub Launch Guide 🚀

This guide is for maintainers and contributors preparing to launch or mirror OSWispa.

## Release Checklist

1.  **Branching**: Ensure you are on the `master` branch.
2.  **Versioning**: Update the version in `Cargo.toml` if making a formal release.
3.  **Tags**: Create a git tag: `git tag -a v0.1.0 -m \"Initial MIT Release\"`.
4.  **GitHub Actions**: (Optional) Set up a CI pipeline to build binaries for multiple platforms.

## Multi-Platform Roadmap

We are looking for help porting OSWispa to more systems!

### High Priority
- **macOS Native Support**: Utilizing CoreML and Metal for M-series chips.
- **Windows Installer**: A simple `.exe` or `.msi` installer using MSIX or similar.
- **Flatpak/Snap**: Packaging for generic Linux distribution.

## Contributing

Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines. We welcome PRs for:
- Localization (more languages).
- UI/UX improvements.
- Support for alternative backends (e.g. Faster-Whisper).

