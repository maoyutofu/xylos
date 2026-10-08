# WebDAV Web 客户端方案

## 目标

在 `app/` 目录下实现一个基于 Dioxus 的 WebDAV 通用客户端。编译后生成可被 Xylos 主项目作为静态文件托管的 Web 产物，同时具备 PWA 能力，可以安装到桌面或移动设备，像应用一样使用。

客户端不依赖 Xylos 的私有接口，应尽量兼容标准 WebDAV 服务，包括 Xylos、Nextcloud、Apache `mod_dav`、Nginx WebDAV 以及其他兼容实现。

## 产品能力

### 第一阶段：在线文件管理

- 配置 WebDAV 服务地址
- Basic、Bearer、Digest 认证
- 浏览目录和文件
- 面包屑导航
- 上传和下载文件
- 新建目录
- 删除文件和目录
- 移动、复制、重命名
- 查看大小、修改时间和文件类型
- 显示加载、上传、下载和错误状态
- 桌面端和移动端响应式布局

客户端使用标准 WebDAV 方法：

- `OPTIONS`
- `GET`、`HEAD`
- `PUT`
- `DELETE`
- `MKCOL`
- `COPY`、`MOVE`
- `PROPFIND`
- 后续可加入 `PROPPATCH`、`LOCK`、`UNLOCK`

### 第二阶段：PWA

编译产物需要包含：

```text
app/dist/
  index.html
  manifest.webmanifest
  sw.js
  assets/
    *.js
    *.wasm
    *.css
    icons/
```

PWA 应支持：

- Web App Manifest
- Service Worker
- 安装到桌面或主屏幕
- 独立窗口模式
- 应用图标、主题色和启动配置
- 离线启动
- 版本更新检测和旧缓存清理

PWA 生产环境应使用 HTTPS；`localhost` 可用于本地开发。

## 离线策略

离线能力分为三层，不能把离线启动等同于离线访问远程服务器。

### 应用离线

没有网络时，应用仍可以启动，显示连接配置、最近访问记录、网络状态和缓存内容。

### 离线只读

用户主动缓存的目录和文件可以离线查看、搜索、预览和下载。建议使用：

- Cache Storage：静态资源和适合直接缓存的 GET 响应
- IndexedDB：目录模型、文件元数据、连接信息和缓存索引
- OPFS 或 IndexedDB：较大的离线文件内容

不应默认缓存所有远程文件，应提供缓存目录、缓存文件、清理缓存和缓存大小管理。

### 离线写操作

离线新建、删除、移动、重命名和上传需要进入 IndexedDB 操作队列，联网后再执行。同步流程需要处理远端版本变化、冲突、鉴权过期、失败重试和操作顺序。

第一版优先实现离线启动和离线只读；离线写入作为后续阶段，不能只依赖浏览器的 Background Sync。

## 前端结构

建议分为四层：

1. **WebDAV Transport**：发送 DAV 请求、处理认证、请求头、请求体和响应流。
2. **协议解析**：解析 `PROPFIND` XML，处理状态码、路径、URL 和 DAV 响应头。
3. **应用状态**：管理连接、当前目录、文件列表、选中项、任务、缓存和错误。
4. **Dioxus UI**：实现连接页、文件浏览器、工具栏、菜单、对话框、拖拽上传和移动端布局。

协议层应与 UI 解耦，便于以后复用到桌面端或移动端。

## 静态产物运行方式

推荐由 Xylos 主服务同源托管：

```text
https://example.com/app/
https://example.com/dav/
```

其中：

- `/app/`：Dioxus 编译后的 PWA 静态资源
- `/dav/`：Xylos 自身的 WebDAV 服务
- Service Worker 作用域限制在 `/app/`

主服务需要提供：

```text
GET /app/
GET /app/index.html
GET /app/manifest.webmanifest
GET /app/sw.js
GET /app/assets/*
```

还需要保证：

- `.wasm` 使用 `application/wasm`
- JS、CSS、Web Manifest 使用正确 MIME 类型
- `sw.js` 不被长时间缓存
- `/app/` 下的前端路由刷新后仍返回 `index.html`
- 前端资源尽量使用相对路径，支持挂载到子路径

也可以使用 Nginx、Caddy 或其他静态服务器独立托管 `/app/`，但生产环境必须正确配置 HTTPS 和静态资源 MIME 类型。

## CORS 边界

主项目不增加 WebDAV Proxy。客户端直接请求用户配置的 WebDAV 地址，跨域由目标 WebDAV 服务或用户自己的 Nginx、Caddy 等反向代理解决。

这样可以避免在 Xylos 中引入开放代理、SSRF 防护、远端凭据托管、目标 URL 校验、流式转发和 DAV 头重写等额外复杂度。

跨域使用时，目标服务需要允许：

- 前端来源 `Origin`
- `OPTIONS` 预检请求
- `PROPFIND`、`MKCOL`、`COPY`、`MOVE`、`LOCK`、`UNLOCK` 等 DAV 方法
- `Authorization`、`Depth`、`Destination`、`Overwrite`、`If`、`Lock-Token` 等请求头
- `DAV`、`Allow`、`ETag`、`Last-Modified` 等响应头

如果使用 Cookie 或其他凭据，还必须正确配置 `Access-Control-Allow-Credentials`。带凭据的请求不能使用 `Access-Control-Allow-Origin: *`。

客户端不绕过浏览器的同源策略。遇到 CORS、预检失败或认证失败时，应向用户显示明确错误，并提示在 WebDAV 服务或反向代理上配置 CORS。

## 安全与缓存隔离

缓存和本地认证数据必须按以下维度隔离：

```text
服务器地址 + 用户身份 + WebDAV 路径
```

需要注意：

- 不默认长期保存明文密码
- 退出登录时清理 token、认证材料和相关缓存
- 不同服务器之间不能复用目录或文件缓存
- Service Worker 只拦截应用自身范围，不能误拦截 WebDAV 请求
- 文件缓存需要有清理、大小限制和过期策略

## 开发阶段

1. 创建独立的 `app` Dioxus Web crate，完成基础构建和静态资源输出。
2. 增加 Manifest、Service Worker、离线启动和安装能力。
3. 实现 WebDAV Transport 和 `PROPFIND` 目录浏览。
4. 完成上传、下载、新建目录、删除、移动和复制。
5. 增加 Digest 认证、文件预览和缓存管理。
6. 最后实现离线只读增强及离线写操作队列。

## 非目标

第一版不包含：

- 主项目 WebDAV Proxy
- 本地文件夹双向同步
- 服务端账号管理
- 在线文档编辑器
- 任意 URL 的开放转发
- 完整桌面文件系统集成

## 验收标准

- `app` 可以独立编译为 Web 静态产物。
- 产物可以由 Xylos 通过 `/app/` 同源托管。
- 浏览器可以安装该应用并在无网络时启动。
- 可以连接当前 Xylos 和其他已正确配置 CORS 的 WebDAV 服务。
- 可以完成目录浏览、上传、下载、新建目录、删除、移动和复制。
- Basic、Bearer、Digest 的行为与目标服务配置一致。
- 缓存、认证信息和不同服务器之间相互隔离。
- 不需要在 Xylos 中加入代理逻辑。
