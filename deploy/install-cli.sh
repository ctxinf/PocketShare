#!/bin/sh
set -eu
REPO=ctxinf/PocketShare
VERSION=${VERSION:-latest}
INSTALL_BIN_DIR=${INSTALL_BIN_DIR:-"$HOME/.local/bin"}
case $(uname -s) in
  Linux) platform=unknown-linux-gnu ;;
  Darwin) platform=apple-darwin ;;
  *) echo "Use the Windows zip from GitHub Releases." >&2; exit 1 ;;
esac
case $(uname -m) in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) echo "Unsupported architecture" >&2; exit 1 ;;
esac
asset="PocketShare-$arch-$platform.tar.gz"
if [ "$VERSION" = latest ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  base="https://github.com/$REPO/releases/download/$VERSION"
fi
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM
curl -fLsS "$base/$asset" -o "$work_dir/$asset"
curl -fLsS "$base/$asset.sha256" -o "$work_dir/$asset.sha256"
(cd "$work_dir"; if command -v sha256sum >/dev/null 2>&1; then sha256sum -c "$asset.sha256"; else shasum -a 256 -c "$asset.sha256"; fi)
tar -xzf "$work_dir/$asset" -C "$work_dir"
mkdir -p "$INSTALL_BIN_DIR"
install -m 755 "$work_dir/bin/pshare" "$INSTALL_BIN_DIR/.pshare.new"
mv -f "$INSTALL_BIN_DIR/.pshare.new" "$INSTALL_BIN_DIR/pshare"
printf 'Installed pshare to %s. Add it to PATH.\nRun pshare login <API_KEY> --server <URL> to connect.\n' "$INSTALL_BIN_DIR"
