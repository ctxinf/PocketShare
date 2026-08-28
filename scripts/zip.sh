#!/usr/bin/env bash
# Build myshare-server + share cli in release mode on this machine and
# package the binaries into a single zip that can be copied to another
# machine (same OS/arch) and deployed there with unzip-deploy.sh, without
# needing a rust toolchain on the target machine.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT="$SCRIPT_DIR/.."

VERSION=$(date +%Y%m%d-%H%M%S)
PKG_NAME="myshare-${VERSION}"
DIST_DIR="$REPO_ROOT/dist"
STAGE_DIR="$DIST_DIR/$PKG_NAME"

echo "==> building server (release)"
(cd "$REPO_ROOT/server" && cargo build --release)
echo "==> building cli (release)"
(cd "$REPO_ROOT/cli" && cargo build --release)

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/bin"
cp "$REPO_ROOT/server/target/release/myshare-server" "$STAGE_DIR/bin/"
cp "$REPO_ROOT/cli/target/release/share" "$STAGE_DIR/bin/"
cp "$SCRIPT_DIR/unzip-deploy.sh" "$STAGE_DIR/"
cp "$REPO_ROOT/DEPLOY.md" "$STAGE_DIR/"

mkdir -p "$DIST_DIR"
ZIP_FILE="$PKG_NAME.zip"
(cd "$DIST_DIR" && rm -f "$ZIP_FILE" && zip -qr "$ZIP_FILE" "$PKG_NAME")
rm -rf "$STAGE_DIR"

echo
echo "==> packaged: $DIST_DIR/$ZIP_FILE"
echo
echo "拷贝到目标机器(同 OS/架构)后运行:"
echo "  unzip $PKG_NAME.zip && cd $PKG_NAME && ./unzip-deploy.sh"
echo "或者只拷贝这一个 zip 文件, 配合仓库里的 scripts/unzip-deploy.sh:"
echo "  ./unzip-deploy.sh $PKG_NAME.zip"
