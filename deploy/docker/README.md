# Docker

在仓库根目录运行：

```sh
# 拉取 GHCR 镜像并启动
docker compose -f deploy/docker/compose.yaml up -d
# 或从源码构建
docker compose -f deploy/docker/compose.yaml up -d --build
# 查看 API Key
docker compose -f deploy/docker/compose.yaml exec pocketshare cat /data/api_key
# 日志 / 停止（保留数据）
docker compose -f deploy/docker/compose.yaml logs -f
docker compose -f deploy/docker/compose.yaml down
```

默认只发布到本机 `127.0.0.1:3000`，供本机访问或 HTTPS 反向代理使用。
容器以 UID/GID `10001:10001` 运行，命名 volume 自动初始化权限；改成 bind mount 时，需要让该用户可写数据目录。

Compose 默认将同目录的 `config.toml` 只读挂载到 `/etc/pocketshare/config.toml`。启动前按需编辑该文件，保持 `data_dir = "/data"` 以使用持久化 volume。完整配置示例见 [README](../../README.md#1-docker)。

修改后执行 `docker compose -f deploy/docker/compose.yaml up -d --force-recreate`。
`path_prefix` 可配置子路径；设置后 Dockerfile 内默认 `/health` 健康检查也需要加相同前缀。

GitHub Actions 仅在推送 `v*` 标签时发布对应版本和 `latest`。
当前镜像目标为 Linux amd64。首次发布后在 GitHub Packages 中确认包可见性为 Public，以允许匿名拉取。
