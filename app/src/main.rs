use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use std::collections::HashMap;

mod components;
mod i18n;
mod models;
mod webdav;

use components::file_browser::FileBrowser;
use components::pwa_guide::{PwaGuideModal, PwaInstallBanner};
use components::server_modal::ServerModal;
use components::servers_page::ServersPage;
use components::settings_page::SettingsPage;
use components::sidebar::Sidebar;
use i18n::I18n;

use models::{
    AuthType, Language, NavTab, ServerAccount, ACTIVE_SERVER_ID_KEY, LANGUAGE_STORAGE_KEY,
    LEGACY_CONNECTION_KEY, SERVERS_STORAGE_KEY,
};
use webdav::test_server_connection;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let lang = use_signal(load_initial_language);
    provide_context(lang);

    let mut servers = use_signal(load_initial_servers);
    let mut active_server = use_signal(|| -> Option<ServerAccount> {
        let list = servers();
        if let Ok(active_id) = LocalStorage::get::<String>(ACTIVE_SERVER_ID_KEY) {
            list.into_iter().find(|s| s.id == active_id)
        } else {
            None
        }
    });
    let mut current_tab = use_signal(|| NavTab::Servers);
    let mut modal_server = use_signal(|| None::<Option<ServerAccount>>);
    let mut testing_server_id = use_signal(|| None::<String>);
    let mut test_results = use_signal(HashMap::<String, bool>::new);
    let mut toast_message = use_signal(|| None::<(String, bool)>);
    let mut toast_version = use_signal(|| 0u32);
    let mut pwa_guide_open = use_signal(|| false);

    let mut show_toast = move |msg: String, is_err: bool| {
        let next_v = toast_version() + 1;
        toast_version.set(next_v);
        toast_message.set(Some((msg, is_err)));

        let delay_ms = if is_err { 4000 } else { 2800 };
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(delay_ms).await;
            if toast_version() == next_v {
                toast_message.set(None);
            }
        });
    };

    let handle_select_server = move |srv: ServerAccount| {
        let _ = LocalStorage::set(ACTIVE_SERVER_ID_KEY, &srv.id);
        active_server.set(Some(srv));
        current_tab.set(NavTab::Servers);
    };

    let mut show_toast_test = show_toast;
    let handle_test_server = move |srv: ServerAccount| {
        let id = srv.id.clone();
        testing_server_id.set(Some(id.clone()));
        spawn(async move {
            let res = test_server_connection(&srv).await;
            let ok = res.is_ok();
            test_results.write().insert(id.clone(), ok);
            testing_server_id.set(None);
            match res {
                Ok(()) => show_toast_test(I18n::new(lang()).toast_server_connected(&srv.name), false),
                Err(err) => show_toast_test(I18n::new(lang()).toast_conn_failed(&err), true),
            }
        });
    };

    let mut show_toast_save = show_toast;
    let handle_save_modal = move |updated_srv: ServerAccount| {
        let mut list = servers();
        if let Some(pos) = list.iter().position(|s| s.id == updated_srv.id) {
            list[pos] = updated_srv.clone();
        } else {
            list.push(updated_srv.clone());
        }
        let _ = LocalStorage::set(SERVERS_STORAGE_KEY, &list);
        servers.set(list);
        modal_server.set(None);
        show_toast_save(I18n::new(lang()).toast_server_saved(&updated_srv.name), false);
    };

    let mut show_toast_del = show_toast;
    let handle_delete_server = move |srv: ServerAccount| {
        let mut list = servers();
        list.retain(|s| s.id != srv.id);
        let _ = LocalStorage::set(SERVERS_STORAGE_KEY, &list);
        servers.set(list);
        if active_server().as_ref().map(|s| &s.id) == Some(&srv.id) {
            active_server.set(None);
            let _ = LocalStorage::delete(ACTIVE_SERVER_ID_KEY);
        }
        show_toast_del(I18n::new(lang()).toast_server_deleted(&srv.name), false);
    };

    rsx! {
        div { class: "window-shell",
            // PWA Installation Banner
            PwaInstallBanner {
                on_open_guide: move |_| pwa_guide_open.set(true),
            }

            // Client Body
            div { class: "window-body",
                // Left Navigation Rail
                Sidebar {
                    current_tab: current_tab(),
                    on_select_tab: move |tab| {
                        current_tab.set(tab);
                        if tab == NavTab::Servers && active_server().is_some() {
                            active_server.set(None);
                        }
                    },
                }

                // Main Viewport
                main { class: "main-viewport",
                    match current_tab() {
                        NavTab::Servers => {
                            if let Some(srv) = active_server() {
                                rsx! {
                                    FileBrowser {
                                        server: srv,
                                        on_back_to_servers: move |_| active_server.set(None),
                                        on_notify: move |msg| show_toast(msg, false),
                                        on_error: move |err| show_toast(err, true),
                                    }
                                }
                            } else {
                                rsx! {
                                    ServersPage {
                                        servers: servers(),
                                        testing_server_id: testing_server_id(),
                                        test_results: test_results(),
                                        on_select_server: handle_select_server,
                                        on_add_server: move |_| modal_server.set(Some(None)),
                                        on_edit_server: move |srv| modal_server.set(Some(Some(srv))),
                                        on_delete_server: handle_delete_server,
                                        on_test_server: handle_test_server,
                                    }
                                }
                            }
                        }
                        NavTab::Settings => rsx! {
                            SettingsPage {
                                on_clear_cache: move |_| {
                                    servers.set(Vec::new());
                                    active_server.set(None);
                                    show_toast(I18n::new(lang()).toast_data_cleared().into(), false);
                                },
                                on_open_pwa_guide: move |_| pwa_guide_open.set(true),
                            }
                        },

                    }
                }
            }

            // Server Modal Dialog
            if let Some(maybe_srv) = modal_server() {
                ServerModal {
                    server: maybe_srv,
                    on_save: handle_save_modal,
                    on_cancel: move |_| modal_server.set(None),
                }
            }

            // PWA Install Guide Modal
            if pwa_guide_open() {
                PwaGuideModal {
                    on_close: move |_| pwa_guide_open.set(false),
                }
            }

            // Toast Notification
            if let Some((msg, is_err)) = toast_message() {
                div {
                    class: if is_err { "toast-bar error" } else { "toast-bar" },
                    onclick: move |_| toast_message.set(None),
                    span { "{msg}" }
                }
            }
        }
    }
}

