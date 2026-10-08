use crate::models::{AuthType, Entry, ServerAccount};
use gloo_storage::{LocalStorage, Storage};
use quick_xml::de::from_str;
use reqwest::header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MultiStatus {
    #[serde(rename = "response", default)]
    responses: Vec<ResponseEntry>,
}

#[derive(Debug, Deserialize)]
struct ResponseEntry {
    href: String,
    #[serde(rename = "propstat", default)]
    propstats: Vec<PropStat>,
}

#[derive(Debug, Deserialize)]
struct PropStat {
    prop: Prop,
    status: String,
}

#[derive(Debug, Deserialize, Default)]
struct Prop {
    #[serde(rename = "resourcetype")]
    resource_type: Option<ResourceType>,
    #[serde(rename = "getcontentlength")]
    content_length: Option<u64>,
    #[serde(rename = "getlastmodified")]
    modified: Option<String>,
    #[serde(rename = "getetag")]
    etag: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ResourceType {
    collection: Option<String>,
}

pub fn is_online() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().on_line().then_some(true))
        .unwrap_or(true)
}

pub async fn test_server_connection(server: &ServerAccount) -> Result<(), String> {
    if !is_online() {
        return Err("当前处于离线状态，无法测试服务器连接".into());
    }
    let url = join_url(&server.base_url, "/");
    let client = reqwest::Client::new();
    let mut request = client
        .request(reqwest::Method::from_bytes(b"OPTIONS").unwrap(), &url);
    request = add_auth(request, server);

    let response = request.send().await.map_err(network_error)?;
    if response.status().is_success() || response.status().as_u16() == 405 {
        Ok(())
    } else if response.status().as_u16() == 401 {
        Err("认证失败：用户名或密码错误".into())
    } else {
        // Fallback to PROPFIND Depth 0 if OPTIONS is not supported
        let mut propfind_req = client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header(HeaderName::from_static("depth"), "0");
        propfind_req = add_auth(propfind_req, server);
        let resp = propfind_req.send().await.map_err(network_error)?;
        if resp.status().is_success() || resp.status().as_u16() == 207 {
            Ok(())
        } else if resp.status().as_u16() == 401 {
            Err("认证失败：用户名或密码错误".into())
        } else {
            Err(format!("服务器返回状态码 {}", resp.status()))
        }
    }
}

pub async fn list_directory(server: &ServerAccount, path: &str) -> Result<Vec<Entry>, String> {
    let cache_key = format!("xylos.webdav.cache.{}{}", server.base_url, path);
    if !is_online() {
        return LocalStorage::get(&cache_key).map_err(|_| "当前离线，且该目录尚未缓存".into());
    }

    let url = join_url(&server.base_url, path);
    let client = reqwest::Client::new();
    let mut request = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), url)
        .header(HeaderName::from_static("depth"), "1")
        .header(CONTENT_TYPE, "application/xml");
    request = add_auth(request, server);

    let response = request.send().await.map_err(network_error)?;
    if !response.status().is_success() && response.status().as_u16() != 207 {
        if response.status().as_u16() == 401 {
            return Err("认证失败，请检查账号密码".into());
        }
        return Err(format!(
            "WebDAV 返回 {}，请检查地址、认证或 CORS 配置",
            response.status()
        ));
    }

    let body = response.text().await.map_err(network_error)?;
    let parsed: MultiStatus =
        from_str(&body).map_err(|error| format!("无法解析 WebDAV 响应: {error}"))?;

    let entries: Vec<Entry> = parsed
        .responses
        .into_iter()
        .filter_map(|item| {
            let prop = item
                .propstats
                .into_iter()
                .find(|p| p.status.contains("200"))
                .map(|p| p.prop)
                .unwrap_or_default();
            let href = normalize_href(&server.base_url, path, &item.href);
            if href == normalize_path(path) {
                return None;
            }
            let name = href
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("/")
                .to_string();
            let is_dir = prop.resource_type.and_then(|r| r.collection).is_some()
                || item.href.ends_with('/');
            Some(Entry {
                name,
                href,
                is_dir,
                size: prop.content_length,
                modified: prop.modified.unwrap_or_default(),
                etag: prop.etag,
            })
        })
        .collect();

    let _ = LocalStorage::set(&cache_key, &entries);
    Ok(entries)
}

