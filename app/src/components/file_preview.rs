use crate::components::icons::{
    IconChevronLeft, IconChevronRight, IconClose, IconDownload, IconFile, IconImage,
};
use crate::i18n::I18n;
use crate::models::{Entry, Language, ServerAccount};
use crate::webdav::{download_bytes, format_date, format_size, get_direct_ticket_url};
use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const SWIPE_THRESHOLD: f64 = 52.0;

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
            Self::Image | Self::Pdf | Self::Audio | Self::Video | Self::Unsupported => None,
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

fn image_gallery(entries: &[Entry], selected: &Entry) -> Vec<Entry> {
    let mut images = entries
        .iter()
        .filter(|entry| !entry.is_dir && PreviewKind::from_entry(entry) == PreviewKind::Image)
        .cloned()
        .collect::<Vec<_>>();
    if !images.iter().any(|entry| entry.href == selected.href)
        && PreviewKind::from_entry(selected) == PreviewKind::Image
    {
        images.insert(0, selected.clone());
    }
    images
}

fn gallery_index(images: &[Entry], selected: &Entry) -> usize {
    images
        .iter()
        .position(|entry| entry.href == selected.href)
        .unwrap_or(0)
}

fn shifted_index(current: usize, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    if delta < 0 {
        current.checked_sub(delta.unsigned_abs())
    } else {
        let next = current.checked_add(delta as usize)?;
        (next < len).then_some(next)
    }
}

fn swipe_direction(delta_x: f64, delta_y: f64) -> Option<isize> {
    if delta_x.abs() < SWIPE_THRESHOLD || delta_x.abs() <= delta_y.abs() {
        return None;
    }
    Some(if delta_x < 0.0 { 1 } else { -1 })
}

