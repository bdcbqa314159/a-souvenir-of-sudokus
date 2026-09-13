# Building the desktop app & shipping a release

The whole chain on any machine is one script:

```bash
./scripts/build-desktop.sh
```

It scrubs personal paths, checks the pack is present, builds the engine to
wasm (pinned emsdk 3.1.64, cloned on first run), then `cargo tauri build`
(which runs trunk itself). Bundles land under
`web/src-tauri/target/release/bundle/`.

## Before the first build on a new machine

1. **Copy the pack**: `web/assets/grandpere/` is gitignored (it never enters
   git). Bring it over by USB/scp from the main machine into the same path.
2. Install the per-OS prerequisites below.
3. `gh auth login` if you'll upload release binaries from that machine.

## Prerequisites

**All platforms**: git, CMake ≥ 3.24, Python 3 (emsdk needs it),
[rustup](https://rustup.rs) with the stable toolchain.

**Linux** (Debian/Ubuntu — Tauri's webview deps + compilers):
```bash
sudo apt install build-essential curl wget file libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev
```
Produces: `.AppImage` (portable, recommended to ship) and `.deb`.

**Windows**:
- Visual Studio Build Tools with the "Desktop development with C++" workload
- Run the script inside **Git Bash**
- WebView2 runtime is preinstalled on Windows 10/11

Produces: NSIS `.exe` installer (recommended to ship) and `.msi`.

**macOS**: Xcode command line tools. Produces the `.app`
(`cargo tauri build --bundles app`); zip it for shipping:
`ditto -c -k --keepParent <name>.app <name>-macos.zip`.

## Shipping a GitHub release

From any machine with `gh` authenticated (note: a release creates a git tag):

```bash
# first machine creates the release
gh release create v0.1.0 --title "a-souvenir-of-sudokus v0.1.0" \
  --notes "el abuelo would be proud." \
  path/to/bundle.AppImage

# the other machines add their binaries to it
gh release upload v0.1.0 path/to/setup.exe
gh release upload v0.1.0 souvenir-macos.zip
```

Suggested naming: `a-souvenir-of-sudokus_<version>_<os>.<ext>`.

The released binaries embed the grandpere pack; the assets are stamped and
watermarked (see ASSETS-LICENSE.md) and the binaries are scrubbed of personal
paths (verify with `strings <binary> | grep -c "$HOME"` → 0).
