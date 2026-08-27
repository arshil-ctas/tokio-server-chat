#!/usr/bin/env bash
# chattui one-liner launcher:
#
#   curl -fsSL https://raw.githubusercontent.com/arshil-ctas/tokio-server-chat/main/try.sh | bash -s -- <nick>
set -euo pipefail

REPO="${CHAT_REPO:-https://github.com/arshil-ctas/tokio-server-chat}"
HOST="${CHAT_HOST:-tokio-server-chat.onrender.com}"
NICK="${1:-$(whoami 2>/dev/null || echo anon)}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  asset="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64) asset="aarch64-unknown-linux-gnu" ;;
  Darwin-*)      asset="x86_64-apple-darwin" ;;
  *) asset="" ;;
esac

BIN="$(mktemp -d)/chattui"
URL="$REPO/releases/latest/download/chattui-$asset"

fetch_binary() {
  if [ -n "$asset" ] && curl -fsSL "$URL" -o "$BIN" 2>/dev/null; then
    chmod +x "$BIN"; return 0
  fi
  if command -v cargo >/dev/null 2>&1; then
    echo ">> no prebuilt binary for $(uname -s)-$(uname -m), building from source..."
    local dir; dir="$(mktemp -d)"
    git clone --depth 1 "$REPO" "$dir/src" >/dev/null 2>&1
    (cd "$dir/src" && cargo build --release -p chattui)
    cp "$dir/src/target/release/chattui" "$BIN"; return 0
  fi
  echo "!! could not download binary and cargo not found for source build" >&2
  exit 1
}

echo ">> chattui: nick=$NICK  server=$HOST"
fetch_binary
exec "$BIN" "$NICK" "$HOST"
