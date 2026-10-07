#!/usr/bin/env bash
# Privacy gate: nothing personal may be tracked by git — ever.
# Runs locally and in CI on every push; audits the TRACKED tree, so a leak
# fails the build before it can reach a release or Pages deploy.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0
err() { echo "PRIVACY FAIL: $1" >&2; fail=1; }

# 1. forbidden paths must never be tracked
for p in originals web/assets/abuelo; do
  if git ls-files --error-unmatch "$p" >/dev/null 2>&1 || [ -n "$(git ls-files "$p")" ]; then
    err "tracked files under $p/"
  fi
done

# 2. the only tracked raster assets are the generated pack + app icons;
#    every pack glyph must carry the generator's naming (digit_sNN.png)
bad_glyphs=$(git ls-files 'web/assets/grandpere/digits/*' | grep -vE '/[1-9]_s[0-9]{2}\.png$' || true)
[ -z "$bad_glyphs" ] || err "non-generated glyph naming in the pack: $bad_glyphs"
photos=$(git ls-files '*.jpg' '*.jpeg' '*.JPG' '*.heic' '*.HEIC' || true)
[ -z "$photos" ] || err "tracked photo-format files: $photos"

# 3. no personal filesystem paths or private email in any tracked text file
#    (the author's NAME is public by choice — LICENSE, stamps; paths are not)
hits=$(git grep -lIE "/Users/[a-z]+|bernardo\.cohen\.q" -- . ':!scripts/check-privacy.sh' 2>/dev/null || true)
[ -z "$hits" ] || err "personal path/email in tracked files: $hits"

# 4. the family pack dir must be gitignored so it can never be staged
git check-ignore -q web/assets/abuelo/x || err "web/assets/abuelo/ is not gitignored"

# 5. local-only (skipped in CI): built binaries carry no personal strings
APP=web/src-tauri/target/release/bundle/macos/a-souvenir-of-sudokus.app/Contents/MacOS/app
if [ -f "$APP" ]; then
  n=$(strings "$APP" | grep -icE "bernardocohen|/Users/" || true)
  [ "$n" = "0" ] || err "personal strings in the built binary ($n hits) — run scripts/scrub-paths.sh and rebuild"
fi

if [ "$fail" = "0" ]; then echo "privacy gate: clean"; else exit 1; fi
