#!/usr/bin/env bash
# Build clip-ffi staticlib, force-relink the macOS Shell, optionally install the .app.
# SwiftPM does not always invalidate link inputs when only libclip_ffi.a changes —
# this script always cleans .build so the new archive is linked.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

PROFILE="${PROFILE:-release}"
INSTALL="${INSTALL:-1}"
APP_DIR="${APP_DIR:-$HOME/Applications/SyncClip Shell.app}"

cd "$ROOT"

echo "Building clip-ffi ($PROFILE) staticlib…"
if [[ "$PROFILE" == "release" ]]; then
  cargo build -p clip-ffi --release
  LIB="$ROOT/target/release/libclip_ffi.a"
else
  cargo build -p clip-ffi
  LIB="$ROOT/target/debug/libclip_ffi.a"
fi

mkdir -p "$ROOT/apps/macos-shell/lib"
cp "$LIB" "$ROOT/apps/macos-shell/lib/libclip_ffi.a"
# Bump mtime so any watcher/tooling notices the archive change.
touch "$ROOT/apps/macos-shell/lib/libclip_ffi.a"

cd "$ROOT/apps/macos-shell"
echo "Cleaning SwiftPM build (forced relink of libclip_ffi.a)…"
rm -rf .build
if [[ "$PROFILE" == "release" ]]; then
  swift build -c release
  BIN="$(swift build -c release --show-bin-path)/MacosShell"
else
  swift build
  BIN="$(swift build --show-bin-path)/MacosShell"
fi

ls -lh "$BIN"

if [[ "$INSTALL" != "1" ]]; then
  echo "Built: $BIN (INSTALL=0, skipped app install)"
  exit 0
fi

echo "Installing $APP_DIR ..."
pkill -f "SyncClip Shell" 2>/dev/null || true
pkill -f "MacosShell" 2>/dev/null || true
sleep 0.3

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BIN" "$APP_DIR/Contents/MacOS/SyncClip Shell"
chmod +x "$APP_DIR/Contents/MacOS/SyncClip Shell"

cat > "$APP_DIR/Contents/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>SyncClip Shell</string>
	<key>CFBundleIdentifier</key>
	<string>com.syncclip.shell</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>SyncClip Shell</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>LSMinimumSystemVersion</key>
	<string>13.0</string>
	<key>LSUIElement</key>
	<true/>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
EOF

mkdir -p "$HOME/.local/bin"
cp "$BIN" "$HOME/.local/bin/sync-clip-shell"
chmod +x "$HOME/.local/bin/sync-clip-shell"

open "$APP_DIR"
echo "Installed: $APP_DIR"
echo "CLI: $HOME/.local/bin/sync-clip-shell"
