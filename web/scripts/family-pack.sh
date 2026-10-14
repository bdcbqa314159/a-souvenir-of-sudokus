#!/usr/bin/env sh
# Puts the real-scan pack (his actual handwriting) into a build — and only when
# it is asked for by name.
#
# Why opt-in: this machine holds the originals, so every build used to bake them
# in silently. That is right for a build for the family and wrong for anything
# else, and "anything else" includes the Play Store artifact, which is a local
# build by nature. A regular build is now the default even here; the flag is the
# only way to get his hand, and it fails loudly rather than quietly producing a
# public build when the pack is not there.
#
# Run by trunk as a post_build hook, so it covers the browser, the desktop app
# and the Android build alike — all three go through trunk.
set -eu

SRC="${TRUNK_SOURCE_DIR:-.}"
OUT="${TRUNK_STAGING_DIR:?this script is a trunk hook and needs TRUNK_STAGING_DIR}"

if [ "${SOUVENIR_FAMILY:-0}" != "1" ]; then
  exit 0 # a regular build: the generated pack carries it
fi

if [ ! -f "$SRC/assets/abuelo/manifest.json" ]; then
  echo "FATAL: SOUVENIR_FAMILY=1 asks for the family pack, but" >&2
  echo "       web/assets/abuelo/manifest.json is not here. That pack exists only" >&2
  echo "       on the maintainer's machine — run 'atelier/pipeline.py emit' there," >&2
  echo "       or build without the flag for the generated pack." >&2
  exit 1
fi

mkdir -p "$OUT/assets"
cp -R "$SRC/assets/abuelo" "$OUT/assets/"
echo "== pack: HIS HANDWRITING is in this build (SOUVENIR_FAMILY=1) — do not publish it =="
