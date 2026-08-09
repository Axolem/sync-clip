#!/usr/bin/env bash
# Build, codesign, package, and optionally notarize the macOS Shell DMG into dist/.
#
# Required for public GitHub Releases:
#   SYNC_CLIP_CODESIGN_IDENTITY   e.g. "Developer ID Application: Name (TEAMID)"
# Notarization (pick one auth style):
#   SYNC_CLIP_NOTARY_KEY_PATH + SYNC_CLIP_NOTARY_KEY_ID + SYNC_CLIP_NOTARY_ISSUER_ID
#   OR SYNC_CLIP_APPLE_ID + SYNC_CLIP_APP_SPECIFIC_PASSWORD (+ SYNC_CLIP_TEAM_ID)
#
# Without SYNC_CLIP_CODESIGN_IDENTITY: ad-hoc sign + unsigned DMG for local smoke only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(tr -d '[:space:]' <"$ROOT/VERSION")"
DIST_DIR="${DIST_DIR:-$ROOT/dist}"
STAGE="${STAGE:-$ROOT/dist/.macos-stage}"
APP_NAME="Sync Clip"
APP_BUNDLE_NAME="SyncClip Shell.app"
DMG_NAME="SyncClip-Shell-${VERSION}-macos.dmg"
IDENTITY="${SYNC_CLIP_CODESIGN_IDENTITY:-}"
NOTARIZE="${NOTARIZE:-1}"
INSTALL_LOCAL="${INSTALL_LOCAL:-0}"

export OPEN_APP=0
export INSTALL=1
export APP_DIR="${APP_DIR:-$STAGE/$APP_BUNDLE_NAME}"

cd "$ROOT"
rm -rf "$STAGE"
mkdir -p "$STAGE" "$DIST_DIR"

# Build into the staging app dir (do not open).
./scripts/build-macos-shell.sh

APP="$APP_DIR"
if [[ ! -d "$APP" ]]; then
  echo "Expected app bundle missing: $APP" >&2
  exit 1
fi

sign_app() {
  local app="$1"
  if [[ -n "$IDENTITY" ]]; then
    echo "Codesigning with Developer ID…"
    codesign \
      --force \
      --deep \
      --options runtime \
      --timestamp \
      --sign "$IDENTITY" \
      "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
  else
    echo "WARNING: SYNC_CLIP_CODESIGN_IDENTITY unset — ad-hoc signing (not for public release)." >&2
    codesign --force --deep --sign - "$app" || true
    NOTARIZE=0
  fi
}

sign_app "$APP"

DMG_PATH="$DIST_DIR/$DMG_NAME"
rm -f "$DMG_PATH"

VOL="Sync Clip ${VERSION}"
TMP_DMG="$DIST_DIR/.tmp-sync-clip.dmg"
rm -f "$TMP_DMG"
hdiutil create \
  -volname "$VOL" \
  -srcfolder "$STAGE" \
  -ov \
  -format UDZO \
  "$TMP_DMG"
mv "$TMP_DMG" "$DMG_PATH"

if [[ -n "$IDENTITY" ]]; then
  codesign --force --timestamp --sign "$IDENTITY" "$DMG_PATH"
fi

notarize_dmg() {
  local dmg="$1"
  if [[ "$NOTARIZE" != "1" ]]; then
    echo "Skipping notarization (NOTARIZE=$NOTARIZE)."
    return 0
  fi
  if [[ -z "$IDENTITY" ]]; then
    echo "Skipping notarization (no codesign identity)."
    return 0
  fi

  local args=()
  if [[ -n "${SYNC_CLIP_NOTARY_KEY_PATH:-}" && -n "${SYNC_CLIP_NOTARY_KEY_ID:-}" && -n "${SYNC_CLIP_NOTARY_ISSUER_ID:-}" ]]; then
    args+=(
      --key "$SYNC_CLIP_NOTARY_KEY_PATH"
      --key-id "$SYNC_CLIP_NOTARY_KEY_ID"
      --issuer "$SYNC_CLIP_NOTARY_ISSUER_ID"
    )
  elif [[ -n "${SYNC_CLIP_APPLE_ID:-}" && -n "${SYNC_CLIP_APP_SPECIFIC_PASSWORD:-}" ]]; then
    args+=(
      --apple-id "$SYNC_CLIP_APPLE_ID"
      --password "$SYNC_CLIP_APP_SPECIFIC_PASSWORD"
    )
    if [[ -n "${SYNC_CLIP_TEAM_ID:-}" ]]; then
      args+=(--team-id "$SYNC_CLIP_TEAM_ID")
    fi
  else
    echo "WARNING: Notary credentials unset — DMG signed but not notarized." >&2
    echo "Set SYNC_CLIP_NOTARY_KEY_* or SYNC_CLIP_APPLE_ID + SYNC_CLIP_APP_SPECIFIC_PASSWORD." >&2
    return 0
  fi

  echo "Submitting DMG to notarytool…"
  xcrun notarytool submit "$dmg" "${args[@]}" --wait
  xcrun stapler staple "$dmg"
  xcrun stapler validate "$dmg"
}

notarize_dmg "$DMG_PATH"

shasum -a 256 "$DMG_PATH" | awk '{print $1}' >"${DMG_PATH}.sha256"
echo "DMG: $DMG_PATH"
echo "SHA256: $(cat "${DMG_PATH}.sha256")"

if [[ "$INSTALL_LOCAL" == "1" ]]; then
  LOCAL_APP="$HOME/Applications/$APP_BUNDLE_NAME"
  rm -rf "$LOCAL_APP"
  cp -R "$APP" "$LOCAL_APP"
  echo "Installed local copy: $LOCAL_APP"
fi

echo "Verify Gatekeeper (after notarization): spctl --assess -vv --type install \"$DMG_PATH\""
