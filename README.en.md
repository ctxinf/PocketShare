<h1 align="center"><img src="web/icon.svg" width="36" height="36" alt=""> PocketShare</h1>
<p align="center"><a href="README.md">中文</a> · English</p>

## Outline

- [Installation](#1-installation)
  - [Docker](#1-docker)
  - [Binaries/systemd](#2-binariessystemd)
  - [Local installation](#23-local-installation)
  - [CLI installation](#3-cli-installation)
- [Usage](#2-usage)
- [Differences](#3-differences)

Self-hosted sharing for personal use. Send text and files between devices over the internet.

- Rust: server **5.1 MB**, CLI **4.1 MB**, idle server RSS approximately **7 MiB** (Linux amd64, stripped release); memory grows with uploads.
- Plain HTTP C/S architecture with a simple protocol.
- Simple embedded Web client with broad compatibility and PWA support.
- Batteries-included `pshare` CLI: pipes, upload progress and retries.
- API Key authentication only; no accounts or user management.

![PocketShare overview: settings, sharing, share codes, and terminal retrieval](docs/screenshots/overview-full.png)

## 1. Installation

### 1. Docker

Save as `compose.yml`:

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

Open `http://localhost:3000` and enter the API Key. Data persists in the volume. For internet access, place an HTTPS reverse proxy in front of `127.0.0.1:3000`. Installing the PWA requires HTTPS or localhost.

### 2. Binaries/systemd

#### 2.1 Install with a script

Linux / macOS; installs the server to `~/.local/bin`, without Rust:

```sh
curl -fsSL https://raw.githubusercontent.com/ctxinf/PocketShare/main/deploy/install-server.sh | sh
export PATH="$HOME/.local/bin:$PATH"
pocket-share-server
```

On Linux, stop the foreground server and install the systemd user service:

```sh
bash ~/.local/bin/pocketshare-systemd/deploy.sh
```

To pin a version, download the script and run `VERSION=v0.1.0 sh install-server.sh`.

#### 2.2 Download directly

Download your OS/architecture from [Releases](https://github.com/ctxinf/PocketShare/releases/latest), extract, and run `bin/pocket-share-server` (`.exe` on Windows). The CLI is `bin/pshare`. On Linux, `bash deploy.sh` installs a systemd user service.

The first run creates configuration and an API Key; startup logs show their locations. Restart after changing configuration.

#### 2.3 Local installation

Install the Rust toolchain and C/C++ build tools, then build the server from source:

```sh
git clone https://github.com/ctxinf/PocketShare.git
cd PocketShare
cargo build --release --locked --manifest-path server/Cargo.toml
mkdir -p ~/.local/bin
install -m 755 server/target/release/pocket-share-server ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
pocket-share-server
```

### 3. CLI installation

Install `pshare` separately on Linux / macOS, without the server or Rust:

```sh
curl -fsSL https://raw.githubusercontent.com/ctxinf/PocketShare/main/deploy/install-cli.sh | sh
export PATH="$HOME/.local/bin:$PATH"
pshare login <API_KEY> --server https://share.example.com
```

Alternatively, download `bin/pshare` (`bin/pshare.exe` on Windows) from [Releases](https://github.com/ctxinf/PocketShare/releases/latest), or build from the repository root:

```sh
cargo build --release --locked --manifest-path cli/Cargo.toml
install -m 755 cli/target/release/pshare ~/.local/bin/
```

## 2. Usage

<details>
<summary>1. Open Settings and enter the API Key</summary>

Use the same server and API Key on each device.

![1. Open Settings and enter the API Key](docs/screenshots/01-settings.png)

</details>

<details>
<summary>2. Enter text or select a file, then share</summary>

Copy the share code. Set an expiry in advanced options.

![2. Enter text or select a file, then share](docs/screenshots/02-share.png)

</details>

<details>
<summary>3. Enter the code on another device</summary>

Copy the text or download the file.

![3. Enter the code on another device](docs/screenshots/03-receive.png)

</details>

<details>
<summary>4. View, receive or delete shares from the list</summary>

Private shares are accessible by code and hidden from the list.

![4. View, receive or delete shares from the list](docs/screenshots/04-list.png)

</details>

<details>
<summary>5. Share and receive from the terminal</summary>

![List shares and retrieve a file from the terminal](docs/screenshots/05-cli.png)

```bash
pshare login <API_KEY> --server https://share.example.com
pshare put 'hello'
pshare put ./photo.png
cat notes.txt | pshare put
pshare get <CODE>
pshare get <CODE> --link
pshare list
pshare delete <CODE>
```

</details>

## 3. Differences

- [LocalSend](https://localsend.org/): LAN sharing and device discovery. PocketShare connects to your HTTP server over the internet.
- [PocketChest](https://github.com/Hzao/PocketChest): serverless. PocketShare runs on your own machine.
