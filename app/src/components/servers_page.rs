use crate::components::icons::{
    IconDns, IconPencil, IconPlus, IconQr, IconStorage, IconTrash, IconWifi,
};
use crate::i18n::I18n;
use crate::models::{Language, ServerAccount};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn ServersPage(
    servers: Vec<ServerAccount>,
    testing_server_id: Option<String>,
    test_results: HashMap<String, bool>,
    on_select_server: EventHandler<ServerAccount>,
    on_add_server: EventHandler<()>,
    on_edit_server: EventHandler<ServerAccount>,
    on_delete_server: EventHandler<ServerAccount>,
    on_test_server: EventHandler<ServerAccount>,
) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);

    let mut delete_target = use_signal(|| None::<ServerAccount>);

    if servers.is_empty() {
        return rsx! {
            div { class: "page-header", h1 { class: "page-title", "{tr.servers_title()}" } }
            div { class: "empty-state",
                div { class: "empty-icon", IconDns {} }
                div { class: "empty-title", "{tr.servers_empty_title()}" }
                div { class: "empty-desc", "{tr.servers_empty_desc()}" }
                button {
                    class: "btn-add-server",
                    onclick: move |_| on_add_server.call(()),
                    IconPlus {}
                    "{tr.add_server()}"
                }
            }
        };
    }

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "{tr.servers_title()}" }
            button {
                class: "btn-header-action",
                title: "{tr.add_server()}",
                onclick: move |_| on_add_server.call(()),
                IconPlus {}
                span { "{tr.add()}" }
            }
        }

        div { class: "server-cards-list",
            for srv in servers.into_iter() {
                {
                    let srv_for_card = srv.clone();
                    let srv_for_test = srv.clone();
                    let srv_for_edit = srv.clone();
                    let srv_for_delete = srv.clone();
                    let is_testing = testing_server_id.as_deref() == Some(&srv.id);
                    let test_status = test_results.get(&srv.id).copied();

                    let dot_class = if is_testing {
                        "status-dot testing"
                    } else if let Some(ok) = test_status {
                        if ok { "status-dot" } else { "status-dot failed" }
                    } else {
                        "status-dot"
                    };

                    let status_text = if is_testing {
                        tr.status_testing().to_string()
                    } else if let Some(ok) = test_status {
                        if ok { format!("{} · {}", srv.auth_type.label(), tr.status_connected()) }
                        else { format!("{} · {}", srv.auth_type.label(), tr.status_failed()) }
                    } else {
                        srv.auth_type.label().to_string()
                    };

                    rsx! {
                        div {
                            key: "{srv.id}",
                            class: "server-card",
                            div {
                                class: "server-card-header",
                                onclick: move |_| on_select_server.call(srv_for_card.clone()),
                                div { class: "server-card-title",
                                    IconStorage {}
                                    span { "{srv.name}" }
                                }
                                div { class: "server-card-header-actions",
                                    button {
                                        class: "header-icon-btn",
                                        title: "{tr.server_qr_code()}",
                                        onclick: move |e| { e.stop_propagation(); },
                                        IconQr {}
                                    }
                                }
                            }
                            div {
                                class: "server-card-body",
                                onclick: {
                                    let srv_select = srv.clone();
                                    move |_| on_select_server.call(srv_select.clone())
                                },
                                div { class: "server-card-info",
                                    div { class: "server-url", "{srv.base_url}" }
                                    div { class: "server-meta",
                                        span { class: "{dot_class}" }
                                        span { "{status_text}" }
                                    }
                                }
                                div {
                                    class: "server-card-actions",
                                    onclick: move |e| e.stop_propagation(),
                                    button {
                                        class: "tool-btn",
                                        title: "{tr.test_connection()}",
                                        disabled: is_testing,
                                        onclick: move |_| on_test_server.call(srv_for_test.clone()),
                                        IconWifi {}
                                    }
                                    button {
                                        class: "tool-btn",
                                        title: "{tr.edit_server()}",
                                        onclick: move |_| on_edit_server.call(srv_for_edit.clone()),
                                        IconPencil {}
                                    }
                                    button {
                                        class: "tool-btn danger",
                                        title: "{tr.delete_server()}",
                                        onclick: move |_| delete_target.set(Some(srv_for_delete.clone())),
                                        IconTrash {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        div { class: "btn-add-server-wrap",
            button {
                class: "btn-add-server",
                onclick: move |_| on_add_server.call(()),
                IconPlus {}
                "{tr.add_server()}"
            }
        }

        // Delete Server Confirmation Modal
        if let Some(target) = delete_target() {
            div { class: "modal-backdrop",
                onclick: move |_| delete_target.set(None),
                div { class: "modal-dialog",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header", "{tr.delete_server_confirm_title()}" }
                    div { class: "modal-body",
                        p { style: "font-size: 14px; color: var(--app-text); line-height: 1.6;",
                            "{tr.delete_server_confirm_desc(&target.name)}"
                        }
                    }
                    div { class: "modal-footer",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| delete_target.set(None),
                            "{tr.cancel()}"
                        }
                        button {
                            class: "btn-primary btn-danger",
                            onclick: move |_| {
                                on_delete_server.call(target.clone());
                                delete_target.set(None);
                            },
                            "{tr.delete()}"
                        }
                    }
                }
            }
        }
    }
}
