use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

use crate::components::icons::IconImage;
use crate::models::{Entry, ServerAccount};
use crate::webdav::{ThumbnailDownload, download_bytes, download_thumbnail, is_online};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const MAX_THUMBNAIL_BYTES: u64 = 20 * 1024 * 1024;
const THUMBNAIL_WIDTH: u32 = 320;
const MAX_CONCURRENT_THUMBNAILS: usize = 3;
const MAX_MEMORY_CACHE_ENTRIES: usize = 120;

thread_local! {
    static THUMBNAIL_CACHE: RefCell<ThumbnailCache> = RefCell::new(ThumbnailCache::default());
    static THUMBNAIL_ENDPOINT_SUPPORT: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    static ACTIVE_THUMBNAIL_REQUESTS: Cell<usize> = Cell::new(0);
}

#[derive(Clone)]
struct CachedThumbnail {
    bytes: Vec<u8>,
    mime: String,
}

#[derive(Default)]
struct ThumbnailCache {
    entries: HashMap<String, CachedThumbnail>,
    order: VecDeque<String>,
}

impl ThumbnailCache {
    fn get(&self, key: &str) -> Option<CachedThumbnail> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: String, thumbnail: CachedThumbnail) {
        if self.entries.insert(key.clone(), thumbnail).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > MAX_MEMORY_CACHE_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

struct ThumbnailSlot;

impl Drop for ThumbnailSlot {
    fn drop(&mut self) {
        ACTIVE_THUMBNAIL_REQUESTS.with(|active| active.set(active.get().saturating_sub(1)));
    }
}

async fn acquire_thumbnail_slot() -> ThumbnailSlot {
    loop {
        let acquired = ACTIVE_THUMBNAIL_REQUESTS.with(|active| {
            if active.get() < MAX_CONCURRENT_THUMBNAILS {
                active.set(active.get() + 1);
                true
            } else {
                false
            }
        });
        if acquired {
            return ThumbnailSlot;
        }
        TimeoutFuture::new(40).await;
    }
}

pub fn is_thumbnail_image(name: &str) -> bool {
    matches!(
        name.rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "bmp" | "ico")
    )
}

#[component]
pub fn ImageThumbnail(
    server: ServerAccount,
    entry: Entry,
    #[props(default = false)] grid: bool,
) -> Element {
    let mut object_url = use_signal(|| None::<String>);
    let mut visible = use_signal(|| false);
    let mut requested = use_signal(|| false);
    let mut failed = use_signal(|| false);
    let request_server = server.clone();
    let request_entry = entry.clone();

    use_effect(move || {
        if !visible() || requested() || failed() {
            return;
        }
        requested.set(true);

        let server = request_server.clone();
        let entry = request_entry.clone();
        if !is_online() || entry.size.is_some_and(|size| size > MAX_THUMBNAIL_BYTES) {
            failed.set(true);
            return;
        }

        spawn(async move {
            let Some(thumbnail) = load_thumbnail(&server, &entry).await else {
                failed.set(true);
                return;
            };
            let Ok(url) = create_blob_url(&thumbnail.bytes, &thumbnail.mime) else {
                failed.set(true);
                return;
            };
            object_url.set(Some(url));
        });
    });

    use_drop(move || {
        if let Some(url) = object_url() {
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    });

    let class = if grid {
        "grid-thumbnail"
    } else {
        "file-thumbnail"
    };
    let placeholder_class = if grid {
        "grid-thumbnail-placeholder"
    } else {
        "file-thumbnail-placeholder"
    };

    if let Some(url) = object_url() {
        rsx! {
            img {
                class: "{class}",
                src: "{url}",
                alt: "{entry.name}",
                loading: "lazy",
            }
        }
    } else {
        rsx! {
            div {
                class: "{placeholder_class}",
                onvisible: move |event| {
                    if event.is_intersecting().unwrap_or(true) {
                        visible.set(true);
                    }
                },
                IconImage { is_large: grid }
            }
        }
    }
}

async fn load_thumbnail(server: &ServerAccount, entry: &Entry) -> Option<CachedThumbnail> {
    let key = thumbnail_cache_key(server, entry);
    if let Some(cached) = THUMBNAIL_CACHE.with(|cache| cache.borrow().get(&key)) {
        return Some(cached);
    }

    let _slot = acquire_thumbnail_slot().await;
    if let Some(cached) = THUMBNAIL_CACHE.with(|cache| cache.borrow().get(&key)) {
        return Some(cached);
    }

    let version = entry
        .etag
        .clone()
        .unwrap_or_else(|| format!("{}-{}", entry.modified, entry.size.unwrap_or(0)));
    if endpoint_supported(&server.id) {
        match download_thumbnail(server, &entry.href, &version, THUMBNAIL_WIDTH).await {
            ThumbnailDownload::Ready(bytes) => {
                set_endpoint_supported(&server.id, true);
                let thumbnail = CachedThumbnail {
                    bytes,
                    mime: "image/webp".into(),
                };
                remember_thumbnail(key, thumbnail.clone());
                return Some(thumbnail);
            }
            ThumbnailDownload::EndpointUnavailable => {
                set_endpoint_supported(&server.id, false);
            }
            ThumbnailDownload::UnsupportedFormat => {}
            ThumbnailDownload::Failed(error) => {
                let _ = error;
                return None;
            }
        }
    }

    let bytes = download_bytes(server, &entry.href).await.ok()?;
    let thumbnail = CachedThumbnail {
        bytes,
        mime: image_mime(entry),
    };
    remember_thumbnail(key, thumbnail.clone());
    Some(thumbnail)
}

fn endpoint_supported(server_id: &str) -> bool {
    THUMBNAIL_ENDPOINT_SUPPORT
        .with(|support| support.borrow().get(server_id).copied().unwrap_or(true))
}

fn set_endpoint_supported(server_id: &str, supported: bool) {
    THUMBNAIL_ENDPOINT_SUPPORT.with(|support| {
        support
            .borrow_mut()
            .insert(server_id.to_string(), supported);
    });
}

fn thumbnail_cache_key(server: &ServerAccount, entry: &Entry) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        server.id,
        entry.href,
        entry.etag.as_deref().unwrap_or_default(),
        entry.modified,
        entry.size.unwrap_or(0),
        THUMBNAIL_WIDTH
    )
}

fn remember_thumbnail(key: String, thumbnail: CachedThumbnail) {
    THUMBNAIL_CACHE.with(|cache| cache.borrow_mut().insert(key, thumbnail));
}

fn image_mime(entry: &Entry) -> String {
    if let Some(content_type) = entry.content_type.as_deref() {
        let content_type = content_type.split(';').next().unwrap_or_default().trim();
        if content_type.starts_with("image/") {
            return content_type.to_string();
        }
    }

    match entry
        .name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn create_blob_url(bytes: &[u8], mime: &str) -> Result<String, ()> {
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob =
        web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(|_| ())?;
    web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| ())
}
