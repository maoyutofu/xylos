# Xylos

[English](./README.md) | [中文](./README.zh-CN.md)

Xylos 是一个可配置的 Rust WebDAV 服务。

## 发布打包

推送 tag 后，GitHub Actions 会自动构建发布包。

- 触发方式：`git push origin <tag>`
- 支持平台：Linux、macOS Apple Silicon、macOS Intel、Windows
- 包版本号：直接使用 tag 名
- 输出位置：workflow artifacts 和 GitHub Release 附件，命名形式为
  `xylos-<tag>-<platform>`

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
