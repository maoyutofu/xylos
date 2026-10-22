use crate::components::icons::{IconClose, IconDownload, IconFile};
use crate::i18n::I18n;
use crate::models::{Entry, Language, ServerAccount};
use crate::webdav::{download_bytes, format_date, format_size};
use dioxus::prelude::*;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewKind {
    Image,
    Pdf,
    Audio,
    Video,
    Text,
    Unsupported,
}

impl PreviewKind {
    fn from_entry(entry: &Entry) -> Self {
        let mime = entry
            .content_type
            .as_deref()
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let extension = file_extension(&entry.name);

        if mime.starts_with("image/") && mime != "image/svg+xml" {
            return Self::Image;
        }
        if mime == "image/svg+xml" {
            return Self::Image;
        }
        if mime == "application/pdf" {
            return Self::Pdf;
        }
        if mime.starts_with("audio/") {
            return Self::Audio;
        }
        if mime.starts_with("video/") {
            return Self::Video;
        }
        if mime.starts_with("text/")
            || matches!(
                mime.as_str(),
                "application/json"
                    | "application/ld+json"
                    | "application/xml"
                    | "application/javascript"
                    | "application/x-javascript"
                    | "application/x-yaml"
                    | "application/yaml"
                    | "application/toml"
            )
        {
            return Self::Text;
        }

        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "bmp" | "ico" => {
                Self::Image
            }
            "pdf" => Self::Pdf,
            "mp3" | "wav" | "ogg" | "oga" | "m4a" | "aac" | "flac" | "opus" => Self::Audio,
            "mp4" | "webm" | "ogv" | "m4v" | "mov" => Self::Video,
            "txt" | "log" | "md" | "markdown" | "json" | "xml" | "yaml" | "yml" | "toml"
            | "csv" | "tsv" | "ini" | "conf" | "cfg" | "env" | "rs" | "js" | "jsx"
            | "ts" | "tsx" | "css" | "html" | "htm" | "sh" | "bash" | "zsh" | "py"
            | "go" | "java" | "c" | "h" | "cpp" | "hpp" | "sql" | "graphql"
            | "gitignore" | "dockerignore" => Self::Text,
            _ => Self::Unsupported,
        }
    }

    fn max_size(self) -> Option<u64> {
        match self {
            Self::Text => Some(2 * MIB),
            Self::Image => Some(20 * MIB),
            Self::Pdf => Some(40 * MIB),
            Self::Audio | Self::Video => Some(64 * MIB),
            Self::Unsupported => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PreviewBody {
    Loading,
    Media { url: String, kind: PreviewKind },
    Text(String),
    Unsupported,
    TooLarge(u64),
    Error,
}

#[component]
pub fn FilePreviewModal(
    server: ServerAccount,
    entry: Entry,
    on_close: EventHandler<()>,
    on_download: EventHandler<Entry>,
) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);
    let kind = PreviewKind::from_entry(&entry);
    let mut body = use_signal(|| PreviewBody::Loading);

    let request_entry = entry.clone();
    let request_server = server.clone();
    use_effect(move || {
        let entry = request_entry.clone();
        let server = request_server.clone();
        body.set(PreviewBody::Loading);

        if kind == PreviewKind::Unsupported {
            body.set(PreviewBody::Unsupported);
            return;
        }
        if let (Some(size), Some(limit)) = (entry.size, kind.max_size()) {
            if size > limit {
                body.set(PreviewBody::TooLarge(limit));
                return;
            }
        }

        spawn(async move {
            match download_bytes(&server, &entry.href).await {
                Ok(bytes) => {
                    if kind == PreviewKind::Text {
                        match decode_text(&bytes) {
                            Ok(text) => body.set(PreviewBody::Text(text)),
                            Err(()) => body.set(PreviewBody::Unsupported),
                        }
                    } else {
                        match create_blob_url(&bytes, preview_mime(&entry, kind)) {
                            Ok(url) => body.set(PreviewBody::Media { url, kind }),
                            Err(()) => body.set(PreviewBody::Error),
                        }
                    }
                }
                Err(_) => body.set(PreviewBody::Error),
            }
        });
    });

    use_drop(move || {
        if let PreviewBody::Media { url, .. } = body() {
            revoke_blob_url(&url);
        }
    });

    let close = EventHandler::new(move |_| {
        on_close.call(());
    });
    let download_entry = entry.clone();
    let download = EventHandler::new(move |_| on_download.call(download_entry.clone()));
    let extension = file_extension(&entry.name);
    let date = format_date(&entry.modified);
    let meta = match entry.size {
        Some(size) if !date.is_empty() => format!("{} · {}", format_size(size), date),
        Some(size) => format_size(size),
        None if !date.is_empty() => date,
        None => tr.menu_preview().to_string(),
    };

    rsx! {
        div {
            class: "modal-backdrop preview-backdrop",
            onclick: move |_| close.call(()),
            div {
                class: "preview-dialog",
                onclick: move |event| event.stop_propagation(),
                div { class: "preview-header",
                    div { class: "preview-heading",
                        div { class: "preview-filename", title: "{entry.name}", "{entry.name}" }
                        div { class: "preview-meta", "{meta}" }
                    }
                    div { class: "preview-header-actions",
                        button {
                            class: "preview-action-btn",
                            title: "{tr.menu_download()}",
                            onclick: move |_| download.call(()),
                            IconDownload {}
                        }
                        button {
                            class: "preview-action-btn",
                            title: "{tr.tooltip_close_preview()}",
                            onclick: move |_| close.call(()),
                            IconClose {}
                        }
                    }
                }
                div { class: "preview-body",
                    match body() {
                        PreviewBody::Loading => rsx! {
                            div { class: "preview-status",
                                div { class: "preview-spinner" }
                                div { class: "preview-status-desc", "{tr.preview_loading()}" }
                            }
                        },
                        PreviewBody::Media { url, kind } => match kind {
                            PreviewKind::Image => rsx! {
                                img {
                                    class: "preview-image",
                                    src: "{url}",
                                    alt: "{entry.name}",
                                }
                            },
                            PreviewKind::Pdf => rsx! {
                                iframe {
                                    class: "preview-pdf",
                                    src: "{url}",
                                    title: "{entry.name}",
                                }
                            },
                            PreviewKind::Video => rsx! {
                                video {
                                    class: "preview-video",
                                    src: "{url}",
                                    controls: true,
                                    preload: "metadata",
                                }
                            },
                            PreviewKind::Audio => rsx! {
                                div { class: "preview-audio-wrap",
                                    div { class: "preview-file-icon", IconFile { is_large: true } }
                                    audio {
                                        class: "preview-audio",
                                        src: "{url}",
                                        controls: true,
                                        preload: "metadata",
                                    }
                                }
                            },
                            _ => rsx! {},
                        },
                        PreviewBody::Text(text) => rsx! {
                            pre { class: "preview-text", "{text}" }
                        },
                        PreviewBody::Unsupported => rsx! {
                            div { class: "preview-status preview-fallback",
                                div { class: "preview-file-icon", IconFile { is_large: true } }
                                div { class: "preview-status-title", "{tr.preview_unsupported_title()}" }
                                div { class: "preview-status-desc", "{tr.preview_unsupported_desc(&extension)}" }
                                button {
                                    class: "btn-primary preview-download-button",
                                    onclick: move |_| download.call(()),
                                    IconDownload {}
                                    span { "{tr.preview_download_file()}" }
                                }
                            }
                        },
                        PreviewBody::TooLarge(limit) => rsx! {
                            div { class: "preview-status preview-fallback",
                                div { class: "preview-file-icon", IconFile { is_large: true } }
                                div { class: "preview-status-title", "{tr.preview_too_large_title()}" }
                                div { class: "preview-status-desc", "{tr.preview_too_large_desc(&format_size(limit))}" }
                                button {
                                    class: "btn-primary preview-download-button",
                                    onclick: move |_| download.call(()),
                                    IconDownload {}
                                    span { "{tr.preview_download_file()}" }
                                }
                            }
                        },
                        PreviewBody::Error => rsx! {
                            div { class: "preview-status preview-fallback",
                                div { class: "preview-file-icon", IconFile { is_large: true } }
                                div { class: "preview-status-title", "{tr.preview_error_title()}" }
                                div { class: "preview-status-desc", "{tr.preview_error_desc()}" }
                                button {
                                    class: "btn-primary preview-download-button",
                                    onclick: move |_| download.call(()),
                                    IconDownload {}
                                    span { "{tr.preview_download_file()}" }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

fn file_extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .filter(|extension| !extension.is_empty())
        .unwrap_or_default()
}

fn preview_mime(entry: &Entry, kind: PreviewKind) -> &str {
    if let Some(content_type) = entry.content_type.as_deref() {
        let content_type = content_type.split(';').next().unwrap_or_default().trim();
        if !content_type.is_empty() && content_type != "application/octet-stream" {
            return content_type;
        }
    }

    match file_extension(&entry.name).as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "opus" => "audio/opus",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        _ if kind == PreviewKind::Video => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn decode_text(bytes: &[u8]) -> Result<String, ()> {
    if let Some(rest) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return Ok(String::from_utf8_lossy(rest).into_owned());
    }
    if let Some(rest) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return Ok(decode_utf16(rest, true));
    }
    if let Some(rest) = bytes.strip_prefix(&[0xfe, 0xff]) {
        return Ok(decode_utf16(rest, false));
    }

    if bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Err(());
    }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect::<Vec<_>>();
    String::from_utf16_lossy(&units)
}

fn create_blob_url(bytes: &[u8], mime: &str) -> Result<String, ()> {
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|_| ())?;
    web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| ())
}

fn revoke_blob_url(url: &str) {
    let _ = web_sys::Url::revoke_object_url(url);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, content_type: Option<&str>) -> Entry {
        Entry {
            name: name.into(),
            href: format!("/{name}"),
            is_dir: false,
            size: Some(1024),
            modified: String::new(),
            etag: None,
            content_type: content_type.map(str::to_owned),
        }
    }

    #[test]
    fn detects_preview_kind_from_extension() {
        assert_eq!(
            PreviewKind::from_entry(&entry("photo.jpg", None)),
            PreviewKind::Image
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("manual.pdf", None)),
            PreviewKind::Pdf
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("song.flac", None)),
            PreviewKind::Audio
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("clip.webm", None)),
            PreviewKind::Video
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("notes.md", None)),
            PreviewKind::Text
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("archive.zip", None)),
            PreviewKind::Unsupported
        );
    }

    #[test]
    fn prefers_mime_type_when_extension_is_unhelpful() {
        assert_eq!(
            PreviewKind::from_entry(&entry("download", Some("image/png"))),
            PreviewKind::Image
        );
        assert_eq!(
            PreviewKind::from_entry(&entry("document", Some("application/pdf"))),
            PreviewKind::Pdf
        );
    }

    #[test]
    fn decodes_utf8_and_rejects_binary_text() {
        assert_eq!(
            decode_text("hello 世界".as_bytes()),
            Ok("hello 世界".into())
        );
        assert!(decode_text(&[0x68, 0x00, 0x69, 0x00]).is_err());
    }
}