fn load_initial_servers() -> Vec<ServerAccount> {
    if let Ok(saved) = LocalStorage::get::<Vec<ServerAccount>>(SERVERS_STORAGE_KEY) {
        return saved;
    }

    // Try migrating legacy single connection
    #[derive(serde::Deserialize)]
    struct LegacyConnection {
        url: String,
        username: String,
        password: String,
        token: String,
        auth: String,
    }

    if let Ok(legacy) = LocalStorage::get::<LegacyConnection>(LEGACY_CONNECTION_KEY) {
        if !legacy.url.trim().is_empty() {
            let migrated = vec![ServerAccount {
                id: "srv_default".into(),
                name: "Default".into(),
                base_url: legacy.url,
                auth_type: if legacy.auth == "bearer" {
                    AuthType::Bearer
                } else {
                    AuthType::Basic
                },
                username: legacy.username,
                password: legacy.password,
                token: legacy.token,
                allow_http: true,
                trust_self_signed_cert: false,
            }];
            let _ = LocalStorage::set(SERVERS_STORAGE_KEY, &migrated);
            return migrated;
        }
    }

    Vec::new()
}

fn load_initial_language() -> Language {
    if let Ok(saved) = LocalStorage::get::<Language>(LANGUAGE_STORAGE_KEY) {
        return saved;
    }

    if let Some(w) = web_sys::window() {
        if let Some(lang_str) = w.navigator().language() {
            let lower = lang_str.to_lowercase();
            if lower.starts_with("zh") {
                return Language::Zh;
            } else {
                return Language::En;
            }
        }
    }

    Language::Zh
}
