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

脚本需要 Node.js，会执行 `dx build --platform web --release`，将构建结果复制到 `dist/`，并按实际产物生成离线预缓存清单。Xylos 主服务默认从工作目录下的 `app/dist/` 挂载 `/app/`。

当前版本支持 Basic、Bearer、`PROPFIND` 目录浏览、文件上传、目录创建、删除、重命名（`MOVE`）、复制（`COPY`）和目录上下级导航。文件下载直接请求目标 WebDAV 服务。跨域连接需要目标 WebDAV 服务或用户自己的 Nginx/Caddy 正确配置 CORS，主项目不会代理 WebDAV 请求。

文件预览按浏览器原生能力分流：图片、PDF、音视频以及文本/代码文件可直接预览；Office 文档、压缩包、可执行文件和其他未知二进制格式会显示不支持提示，并提供下载入口。为避免一次把过大的文件读入内存，文本限制为 2 MiB、图片 20 MiB、PDF 40 MiB、音视频 64 MiB，超过限制时回退为下载。

重命名使用 WebDAV 标准 `MOVE` 方法：在列表中点击“选择”，输入新名称，再点击“重命名”。

Service Worker 安装时预缓存整个应用静态资源，应用页面和静态资源优先从缓存加载。打开过的目录会先显示本地元数据，再尝试从服务器更新；网络不可用时继续显示缓存。离线状态下客户端只读，上传、删除、重命名、移动、复制和新建目录都会被禁用；文件内容需要在在线状态下从 WebDAV 服务读取。

手机离线启动需要先通过受信任的 HTTPS 地址打开应用，等待 Service Worker 完成安装后再断网。局域网 HTTP 地址（例如 `http://192.168.x.x:8080/app/`）不是安全上下文，浏览器不会注册 Service Worker；`localhost` 仅在访问设备本机时例外。自签名证书也需先在设备上信任。

在线写操作会使用已读取的 ETag 做条件请求，服务端返回 `412 Precondition Failed` 时会提示远端文件已发生变化，用户需要刷新后重新操作。
