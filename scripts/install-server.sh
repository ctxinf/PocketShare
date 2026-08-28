#!/usr/bin/env bash
# Build the server in release mode, install the binary, and register it as a
# systemd --user service so it survives logins/reboots (with linger enabled).
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT="$SCRIPT_DIR/.."
SERVER_DIR="$REPO_ROOT/server"

BIN_NAME=share-server
INSTALL_BIN_DIR="${INSTALL_BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${SHARE_DATA_DIR:-$HOME/.local/share/share-server}"
LISTEN_ADDR="${SHARE_LISTEN_ADDR:-127.0.0.1:3000}"

echo "==> building server (release)"
(cd "$SERVER_DIR" && cargo build --release)

mkdir -p "$INSTALL_BIN_DIR"
cp "$SERVER_DIR/target/release/server" "$INSTALL_BIN_DIR/$BIN_NAME"
chmod +x "$INSTALL_BIN_DIR/$BIN_NAME"
echo "==> installed binary to $INSTALL_BIN_DIR/$BIN_NAME"

mkdir -p "$DATA_DIR"

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"
UNIT_FILE="$SYSTEMD_USER_DIR/share-server.service"

cat > "$UNIT_FILE" <<EOF
[Unit]
Description=share CLI-Web tool server
After=network.target

[Service]
ExecStart=$INSTALL_BIN_DIR/$BIN_NAME --listen-addr $LISTEN_ADDR --data-dir $DATA_DIR
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF

echo "==> wrote systemd user unit to $UNIT_FILE"

systemctl --user daemon-reload
systemctl --user enable --now share-server.service

echo "==> waiting for service to come up..."
sleep 1
systemctl --user status share-server.service --no-pager || true

API_KEY_FILE="$DATA_DIR/api_key"
if [ -f "$API_KEY_FILE" ]; then
  echo
  echo "api_key: $(cat "$API_KEY_FILE")"
  echo "api_key file: $API_KEY_FILE"
fi

echo
echo "listening on: $LISTEN_ADDR"
echo
echo "tips:"
echo "  loginctl enable-linger \$USER            # keep running after logout / across reboots"
echo "  journalctl --user -u share-server -f     # tail logs"
echo "  systemctl --user restart share-server    # restart"
echo "  systemctl --user stop share-server        # stop"
echo "  systemctl --user disable --now share-server  # uninstall the service"
