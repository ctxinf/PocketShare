# myshare 使用说明

这个包里已经包含编译好的二进制,目标机器**不需要装 rust 工具链**。

包内容:

```
bin/myshare-server   # 服务端
bin/share            # 命令行客户端
deploy.sh            # 一键部署脚本
usage.md             # 本文件
VERSION              # 版本号 + 构建时间 + git commit
.gitignore           # 防止二进制/凭据被误提交
```

包的版本号见 `VERSION` 文件,也体现在包名 `myshare-<版本号>.zip` 上。反馈问题时请带上
`VERSION` 的内容。

包里预置了 `.gitignore`:如果你在这个目录里 `git init`(比如想版本化自己改过的
`deploy.sh`),二进制、`api_key`、sqlite 数据文件都不会被误提交。

## 一、部署

解压后进到目录直接跑:

```bash
unzip myshare-<版本号>.zip
cd myshare-<版本号>
./deploy.sh
```

脚本会做:

1. 把 `myshare-server` 和 `share` 装到 `~/.local/bin/`
2. 写 systemd `--user` unit 并 `enable --now`
3. server 首次启动时生成配置文件 `~/.config/myshare-server/config.toml`(带注释的默认值)
4. 按配置建数据目录(默认 `~/.local/share/myshare-server`,存 sqlite db + 文件 + api_key)
5. 打印出配置文件路径和 api_key(登录 CLI / Web 都要用)

也可以不解压,直接对着 zip 跑:

```bash
./deploy.sh myshare-<版本号>.zip
```

### 让服务在登出/重启后依然运行

```bash
loginctl enable-linger $USER
```

## 二、配置

所有运行时配置都在一个 TOML 文件里,默认路径:

```
~/.config/myshare-server/config.toml
```

(遵循 `XDG_CONFIG_HOME`;首次启动 server 时自动生成,带完整注释。)

**改配置的流程:编辑这个文件 → 重启服务。**

```bash
$EDITOR ~/.config/myshare-server/config.toml
systemctl --user restart myshare-server
```

配置项:

```toml
# 监听地址。默认只监听本机;要让局域网/公网访问,改成 "0.0.0.0:3000",
# 并务必配合防火墙或反向代理 + HTTPS。
listen_addr = "127.0.0.1:3000"

# 数据目录:sqlite 数据库、上传的文件、api_key 都放在这里。
data_dir = "/home/<user>/.local/share/myshare-server"

# 路径前缀。留空表示挂在域名根路径下,详见下一节。
path_prefix = ""

# 单个文件大小上限(字节)。默认 100 MiB。
max_file_bytes = 104857600

# 分享的默认有效期(秒)。默认 1 小时。
default_ttl_secs = 3600

# 过期数据的清理间隔(秒)。
cleanup_interval_secs = 60
```

想把配置文件放别处,改 systemd unit 里的 `--config` 参数即可:

```bash
$EDITOR ~/.config/systemd/user/myshare-server.service   # ExecStart=... --config /path/to/config.toml
systemctl --user daemon-reload && systemctl --user restart myshare-server
```

部署时也可以用 `MYSHARE_CONFIG=/path/to/config.toml ./deploy.sh` 直接指定,
`deploy.sh` 会把这个路径写进 unit 文件。

## 三、部署到共有域名的子路径下

如果这台机器上已经有别的服务占着域名根路径,可以把 myshare 挂到一个子路径上,比如
`https://example.com/myshare/`。在配置文件里设:

```toml
path_prefix = "/myshare"
```

然后 `systemctl --user restart myshare-server`。

这时所有接口都在前缀下面:`/myshare/health`、`/myshare/put`、`/myshare/get/<key>`、
`/myshare/dl/<key>` 等等,Web 页面在 `https://example.com/myshare/`。

前面挂 Nginx 反代时**不要**再 strip 前缀,直接原样透传即可:

```nginx
location /myshare/ {
    proxy_pass http://127.0.0.1:3000;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
    client_max_body_size 100m;
}
```

Caddy:

```
example.com {
    handle /myshare/* {
        reverse_proxy 127.0.0.1:3000
    }
}
```

Web 页面会自动感知自己所在的前缀,不需要手工填服务器地址。CLI 端登录时把前缀带上即可:

```bash
share login <api_key> --server https://example.com/myshare
```

## 四、日常使用

### CLI

```bash
share login <api_key> --server http://127.0.0.1:3000
share put "hello"          # -> 打印 share_key
share put ./some-file.zip  # 传文件
share get <share_key>
share list
share delete <share_key>
share help                 # 会打印配置文件所在路径
```

CLI 配置(server_url + api_key)保存在 `~/.config/share-cli/config.json`
(或 `$XDG_CONFIG_HOME/share-cli/config.json`)。

如果 `~/.local/bin` 不在 `PATH` 里,在 `~/.bashrc` / `~/.zshrc` 里加:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Web

Web 页面已经编译进 server 二进制,启动后浏览器直接打开服务地址即可
(根路径部署是 `http://<host>:3000/`,带前缀就是 `http://<host>:3000/myshare/`)。
首次打开会要求填 api_key。

## 五、运维命令

```bash
systemctl --user status myshare-server        # 查看状态
journalctl --user -u myshare-server -f        # 查看日志
systemctl --user restart myshare-server       # 重启(改完配置后)
systemctl --user stop myshare-server          # 停止
systemctl --user disable --now myshare-server # 卸载服务(不删二进制和数据)
```

配置写错了(比如 TOML 语法错误、字段名拼错)服务会启动失败并退出,
`journalctl --user -u myshare-server -n 20` 里能看到具体是哪一行的问题。

## 六、卸载

```bash
systemctl --user disable --now myshare-server
rm ~/.config/systemd/user/myshare-server.service
rm ~/.local/bin/myshare-server ~/.local/bin/share
rm ~/.config/myshare-server/config.toml
rm -rf ~/.local/share/myshare-server   # 会删除所有已分享的数据,谨慎操作
```

## 七、安全提醒

默认只监听 `127.0.0.1`,只有本机能访问。要暴露到局域网/公网,请务必配合防火墙限制来源 IP,
或在前面加一层反向代理 + HTTPS。鉴权只是简单的固定 `api_key`,加密是自定义混淆而非标准
加密强度 —— 这是仅供个人使用的工具,不建议未加防护直接裸奔在公网上。
