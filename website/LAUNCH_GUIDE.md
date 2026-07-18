# OSWispa GitHub Launch Guide 🚀

This guide is for maintainers and contributors preparing to launch or mirror OSWispa.

## Release Checklist

1. **Pull request**: Merge the release changes only after Linux, macOS, and Windows CI is green.
2. **Versioning**: Update `Cargo.toml`, refresh `Cargo.lock`, and move the relevant `CHANGELOG.md` entries out of Unreleased.
3. **Source commit**: Confirm the release commit is on `master` and the working tree is clean.
4. **Tag**: Create and push `vX.Y.Z`, exactly matching the Cargo package version. The workflow rejects tags that do not match or are not contained in `master`.
5. **Assets**: Confirm the release contains the Linux tarball, Debian package, RPM package, both macOS ZIP/DMG pairs, and `SHA256SUMS`.
6. **Smoke test**: Install the package appropriate to each supported platform and run one real microphone, hotkey, clipboard, and auto-paste check.

## Multi-Platform Roadmap

We are looking for help porting OSWispa to more systems!

### High Priority
- **Windows Installer**: A simple `.exe` or `.msi` installer using MSIX or similar.
- **Flatpak/Snap**: Packaging for generic Linux distribution.
- **macOS Signing**: Sign and notarise the existing app bundles.

## Contributing

Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines. We welcome PRs for:
- Localization (more languages).
- UI/UX improvements.
- Support for alternative backends (e.g. Faster-Whisper).
