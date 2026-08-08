#!/usr/bin/env bash
# Cross-compile clip-ffi for Android ABIs and install into jniLibs.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
NDK_VERSION="${NDK_VERSION:-27.0.12077973}"
NDK_HOME="${NDK_HOME:-$ANDROID_HOME/ndk/$NDK_VERSION}"

if [[ ! -d "$NDK_HOME" ]]; then
  # Fall back to any installed NDK.
  if [[ -d "$ANDROID_HOME/ndk" ]]; then
    NDK_HOME="$(ls -d "$ANDROID_HOME/ndk"/* | sort -V | tail -1)"
  fi
fi

if [[ ! -d "$NDK_HOME" ]]; then
  echo "Android NDK not found under $ANDROID_HOME/ndk. Install via sdkmanager." >&2
  exit 1
fi

HOST_TAG="$(uname -s | tr '[:upper:]' '[:lower:]')-x86_64"
if [[ "$(uname -s)" == "Darwin" && "$(uname -m)" == "arm64" ]]; then
  # Apple Silicon NDK still uses darwin-x86_64 toolchain path historically;
  # newer NDKs also ship darwin-aarch64.
  if [[ -d "$NDK_HOME/toolchains/llvm/prebuilt/darwin-aarch64" ]]; then
    HOST_TAG="darwin-aarch64"
  else
    HOST_TAG="darwin-x86_64"
  fi
elif [[ "$(uname -s)" == "Linux" ]]; then
  HOST_TAG="linux-x86_64"
fi

TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG"
API="${ANDROID_API:-26}"

rustup_add_if_needed() {
  local target="$1"
  if rustup target list --installed | grep -qx "$target"; then
    return 0
  fi
  rustup target add "$target"
}

build_abi() {
  local rust_target="$1"
  local jni_abi="$2"
  local clang_triple="$3"
  local clang="$TOOLCHAIN/bin/${clang_triple}${API}-clang"
  local ar="$TOOLCHAIN/bin/llvm-ar"
  local target_env
  target_env="$(echo "$rust_target" | tr '[:lower:]-' '[:upper:]_')"

  if [[ ! -x "$clang" ]]; then
    echo "Missing NDK clang: $clang" >&2
    exit 1
  fi

  rustup_add_if_needed "$rust_target"

  export "AR_${target_env}=$ar"
  export "CC_${target_env}=$clang"
  export "CARGO_TARGET_${target_env}_LINKER=$clang"

  echo "Building clip-ffi for $rust_target ($jni_abi)…"
  cargo build -p clip-ffi --target "$rust_target"

  local dest="$ROOT/apps/android-shell/app/src/main/jniLibs/$jni_abi"
  mkdir -p "$dest"
  cp "$ROOT/target/$rust_target/debug/libclip_ffi.so" "$dest/libclip_ffi.so"
}

cd "$ROOT"
# Prefer arm64 for devices; build x86_64 when the Rust target is already installed (CI).
build_abi aarch64-linux-android arm64-v8a aarch64-linux-android
if rustup target list --installed | grep -qx x86_64-linux-android; then
  build_abi x86_64-linux-android x86_64 x86_64-linux-android
else
  echo "Skipping x86_64-linux-android (run: rustup target add x86_64-linux-android)." >&2
fi

echo "Installed libclip_ffi.so into apps/android-shell/app/src/main/jniLibs."
