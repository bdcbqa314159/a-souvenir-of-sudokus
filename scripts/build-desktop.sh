#!/usr/bin/env bash
# One-shot desktop build on any machine: engine wasm -> web dist -> native
# bundle. On Windows, run inside Git Bash. See BUILDING.md for the per-OS
# prerequisites and for shipping the bundles as a GitHub release.
set -euo pipefail
cd "$(dirname "$0")/.."

# privacy first: strip personal paths from anything we might distribute
[ -f .cargo/config.toml ] || ./scripts/scrub-paths.sh

# the pack never travels through git — copy it from the mac (USB/scp)
[ -d web/assets/grandpere ] || {
  echo "MISSING web/assets/grandpere/ — copy the pack from the main machine first" >&2
  exit 1
}

# engine -> wasm, with the pinned emsdk (cloned per machine, gitignored)
if [ ! -d .emsdk ]; then
  git clone --depth 1 https://github.com/emscripten-core/emsdk.git .emsdk
  ./.emsdk/emsdk install 3.1.64
  ./.emsdk/emsdk activate 3.1.64
fi
source ./.emsdk/emsdk_env.sh
emcmake cmake -S engine -B engine/build/wasm -DCMAKE_BUILD_TYPE=Release
cmake --build engine/build/wasm -j

# rust toolchain pieces (no-ops when already present)
rustup target add wasm32-unknown-unknown
command -v trunk >/dev/null || cargo install trunk --locked
command -v cargo-tauri >/dev/null || cargo install tauri-cli --locked

# tauri's generate_context! caches its asset index across builds; stale
# entries break the build when pack files changed — force a fresh scan
rm -rf web/dist
touch web/src-tauri/src/lib.rs

if [ "$(uname -s)" = Darwin ]; then
  # dmg bundling needs an interactive terminal; the shipped artifact is the
  # zipped .app anyway (BUILDING.md)
  (cd web && cargo tauri build --bundles app)
else
  (cd web && cargo tauri build)
fi

echo
echo "== bundles =="
find web/src-tauri/target/release/bundle -type f \
  \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.exe' -o -name '*.msi' -o -name '*.dmg' \) 2>/dev/null || true
[ -d web/src-tauri/target/release/bundle/macos ] && ls -d web/src-tauri/target/release/bundle/macos/*.app 2>/dev/null || true
