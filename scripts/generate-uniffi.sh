#!/usr/bin/env bash
# Build clip-ffi and regenerate UniFFI Swift/Kotlin bindings into Shell trees.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=/dev/null
source "$HOME/.cargo/env"

cd "$ROOT"
cargo build -p clip-ffi

LIB="$ROOT/target/debug/libclip_ffi.dylib"
if [[ ! -f "$LIB" ]]; then
  LIB="$ROOT/target/debug/libclip_ffi.so"
fi

OUT="$ROOT/target/uniffi-out"
rm -rf "$OUT"
cargo run -q -p clip-ffi --bin uniffi-bindgen -- generate \
  --library "$LIB" \
  --language swift \
  --language kotlin \
  --out-dir "$OUT"

MAC_GEN="$ROOT/apps/macos-shell/Generated"
MAC_CORE="$ROOT/apps/macos-shell/Sources/MacosShellCore"
mkdir -p "$MAC_GEN" "$MAC_CORE" "$ROOT/apps/macos-shell/lib"
cp "$OUT/clip_ffi.swift" "$MAC_CORE/clip_ffi.swift"
cp "$OUT/clip_ffiFFI.h" "$MAC_GEN/"
# Rewrite modulemap path for SPM layout.
cat > "$MAC_GEN/module.modulemap" <<'EOF'
module clip_ffiFFI {
    header "clip_ffiFFI.h"
    export *
}
EOF

cp "$ROOT/target/debug/libclip_ffi.a" "$ROOT/apps/macos-shell/lib/libclip_ffi.a"

AND_KT="$ROOT/apps/android-shell/app/src/main/java/uniffi/clip_ffi"
mkdir -p "$AND_KT"
cp "$OUT/uniffi/clip_ffi/clip_ffi.kt" "$AND_KT/clip_ffi.kt"

echo "UniFFI bindings updated under apps/macos-shell/Generated and apps/android-shell."
