#!/usr/bin/env bash
# Deploy a package built by zip.sh: installs the myshare-server binary as a
# systemd --user service and the share cli binary. Does NOT need a rust
# toolchain (binaries are already built).
#
# This file is what zip.sh ships inside the package as `deploy.sh`, so it runs
# on the VPS, not here. Edit it here; it reaches the server via the next zip.
#
# Two ways to run it:
#   1. Inside an already-unzipped package dir:  ./deploy.sh
#   2. Against a zip file from anywhere:        ./deploy.sh myshare-<version>.zip
#
# All runtime settings (listen address, data dir, path prefix, limits) live in
# the config file, not in this script. The server creates it with documented
# defaults on first start; edit it and restart the service to apply changes.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

INSTALL_BIN_DIR="${INSTALL_BIN_DIR:-$HOME/.local/bin}"
CONFIG_FILE="${MYSHARE_CONFIG:-${XDG_CONFIG_HOME:-$HOME/.config}/myshare-server/config.toml}"

ZIP_FILE="${1:-}"

if [ -z "$ZIP_FILE" ] && [ -f "$SCRIPT_DIR/bin/myshare-server" ]; then
  # Case 1: we're sitting inside an unzipped package, deploy in place.
  PKG_DIR="$SCRIPT_DIR"
else
  # Case 2: locate and unzip a package file.
  if [ -z "$ZIP_FILE" ]; then
    ZIP_FILE=$(ls ./myshare-*.zip 2>/dev/null | head -n1 || true)
  fi
  if [ -z "$ZIP_FILE" ] || [ ! -f "$ZIP_FILE" ]; then
    echo "usage: $0 [myshare-*.zip]" >&2
    echo "  (或者在解压后的包目录里直接运行 ./deploy.sh)" >&2
    exit 1
  fi

  WORK_DIR=$(mktemp -d)
  trap 'rm -rf "$WORK_DIR"' EXIT

  echo "==> unzip $ZIP_FILE"
  unzip -q "$ZIP_FILE" -d "$WORK_DIR"
  PKG_DIR=$(find "$WORK_DIR" -mindepth 1 -maxdepth 1 -type d | head -n1)
  if [ -z "$PKG_DIR" ] || [ ! -f "$PKG_DIR/bin/myshare-server" ]; then
    echo "error: $ZIP_FILE 里没找到 bin/myshare-server, 是不是不是 zip.sh 打的包?" >&2
    exit 1
  fi
fi

# When re-deploying over an existing install, the old server is still running
# and holding the binary + sqlite db open. Stop it before overwriting, so an
# upgrade is a clean stop -> replace -> start rather than a live swap.
if systemctl --user list-unit-files myshare-server.service >/dev/null 2>&1 &&
   systemctl --user is-active --quiet myshare-server.service; then
  echo "==> stopping running myshare-server before update"
  systemctl --user stop myshare-server.service
fi

mkdir -p "$INSTALL_BIN_DIR"
cp "$PKG_DIR/bin/myshare-server" "$INSTALL_BIN_DIR/myshare-server"
cp "$PKG_DIR/bin/share" "$INSTALL_BIN_DIR/share"
chmod +x "$INSTALL_BIN_DIR/myshare-server" "$INSTALL_BIN_DIR/share"
echo "==> installed binaries to $INSTALL_BIN_DIR"

mkdir -p "$(dirname "$CONFIG_FILE")"

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"
UNIT_FILE="$SYSTEMD_USER_DIR/myshare-server.service"

cat > "$UNIT_FILE" <<EOF
[Unit]
Description=myshare CLI-Web tool server
After=network.target

[Service]
ExecStart=$INSTALL_BIN_DIR/myshare-server --config $CONFIG_FILE
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

echo
echo "config file: $CONFIG_FILE"
echo "  监听地址 / 数据目录 / 路径前缀等都在这个文件里改, 改完执行:"
echo "  systemctl --user restart myshare-server"

# Read back what the server actually resolved, so the printed api_key path and
# URL match a config the user may already have customized.
read_conf() {
  sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*\"\(.*\)\".*/\1/p" "$CONFIG_FILE" 2>/dev/null | tail -n1
}
DATA_DIR=$(read_conf data_dir)
DATA_DIR="${DATA_DIR:-$HOME/.local/share/myshare-server}"
LISTEN_ADDR=$(read_conf listen_addr)
LISTEN_ADDR="${LISTEN_ADDR:-127.0.0.1:3000}"
PATH_PREFIX=$(read_conf path_prefix)

API_KEY_FILE="$DATA_DIR/api_key"
if [ -f "$API_KEY_FILE" ]; then
  echo
  echo "api_key: $(cat "$API_KEY_FILE")"
  echo "api_key file: $API_KEY_FILE"
fi

echo
echo "listening on: ${LISTEN_ADDR}${PATH_PREFIX}"

case ":$PATH:" in
  *":$INSTALL_BIN_DIR:"*) ;;
  *)
    echo
    echo "NOTE: $INSTALL_BIN_DIR is not on your PATH. add this to your shell profile:"
    echo "  export PATH=\"$INSTALL_BIN_DIR:\$PATH\""
    ;;
esac

echo
echo "next: share login <api_key> --server http://<this-host>:${LISTEN_ADDR##*:}${PATH_PREFIX}"
echo
echo "更多用法见同目录下的 usage.md"
