# Building MorpheOS Voice

The transition source package and compatibility executable remain named `oswispa`. The public product name is MorpheOS Voice.

## Common requirements

- Current stable Rust toolchain
- CMake 3.16 or newer
- Git
- Internet access for Rust dependencies and the first speech-model download

Clone the current official repository:

```bash
git clone https://github.com/tylerbuilds/OSWispa.git
cd OSWispa
```

## Linux

Ubuntu/Debian build dependencies used by CI:

```bash
sudo apt install pkg-config libssl-dev libasound2-dev libdbus-1-dev \
  libgtk-4-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev desktop-file-utils rpm cmake
```

Build the compatibility CLI with the Linux GUI:

```bash
cargo build --locked
```

Build the core without GTK:

```bash
cargo build --locked --no-default-features
```

Run the source installer for a normal Linux setup, including runtime tools and the user service:

```bash
./install.sh
```

The installer may request administrator approval to install packages, copy the compatibility binary to `/usr/local/bin/oswispa` and add Linux input permissions. Review the script before running it.

## macOS

```bash
xcode-select --install
brew install cmake
cargo build --release --locked --no-default-features
```

Apple Silicon Metal build:

```bash
cargo build --release --locked --no-default-features --features gpu-metal
```

The current package is unsigned and requires Microphone plus Accessibility permission for the physical dictation loop.

## Windows

Install the stable Rust MSVC toolchain, Visual Studio Build Tools with C++ support and CMake, then run in a Developer PowerShell:

```powershell
cargo build --release --locked --no-default-features
```

The output remains `target\release\oswispa.exe` during the compatibility transition.

## Optional acceleration

```bash
# NVIDIA CUDA toolkit installed
cargo build --release --locked --features gpu-cuda

# AMD ROCm installed; list every target present on the build machine
AMDGPU_TARGETS="gfx1100" cargo build --release --locked --features gpu-hipblas
```

Acceleration features compile platform-specific native code. A successful build does not prove that the target user's driver or GPU can execute it.

## Tauri desktop foundation

Linux additionally needs WebKitGTK 4.1 and AppIndicator development packages.

```bash
cargo check --locked -p oswispa-desktop
cargo test --locked -p oswispa-desktop --no-default-features
```

Tauri bundling remains disabled while onboarding, signing and installer work is incomplete.

## Test gate

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --no-default-features
python3 -m unittest discover -s desktop/ui/tests -p 'test_*.py'
python3 scripts/check_website.py
```

Hardware tests are separate. Follow [the rebrand test plan](rebrand/03_TEST_PLAN.md) before describing a platform as fully proved.
