# OSWispa GitHub Launch Guide 🚀

This guide is for maintainers and contributors preparing to launch or mirror OSWispa.

## Release Checklist

1. **Pull request**: Merge the release changes only after Linux, macOS, and Windows CI is green.
2. **Versioning**: Update `Cargo.toml`, refresh `Cargo.lock`, and move the relevant `CHANGELOG.md` entries out of Unreleased.
3. **Source commit**: Confirm the release commit is on `master` and the working tree is clean.
4. **Tag**: Create and push `vX.Y.Z`, exactly matching the Cargo package version. The workflow rejects tags that do not match or are not contained in `master`.
5. **Assets**: Confirm the release contains the Linux tarball, Debian package, RPM package, both macOS ZIP/DMG pairs, the Windows x86-64 ZIP, and `SHA256SUMS`.
6. **VM package gate**: Mount and install both macOS DMGs and extract the Windows ZIP on clean hosted VMs. Execute the packaged launcher and require the version, platform, WAV contract, clipboard round-trip, and native backend checks to pass.
7. **Public-download gate**: After publication, dispatch `Public Release Smoke` with the new tag. It downloads the public assets, verifies `SHA256SUMS`, reinstalls them on clean macOS and Windows VMs, and reruns the package checks.
8. **Hardware boundary**: VM automation does not prove microphone permission prompts, physical hotkeys, focus-sensitive auto-paste, or GPU acceleration. Record those separately whenever suitable hardware is available; never describe them as covered by the VM gate.

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
