#!/usr/bin/env bash
# chattui installer — run with:
#   curl -fsSL https://raw.githubusercontent.com/<you>/chattui/main/install.sh | bash
# or one-shot run without installing:
#   curl -fsSL .../install.sh | bash -s -- --run <nick> <host:port>
set -euo pipefail

REPO="${CHAT_REPO:-}"
DEST="${DEST:-$HOME/.local/bin}"
RUN_ARGS=""

if [ "${1:-}" = "--run" ]; then shift; RUN_ARGS="$*"; fi

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  asset="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64) asset="aarch64-unknown-linux-gnu" ;;
  Darwin-*)      asset="x86_64-apple-darwin" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

[ -n "$REPO" ] || { echo "Set CHAT_REPO=https://github.com/you/chattui and retry:"; echo "  curl -fsSL <raw-url>/install.sh | CHAT_REPO=https://github.com/you/chattui bash"; exit 1; }

mkdir -p "$DEST"
URL="$REPO/releases/latest/download/chattui-$asset"
echo ">> downloading chattui ($asset) from $URL"
curl -fSL "$URL" -o "$DEST/chattui"
chmod +x "$DEST/chattui"

if [ -n "$RUN_ARGS" ]; then
  exec "$DEST/chattui" $RUN_ARGS   # curl | bash -s -- --run alice host:7777
fi

case ":$PATH:" in *":$DEST:"*) ;; *) echo ">> note: add $DEST to your PATH (export PATH=\$PATH:$DEST)";; esac
echo ">> installed: $DEST/chattui"
echo ">> usage: chattui <nick> <host:port>   e.g. chattui alice chat.example.com:7777"
