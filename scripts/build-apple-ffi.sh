#!/usr/bin/env bash
# Cross-compile clip-ffi for Apple platforms (macOS + iOS device/simulator) into an xcframework.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

PROFILE="${PROFILE:-release}"
OUT_DIR="${OUT_DIR:-$ROOT/apps/apple-shell/lib}"
XCFRAMEWORK="$OUT_DIR/ClipFfi.xcframework"

rustup_add_if_needed() {
  local target="$1"
  if rustup target list --installed | grep -qx "$target"; then
    return 0
  fi
  rustup target add "$target"
}

build_target() {
  local target="$1"
  rustup_add_if_needed "$target"
  echo "Building clip-ffi ($PROFILE) for $target…"
  if [[ "$PROFILE" == "release" ]]; then
    cargo build -p clip-ffi --release --target "$target"
    echo "$ROOT/target/$target/release/libclip_ffi.a"
  else
    cargo build -p clip-ffi --target "$target"
    echo "$ROOT/target/$target/debug/libclip_ffi.a"
  fi
}

cd "$ROOT"

MAC_LIB="$(build_target aarch64-apple-darwin)"
# Intel Mac slice when the toolchain is present (optional).
X86_MAC_LIB=""
if rustup target list --installed | grep -qx x86_64-apple-darwin; then
  X86_MAC_LIB="$(build_target x86_64-apple-darwin)"
fi

IOS_LIB="$(build_target aarch64-apple-ios)"
SIM_LIB=""
if rustup target list --installed | grep -qx aarch64-apple-ios-sim; then
  SIM_LIB="$(build_target aarch64-apple-ios-sim)"
else
  rustup_add_if_needed aarch64-apple-ios-sim
  SIM_LIB="$(build_target aarch64-apple-ios-sim)"
fi

# visionOS slices when the SDK + Rust targets exist (optional).
XROS_LIB=""
XROS_SIM_LIB=""
if xcrun --sdk xros --show-sdk-path >/dev/null 2>&1; then
  if rustup target list | grep -q '^aarch64-apple-visionos'; then
    rustup_add_if_needed aarch64-apple-visionos || true
    if rustup target list --installed | grep -qx aarch64-apple-visionos; then
      XROS_LIB="$(build_target aarch64-apple-visionos)" || XROS_LIB=""
    fi
  fi
  if rustup target list | grep -q '^aarch64-apple-visionos-sim'; then
    rustup_add_if_needed aarch64-apple-visionos-sim || true
    if rustup target list --installed | grep -qx aarch64-apple-visionos-sim; then
      XROS_SIM_LIB="$(build_target aarch64-apple-visionos-sim)" || XROS_SIM_LIB=""
    fi
  fi
else
  echo "visionOS SDK not installed — skipping xros xcframework slices."
fi

mkdir -p "$OUT_DIR"
rm -rf "$XCFRAMEWORK"

MAC_UNIVERSAL="$OUT_DIR/libclip_ffi_macos.a"
if [[ -n "$X86_MAC_LIB" ]]; then
  lipo -create -output "$MAC_UNIVERSAL" "$MAC_LIB" "$X86_MAC_LIB"
else
  cp "$MAC_LIB" "$MAC_UNIVERSAL"
fi

# SPM / menu-bar macOS path still expects libclip_ffi.a next to Package.swift.
cp "$MAC_UNIVERSAL" "$OUT_DIR/libclip_ffi.a"
mkdir -p "$ROOT/apps/macos-shell/lib"
cp "$MAC_UNIVERSAL" "$ROOT/apps/macos-shell/lib/libclip_ffi.a"

HDR_DIR="$OUT_DIR/headers"
rm -rf "$HDR_DIR"
mkdir -p "$HDR_DIR"
cp "$ROOT/apps/apple-shell/Generated/clip_ffiFFI.h" "$HDR_DIR/"

ARGS=(
  -create-xcframework
  -library "$MAC_UNIVERSAL" -headers "$HDR_DIR"
  -library "$IOS_LIB" -headers "$HDR_DIR"
)
if [[ -n "$SIM_LIB" ]]; then
  ARGS+=(-library "$SIM_LIB" -headers "$HDR_DIR")
fi
if [[ -n "$XROS_LIB" ]]; then
  ARGS+=(-library "$XROS_LIB" -headers "$HDR_DIR")
fi
if [[ -n "$XROS_SIM_LIB" ]]; then
  ARGS+=(-library "$XROS_SIM_LIB" -headers "$HDR_DIR")
fi
ARGS+=(-output "$XCFRAMEWORK")

xcodebuild "${ARGS[@]}"
echo "Wrote $XCFRAMEWORK"
ls -lh "$OUT_DIR"
