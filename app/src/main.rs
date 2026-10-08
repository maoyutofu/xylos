use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use quick_xml::de::from_str;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName};
use serde::{Deserialize, Serialize};

const CONNECTION_KEY: &str = "xylos.webdav.connection";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
struct Connection {
    url: String,
    username: String,
    password: String,
    token: String,
    auth: AuthMode,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AuthMode { #[default] Basic, Bearer }

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Entry { name: String, href: String, is_dir: bool, size: Option<u64>, modified: String, etag: Option<String> }


#[derive(Debug, Deserialize)]
struct MultiStatus { #[serde(rename = "response", default)] responses: Vec<ResponseEntry> }

#[derive(Debug, Deserialize)]
struct ResponseEntry { href: String, #[serde(rename = "propstat", default)] propstats: Vec<PropStat> }

#[derive(Debug, Deserialize)]
struct PropStat { prop: Prop, status: String }

#[derive(Debug, Deserialize, Default)]
struct Prop { #[serde(rename = "resourcetype")] resource_type: Option<ResourceType>, #[serde(rename = "getcontentlength")] content_length: Option<u64>, #[serde(rename = "getlastmodified")] modified: Option<String>, #[serde(rename = "getetag")] etag: Option<String> }

#[derive(Debug, Deserialize, Default)]
struct ResourceType { collection: Option<String> }


fn main() { dioxus::launch(App); }

#[component]
fn App() -> Element {
    let mut connection = use_signal(|| LocalStorage::get::<Connection>(CONNECTION_KEY).unwrap_or_default());
    let mut connected = use_signal(|| false);
    let mut current_path = use_signal(|| "/".to_string());
    let mut entries = use_signal(Vec::<Entry>::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);
    let mut new_name = use_signal(String::new);
    let mut move_destination = use_signal(String::new);
    let mut selected = use_signal(|| None::<Entry>);
    let auth_value = match connection().auth { AuthMode::Basic => "basic", AuthMode::Bearer => "bearer" };

    use_effect(move || {
        let _ = dioxus::document::eval("if ('serviceWorker' in navigator) navigator.serviceWorker.register('/app/sw.js')");
    });

    let load = move |path: String| {
        let conn = connection();
        spawn(async move {
            loading.set(true); error.set(None);
            match list_directory(&conn, &path).await {
                Ok(items) => { entries.set(items); current_path.set(path); connected.set(true); }
                Err(message) => { error.set(Some(message)); }
            }
            loading.set(false);
        });
    };

    let save_and_connect = move |_| {
        let conn = connection();
        if conn.url.trim().is_empty() { error.set(Some("请输入 WebDAV 服务地址".into())); return; }
        let _ = LocalStorage::set(CONNECTION_KEY, &conn);
        load("/".into());
    };

    let create_directory = move |_| {
        let conn = connection();
        let path = current_path();
        let name = new_name().trim().to_string();
        if name.is_empty() { error.set(Some("请输入目录名称".into())); return; }
        if name.contains('/') || name.contains('\\') { error.set(Some("目录名称不能包含路径分隔符".into())); return; }
        spawn(async move {
            loading.set(true); error.set(None);
            match execute_write(&conn, reqwest::Method::from_bytes(b"MKCOL").unwrap(), &join_path(&path, &name), None, None, None, None).await {
                Ok(()) => { new_name.set(String::new()); notice.set(Some("目录已创建".into())); load(path); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let delete_entry = move |entry: Entry| {
        let conn = connection();
        let path = entry.href.clone();
        let parent = current_path();
        spawn(async move {
            loading.set(true); error.set(None);
            match execute_write(&conn, reqwest::Method::DELETE, &path, None, None, None, entry.etag.clone()).await {
                Ok(()) => { notice.set(Some(format!("已删除 {}", entry.name))); load(parent); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let rename_entry = move |_| {
        let Some(entry) = selected() else { error.set(Some("请先选择一个条目".into())); return; };
        let name = new_name().trim().to_string();
        if name.is_empty() || name.contains('/') || name.contains('\\') { error.set(Some("请输入不含路径分隔符的新名称".into())); return; }
        let conn = connection(); let parent = current_path();
        spawn(async move {
            loading.set(true); error.set(None);
            let destination = join_path(&parent, &name);
            match execute_write(&conn, reqwest::Method::from_bytes(b"MOVE").unwrap(), &entry.href, None, None, Some(destination), entry.etag).await {
                Ok(()) => { selected.set(None); new_name.set(String::new()); notice.set(Some("已重命名".into())); load(parent); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let copy_entry = move |_| {
        let Some(entry) = selected() else { error.set(Some("请先选择一个条目".into())); return; };
        let name = new_name().trim().to_string();
        if name.is_empty() || name.contains('/') || name.contains('\\') { error.set(Some("请输入复制后的名称".into())); return; }
        let conn = connection(); let parent = current_path();
        spawn(async move {
            loading.set(true); error.set(None);
            match execute_write(&conn, reqwest::Method::from_bytes(b"COPY").unwrap(), &entry.href, None, None, Some(join_path(&parent, &name)), entry.etag).await {
                Ok(()) => { new_name.set(String::new()); notice.set(Some("已复制".into())); load(parent); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let move_entry = move |_| {
        let Some(entry) = selected() else { error.set(Some("请先选择一个条目".into())); return; };
        let destination = move_destination().trim().to_string();
        if destination.is_empty() || !destination.starts_with('/') { error.set(Some("请输入目标目录路径，例如 /archive".into())); return; }
        let conn = connection(); let parent = current_path();
        spawn(async move {
            loading.set(true); error.set(None);
            let name = entry.name.clone();
            match execute_write(&conn, reqwest::Method::from_bytes(b"MOVE").unwrap(), &entry.href, None, None, Some(join_path(&destination, &name)), entry.etag).await {
                Ok(()) => { selected.set(None); move_destination.set(String::new()); notice.set(Some("已移动".into())); load(parent); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let upload_file = move |event: FormEvent| {
        let Some(file) = event.files().into_iter().next() else { return; };
        let conn = connection(); let path = current_path();
        spawn(async move {
            loading.set(true); error.set(None);
            let name = file.name();
            let operation = match file.read_bytes().await {
                Ok(bytes) => (bytes, file.content_type()),
                Err(_) => { error.set(Some("无法读取所选文件".into())); loading.set(false); return; }
            };
            match execute_write(&conn, reqwest::Method::PUT, &join_path(&path, &name), Some(operation.0.to_vec()), operation.1, None, None).await {
                Ok(()) => { notice.set(Some("上传完成".into())); load(path); }
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    let download_entry = move |entry: Entry| {
        let conn = connection();
        spawn(async move {
            match download_bytes(&conn, &entry.href).await {
                Ok(bytes) => {
                    let encoded = base64_encode(&bytes);
                    let mime = "application/octet-stream";
                    let script = format!("const a=document.createElement('a');a.href='data:{mime};base64,{encoded}';a.download={};a.click();", serde_json::to_string(&entry.name).unwrap_or_else(|_| "'download'".into()));
                    let _ = dioxus::document::eval(&script);
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };


    let rows = entries().into_iter().map(|entry| {
        let href = entry.href.clone();
        let item = entry.clone();
        let selected_item = item.clone();
        let download_item = item.clone();
        rsx! {
            tr { key: "{entry.href}",
                td { if entry.is_dir { button { class: "item", onclick: move |_| load(href.clone()), "📁 {entry.name}" } } else { "📄 {entry.name}" } }
                td { "{entry.size.map(|n| format_size(n)).unwrap_or_else(|| \"-\".into())}" }
                td { "{entry.modified}" }
                td { button { class: "secondary", onclick: move |_| download_entry(download_item.clone()), "下载" } button { class: "secondary", onclick: move |_| selected.set(Some(selected_item.clone())), "选择" } button { class: "danger", disabled: loading() || !is_online(), onclick: move |_| delete_entry(item.clone()), "删除" } }
            }
        }
    });

    rsx! {
        div { class: "shell",
            header { class: "topbar", div { class: "brand", "Xylos WebDAV" }, div { class: "status", if connected() { "在线" } else { "未连接" } } }
            main { class: "content",
                if !connected() {
                    section { class: "panel connect",
                        h1 { "连接 WebDAV" }
                        p { class: "muted", "连接任意已允许浏览器跨域访问的 WebDAV 服务。" }
                        div { class: "form",
                            label { "服务地址", input { r#type: "url", placeholder: "https://example.com/webdav", value: "{connection().url}", oninput: move |event| connection.write().url = event.value() } }
                            label { "认证方式", select { value: auth_value, onchange: move |event| connection.write().auth = if event.value() == "bearer" { AuthMode::Bearer } else { AuthMode::Basic }, option { value: "basic", "Basic" } option { value: "bearer", "Bearer" } } }
                            if matches!(connection().auth, AuthMode::Basic) {
                                label { "用户名", input { value: "{connection().username}", oninput: move |event| connection.write().username = event.value() } }
                                label { "密码", input { r#type: "password", value: "{connection().password}", oninput: move |event| connection.write().password = event.value() } }
                            } else {
                                label { "Bearer Token", input { r#type: "password", value: "{connection().token}", oninput: move |event| connection.write().token = event.value() } }
                            }
                            div { class: "actions", button { class: "primary", disabled: loading(), onclick: save_and_connect, if loading() { "连接中..." } else { "连接" } } }
                        }
                        if let Some(message) = error() { div { class: "error", "{message}" } }
                    }
                } else {
                    section {
                        div { class: "toolbar",
                            div { h2 { "文件" } div { class: "muted", "{current_path()}" } }
                            div { class: "actions", button { disabled: loading(), onclick: move |_| load(current_path()), "刷新" } button { onclick: move |_| { connected.set(false); }, "断开" } }
                        }
                        div { class: "actions create-row",
                            input { placeholder: "新目录名称 / 新名称", value: "{new_name()}", oninput: move |event| new_name.set(event.value()) }
                            input { placeholder: "移动到目录，如 /archive", value: "{move_destination()}", oninput: move |event| move_destination.set(event.value()) }
                            button { disabled: loading() || !is_online(), onclick: create_directory, "新建目录" }
                            button { disabled: loading() || !is_online(), onclick: rename_entry, "重命名" }
                            button { disabled: loading() || !is_online(), onclick: copy_entry, "复制" }
                            button { disabled: loading() || !is_online(), onclick: move_entry, "移动" }
                            label { class: "file-picker", "上传文件", input { r#type: "file", disabled: !is_online(), onchange: upload_file } }
                            if current_path() != "/" { button { onclick: move |_| load(parent_path(&current_path())), "上一级" } }
                            span { class: "muted", "离线时仅可查看已缓存目录" }
                            if let Some(entry) = selected() { span { class: "muted", "已选择: {entry.name}，输入新名称后点击重命名" } }
                        }
                        if let Some(message) = error() { div { class: "error", "{message}" } }
                        if let Some(message) = notice() { p { class: "muted", "{message}" } }
                        table { class: "table",
                            thead { tr { th { "名称" } th { "大小" } th { "修改时间" } th { "操作" } } }
                            tbody {
                                for row in rows { {row} }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn list_directory(connection: &Connection, path: &str) -> Result<Vec<Entry>, String> {
    let cache_key = format!("xylos.webdav.cache.{}{}", connection.url, path);
    if !is_online() {
        return LocalStorage::get(&cache_key).map_err(|_| "当前离线，且该目录尚未缓存".into());
    }
    let url = join_url(&connection.url, path);
    let client = reqwest::Client::new();
    let mut request = client.request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), url).header(HeaderName::from_static("depth"), "1").header(CONTENT_TYPE, "application/xml");
    request = add_auth(request, connection);
    let response = request.send().await.map_err(network_error)?;
    if !response.status().is_success() { return Err(format!("WebDAV 返回 {}，请检查地址、认证或 CORS 配置", response.status())); }
    let body = response.text().await.map_err(network_error)?;
    let parsed: MultiStatus = from_str(&body).map_err(|error| format!("无法解析 WebDAV 响应: {error}"))?;
    let entries: Vec<Entry> = parsed.responses.into_iter().filter_map(|item| {
        let prop = item.propstats.into_iter().find(|p| p.status.contains("200")).map(|p| p.prop).unwrap_or_default();
        let href = normalize_href(&connection.url, path, &item.href);
        if href == normalize_path(path) { return None; }
        let name = href.trim_end_matches('/').rsplit('/').next().unwrap_or("/").to_string();
        let is_dir = prop.resource_type.and_then(|r| r.collection).is_some() || item.href.ends_with('/');
        Some(Entry { name, href, is_dir, size: prop.content_length, modified: prop.modified.unwrap_or_default(), etag: prop.etag })
    }).collect();
    let _ = LocalStorage::set(&cache_key, &entries);
    Ok(entries)
}

async fn execute_write(connection: &Connection, method: reqwest::Method, path: &str, body: Option<Vec<u8>>, content_type: Option<String>, destination: Option<String>, etag: Option<String>) -> Result<(), String> {
    if !is_online() { return Err("当前处于离线状态，写操作已禁用".into()); }
    let client = reqwest::Client::new();
    let mut request = add_auth(client.request(method, join_url(&connection.url, path)), connection);
    if let Some(body) = body { request = request.body(body); }
    if let Some(content_type) = content_type { request = request.header(CONTENT_TYPE, content_type); }
    if let Some(destination) = destination { request = request.header(HeaderName::from_static("destination"), join_url(&connection.url, &destination)); request = request.header(HeaderName::from_static("overwrite"), "F"); }
    if let Some(etag) = etag { request = request.header(HeaderName::from_static("if-match"), etag); }
    let response = request.send().await.map_err(network_error)?;
    if response.status().is_success() || response.status().as_u16() == 207 { Ok(()) }
    else if response.status().as_u16() == 412 { Err("写操作发生冲突：远端文件已被修改，请刷新后重试".into()) }
    else { Err(format!("WebDAV 返回 {}，操作未完成（请求: {}）", response.status(), join_url(&connection.url, path))) }
}

fn is_online() -> bool {
    web_sys::window().map(|window| window.navigator().on_line()).unwrap_or(true)
}

async fn download_bytes(connection: &Connection, path: &str) -> Result<Vec<u8>, String> {
    if !is_online() { return Err("当前处于离线状态，文件内容尚未缓存".into()); }
    let client = reqwest::Client::new();
    let response = add_auth(client.get(join_url(&connection.url, path)), connection).send().await.map_err(network_error)?;
    if !response.status().is_success() { return Err(format!("下载失败：WebDAV 返回 {}", response.status())); }
    response.bytes().await.map(|bytes| bytes.to_vec()).map_err(network_error)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as usize;
        let b = chunk.get(1).copied().unwrap_or(0) as usize;
        let c = chunk.get(2).copied().unwrap_or(0) as usize;
        result.push(TABLE[a >> 2] as char);
        result.push(TABLE[((a & 3) << 4) | (b >> 4)] as char);
        result.push(if chunk.len() > 1 { TABLE[((b & 15) << 2) | (c >> 6)] as char } else { '=' });
        result.push(if chunk.len() > 2 { TABLE[c & 63] as char } else { '=' });
    }
    result
}

fn add_auth(request: reqwest::RequestBuilder, connection: &Connection) -> reqwest::RequestBuilder {
    match connection.auth { AuthMode::Basic => request.basic_auth(&connection.username, Some(&connection.password)), AuthMode::Bearer => request.header(AUTHORIZATION, format!("Bearer {}", connection.token)) }
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    let Some((scheme, rest)) = base.split_once("://") else {
        return format!("{base}/{}", path.trim_start_matches('/'));
    };
    let (host, base_path) = rest.split_once('/').unwrap_or((rest, ""));
    let base_path = format!("/{}", base_path.trim_matches('/'));
    let path = if path.starts_with(&base_path)
        && (path.len() == base_path.len() || path.as_bytes().get(base_path.len()) == Some(&b'/'))
    {
        path.to_string()
    } else {
        format!("{}/{}", base_path.trim_end_matches('/'), path.trim_start_matches('/'))
    };
    format!("{scheme}://{host}{path}")
}
fn join_path(base: &str, name: &str) -> String { format!("{}/{}", base.trim_end_matches('/'), urlencoding::encode(name)) }
fn normalize_path(path: &str) -> String {
    let mut result = String::from("/");
    for part in path.split('/') {
        if part.is_empty() || part == "." { continue; }
        if part == ".." {
            if result.len() > 1 { result.pop(); }
            if let Some(index) = result.rfind('/') { result.truncate(index + 1); }
            continue;
        }
        result.push_str(part); result.push('/');
    }
    if result.len() > 1 { result.pop(); }
    result
}
fn normalize_href(base: &str, current: &str, href: &str) -> String {
    let absolute_path = href.starts_with('/') || href.starts_with("http://") || href.starts_with("https://");
    let path = if href.starts_with("http://") || href.starts_with("https://") {
        href.split_once("//").and_then(|(_, rest)| rest.split_once('/').map(|(_, path)| path)).unwrap_or("/")
    } else { href };
    let decoded = urlencoding::decode(path).map(|value| value.into_owned()).unwrap_or_else(|_| path.to_string());
    let base_path = base.split_once("//").and_then(|(_, rest)| rest.split_once('/').map(|(_, path)| path)).unwrap_or("");
    let relative = if !base_path.is_empty() && decoded.starts_with(base_path) {
        decoded[base_path.len()..].to_string()
    } else { decoded };
    if absolute_path { normalize_path(&format!("/{}", relative.trim_start_matches('/'))) }
    else { normalize_path(&format!("{}/{}", current.trim_end_matches('/'), relative)) }
}
fn parent_path(path: &str) -> String {
    let normalized = normalize_path(path);
    normalized.rsplit_once('/').map(|(parent, _)| if parent.is_empty() { "/".into() } else { parent.into() }).unwrap_or_else(|| "/".into())
}
fn network_error(error: reqwest::Error) -> String { format!("网络请求失败: {error}。跨域访问时请检查 WebDAV 服务的 CORS 配置。") }
fn format_size(size: u64) -> String { if size < 1024 { format!("{size} B") } else if size < 1024 * 1024 { format!("{:.1} KB", size as f64 / 1024.0) } else { format!("{:.1} MB", size as f64 / 1024.0 / 1024.0) } }
