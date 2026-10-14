#!/usr/bin/env bash
# Build the Android app without having to remember three environment
# variables. Finds the SDK, the NDK and a JDK Gradle can actually run on, then
# hands over to the tauri CLI.
#
#   ./scripts/android.sh apk     debug APK, for an emulator or a cable
#   ./scripts/android.sh aab     release bundle, the artifact Play wants
#   ./scripts/android.sh init    first time only, on a machine with no project
#
# Add --family to bake in his real handwriting (see BUILDING.md). Without it
# this is a regular build, which is what anyone else would get.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="${1:-apk}"
shift || true
for arg in "$@"; do
  case "$arg" in
    --family) export SOUVENIR_FAMILY=1 ;;
    *) echo "unknown argument: $arg (only --family is understood)" >&2; exit 1 ;;
  esac
done

# --- the SDK -----------------------------------------------------------------
: "${ANDROID_HOME:=$HOME/Library/Android/sdk}"
[ -d "$ANDROID_HOME" ] || ANDROID_HOME="$HOME/Android/Sdk" # Linux layout
if [ ! -d "$ANDROID_HOME" ]; then
  echo "FATAL: no Android SDK. Install Android Studio, or set ANDROID_HOME." >&2
  exit 1
fi
export ANDROID_HOME

# --- the NDK (newest installed; it compiles the Rust shell, not the engine) ---
if [ -z "${NDK_HOME:-}" ]; then
  NDK_HOME=$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -1 || true)
fi
if [ -z "$NDK_HOME" ] || [ ! -d "$NDK_HOME" ]; then
  echo "FATAL: no NDK under $ANDROID_HOME/ndk." >&2
  echo "       Android Studio: Settings > Languages & Frameworks > Android SDK" >&2
  echo "       > SDK Tools > NDK (Side by side). About 2GB." >&2
  exit 1
fi
export NDK_HOME

# --- a JDK Gradle will run on ------------------------------------------------
# Gradle 8.14 (pinned by the generated project, because AGP requires it) cannot
# run on Java 25, which is what Android Studio bundles: it stops with
# "Unsupported class file major version 69". So prefer a JDK in the supported
# range and only fall back to whatever is around.
jdk_major() { # $1 = JAVA_HOME candidate -> prints the major version, or nothing
  [ -x "$1/bin/java" ] || return 0
  "$1/bin/java" -version 2>&1 | sed -n '1s/.*version "\([0-9]*\).*/\1/p'
}
if [ -z "${JAVA_HOME:-}" ] || [ "$(jdk_major "${JAVA_HOME:-}")" -gt 24 ] 2>/dev/null; then
  for cand in "$HOME"/.jdks/*/Contents/Home "$HOME"/.jdks/* \
              /Library/Java/JavaVirtualMachines/*/Contents/Home /usr/lib/jvm/*; do
    v=$(jdk_major "$cand")
    if [ -n "$v" ] && [ "$v" -ge 17 ] && [ "$v" -le 24 ]; then
      JAVA_HOME="$cand"
      break
    fi
  done
fi
if [ -z "${JAVA_HOME:-}" ] || [ ! -x "${JAVA_HOME}/bin/java" ]; then
  cat >&2 <<'MSG'
FATAL: no JDK that Gradle can run on (it needs 17-24; Android Studio ships 25).
       Install one beside it, no admin rights needed:

         mkdir -p ~/.jdks && cd ~/.jdks
         curl -L -o t.tar.gz "https://api.adoptium.net/v3/binary/latest/21/ga/mac/aarch64/jdk/hotspot/normal/eclipse"
         tar xzf t.tar.gz && rm t.tar.gz
MSG
  exit 1
fi
export JAVA_HOME

echo "== sdk:  $ANDROID_HOME"
echo "== ndk:  $(basename "$NDK_HOME")"
echo "== jdk:  $(jdk_major "$JAVA_HOME") at $JAVA_HOME"
if [ "${SOUVENIR_FAMILY:-0}" = "1" ]; then
  echo "== pack: HIS HANDWRITING will be in this build — for the family, not for publishing"
else
  echo "== pack: regular build (pass --family for his handwriting)"
fi

# trunk serve and the android build both write web/dist
pkill -f "trunk serve" 2>/dev/null && echo "== stopped the dev server (it shares web/dist)" || true

cd web
case "$TARGET" in
  init) cargo tauri android init ;;
  apk)  cargo tauri android build --apk --debug ;;
  aab)  cargo tauri android build --aab ;;
  *) echo "unknown target: $TARGET (apk | aab | init)" >&2; exit 1 ;;
esac
