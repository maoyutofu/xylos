use crate::components::icons::IconImage;
use crate::models::{Entry, ServerAccount};
use crate::webdav::{download_bytes, is_online};
use dioxus::prelude::*;

const MAX_THUMBNAIL_BYTES: u64 = 20 * 1024 * 1024;

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
    let request_server = server.clone();
    let request_entry = entry.clone();

    use_effect(move || {
        let server = request_server.clone();
        let entry = request_entry.clone();

        if !is_online() || entry.size.is_some_and(|size| size > MAX_THUMBNAIL_BYTES) {
            return;
        }

        spawn(async move {
            let Ok(bytes) = download_bytes(&server, &entry.href).await else {
                return;
            };
            let Ok(url) = create_blob_url(&bytes, &image_mime(&entry)) else {
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
        rsx! { IconImage { is_large: grid } }
    }
}

fn image_mime(entry: &Entry) -> String {
    if let Some(content_type) = entry.content_type.as_deref() {
        let content_type = content_type.split(';').next().unwrap_or_default().trim();
        if content_type.starts_with("image/") && content_type != "image/svg+xml" {
            return content_type.to_string();
        }
        if content_type == "image/svg+xml" {
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
