# Building the desktop app & shipping a release

The whole chain on any machine is one script:

```bash
./scripts/build-desktop.sh
```

It scrubs personal paths, checks the pack is present, builds the engine to
wasm (pinned emsdk 3.1.64, cloned on first run), then `cargo tauri build`
(which runs trunk itself). Bundles land under
`web/src-tauri/target/release/bundle/`.

## Android (APK)

The engine needs no NDK work: it stays wasm, loaded by Android's WebView the
same way the browser loads it. The NDK is for the Rust shell.

What the machine needs:

- **Android Studio** — its SDK Manager installs the NDK (Settings → Languages
  & Frameworks → Android SDK → SDK Tools → NDK (Side by side), ~2GB).
- **A JDK 21** — and this one bites: Studio's bundled JDK is 25, which Gradle
  8.14 (pinned by the generated project, as AGP requires) refuses with
  `Unsupported class file major version 69`. Install it beside, no sudo:
  ```
  mkdir -p ~/.jdks && cd ~/.jdks
  curl -sL -o t.tar.gz "https://api.adoptium.net/v3/binary/latest/21/ga/mac/aarch64/jdk/hotspot/normal/eclipse"
  tar xzf t.tar.gz && rm t.tar.gz
  ```
- **The Rust targets**: `rustup target add aarch64-linux-android
  armv7-linux-androideabi i686-linux-android x86_64-linux-android`

Then, from `web/`, with the three variables set:

```
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/<version>"
export JAVA_HOME="$HOME/.jdks/jdk-21.<version>/Contents/Home"
cargo tauri android init          # generates src-tauri/gen/android (gitignored)
cargo tauri android build --apk --debug
```

On an emulator (`~/Library/Android/sdk/emulator/emulator -avd <name>`), install
with `adb install -r` and launch
`com.delapatada.souvenirsudokus.debug/com.delapatada.souvenirsudokus.MainActivity`.
The page loads from `http://tauri.localhost/`, which is why `web/index.html`'s
stray-URL gate exempts that host — without it the app bounces itself to the
valvula page on launch.

The Android project at `src-tauri/gen/android` is **committed**, not generated
on demand: it carries the portrait lock, a manifest that asks for no
permissions, and the launcher icons. `cargo tauri android init` would put the
defaults back — run it only on a fresh clone that has no project yet.

`cargo tauri android build --aab` produces the Play artifact;
`--apk --debug` is the one to put on an emulator. The debug build alone asks
for `INTERNET` (from `app/src/debug/AndroidManifest.xml`), because
`android dev` serves the frontend from this machine; the release manifest asks
for nothing at all.

A release APK/AAB needs a **keystore**, which is a third key to back up
alongside the updater key and `wm.key`. Create it in Studio (Build → Generate
Signed Bundle / APK); once the app is on Play it can never be replaced.

## Before the first build on a new machine

1. Install the per-OS prerequisites below. The public pack (generated
   typeface + synthetic paper) is committed to the repo — a bare clone
   builds the full game. (The real-scan family pack lives only in
   `web/assets/abuelo/` on the main machine, gitignored; builds made there
   automatically prefer it.)
2. `gh auth login` if you'll upload release binaries from that machine.

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

The released binaries embed the souvenir pack; the assets are stamped and
watermarked (see ASSETS-LICENSE.md) and the binaries are scrubbed of personal
paths (verify with `strings <binary> | grep -c "$HOME"` → 0).
