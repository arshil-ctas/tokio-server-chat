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

ensure_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi
  echo ">> cargo not found, bootstrapping Rust via rustup..."
  local rustup_init; rustup_init="$(mktemp)"
  if ! curl -fsSL https://sh.rustup.rs -o "$rustup_init"; then
    echo "!! could not download rustup installer; install Rust manually: https://rustup.rs" >&2
    return 1
  fi
  sh "$rustup_init" -y --profile minimal 1>/dev/null 2>&1
  rm -f "$rustup_init"
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env" || true
  command -v cargo >/dev/null 2>&1
}

fetch_binary() {
  if [ -n "$asset" ] && curl -fsSL "$URL" -o "$BIN" 2>/dev/null; then
    chmod +x "$BIN"; return 0
  fi
  echo ">> no prebuilt binary for $(uname -s)-$(uname -m) or download failed; falling back to source build... (installs Rust if missing)"
  if ! ensure_cargo; then
    echo "!! could not download binary and could not install Rust for source build" >&2
    exit 1
  fi
  local dir; dir="$(mktemp -d)"
  git clone --depth 1 "$REPO" "$dir/src" >/dev/null 2>&1
  (cd "$dir/src" && cargo build --release -p chattui)
  cp "$dir/src/target/release/chattui" "$BIN"; return 0
}

echo ">> chattui: nick=$NICK  server=$HOST"
fetch_binary
exec "$BIN" "$NICK" "$HOST"
