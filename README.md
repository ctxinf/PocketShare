<h1 align="center"><img src="web/icon.svg" width="36" height="36" alt=""> PocketShare</h1>
<p align="center">中文 · <a href="README.en.md">English</a></p>

面向个人自用的自托管多端分享工具，通过互联网分享文本和文件。

- Rust 开发：Linux amd64 release + strip 实测，服务端 **5.1 MB**，CLI **4.1 MB**；服务空闲 RSS 约 **7 MiB**，上传时随文件大小增加。
- 纯 HTTP C/S 架构，交互协议简单。
- 内嵌 Web 客户端，保持简单、兼容性强，支持 PWA。
- `pshare` CLI，终端 batteries included：管道、上传进度和重试。
- 无登录注册、无用户管理，仅 API Key 鉴权。

## 1. 区分

- [LocalSend](https://localsend.org/)：局域网共享、自动发现设备；本项目通过互联网访问部署好的 HTTP 服务。
- [PocketChest](https://github.com/Hzao/PocketChest)：serverless；本项目需要部署在自己的机器上。

## 2. 安装

### 1. Docker

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

### 2. 二进制/systemd

#### 1. 通过脚本安装

Linux / macOS，无需 Rust；安装服务端和 CLI 到 `~/.local/bin`：

```sh
curl -fsSL https://raw.githubusercontent.com/ctxinf/PocketShare/main/deploy/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
pocket-share-server
```

Linux 如需 systemd 后台运行，停止前台服务后执行：

```sh
bash ~/.local/bin/pocketshare-systemd/deploy.sh
```

可用 `VERSION=v0.1.0` 指定版本（下载脚本后执行 `VERSION=v0.1.0 sh install.sh`）。

#### 2. 直接下载安装

从 [Releases](https://github.com/ctxinf/PocketShare/releases/latest) 下载对应系统和架构的压缩包，解压后运行 `bin/pocket-share-server`（Windows 为 `.exe`）；CLI 为 `bin/pshare`。Linux 也可执行 `bash deploy.sh` 安装 systemd 用户服务。

首次运行自动生成配置与 API Key，位置见启动日志。修改配置后重启即可。systemd 细节见 [部署说明](deploy/systemd/package-usage.md)。

## 3. 使用

<details>
<summary>1. 打开网页，在设置中填入 API Key</summary>

服务地址默认为当前页面地址；各设备使用同一个服务和 API Key。

![连接设置](docs/screenshots/01-settings.png)

</details>

<details>
<summary>2. 输入文本或选择文件，点击「立即分享」</summary>

获得分享码；可在高级选项中设置有效期。

![分享文本](docs/screenshots/02-share.png)

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
