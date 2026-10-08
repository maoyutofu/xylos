# Xylos WebDAV Client

独立的 Dioxus Web 客户端。开发环境需要安装 Dioxus CLI：

```sh
cargo install dioxus-cli
dx serve --platform web
```

构建静态产物：

```sh
./build.sh
```

脚本会执行 `dx build --platform web --release`，然后将构建结果复制到 `dist/`。Xylos 主服务默认从工作目录下的 `app/dist/` 挂载 `/app/`。

当前版本支持 Basic、Bearer、`PROPFIND` 目录浏览、文件上传、目录创建、删除、重命名（`MOVE`）、复制（`COPY`）和目录上下级导航。文件下载直接请求目标 WebDAV 服务。跨域连接需要目标 WebDAV 服务或用户自己的 Nginx/Caddy 正确配置 CORS，主项目不会代理 WebDAV 请求。

重命名使用 WebDAV 标准 `MOVE` 方法：在列表中点击“选择”，输入新名称，再点击“重命名”。

Service Worker 会缓存应用静态资源，客户端也会缓存已经成功读取的目录元数据。离线状态下客户端只读，上传、删除、重命名、移动、复制和新建目录都会被禁用；文件内容需要在在线状态下从 WebDAV 服务读取。

在线写操作会使用已读取的 ETag 做条件请求，服务端返回 `412 Precondition Failed` 时会提示远端文件已发生变化，用户需要刷新后重新操作。
