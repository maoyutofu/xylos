use dioxus::prelude::*;

pub fn is_standalone_mode() -> bool {
    web_sys::window()
        .and_then(|w| {
            // Check display-mode: standalone
            if let Ok(Some(mql)) = w.match_media("(display-mode: standalone)") {
                if mql.matches() {
                    return Some(true);
                }
            }
            // Check iOS navigator.standalone
            let nav = w.navigator();
            let js_val = js_sys::Reflect::get(&nav, &wasm_bindgen::JsValue::from_str("standalone")).ok()?;
            Some(js_val.as_bool().unwrap_or(false))
        })
        .unwrap_or(false)
}

pub fn is_ios_device() -> bool {
    web_sys::window()
        .and_then(|w| {
            let ua = w.navigator().user_agent().ok()?;
            Some(ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod"))
        })
        .unwrap_or(false)
}

#[component]
pub fn PwaInstallBanner(on_open_guide: EventHandler<()>) -> Element {
    let mut dismissed = use_signal(|| false);
    let is_standalone = use_signal(is_standalone_mode);

    if is_standalone() || dismissed() {
        return rsx! {};
    }

    rsx! {
        div { class: "pwa-banner",
            div { class: "pwa-banner-left",
                img {
                    class: "pwa-banner-icon",
                    src: "/app/assets/icon.svg",
                    alt: "Xylos",
                }
                span { class: "pwa-banner-text", "安装 Xylos 到手机主屏幕，享受全屏原生客户端体验" }
            }
            div { class: "pwa-banner-actions",
                button {
                    class: "pwa-btn-install",
                    onclick: move |_| on_open_guide.call(()),
                    "安装指引"
                }
                button {
                    class: "pwa-btn-dismiss",
                    title: "暂不安装",
                    onclick: move |_| dismissed.set(true),
                    "×"
                }
            }
        }
    }
}

#[component]
pub fn PwaGuideModal(on_close: EventHandler<()>) -> Element {
    let is_ios = is_ios_device();

    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                style: "max-width: 440px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header", "添加到手机主屏幕 (PWA)" }
                div { class: "modal-body",
                    p { style: "font-size: 13px; color: #475569; line-height: 1.6;",
                        "将 Xylos 安装到手机主屏幕后，将移除浏览器地址栏与底部工具栏，提供独立应用的高性能全屏体验。"
                    }
                    div { class: "pwa-guide-steps",
                        if is_ios {
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "1" }
                                div { class: "pwa-step-desc",
                                    "点击 Safari 浏览器底部的 "
                                    strong { "「分享」按钮" }
                                    " (方框带向上箭头图标 ⎋)"
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "2" }
                                div { class: "pwa-step-desc",
                                    "在弹出菜单中向下滑动，找到并点击 "
                                    strong { "「添加到主屏幕」" }
                                    " (Add to Home Screen)"
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "3" }
                                div { class: "pwa-step-desc",
                                    "点击右上角 "
                                    strong { "「添加」" }
                                    "。返回手机桌面点击 Xylos 图标，即可开启全屏无地址栏体验！"
                                }
                            }
                        } else {
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "1" }
                                div { class: "pwa-step-desc",
                                    "点击浏览器右上角菜单图标 "
                                    strong { "「···」" }
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "2" }
                                div { class: "pwa-step-desc",
                                    "在菜单列表中选择 "
                                    strong { "「安装应用」" }
                                    " 或 "
                                    strong { "「添加至主屏幕」" }
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "3" }
                                div { class: "pwa-step-desc",
                                    "确认添加后，从桌面图标直接启动应用，享受全屏原生体验。"
                                }
                            }
                        }
                    }
                }
                div { class: "modal-footer",
                    button {
                        class: "btn-primary",
                        onclick: move |_| on_close.call(()),
                        "我知道了"
                    }
                }
            }
        }
    }
}
