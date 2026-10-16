<h1 align="center"><img src="web/icon.svg" width="26" height="26" alt=""> PocketShare</h1>
<p align="center">中文 · <a href="README.en.md">English</a></p>

<p align="center"> 面向个人自用的自托管多端分享工具，用于分享文本和文件。</p>

![PocketShare 操作概览：设置、分享、分享码与终端获取](docs/screenshots/overview-full.png)

- 纯 HTTP 协议，简单 C/S 架构，交互简单、兼容性强。
- Web 客户端：简单、多端适用，适配移动端，支持 PWA。
- CLI 客户端：方便终端操作，也可接入 Agent。
- Rust 开发，资源占用低：服务端 **5.1 MB**，CLI **4.1 MB**；服务空闲内存占用约 **7 MiB**。
- 使用简单：无登录注册、无用户管理，仅 API Key 鉴权。


## 目录

- [1. 安装](#1-安装)
  - [1. Docker](#1-docker)
  - [2. 二进制/systemd](#2-二进制systemd)
    - [2.1 通过脚本安装](#21-通过脚本安装)
    - [2.2 直接下载安装](#22-直接下载安装)
    - [2.3 本地安装](#23-本地安装)
  - [3. CLI 安装](#3-cli-安装)
- [2. 使用](#2-使用)
- [3. 对比](#3-对比)

## 1. 安装

<details id="1-docker">
<summary>1. Docker</summary>

保存为 `compose.yml`：

```yaml
services:
  pocketshare:
    image: ghcr.io/ctxinf/pocketshare:latest
    ports:
      - "127.0.0.1:3000:3000"
    volumes:
      - pocketshare-data:/data
    restart: unless-stopped
volumes:
  pocketshare-data:
```

```sh
docker compose up -d
docker compose exec pocketshare cat /data/api_key
```

打开 `http://localhost:3000`，填入 API Key。数据保存在 volume 中。公网访问请配置 HTTPS 反向代理至 `127.0.0.1:3000`；PWA 安装需要 HTTPS 或 localhost。

</details>

<details id="2-二进制systemd">
<summary>2. 二进制/systemd</summary>

服务端程序名为 `pocket-share-server`；CLI 程序名为 `pshare`，采用缩写方便手动输入。

<details id="21-通过脚本安装">
<summary>2.1 通过脚本安装</summary>

Linux / macOS，无需 Rust；安装服务端到 `~/.local/bin`：

```sh
curl -fsSL https://raw.githubusercontent.com/ctxinf/PocketShare/main/deploy/install-server.sh | sh
export PATH="$HOME/.local/bin:$PATH"
pocket-share-server
```

Linux 如需 systemd 后台运行，停止前台服务后执行：

```sh
bash ~/.local/bin/pocketshare-systemd/deploy.sh
```

可用 `VERSION=v0.1.0` 指定版本（下载脚本后执行 `VERSION=v0.1.0 sh install-server.sh`）。

</details>

<details id="22-直接下载安装">
<summary>2.2 直接下载安装</summary>

从 [Releases](https://github.com/ctxinf/PocketShare/releases/latest) 下载对应系统和架构的压缩包，解压后运行 `bin/pocket-share-server`（Windows 为 `.exe`）；CLI 为 `bin/pshare`。Linux 也可执行 `bash deploy.sh` 安装 systemd 用户服务。

首次运行自动生成配置与 API Key，位置见启动日志。修改配置后重启即可。systemd 细节见 [部署说明](deploy/systemd/package-usage.md)。

</details>

<details id="23-本地安装">
<summary>2.3 本地安装</summary>

安装 Rust 工具链和 C/C++ 编译工具后，从源码构建服务端：

```sh
git clone https://github.com/ctxinf/PocketShare.git
cd PocketShare
cargo build --release --locked --manifest-path server/Cargo.toml
mkdir -p ~/.local/bin
install -m 755 server/target/release/pocket-share-server ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
pocket-share-server
```

</details>

</details>

<details id="3-cli-安装">
<summary>3. CLI 安装</summary>

Linux / macOS 可单独安装 `pshare`，无需安装服务端或 Rust：

```sh
curl -fsSL https://raw.githubusercontent.com/ctxinf/PocketShare/main/deploy/install-cli.sh | sh
export PATH="$HOME/.local/bin:$PATH"
pshare login <API_KEY> --server https://share.example.com
```

也可从 [Releases](https://github.com/ctxinf/PocketShare/releases/latest) 下载并提取 `bin/pshare`（Windows 为 `bin/pshare.exe`），或在仓库根目录从源码构建：

```sh
cargo build --release --locked --manifest-path cli/Cargo.toml
install -m 755 cli/target/release/pshare ~/.local/bin/
```

</details>

## 2. 使用

<details>
<summary>1. 打开网页，在设置中填入 API Key</summary>

服务地址默认为当前页面地址；各设备使用同一个服务和 API Key。

![连接设置](docs/screenshots/01-settings.png)

</details>

<details>
<summary>2. 输入文本或选择文件，点击「立即分享」</summary>

获得分享码；可在高级选项中设置有效期。

![分享文件](docs/screenshots/02-share.png)

</details>

<details>
<summary>3. 在另一台设备输入分享码，接收内容</summary>

文本可复制，文件可下载。

![接收内容](docs/screenshots/03-receive.png)

</details>

<details>
<summary>4. 在列表中查看、接收或删除分享</summary>

私密分享不会出现在列表中，仍可通过分享码接收。

![分享列表](docs/screenshots/04-list.png)

</details>

<details>
<summary>5. 在终端中也可以进行分享和获取</summary>

![终端查看列表并获取文件](docs/screenshots/05-cli.png)

```bash
pshare login <API_KEY> --server https://share.example.com
pshare put 'hello'
pshare put ./photo.png
cat notes.txt | pshare put
pshare get <分享码>
pshare get <分享码> --link
pshare list
pshare delete <分享码>
```

</details>

## 3. 对比

| 对比项目 | 特点 | PocketShare |
| --- | --- | --- |
| 微信文件助手 | 云端服务 | 自托管替代品 |
| [LocalSend](https://localsend.org/) | 局域网共享、自动发现 | 通过互联网访问 HTTP 服务 |
| [PocketChest](https://github.com/Hzao/PocketChest) | Serverless | 部署在自己的机器上 |
