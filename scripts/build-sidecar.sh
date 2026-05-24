#!/bin/bash
# 构建 omniown sidecar 并复制到 src-tauri/binaries/（带 target-triple 后缀）
# 用法: ./scripts/build-sidecar.sh [--release]

set -euo pipefail

PROFILE="${1:-release}"
if [[ "$PROFILE" == "--release" ]]; then
    PROFILE="release"
fi

TRIPLE=$(rustc -vV | grep 'host:' | cut -d' ' -f2)
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building omniown ($PROFILE)..."
cargo build "--$PROFILE"

BIN="$ROOT/target/$PROFILE/omniown"
DEST="$ROOT/src-tauri/binaries/omniown-$TRIPLE"

echo "==> Copying $BIN -> $DEST"
cp "$BIN" "$DEST"

ls -lh "$DEST"
echo "==> Done: sidecar ready at binaries/omniown-$TRIPLE"
