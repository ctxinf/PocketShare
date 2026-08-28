#!/usr/bin/env bash
# Unzip a package built by zip.sh and deploy it on this machine: installs
# the myshare-server binary as a systemd --user service and the share cli
# binary. Does NOT need a rust toolchain (binaries are already built).
set -euo pipefail

ZIP_FILE="${1:-}"
if [ -z "$ZIP_FILE" ]; then
  ZIP_FILE=$(ls ./myshare-*.zip 2>/dev/null | head -n1 || true)
fi
if [ -z "$ZIP_FILE" ] || [ ! -f "$ZIP_FILE" ]; then
  echo "usage: $0 <myshare-*.zip>" >&2
  exit 1
fi

INSTALL_BIN_DIR="${INSTALL_BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${MYSHARE_DATA_DIR:-$HOME/.local/share/myshare-server}"
LISTEN_ADDR="${MYSHARE_LISTEN_ADDR:-127.0.0.1:3000}"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

echo "==> unzip $ZIP_FILE"
unzip -q "$ZIP_FILE" -d "$WORK_DIR"
PKG_DIR=$(find "$WORK_DIR" -mindepth 1 -maxdepth 1 -type d | head -n1)
if [ -z "$PKG_DIR" ] || [ ! -f "$PKG_DIR/bin/myshare-server" ]; then
  echo "error: $ZIP_FILE 里没找到 bin/myshare-server, 是不是不是 zip.sh 打的包?" >&2
  exit 1
fi

mkdir -p "$INSTALL_BIN_DIR"
cp "$PKG_DIR/bin/myshare-server" "$INSTALL_BIN_DIR/myshare-server"
cp "$PKG_DIR/bin/share" "$INSTALL_BIN_DIR/share"
chmod +x "$INSTALL_BIN_DIR/myshare-server" "$INSTALL_BIN_DIR/share"
echo "==> installed binaries to $INSTALL_BIN_DIR"

mkdir -p "$DATA_DIR"

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"
UNIT_FILE="$SYSTEMD_USER_DIR/myshare-server.service"

cat > "$UNIT_FILE" <<EOF
[Unit]
Description=myshare CLI-Web tool server
After=network.target

[Service]
ExecStart=$INSTALL_BIN_DIR/myshare-server --listen-addr $LISTEN_ADDR --data-dir $DATA_DIR
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF

echo "==> wrote systemd user unit to $UNIT_FILE"

systemctl --user daemon-reload
systemctl --user enable --now myshare-server.service

echo "==> waiting for service to come up..."
sleep 1
systemctl --user status myshare-server.service --no-pager || true

API_KEY_FILE="$DATA_DIR/api_key"
if [ -f "$API_KEY_FILE" ]; then
  echo
  echo "api_key: $(cat "$API_KEY_FILE")"
  echo "api_key file: $API_KEY_FILE"
fi

echo
echo "listening on: $LISTEN_ADDR"

case ":$PATH:" in
  *":$INSTALL_BIN_DIR:"*) ;;
  *)
    echo
    echo "NOTE: $INSTALL_BIN_DIR is not on your PATH. add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_BIN_DIR:\$PATH\""
    ;;
esac

echo
echo "next: share login <api_key> --server http://<this-host>:${LISTEN_ADDR##*:}"
echo
echo "如需暴露到局域网/公网, 参考 DEPLOY.md 里 MYSHARE_LISTEN_ADDR 的说明"
