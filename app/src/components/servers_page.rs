use crate::components::icons::{
    IconDns, IconPencil, IconPlus, IconQr, IconStorage, IconTrash, IconWifi,
};
use crate::models::ServerAccount;
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
    let mut delete_target = use_signal(|| None::<ServerAccount>);

    if servers.is_empty() {
        return rsx! {
            div { class: "page-header", h1 { class: "page-title", "服务器" } }
            div { class: "empty-state",
                div { class: "empty-icon", IconDns {} }
                div { class: "empty-title", "暂无服务器" }
                div { class: "empty-desc", "添加一个 WebDAV 服务器后，点击服务器即可进入文件列表。" }
                button {
                    class: "btn-add-server",
                    onclick: move |_| on_add_server.call(()),
                    IconPlus {}
                    "添加服务器"
                }
            }
        };
    }

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "服务器" }
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
                        "测试中...".to_string()
                    } else if let Some(ok) = test_status {
                        if ok { format!("{} · 连接成功", srv.auth_type.label()) }
                        else { format!("{} · 连接失败", srv.auth_type.label()) }
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
                                        title: "服务器二维码",
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
                                        title: "测试连接",
                                        disabled: is_testing,
                                        onclick: move |_| on_test_server.call(srv_for_test.clone()),
                                        IconWifi {}
                                    }
                                    button {
                                        class: "tool-btn",
                                        title: "编辑服务器",
                                        onclick: move |_| on_edit_server.call(srv_for_edit.clone()),
                                        IconPencil {}
                                    }
                                    button {
                                        class: "tool-btn danger",
                                        title: "删除服务器",
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
                "添加服务器"
            }
        }

        // Delete Server Confirmation Modal
        if let Some(target) = delete_target() {
            div { class: "modal-backdrop",
                onclick: move |_| delete_target.set(None),
                div { class: "modal-dialog",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header", "删除服务器" }
                    div { class: "modal-body",
                        p { style: "font-size: 14px; color: #111; line-height: 1.6;",
                            "确定要删除服务器 \"{target.name}\" 吗？该服务器的连接地址与登录凭据将被移除。"
                        }
                    }
                    div { class: "modal-footer",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| delete_target.set(None),
                            "取消"
                        }
                        button {
                            class: "btn-primary",
                            style: "background: #c84b4b;",
                            onclick: move |_| {
                                on_delete_server.call(target.clone());
                                delete_target.set(None);
                            },
                            "删除"
                        }
                    }
                }
            }
        }
    }
}

