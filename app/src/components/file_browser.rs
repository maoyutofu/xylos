use crate::components::icons::{
    IconArrowBack, IconDownload, IconFile, IconFolder, IconGrid, IconImage, IconList, IconMore,
    IconNewFolder, IconPencil, IconRefresh, IconSort, IconTrash, IconUpload,
};
use crate::i18n::I18n;
use crate::models::{Entry, Language, ServerAccount, SortField, ViewMode};
use crate::webdav::{
    download_bytes, execute_write, format_date, format_size, is_online, join_path, list_directory,
    parent_path,
};
use dioxus::prelude::*;

fn is_image_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".gif")
        || lower.ends_with(".svg")
}

#[component]
pub fn FileBrowser(
    server: ServerAccount,
    on_back_to_servers: EventHandler<()>,
    on_notify: EventHandler<String>,
    on_error: EventHandler<String>,
) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);

    let server_sig = use_signal(|| server.clone());
    let mut current_path = use_signal(|| "/".to_string());
    let mut entries = use_signal(Vec::<Entry>::new);
    let mut loading = use_signal(|| false);
    let mut view_mode = use_signal(|| ViewMode::List);
    let mut sort_field = use_signal(|| SortField::Name);
    let mut sort_asc = use_signal(|| true);
    let mut active_menu = use_signal(|| None::<String>);

    // Modals
    let mut new_folder_open = use_signal(|| false);
    let mut new_folder_name = use_signal(String::new);

    let mut rename_target = use_signal(|| None::<Entry>);
    let mut rename_name = use_signal(String::new);
    let mut delete_target = use_signal(|| None::<Entry>);

    let load_dir = move |path: String| {
        let mut menu_sig = active_menu;
        menu_sig.set(None);
        let srv = server_sig();
        spawn(async move {
            loading.set(true);
            match list_directory(&srv, &path).await {
                Ok(items) => {
                    entries.set(items);
                    current_path.set(path);
                }
                Err(err) => on_error.call(err),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_dir("/".to_string());
    });

    let handle_upload = move |evt: FormEvent| {
        let Some(file) = evt.files().into_iter().next() else { return };
        let srv = server_sig();
        let path = current_path();
        spawn(async move {
            loading.set(true);
            let name = file.name();
            let bytes = match file.read_bytes().await {
                Ok(b) => b,
                Err(_) => {
                    on_error.call(tr.notify_read_failed().into());
                    loading.set(false);
                    return;
                }
            };
            let content_type = file.content_type();
            let dest_path = join_path(&path, &name);
            match execute_write(&srv, reqwest::Method::PUT, &dest_path, Some(bytes.to_vec()), content_type, None, None).await {
                Ok(()) => {
                    on_notify.call(tr.notify_uploaded(&name));
                    load_dir(path);
                }
                Err(err) => on_error.call(err),
            }
            loading.set(false);
        });
    };

    let handle_download = move |entry: Entry| {
        let srv = server_sig();
        spawn(async move {
            match download_bytes(&srv, &entry.href).await {
                Ok(bytes) => {
                    let encoded = base64_encode(&bytes);
                    let safe_name = serde_json::to_string(&entry.name).unwrap_or_else(|_| "'file'".into());
                    let script = format!(
                        "const a=document.createElement('a');a.href='data:application/octet-stream;base64,{encoded}';a.download={safe_name};a.click();"
                    );
                    let _ = dioxus::document::eval(&script);
                    on_notify.call(tr.notify_downloading(&entry.name));
                }
                Err(err) => on_error.call(err),
            }
        });
    };

    let handle_create_folder = move || {
        let name = new_folder_name().trim().to_string();
        if name.is_empty() {
            on_error.call(tr.err_folder_name_empty().into());
            return;
        }
        let srv = server_sig();
        let path = current_path();
        let full_path = join_path(&path, &name);
        spawn(async move {
            loading.set(true);
            match execute_write(&srv, reqwest::Method::from_bytes(b"MKCOL").unwrap(), &full_path, None, None, None, None).await {
                Ok(()) => {
                    new_folder_open.set(false);
                    new_folder_name.set(String::new());
                    on_notify.call(tr.notify_folder_created().into());
                    load_dir(path);
                }
                Err(err) => on_error.call(err),
            }
            loading.set(false);
        });
    };

    let handle_rename = move || {
        let Some(target) = rename_target() else { return };
        let new_n = rename_name().trim().to_string();
        if new_n.is_empty() {
            on_error.call(tr.err_new_name_empty().into());
            return;
        }
        let srv = server_sig();
        let parent = current_path();
        let dest = join_path(&parent, &new_n);
        spawn(async move {
            loading.set(true);
            match execute_write(&srv, reqwest::Method::from_bytes(b"MOVE").unwrap(), &target.href, None, None, Some(dest), target.etag).await {
                Ok(()) => {
                    rename_target.set(None);
                    rename_name.set(String::new());
                    on_notify.call(tr.notify_renamed().into());
                    load_dir(parent);
                }
                Err(err) => on_error.call(err),
            }
            loading.set(false);
        });
    };

    let handle_delete = move || {
        let Some(target) = delete_target() else { return };
        let srv = server_sig();
        let parent = current_path();
        spawn(async move {
            loading.set(true);
            match execute_write(&srv, reqwest::Method::DELETE, &target.href, None, None, None, target.etag).await {
                Ok(()) => {
                    delete_target.set(None);
                    on_notify.call(tr.notify_deleted(&target.name));
                    load_dir(parent);
                }
                Err(err) => on_error.call(err),
            }
            loading.set(false);
        });
    };

    // Sort items
    let mut sorted_entries = entries().clone();
    sorted_entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            return b.is_dir.cmp(&a.is_dir); // Folders first
        }
        let cmp = match sort_field() {
            SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortField::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
            SortField::Modified => a.modified.cmp(&b.modified),
            SortField::Type => a.is_dir.cmp(&b.is_dir),
        };
        if sort_asc() { cmp } else { cmp.reverse() }
    });

    rsx! {
        div { class: "file-browser",
            // Header Top
            div { class: "browser-header",
                div { class: "browser-title-wrap",
                    h2 { class: "browser-title", "{server.name} · {tr.files_title_suffix()}" }
                }
                div { class: "browser-actions",
                    // Sort toggle
                    button {
                        class: "tool-btn",
                        title: "{tr.tooltip_sort()}",
                        onclick: move |_| {
                            let next = match sort_field() {
                                SortField::Name => SortField::Modified,
                                SortField::Modified => SortField::Size,
                                SortField::Size => SortField::Name,
                                SortField::Type => SortField::Name,
                            };
                            sort_field.set(next);
                            sort_asc.set(!sort_asc());
                        },
                        IconSort {}
                    }
                    // View mode toggle
                    button {
                        class: "tool-btn",
                        title: if view_mode() == ViewMode::List { tr.tooltip_grid_view() } else { tr.tooltip_list_view() },
                        onclick: move |_| {
                            view_mode.set(if view_mode() == ViewMode::List { ViewMode::Grid } else { ViewMode::List });
                        },
                        if view_mode() == ViewMode::List { IconGrid {} } else { IconList {} }
                    }
                    // Upload button
                    label {
                        class: "tool-btn",
                        title: "{tr.tooltip_upload()}",
                        style: "cursor: pointer;",
                        IconUpload {}
                        input {
                            r#type: "file",
                            style: "display: none;",
                            disabled: !is_online() || loading(),
                            onchange: handle_upload,
                        }
                    }
                    // New folder button
                    button {
                        class: "tool-btn",
                        title: "{tr.tooltip_new_folder()}",
                        disabled: !is_online() || loading(),
                        onclick: move |_| new_folder_open.set(true),
                        IconNewFolder {}
                    }
                }
            }

            // Path & Navigation bar
            div { class: "path-bar",
                button {
                    class: "tool-btn",
                    title: "{tr.tooltip_back()}",
                    disabled: current_path() == "/",
                    onclick: move |_| {
                        let p = parent_path(&current_path());
                        load_dir(p);
                    },
                    IconArrowBack {}
                }
                button {
                    class: "tool-btn",
                    title: "{tr.tooltip_refresh()}",
                    disabled: loading(),
                    onclick: move |_| load_dir(current_path()),
                    IconRefresh {}
                }
                input {
                    class: "path-input",
                    value: "{current_path()}",
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            load_dir(current_path());
                        }
                    },
                    oninput: move |e| current_path.set(e.value()),
                }
            }

            // Main files viewport
            if loading() && sorted_entries.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-desc", "{tr.loading_files()}" }
                }
            } else if sorted_entries.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-icon", IconFolder { is_large: true } }
                    div { class: "empty-title", "{tr.empty_directory_title()}" }
                    div { class: "empty-desc", "{tr.empty_directory_desc()}" }
                }
            } else if view_mode() == ViewMode::List {
                div { class: "file-list-view",
                    for item in sorted_entries.into_iter() {
                        {
                            let item_click = item.clone();
                            let item_download = item.clone();
                            let item_rename = item.clone();
                            let item_delete = item.clone();
                            let href = item.href.clone();
                            let is_dir = item.is_dir;
                            let is_img = !is_dir && is_image_name(&item.name);
                            let is_menu_open = active_menu() == Some(item.href.clone());
                            let item_href_for_menu = item.href.clone();
                            let formatted_date = format_date(&item.modified);
                            let meta_text = if is_dir {
                                if formatted_date.is_empty() {
                                    tr.folder_tag().to_string()
                                } else {
                                    tr.folder_tag_with_date(&formatted_date)
                                }
                            } else {
                                let sz = format_size(item.size.unwrap_or(0));
                                if formatted_date.is_empty() {
                                    sz
                                } else {
                                    format!("{sz} · {formatted_date}")
                                }
                            };

                            rsx! {
                                div {
                                    key: "{item.href}",
                                    class: "file-list-row",
                                    onclick: move |_| {
                                        if active_menu().is_some() {
                                            active_menu.set(None);
                                        } else if is_dir {
                                            load_dir(href.clone());
                                        } else {
                                            handle_download(item_click.clone());
                                        }
                                    },
                                    div {
                                        class: if is_dir { "file-icon-box folder" } else if is_img { "file-icon-box image" } else { "file-icon-box file" },
                                        if is_dir {
                                            IconFolder { is_large: false }
                                        } else if is_img {
                                            IconImage {}
                                        } else {
                                            IconFile { is_large: false }
                                        }
                                    }
                                    div { class: "file-info-box",
                                        div { class: "file-name", title: "{item.name}", "{item.name}" }
                                        div { class: "file-meta", "{meta_text}" }
                                    }
                                    div { class: "file-actions-box",
                                        button {
                                            class: if is_menu_open { "file-more-btn active" } else { "file-more-btn" },
                                            title: "{tr.tooltip_actions_menu()}",
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                if active_menu() == Some(item_href_for_menu.clone()) {
                                                    active_menu.set(None);
                                                } else {
                                                    active_menu.set(Some(item_href_for_menu.clone()));
                                                }
                                            },
                                            IconMore {}
                                        }
                                        if is_menu_open {
                                            div {
                                                class: "file-popover-menu",
                                                onclick: move |e| e.stop_propagation(),
                                                if !is_dir {
                                                    button {
                                                        class: "popover-item",
                                                        onclick: move |_| {
                                                            active_menu.set(None);
                                                            handle_download(item_download.clone());
                                                        },
                                                        IconDownload {}
                                                        span { "{tr.menu_download()}" }
                                                    }
                                                }
                                                button {
                                                    class: "popover-item",
                                                    onclick: move |_| {
                                                        active_menu.set(None);
                                                        rename_name.set(item_rename.name.clone());
                                                        rename_target.set(Some(item_rename.clone()));
                                                    },
                                                    IconPencil {}
                                                    span { "{tr.menu_rename()}" }
                                                }
                                                button {
                                                    class: "popover-item danger",
                                                    onclick: move |_| {
                                                        active_menu.set(None);
                                                        delete_target.set(Some(item_delete.clone()));
                                                    },
                                                    IconTrash {}
                                                    span { "{tr.menu_delete()}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {

                // Grid View
                div { class: "file-grid",
                    for item in sorted_entries.into_iter() {
                        {
                            let item_click = item.clone();
                            let item_rename = item.clone();
                            let item_delete = item.clone();
                            let is_dir = item.is_dir;
                            let href = item.href.clone();

                            rsx! {
                                div {
                                    key: "{item.href}",
                                    class: "grid-card",
                                    onclick: move |_| {
                                        if is_dir {
                                            load_dir(href.clone());
                                        } else {
                                            handle_download(item_click.clone());
                                        }
                                    },
                                    div {
                                        class: if is_dir { "grid-card-preview is-folder" } else { "grid-card-preview" },
                                        if is_dir {
                                            IconFolder { is_large: true }
                                        } else {
                                            IconFile { is_large: true }
                                        }
                                    }
                                    div { class: "grid-card-body",
                                        div { class: "grid-card-info",
                                            div { class: "grid-card-title", "{item.name}" }
                                            div { class: "grid-card-sub",
                                                if is_dir { "{tr.folder_tag()}" } else { "{format_size(item.size.unwrap_or(0))}" }
                                            }
                                        }
                                        div {
                                            style: "display: flex; gap: 4px;",
                                            onclick: move |e| e.stop_propagation(),
                                            button {
                                                class: "tool-btn",
                                                style: "width: 28px; height: 28px;",
                                                title: "{tr.menu_rename()}",
                                                onclick: move |_| {
                                                    rename_name.set(item_rename.name.clone());
                                                    rename_target.set(Some(item_rename.clone()));
                                                },
                                                IconPencil {}
                                            }
                                            button {
                                                class: "tool-btn danger",
                                                style: "width: 28px; height: 28px;",
                                                title: "{tr.menu_delete()}",
                                                onclick: move |_| delete_target.set(Some(item_delete.clone())),
                                                IconTrash {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Popover Dismiss Backdrop
            if active_menu().is_some() {
                div {
                    class: "popover-backdrop",
                    onclick: move |_| active_menu.set(None),
                }
            }

            // New Folder Modal
            if new_folder_open() {
                div { class: "modal-backdrop",
                    onclick: move |_| new_folder_open.set(false),
                    div { class: "modal-dialog",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "modal-header", "{tr.modal_new_folder_title()}" }
                        div { class: "modal-body",
                            div { class: "form-field",
                                label { class: "form-label", "{tr.modal_new_folder_label()}" }
                                input {
                                    class: "form-input",
                                    placeholder: "{tr.modal_new_folder_placeholder()}",
                                    value: "{new_folder_name()}",
                                    oninput: move |e| new_folder_name.set(e.value()),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter {
                                            handle_create_folder();
                                        }
                                    },
                                }
                            }
                        }
                        div { class: "modal-footer",
                            button { class: "btn-secondary", onclick: move |_| new_folder_open.set(false), "{tr.cancel()}" }
                            button { class: "btn-primary", onclick: move |_| handle_create_folder(), "{tr.confirm()}" }
                        }
                    }
                }
            }

            // Rename Modal
            if let Some(target) = rename_target() {
                div { class: "modal-backdrop",
                    onclick: move |_| rename_target.set(None),
                    div { class: "modal-dialog",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "modal-header", "{tr.modal_rename_title()}" }
                        div { class: "modal-body",
                            div { class: "form-field",
                                label { class: "form-label", "{tr.modal_rename_label(&target.name)}" }
                                input {
                                    class: "form-input",
                                    placeholder: "{tr.modal_rename_placeholder()}",
                                    value: "{rename_name()}",
                                    oninput: move |e| rename_name.set(e.value()),
                                    onkeydown: move |e| {
                                        if e.key() == Key::Enter {
                                            handle_rename();
                                        }
                                    },
                                }
                            }
                        }
                        div { class: "modal-footer",
                            button { class: "btn-secondary", onclick: move |_| rename_target.set(None), "{tr.cancel()}" }
                            button { class: "btn-primary", onclick: move |_| handle_rename(), "{tr.save()}" }
                        }
                    }
                }
            }

            // Delete Confirm Modal
            if let Some(target) = delete_target() {
                div { class: "modal-backdrop",
                    onclick: move |_| delete_target.set(None),
                    div { class: "modal-dialog",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "modal-header", "{tr.modal_delete_confirm_title()}" }
                        div { class: "modal-body",
                            p { style: "font-size: 14px; color: var(--app-text); line-height: 1.6;",
                                if target.is_dir {
                                    "{tr.modal_delete_folder_desc(&target.name)}"
                                } else {
                                    "{tr.modal_delete_file_desc(&target.name)}"
                                }
                            }
                        }
                        div { class: "modal-footer",
                            button { class: "btn-secondary", onclick: move |_| delete_target.set(None), "{tr.cancel()}" }
                            button {
                                class: "btn-primary btn-danger",
                                onclick: move |_| handle_delete(),
                                "{tr.delete()}"
                            }
                        }
                    }
                }
            }

        }
    }
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