pub async fn execute_write(
    server: &ServerAccount,
    method: reqwest::Method,
    path: &str,
    body: Option<Vec<u8>>,
    content_type: Option<String>,
    destination: Option<String>,
    etag: Option<String>,
) -> Result<(), String> {
    if !is_online() {
        return Err("当前处于离线状态，写操作已禁用".into());
    }
    let client = reqwest::Client::new();
    let mut request = add_auth(
        client.request(method, join_url(&server.base_url, path)),
        server,
    );
    if let Some(body) = body {
        request = request.body(body);
    }
    if let Some(content_type) = content_type {
        request = request.header(CONTENT_TYPE, content_type);
    }
    if let Some(destination) = destination {
        request = request.header(
            HeaderName::from_static("destination"),
            join_url(&server.base_url, &destination),
        );
        request = request.header(HeaderName::from_static("overwrite"), "F");
    }
    if let Some(etag) = etag {
        request = request.header(HeaderName::from_static("if-match"), etag);
    }
    let response = request.send().await.map_err(network_error)?;
    if response.status().is_success() || response.status().as_u16() == 207 {
        Ok(())
    } else if response.status().as_u16() == 412 {
        Err("写操作冲突：远端文件已被修改，请刷新后重试".into())
    } else {
        Err(format!(
            "WebDAV 返回 {}，操作未完成",
            response.status()
        ))
    }
}

pub async fn download_bytes(server: &ServerAccount, path: &str) -> Result<Vec<u8>, String> {
    if !is_online() {
        return Err("当前处于离线状态，文件内容尚未缓存".into());
    }
    let client = reqwest::Client::new();
    let response = add_auth(client.get(join_url(&server.base_url, path)), server)
        .send()
        .await
        .map_err(network_error)?;
    if !response.status().is_success() {
        return Err(format!("下载失败：WebDAV 返回 {}", response.status()));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(network_error)
}

fn add_auth(
    request: reqwest::RequestBuilder,
    server: &ServerAccount,
) -> reqwest::RequestBuilder {
    match server.auth_type {
        AuthType::Basic => request.basic_auth(&server.username, Some(&server.password)),
        AuthType::Bearer => request.header(AUTHORIZATION, format!("Bearer {}", server.token)),
        AuthType::Digest => {
            // Standard Basic or raw Auth fallback for initial digest handshake
            request.basic_auth(&server.username, Some(&server.password))
        }
    }
}

pub fn join_url(base: &str, path: &str) -> String {
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
        format!(
            "{}/{}",
            base_path.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    };
    format!("{scheme}://{host}{path}")
}

pub fn join_path(base: &str, name: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), urlencoding::encode(name))
}

pub fn normalize_path(path: &str) -> String {
    let mut result = String::from("/");
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            if result.len() > 1 {
                result.pop();
            }
            if let Some(index) = result.rfind('/') {
                result.truncate(index + 1);
            }
            continue;
        }
        result.push_str(part);
        result.push('/');
    }
    if result.len() > 1 {
        result.pop();
    }
    result
}

pub fn normalize_href(base: &str, current: &str, href: &str) -> String {
    let absolute_path = href.starts_with('/') || href.starts_with("http://") || href.starts_with("https://");
    let path = if href.starts_with("http://") || href.starts_with("https://") {
        href.split_once("//")
            .and_then(|(_, rest)| rest.split_once('/').map(|(_, path)| path))
            .unwrap_or("/")
    } else {
        href
    };
    let decoded = urlencoding::decode(path)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| path.to_string());
    let base_path = base
        .split_once("//")
        .and_then(|(_, rest)| rest.split_once('/').map(|(_, path)| path))
        .unwrap_or("");
    let relative = if !base_path.is_empty() && decoded.starts_with(base_path) {
        decoded[base_path.len()..].to_string()
    } else {
        decoded
    };
    if absolute_path {
        normalize_path(&format!("/{}", relative.trim_start_matches('/')))
    } else {
        normalize_path(&format!("{}/{}", current.trim_end_matches('/'), relative))
    }
}

pub fn parent_path(path: &str) -> String {
    let normalized = normalize_path(path);
    normalized
        .rsplit_once('/')
        .map(|(parent, _)| if parent.is_empty() { "/".into() } else { parent.into() })
        .unwrap_or_else(|| "/".into())
}

pub fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB", size as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

pub fn format_date(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let ts = js_sys::Date::parse(raw);
    if ts.is_nan() {
        return raw.to_string();
    }
    let d = js_sys::Date::new(&js_sys::wasm_bindgen::JsValue::from_f64(ts));
    let y = d.get_full_year();
    let m = d.get_month() + 1;
    let day = d.get_date();
    let h = d.get_hours();
    let min = d.get_minutes();
    let s = d.get_seconds();
    format!("{y:04}-{m:02}-{day:02} {h:02}:{min:02}:{s:02}")
}


fn network_error(error: reqwest::Error) -> String {
    format!("网络请求失败: {error}。跨域访问时请检查 WebDAV 服务的 CORS 配置。")
}