#[component]
pub fn FilePreviewModal(
    server: ServerAccount,
    entry: Entry,
    directory_entries: Vec<Entry>,
    on_close: EventHandler<()>,
    on_download: EventHandler<Entry>,
    #[props(default)] on_notify: Option<EventHandler<String>>,
) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);
    let images = image_gallery(&directory_entries, &entry);
    let image_count = images.len();
    let initial_index = gallery_index(&images, &entry);
    let mut current_index = use_signal(|| initial_index);
    let mut body = use_signal(|| PreviewBody::Loading);
    let mut active_blob_url = use_signal(|| None::<String>);
    let mut drag_start = use_signal(|| None::<(i32, f64, f64)>);
    let mut drag_offset = use_signal(|| 0.0_f64);
    let mut dragging = use_signal(|| false);
    let request_version = use_hook(|| Rc::new(Cell::new(0_u32)));

    let request_images = images.clone();
    let request_fallback = entry.clone();
    let request_version_for_effect = request_version.clone();
    let effect_server = server.clone();
    use_effect(move || {
        let index = current_index();
        let selected_entry = request_images
            .get(index)
            .cloned()
            .unwrap_or_else(|| request_fallback.clone());
        let kind = PreviewKind::from_entry(&selected_entry);
        let version = request_version_for_effect.get().wrapping_add(1);
        request_version_for_effect.set(version);
        body.set(PreviewBody::Loading);

        if kind == PreviewKind::Unsupported {
            body.set(PreviewBody::Unsupported);
            return;
        }
        if let (Some(size), Some(limit)) = (selected_entry.size, kind.max_size()) {
            if size > limit {
                body.set(PreviewBody::TooLarge(limit));
                return;
            }
        }

        let server = effect_server.clone();
        let request_version = request_version_for_effect.clone();
        spawn(async move {
            if matches!(kind, PreviewKind::Image | PreviewKind::Pdf | PreviewKind::Video | PreviewKind::Audio) {
                if let Ok(direct_url) = get_direct_ticket_url(&server, &selected_entry.href, true, Some(7200)).await {
                    if request_version.get() != version {
                        return;
                    }
                    if let Some(previous_url) = active_blob_url() {
                        revoke_blob_url(&previous_url);
                        active_blob_url.set(None);
                    }
                    body.set(PreviewBody::Media { url: direct_url, kind });
                    return;
                }
            }

            let result = download_bytes(&server, &selected_entry.href).await;
            if request_version.get() != version {
                return;
            }

            match result {
                Ok(bytes) => {
                    if kind == PreviewKind::Text {
                        match decode_text(&bytes) {
                            Ok(text) => body.set(PreviewBody::Text(text)),
                            Err(()) => body.set(PreviewBody::Unsupported),
                        }
                    } else {
                        match create_blob_url(&bytes, preview_mime(&selected_entry, kind)) {
                            Ok(url) => {
                                if request_version.get() != version {
                                    revoke_blob_url(&url);
                                    return;
                                }
                                if let Some(previous_url) = active_blob_url() {
                                    revoke_blob_url(&previous_url);
                                }
                                active_blob_url.set(Some(url.clone()));
                                body.set(PreviewBody::Media { url, kind });
                            }
                            Err(()) => body.set(PreviewBody::Error),
                        }
                    }
                }
                Err(_) => body.set(PreviewBody::Error),
            }
        });
    });

    use_drop(move || {
        if let Some(url) = active_blob_url() {
            revoke_blob_url(&url);
        }
    });

    let navigation_images = images.clone();
    let navigate = EventHandler::new(move |delta: isize| {
        let Some(next) = shifted_index(current_index(), navigation_images.len(), delta) else {
            return;
        };
        if let Some(url) = active_blob_url() {
            revoke_blob_url(&url);
            active_blob_url.set(None);
        }
        body.set(PreviewBody::Loading);
        current_index.set(next);
    });

    let close = EventHandler::new(move |_| on_close.call(()));
    let download_images = images.clone();
    let download_fallback = entry.clone();
    let download = EventHandler::new(move |_| {
        let selected_entry = download_images
            .get(current_index())
            .cloned()
            .unwrap_or_else(|| download_fallback.clone());
        on_download.call(selected_entry);
    });

    let selected_entry = images
        .get(current_index())
        .cloned()
        .unwrap_or_else(|| entry.clone());
    let kind = PreviewKind::from_entry(&selected_entry);
    let extension = file_extension(&selected_entry.name);
    let date = format_date(&selected_entry.modified);
    let meta = match selected_entry.size {
        Some(size) if !date.is_empty() => format!("{} · {}", format_size(size), date),
        Some(size) => format_size(size),
        None if !date.is_empty() => date,
        None => tr.menu_preview().to_string(),
    };
    let is_image = kind == PreviewKind::Image;
    let has_gallery = is_image && image_count > 1;
    let position = current_index().min(image_count.saturating_sub(1)) + 1;
    let body_class = if is_image {
        "preview-body preview-image-body"
    } else {
        "preview-body"
    };
    let stage_class = if dragging() {
        "preview-image-stage is-dragging"
    } else {
        "preview-image-stage"
    };
    let image_style = format!("transform: translate3d({}px, 0, 0);", drag_offset());

    rsx! {
        div {
            class: "modal-backdrop preview-backdrop",
            onclick: move |_| close.call(()),
            div {
                class: "preview-dialog",
                tabindex: "0",
                role: "dialog",
                aria_modal: "true",
                onclick: move |event| event.stop_propagation(),
                onmounted: move |event| {
                    spawn(async move {
                        let _ = event.set_focus(true).await;
                    });
                },
                onkeydown: move |event| {
                    match event.key() {
                        Key::Escape => {
                            event.prevent_default();
                            close.call(());
                        }
                        Key::ArrowLeft if has_gallery => {
                            event.prevent_default();
                            navigate.call(-1);
                        }
                        Key::ArrowRight if has_gallery => {
                            event.prevent_default();
                            navigate.call(1);
                        }
                        _ => {}
                    }
                },
                div { class: "preview-header",
                    div { class: "preview-heading",
                        div { class: "preview-heading-top",
                            div { class: "preview-kind-badge",
                                if is_image {
                                    IconImage { is_large: false }
                                } else {
                                    IconFile { is_large: false }
                                }
                            }
                            div { class: "preview-heading-copy",
                                div {
                                    class: "preview-filename",
                                    title: "{selected_entry.name}",
                                    "{selected_entry.name}"
                                }
                                div { class: "preview-meta", "{meta}" }
                            }
                        }
                    }
                    div { class: "preview-header-actions",
                        if has_gallery {
                            span { class: "preview-position-pill", "{position} / {image_count}" }
                        }
                        button {
                            class: "preview-action-btn",
                            title: "{tr.menu_download()}",
                            aria_label: "{tr.menu_download()}",
                            onclick: move |_| download.call(()),
                            IconDownload {}
                        }
                        button {
                            class: "preview-action-btn preview-close-btn",
                            title: "{tr.tooltip_close_preview()}",
                            aria_label: "{tr.tooltip_close_preview()}",
                            onclick: move |_| close.call(()),
                            IconClose {}
                        }
                    }
                }
                div {
                    class: "{body_class}",
                    onpointerdown: move |event| {
                        if !is_image {
                            return;
                        }
                        let point = event.client_coordinates();
                        drag_start.set(Some((event.pointer_id(), point.x, point.y)));
                        drag_offset.set(0.0);
                        dragging.set(false);
                    },
                    onpointermove: move |event| {
                        if !is_image {
                            return;
                        }
                        let Some((pointer_id, start_x, start_y)) = drag_start() else {
                            return;
                        };
                        if pointer_id != event.pointer_id() {
                            return;
                        }
                        let point = event.client_coordinates();
                        let delta_x = point.x - start_x;
                        let delta_y = point.y - start_y;
                        if !dragging() {
                            if delta_x.abs() < 6.0 || delta_x.abs() <= delta_y.abs() {
                                return;
                            }
                            dragging.set(true);
                        }
                        event.prevent_default();
                        drag_offset.set(delta_x.clamp(-180.0, 180.0));
                    },
                    onpointerup: move |event| {
                        if !is_image || drag_start().map(|state| state.0) != Some(event.pointer_id()) {
                            return;
                        }
                        let delta_x = drag_offset();
                        let was_dragging = dragging();
                        drag_start.set(None);
                        dragging.set(false);
                        drag_offset.set(0.0);
                        if was_dragging {
                            if let Some(delta) = swipe_direction(delta_x, 0.0) {
                                navigate.call(delta);
                            }
                        }
                    },
                    onpointercancel: move |_| {
                        drag_start.set(None);
                        dragging.set(false);
                        drag_offset.set(0.0);
                    },
                    match body() {
                        PreviewBody::Loading => rsx! {
                            div { class: "preview-status",
                                div { class: "preview-spinner" }
                                div { class: "preview-status-desc", "{tr.preview_loading()}" }
                            }
                        },
                        PreviewBody::Media { url, kind } => match kind {
                            PreviewKind::Image => rsx! {
                                div { class: stage_class,
                                    img {
                                        class: "preview-image",
                                        src: "{url}",
                                        alt: "{selected_entry.name}",
                                        draggable: false,
                                        style: "{image_style}",
                                    }
                                }
                            },
                            PreviewKind::Pdf => rsx! {
                                iframe {
                                    class: "preview-pdf",
                                    src: "{url}",
                                    title: "{selected_entry.name}",
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
                                div {
                                    class: "preview-status-desc",
                                    "{tr.preview_too_large_desc(&format_size(limit))}"
                                }
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
                    if has_gallery {
                        button {
                            class: "preview-gallery-btn previous",
                            title: "{tr.preview_previous_image()}",
                            aria_label: "{tr.preview_previous_image()}",
                            disabled: current_index() == 0,
                            onpointerdown: move |event| event.stop_propagation(),
                            onclick: move |_| navigate.call(-1),
                            IconChevronLeft {}
                        }
                        button {
                            class: "preview-gallery-btn next",
                            title: "{tr.preview_next_image()}",
                            aria_label: "{tr.preview_next_image()}",
                            disabled: current_index() + 1 >= image_count,
                            onpointerdown: move |event| event.stop_propagation(),
                            onclick: move |_| navigate.call(1),
                            IconChevronRight {}
                        }
                        div { class: "preview-swipe-hint", "{tr.preview_swipe_hint()}" }
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

    #[test]
    fn builds_image_gallery_in_display_order() {
        let folder = Entry {
            name: "album".into(),
            href: "/album".into(),
            is_dir: true,
            ..Entry::default()
        };
        let entries = vec![
            folder,
            entry("first.jpg", None),
            entry("notes.txt", None),
            entry("second.png", None),
        ];

        let images = image_gallery(&entries, &entries[3]);

        assert_eq!(
            images
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first.jpg", "second.png"]
        );
        assert_eq!(gallery_index(&images, &entries[3]), 1);
    }

    #[test]
    fn keeps_gallery_navigation_within_bounds() {
        assert_eq!(shifted_index(1, 3, 1), Some(2));
        assert_eq!(shifted_index(1, 3, -1), Some(0));
        assert_eq!(shifted_index(0, 3, -1), None);
        assert_eq!(shifted_index(2, 3, 1), None);
        assert_eq!(shifted_index(0, 0, 1), None);
    }

    #[test]
    fn detects_horizontal_swipes_only() {
        assert_eq!(swipe_direction(-80.0, 20.0), Some(1));
        assert_eq!(swipe_direction(80.0, 20.0), Some(-1));
        assert_eq!(swipe_direction(30.0, 5.0), None);
        assert_eq!(swipe_direction(80.0, 90.0), None);
    }
}
