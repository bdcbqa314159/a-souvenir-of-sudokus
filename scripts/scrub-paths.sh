#!/usr/bin/env bash
# Generate the gitignored .cargo/config.toml that strips personal paths from
# every Rust artifact (native app + wasm). Run once per clone, before building
# anything you intend to distribute. The config is generated (not committed)
# so no absolute path ever reaches the repo.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$root/.cargo"
cat > "$root/.cargo/config.toml" <<CFG
[build]
rustflags = [
  "--remap-path-prefix", "$root=/src",
  "--remap-path-prefix", "$HOME=/anon",
]
CFG
echo "wrote $root/.cargo/config.toml (remaps \$HOME and the repo path)"
