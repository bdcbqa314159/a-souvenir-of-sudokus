#!/usr/bin/env bash
# One-shot desktop build on any machine: engine wasm -> web dist -> native
# bundle. On Windows, run inside Git Bash. See BUILDING.md for the per-OS
# prerequisites and for shipping the bundles as a GitHub release.
set -euo pipefail
cd "$(dirname "$0")/.."

# privacy first: strip personal paths from anything we might distribute
[ -f .cargo/config.toml ] || ./scripts/scrub-paths.sh

# pack resolution: the ORIGINAL family pack (web/assets/abuelo/, real
# handwriting, this-machine-only) is preferred by the game at runtime when
# baked in; otherwise the GENERATED pack (committed to the repo) carries the
# build. Validate whatever is present and say clearly which game this is.
check_pack() {  # $1 = pack dir; prints glyph count or returns 1
  python3 - "$1" <<'PY'
import json, pathlib, sys
d = pathlib.Path(sys.argv[1])
try:
    m = json.loads((d / "manifest.json").read_text())
    refs = [p for role in m["digits"].values() for v in role.values() for p in v]
    missing = [p for p in refs if not (d / p).exists()]
    assert refs and not missing, f"{len(missing)} referenced files missing"
    assert "paper" not in m or (d / m["paper"]).exists(), "paper missing"
except Exception as e:
    print(f"invalid: {e}", file=sys.stderr); sys.exit(1)
print(len(refs))
PY
}

HAVE_ORIGINAL=no
if [ -d web/assets/abuelo ]; then
  if n=$(check_pack web/assets/abuelo); then
    HAVE_ORIGINAL=yes
    echo "== pack: ORIGINAL family pack found ($n glyphs) — this build will show the real handwriting =="
  else
    echo "WARNING: web/assets/abuelo/ exists but is invalid — it will be ignored at runtime" >&2
  fi
fi
if n=$(check_pack web/assets/grandpere); then
  [ "$HAVE_ORIGINAL" = yes ] || echo "== pack: generated pack ($n glyphs) — public build =="
else
  [ "$HAVE_ORIGINAL" = yes ] || {
    echo "FATAL: no usable pack. web/assets/grandpere is missing or invalid (corrupted checkout?)" >&2
    echo "       re-clone the repo, or run 'atelier/pipeline.py synth && genpaper' on the main machine." >&2
    exit 1
  }
fi

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
  BUNDLE_ARGS="--bundles app"
else
  BUNDLE_ARGS=""
fi
# tauri's asset scan flakes on the first build after pack files changed;
# one retry with a forced macro re-eval reliably clears it
(cd web && cargo tauri build $BUNDLE_ARGS) || {
  touch web/src-tauri/src/lib.rs
  (cd web && cargo tauri build $BUNDLE_ARGS)
}

echo
echo "== bundles =="
find web/src-tauri/target/release/bundle -type f \
  \( -name '*.AppImage' -o -name '*.deb' -o -name '*.rpm' -o -name '*.exe' -o -name '*.msi' -o -name '*.dmg' \) 2>/dev/null || true
[ -d web/src-tauri/target/release/bundle/macos ] && ls -d web/src-tauri/target/release/bundle/macos/*.app 2>/dev/null || true
