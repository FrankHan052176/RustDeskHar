#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source "$repo_root/scripts/ohos-env.sh"
"$repo_root/scripts/build-libvpx-ohos.sh"
"$repo_root/scripts/build-libaom-ohos.sh"
"$repo_root/scripts/build-libyuv-ohos.sh"
"$repo_root/scripts/build-libopus-ohos.sh"

clear_native_outputs() {
  local include_target="${1:-false}"
  local root
  local arch
  for root in "$repo_root/dist" "$repo_root/package/libs"; do
    [[ -d "$root" ]] || continue
    for arch in arm64-v8a armeabi-v7a x86 x86_64; do
      if [[ "$include_target" == "true" || "$arch" != "arm64-v8a" ]]; then
        rm -rf "$root/$arch"
      fi
    done
  done
}

if ! command -v ohrs >/dev/null 2>&1; then
  cat >&2 <<'EOF'
ohrs was not found in PATH.

Install it before building:
  cargo install ohrs
EOF
  exit 2
fi

native_lib="$repo_root/target/aarch64-unknown-linux-ohos/release/librustdesk_native_har.so"
dist_lib="$repo_root/dist/arm64-v8a/librustdesk_native_har.so"
cxx_runtime="$OHOS_NDK_HOME/native/llvm/lib/aarch64-linux-ohos/libc++_shared.so"
dist_cxx_runtime="$repo_root/dist/arm64-v8a/libc++_shared.so"
type_definition="$repo_root/types/index.d.ts"
dist_type_definition="$repo_root/dist/index.d.ts"

clear_native_outputs true

if [[ ! -f "$type_definition" ]]; then
  echo "Missing NAPI type declaration: $type_definition" >&2
  exit 1
fi

if [[ ! -f "$cxx_runtime" ]]; then
  echo "Missing OHOS arm64 C++ runtime: $cxx_runtime" >&2
  exit 1
fi

echo "Building RustDesk native HAR for arm64-v8a..."
cargo build --target aarch64-unknown-linux-ohos --release --locked

if [[ ! -f "$native_lib" ]]; then
  echo "ohrs build did not produce $native_lib." >&2
  exit 1
fi

mkdir -p "$(dirname "$dist_lib")"
cp "$native_lib" "$dist_lib"
cp "$cxx_runtime" "$dist_cxx_runtime"
cp "$type_definition" "$dist_type_definition"

echo "Packaging HAR..."
clear_native_outputs false
ohrs artifact

echo "Done: $repo_root/package.har"
