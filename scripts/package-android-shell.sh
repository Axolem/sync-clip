#!/usr/bin/env bash
# Package the Android Shell release APK (and optionally AAB) into dist/.
# Env signing (recommended for public releases):
#   SYNC_CLIP_ANDROID_KEYSTORE
#   SYNC_CLIP_ANDROID_STORE_PASSWORD
#   SYNC_CLIP_ANDROID_KEY_ALIAS
#   SYNC_CLIP_ANDROID_KEY_PASSWORD
# Set BUNDLE=1 to also emit an .aab for Play Console prep.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(tr -d '[:space:]' <"$ROOT/VERSION")"
BUNDLE="${BUNDLE:-0}"
SKIP_JNI="${SKIP_JNI:-0}"
DIST_DIR="${DIST_DIR:-$ROOT/dist}"

export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
export PATH="$ANDROID_HOME/platform-tools:$PATH"

cd "$ROOT"

if [[ "$SKIP_JNI" != "1" ]]; then
  ./scripts/build-android-jni.sh
fi

cd "$ROOT/apps/android-shell"
./gradlew assembleRelease --no-daemon
if [[ "$BUNDLE" == "1" ]]; then
  ./gradlew bundleRelease --no-daemon
fi

mkdir -p "$DIST_DIR"
APK_SRC="$(find app/build/outputs/apk/release -name '*.apk' | head -1)"
if [[ -z "$APK_SRC" || ! -f "$APK_SRC" ]]; then
  echo "Release APK not found under app/build/outputs/apk/release" >&2
  exit 1
fi

APK_DEST="$DIST_DIR/sync-clip-shell-${VERSION}.apk"
cp "$APK_SRC" "$APK_DEST"
# Keep a stable name for local adb install scripts.
cp "$APK_SRC" "$DIST_DIR/sync-clip-shell-release.apk"
shasum -a 256 "$APK_DEST" | awk '{print $1}' >"${APK_DEST}.sha256"

echo "APK: $APK_DEST"
echo "SHA256: $(cat "${APK_DEST}.sha256")"

if [[ "$BUNDLE" == "1" ]]; then
  AAB_SRC="$(find app/build/outputs/bundle/release -name '*.aab' | head -1)"
  if [[ -z "$AAB_SRC" || ! -f "$AAB_SRC" ]]; then
    echo "Release AAB not found under app/build/outputs/bundle/release" >&2
    exit 1
  fi
  AAB_DEST="$DIST_DIR/sync-clip-shell-${VERSION}.aab"
  cp "$AAB_SRC" "$AAB_DEST"
  shasum -a 256 "$AAB_DEST" | awk '{print $1}' >"${AAB_DEST}.sha256"
  echo "AAB: $AAB_DEST"
  echo "SHA256: $(cat "${AAB_DEST}.sha256")"
fi

if [[ -z "${SYNC_CLIP_ANDROID_KEYSTORE:-}" ]]; then
  echo "WARNING: SYNC_CLIP_ANDROID_KEYSTORE unset — APK is debug-keystore signed; do not publish." >&2
fi
