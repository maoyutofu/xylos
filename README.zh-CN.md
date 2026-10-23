# Xylos

[English](./README.md) | [中文](./README.zh-CN.md)

Xylos 是一个可配置的 Rust WebDAV 服务。

## 发布打包

推送 tag 后，GitHub Actions 会自动构建发布包。

- 触发方式：`git push origin <tag>`
- 支持平台：Linux、macOS Apple Silicon、macOS Intel、Windows
- 包版本号：直接使用 tag 名
- 输出位置：workflow artifacts 和 GitHub Release 附件
- 每个平台提供两种包：
  - `xylos-<tag>-<platform>`：仅包含服务端程序和配置文件
  - `xylos-<tag>-<platform>-with-app`：额外包含 `app/dist` PWA 静态文件

使用 `-with-app` 包时，请在解压后的目录中启动 `xylos`，即可访问 `/app/`
页面。普通包不包含 Web 客户端，需要自行构建并放置 `app/dist`。

## 配置

从 `config.example.toml` 开始。

每个用户都必须在自己的 `[[users]]` 配置块下定义一个文件系统根目录
`root_dir`：

```toml
[server]
base_path = "/dav"

# 可选 HTTPS。
# [server.tls]
# enabled = true
# cert_path = "/path/to/cert.pem"
# key_path = "/path/to/key.pem"

[auth]
realm = "xylos"
digest_algorithm = "md5"

[[users]]
username = "admin"
root_dir = "/tmp/xylos-data/admin"
password_hash = "$argon2id$v=19$..."
digest_ha1 = "<md5-of-admin:xylos:password>"
permissions = ["read", "write", "delete", "mkdir", "move", "copy"]

[[users]]
username = "guest"
root_dir = "/tmp/xylos-data/guest"
password_hash = "$argon2id$v=19$..."
digest_ha1 = "<md5-of-guest:xylos:password>"
permissions = ["read"]
```

配置中不接受明文密码。Basic 认证使用 `password_hash`，它必须是 Argon2
PHC 格式的密码哈希。Digest 认证使用 `digest_ha1`，它是配置中的
`username:realm:password` 摘要值（默认使用 `md5`，如果配置了则可使用
`sha-256`）。

为 `config.toml` 生成 `password_hash` 和 `digest_ha1`：

```sh
cargo run -- hash-password
cargo run -- hash-password 'your-password'
cargo run -- hash-password --username admin --password 'your-password'
```

`PASSWORD` 或 `--password` 需要传入明文密码。Xylos 会帮你生成 Argon2 `password_hash`
以及对应的 `digest_ha1`。

所有解析后的路径在执行文件系统操作前都会先规范化，并检查是否位于生效的
根目录之下。每个用户都会被限制在各自配置的 `root_dir` 中。

## 运行

```sh
cargo run -- --config config.example.toml
```

示例配置会将 WebDAV 暴露在 `/dav`。

Web 客户端会按需请求服务端生成的 WebP 缩略图。默认缓存放在用户根目录旁边的
`.xylos-cache` 中，不会出现在 WebDAV 文件列表里。可通过 `[thumbnails]` 配置
`enabled`、`cache_dir`、`width`、`max_source_bytes`、`max_pixels`、
`max_cache_bytes`、`ttl_days` 和 `prewarm_on_upload`；完整示例见
`config.full.toml`。关闭后，客户端会回退到直接读取受支持的原图。

你也可以不提供配置文件，只通过一组最小 CLI 参数启动：

```sh
cargo run -- \
  --root-dir /tmp/xylos-data/admin \
  --username admin \
  --password your-password
```

快速启动中的 `--password` 同样要求传入明文密码，程序会在内部将其转换成
存储所需的认证哈希。

快速启动默认值：

- host: `0.0.0.0`
- port: `8080`
- base path: `/dav`
- auth: Basic + Digest
- realm: `xylos`
- permissions: 完整的 read/write/delete/mkdir/move/copy

可以通过 `--host`、`--port`、`--base-path`、`--realm`、`--log-level`
等参数覆盖默认值。

如需启用 HTTPS，请添加 `[server.tls]`，设置 `enabled = true`，并提供
PEM 证书和私钥路径。没有该配置块，或 `enabled = false` 时，Xylos 将使用
纯 HTTP 提供服务。

本地开发时，可以用下面的命令生成自签名证书：

```sh
./generate-tls-cert.sh
```

若手机通过局域网 IP 访问 PWA，先给服务器分配固定局域网 IP，再用 mkcert 为该 IP 签发证书。将示例 IP 换成服务器的实际地址：

```sh
brew install mkcert # macOS；其他系统请先安装 mkcert
./generate-mkcert-cert.sh 192.168.1.10
```

脚本不会覆盖现有证书。将输出的路径写入 `config.toml` 中已有的 `[server.tls]` 配置块（不要重复添加配置块），例如：

```toml
[server.tls]
enabled = true
cert_path = "certs/xylos-lan-192.168.1.10-cert.pem"
key_path = "certs/xylos-lan-192.168.1.10-key.pem"
```

运行 `mkcert -CAROOT` 可查看本机 CA 目录。将其中的 `rootCA.pem` 安装到每台需要访问的手机：iPhone/iPad 安装描述文件后，还需到“设置 → 通用 → 关于本机 → 证书信任设置”开启完全信任；Android 在系统设置中安装 CA 证书。不要分发 `rootCA-key.pem`。

重启 Xylos 后，手机通过 `https://192.168.1.10:8081/app/` 访问（端口以实际配置为准），确认浏览器不再显示证书警告。用此 HTTPS 地址重新添加到主屏幕，保持联网完成首次加载，再测试飞行模式。原来从 HTTP 地址添加的主屏幕入口需要删除。
