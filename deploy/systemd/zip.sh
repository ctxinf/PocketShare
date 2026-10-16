#!/usr/bin/env bash
# Build pocket-share-server + pshare cli in release mode on this machine and
# package the binaries into a single zip that can be copied to another
# machine (same OS/arch) and deployed there with the bundled deploy.sh,
# without needing a rust toolchain on the target machine.
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Version comes from the server crate, so a package is traceable back to the
# source it was built from. Build metadata (timestamp / git commit) goes into
# the VERSION file rather than the name, keeping the name stable per release.
VERSION=$(grep -m1 '^version' "$REPO_ROOT/server/Cargo.toml" | sed -E 's/.*"(.*)".*/\1/')
if [ -z "$VERSION" ]; then
  echo "error: 无法从 server/Cargo.toml 解析版本号" >&2
  exit 1
fi
BUILD_TIME=$(date +%Y-%m-%dT%H:%M:%S%z)
GIT_COMMIT=$(cd "$REPO_ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)
if ! (cd "$REPO_ROOT" && git diff --quiet HEAD 2>/dev/null); then
  GIT_COMMIT="${GIT_COMMIT}-dirty"
fi

PKG_NAME="PocketShare-${VERSION}"
DIST_DIR="$REPO_ROOT/dist"
STAGE_DIR="$DIST_DIR/$PKG_NAME"

echo "==> building server (release)"
(cd "$REPO_ROOT/server" && cargo build --release --locked)
echo "==> building cli (release)"
(cd "$REPO_ROOT/cli" && cargo build --release --locked)

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/bin"
cp "$REPO_ROOT/server/target/release/pocket-share-server" "$STAGE_DIR/bin/"
cp "$REPO_ROOT/cli/target/release/pshare" "$STAGE_DIR/bin/"
# Ship the deploy script inside the package, so the target machine only needs
# to unzip and run it -- no separate script to copy over.
cp "$SCRIPT_DIR/deploy.sh" "$STAGE_DIR/deploy.sh"
chmod +x "$STAGE_DIR/deploy.sh"
cp "$SCRIPT_DIR/package-usage.md" "$STAGE_DIR/usage.md"

cat > "$STAGE_DIR/VERSION" <<EOF
version: $VERSION
build_time: $BUILD_TIME
git_commit: $GIT_COMMIT
build_host: $(uname -s)/$(uname -m)
EOF

# The unpacked package is a plausible place for someone to `git init` (e.g.
# tracking edits to deploy.sh); make sure the binaries and anything the server
# writes next to them can never be committed.
cat > "$STAGE_DIR/.gitignore" <<'EOF'
# 编译好的二进制,不要提交
bin/
*.zip

# 运行时数据 / 凭据,绝对不要提交
data/
api_key
*.sqlite
*.sqlite-journal
*.sqlite-wal
*.sqlite-shm
assets/
config.toml

# OS / 编辑器
.DS_Store
Thumbs.db
*.swp
EOF

mkdir -p "$DIST_DIR"
ZIP_FILE="$PKG_NAME.zip"
(cd "$DIST_DIR" && rm -f "$ZIP_FILE" && zip -qr "$ZIP_FILE" "$PKG_NAME")
rm -rf "$STAGE_DIR"

echo
echo "==> packaged: $DIST_DIR/$ZIP_FILE"
echo "    version: $VERSION ($GIT_COMMIT, $BUILD_TIME)"
echo "    (内含 bin/, deploy.sh, usage.md, VERSION, .gitignore)"
echo
echo "拷贝到目标机器(同 OS/架构)后, 解压即可直接部署:"
echo "  unzip $PKG_NAME.zip && cd $PKG_NAME && ./deploy.sh"
echo
echo "监听地址 / 数据目录 / 路径前缀等在配置文件里改, 改完重启服务:"
echo "  \$EDITOR ~/.config/myshare-server/config.toml"
echo "  systemctl --user restart pocket-share-server"
