#!/usr/bin/env bash
# Build the cli in release mode and install the `pshare` binary.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT="$SCRIPT_DIR/.."
CLI_DIR="$REPO_ROOT/cli"

INSTALL_BIN_DIR="${INSTALL_BIN_DIR:-$HOME/.local/bin}"

echo "==> building cli (release)"
(cd "$CLI_DIR" && cargo build --release)

mkdir -p "$INSTALL_BIN_DIR"
cp "$CLI_DIR/target/release/pshare" "$INSTALL_BIN_DIR/pshare"
chmod +x "$INSTALL_BIN_DIR/pshare"

echo "==> installed to $INSTALL_BIN_DIR/pshare"

case ":$PATH:" in
  *":$INSTALL_BIN_DIR:"*) ;;
  *)
    echo
    echo "NOTE: $INSTALL_BIN_DIR is not on your PATH. add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_BIN_DIR:\$PATH\""
    ;;
esac

echo
echo "next: pshare login <api_key> [--server <url>]"
echo "run 'pshare help' for the config file location and full usage"
