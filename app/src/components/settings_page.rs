use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};

#[component]
pub fn SettingsPage(
    on_clear_cache: EventHandler<()>,
    on_open_pwa_guide: EventHandler<()>,
) -> Element {
    let cleared = use_signal(|| false);
    let mut clear_confirm_open = use_signal(|| false);


    let handle_clear = move || {
        LocalStorage::clear();
        let mut c = cleared;
        c.set(true);
        on_clear_cache.call(());
    };



    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "设置" }
        }
        div { style: "display: flex; flex-direction: column; gap: 20px; width: 100%;",
            div { style: "background: #fff; border: 1px solid #e4e6e8; border-radius: 6px; padding: 18px 20px;",
                h3 { style: "font-size: 16px; margin-bottom: 10px;", "关于 Xylos" }
                p { style: "font-size: 13px; color: #6f716f; line-height: 1.6;",
                    "Xylos WebDAV Client 是一个基于 Rust + Dioxus 构建的跨平台客户端应用。支持多 WebDAV 服务器管理、离线浏览与 PWA 安装。"
                }
                div { style: "margin-top: 10px; font-size: 13px; color: #111;",
                    strong { "客户端版本: " }
                    span { "0.2.0" }
                }
            }

            div { style: "background: #fff; border: 1px solid #e4e6e8; border-radius: 6px; padding: 18px 20px;",
                h3 { style: "font-size: 16px; margin-bottom: 10px;", "跨域与 CORS 说明" }
                p { style: "font-size: 13px; color: #6f716f; line-height: 1.6;",
                    "由于浏览器安全限制，若连接外部 WebDAV 服务，目标服务器需配置允许跨域（CORS）访问，并允许 PROPFIND、MKCOL、MOVE、COPY 等 WebDAV 特殊 HTTP 方法。"
                }
            }

            div { style: "background: #fff; border: 1px solid #e4e6e8; border-radius: 6px; padding: 18px 20px; display: flex; align-items: center; justify-content: space-between;",
                div {
                    h3 { style: "font-size: 15px; margin-bottom: 4px;", "添加到主屏幕 (PWA)" }
                    p { style: "font-size: 12px; color: #6f716f;", "安装到手机主屏幕可移除浏览器地址栏，享受原生应用全屏体验" }
                }
                button {
                    class: "btn-secondary",
                    style: "color: var(--app-primary); border-color: var(--app-primary);",
                    onclick: move |_| on_open_pwa_guide.call(()),
                    "查看安装指引"
                }
            }

            div { style: "background: #fff; border: 1px solid #e4e6e8; border-radius: 6px; padding: 18px 20px; display: flex; align-items: center; justify-content: space-between;",
                div {
                    h3 { style: "font-size: 15px; margin-bottom: 4px;", "PWA 离线缓存与更新" }
                    p { style: "font-size: 12px; color: #6f716f;", "若浏览器由于离线缓存未展示最新改动，可一键清空 Service Worker 缓存并强制刷新" }
                }
                button {
                    class: "btn-secondary",
                    style: "color: var(--app-primary); border-color: var(--app-primary);",
                    onclick: move |_| {
                        let _ = dioxus::document::eval(
                            r#"
                            (async () => {
                                if ('caches' in window) {
                                    const keys = await caches.keys();
                                    await Promise.all(keys.map(k => caches.delete(k)));
                                }
                                if ('serviceWorker' in navigator) {
                                    const regs = await navigator.serviceWorker.getRegistrations();
                                    for (const reg of regs) {
                                        await reg.unregister();
                                    }
                                }
                                window.location.reload(true);
                            })();
                            "#,
                        );
                    },
                    "清除缓存并刷新应用"
                }
            }

            div { style: "background: #fff; border: 1px solid #e4e6e8; border-radius: 6px; padding: 18px 20px; display: flex; align-items: center; justify-content: space-between;",
                div {
                    h3 { style: "font-size: 15px; margin-bottom: 4px;", "本地数据管理" }
                    p { style: "font-size: 12px; color: #6f716f;", "清除所有已保存的服务器配置及凭据" }
                }
                button {
                    class: "btn-secondary",
                    style: "color: #c84b4b;",
                    onclick: move |_| clear_confirm_open.set(true),
                    if cleared() { "已清除" } else { "清除所有服务器数据" }
                }
            }
        }

        // Clear Data Confirmation Modal
        if clear_confirm_open() {
            div { class: "modal-backdrop",
                onclick: move |_| clear_confirm_open.set(false),
                div { class: "modal-dialog",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header", "确认清除数据" }
                    div { class: "modal-body",
                        p { style: "font-size: 14px; color: #111; line-height: 1.6;",
                            "确定要清空本地保存的所有服务器配置及登录凭据吗？清除后需重新添加服务器，此操作无法撤销。"
                        }
                    }
                    div { class: "modal-footer",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| clear_confirm_open.set(false),
                            "取消"
                        }
                        button {
                            class: "btn-primary",
                            style: "background: #c84b4b;",
                            onclick: move |_| {
                                clear_confirm_open.set(false);
                                handle_clear();
                            },

                            "确定清除"
                        }
                    }
                }
            }
        }
    }
}


