# Xylos

[English](./README.md) | [中文](./README.zh-CN.md)

Xylos is a configurable Rust WebDAV service.

## Release Packaging

GitHub Actions builds release packages automatically when you push a tag.

- trigger: `git push origin <tag>`
- platforms: Linux, macOS Apple Silicon, macOS Intel, Windows
- package version: uses the tag name directly
- outputs: workflow artifacts and GitHub Release assets
- each platform has two packages:
  - `xylos-<tag>-<platform>`: server binary and configuration files only
  - `xylos-<tag>-<platform>-with-app`: also includes the `app/dist` PWA files

When using a `-with-app` package, start `xylos` from the extracted directory to
serve the web client at `/app/`. The regular package does not include the web
client; build it separately and place it at `app/dist`.

The Web client requests server-generated WebP thumbnails on demand. By default,
the cache lives in `.xylos-cache` beside each user's WebDAV root, outside the
served files. Configure size limits, cache retention, and upload prewarming in
the optional `[thumbnails]` section of `config.full.toml`.

## Configuration

Start from `config.example.toml`.

Each user must define a filesystem root with `root_dir` under that user's
`[[users]]` block:

```toml
[server]
base_path = "/dav"

# Optional HTTPS.
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

Plaintext passwords are not accepted in configuration. Basic auth uses
`password_hash`, which must be an Argon2 PHC password hash. Digest auth uses
`digest_ha1`, which is the configured digest of `username:realm:password`
(`md5` by default, or `sha-256` when configured).

Generate `password_hash` and `digest_ha1` lines for `config.toml`:

```sh
cargo run -- hash-password
cargo run -- hash-password 'your-password'
cargo run -- hash-password --username admin --password 'your-password'
```

`PASSWORD` or `--password` expects the plaintext password. Xylos generates the Argon2
`password_hash` and matching `digest_ha1` for you.

All resolved paths are canonicalized and checked against the effective root
before filesystem operations. Each user is confined to their configured
`root_dir`.

## Run

```sh
cargo run -- --config config.example.toml
```

The example config exposes WebDAV at `/dav`.

You can also start without any config file by passing a minimal set of CLI
arguments:

```sh
cargo run -- \
  --root-dir /tmp/xylos-data/admin \
  --username admin \
  --password your-password
```

The quick start `--password` flag also expects plaintext and is converted
internally into the stored auth hashes.

Quick start defaults:

- host: `0.0.0.0`
- port: `8080`
- base path: `/dav`
- auth: Basic + Digest
- realm: `xylos`
- permissions: full read/write/delete/mkdir/move/copy

Override them with flags such as `--host`, `--port`, `--base-path`,
`--realm`, and `--log-level`.

To enable HTTPS, add `[server.tls]` with `enabled = true` and PEM certificate
and private key paths. Without that block, or with `enabled = false`, Xylos
serves plain HTTP.

For local development, generate a self-signed certificate with:

```sh
./generate-tls-cert.sh
```

For a PWA accessed from a phone by LAN IP, assign the server a fixed LAN IP, install mkcert, and issue a certificate for that IP:

```sh
./generate-mkcert-cert.sh 192.168.1.10
```

The script preserves existing certificates. Update the existing `[server.tls]` block in `config.toml` using the paths it prints (do not add a second block):

```toml
[server.tls]
enabled = true
cert_path = "certs/xylos-lan-192.168.1.10-cert.pem"
key_path = "certs/xylos-lan-192.168.1.10-key.pem"
```

Run `mkcert -CAROOT` to locate `rootCA.pem`. Install it on each phone; on iOS, enable full trust under Settings > General > About > Certificate Trust Settings. On Android, install it as a CA certificate in system settings. Never distribute `rootCA-key.pem`.

Restart Xylos and open `https://192.168.1.10:8081/app/` on the phone (adjust the port to your configuration). Confirm there is no certificate warning, then add this HTTPS page to the home screen. Keep the phone online for the first load before testing airplane mode. Remove any home-screen shortcut previously installed from HTTP.
