use crate::i18n::I18n;
use crate::models::Language;
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

pub fn is_mobile_device() -> bool {
    web_sys::window()
        .and_then(|w| {
            let ua = w.navigator().user_agent().ok()?.to_lowercase();
            let is_mobile_ua = ua.contains("iphone")
                || ua.contains("ipad")
                || ua.contains("ipod")
                || ua.contains("android")
                || ua.contains("mobile")
                || ua.contains("webos")
                || ua.contains("blackberry")
                || ua.contains("phone");

            if is_mobile_ua {
                return Some(true);
            }

            // Check mobile screen width and coarse pointer (touch device)
            if let Ok(Some(mql)) = w.match_media("(max-width: 768px) and (pointer: coarse)") {
                if mql.matches() {
                    return Some(true);
                }
            }

            Some(false)
        })
        .unwrap_or(false)
}

#[component]
pub fn PwaInstallBanner(on_open_guide: EventHandler<()>) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);

    let mut dismissed = use_signal(|| false);
    let is_standalone = use_signal(is_standalone_mode);
    let is_mobile = use_signal(is_mobile_device);

    if is_standalone() || dismissed() || !is_mobile() {
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
                span { class: "pwa-banner-text", "{tr.pwa_banner_text()}" }
            }
            div { class: "pwa-banner-actions",
                button {
                    class: "pwa-btn-install",
                    onclick: move |_| on_open_guide.call(()),
                    "{tr.pwa_banner_btn()}"
                }
                button {
                    class: "pwa-btn-dismiss",
                    title: "{tr.pwa_banner_dismiss()}",
                    onclick: move |_| dismissed.set(true),
                    "×"
                }
            }
        }
    }
}

#[component]
pub fn PwaGuideModal(on_close: EventHandler<()>) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);
    let is_ios = is_ios_device();

    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-dialog",
                style: "max-width: 440px;",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header", "{tr.pwa_guide_title()}" }
                div { class: "modal-body",
                    p { style: "font-size: 13px; color: var(--app-muted); line-height: 1.6;",
                        "{tr.pwa_guide_desc()}"
                    }
                    div { class: "pwa-guide-steps",
                        if is_ios {
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "1" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_ios_step1_prefix()}"
                                    strong { "{tr.pwa_ios_step1_strong()}" }
                                    "{tr.pwa_ios_step1_suffix()}"
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "2" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_ios_step2_prefix()}"
                                    strong { "{tr.pwa_ios_step2_strong()}" }
                                    "{tr.pwa_ios_step2_suffix()}"
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "3" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_ios_step3_prefix()}"
                                    strong { "{tr.pwa_ios_step3_strong()}" }
                                    "{tr.pwa_ios_step3_suffix()}"
                                }
                            }
                        } else {
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "1" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_other_step1_prefix()}"
                                    strong { "{tr.pwa_other_step1_strong()}" }
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "2" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_other_step2_prefix()}"
                                    strong { "{tr.pwa_other_step2_install()}" }
                                    "{tr.pwa_other_step2_or()}"
                                    strong { "{tr.pwa_other_step2_homescreen()}" }
                                }
                            }
                            div { class: "pwa-step-item",
                                div { class: "pwa-step-badge", "3" }
                                div { class: "pwa-step-desc",
                                    "{tr.pwa_other_step3()}"
                                }
                            }
                        }
                    }
                }
                div { class: "modal-footer",
                    button {
                        class: "btn-primary",
                        onclick: move |_| on_close.call(()),
                        "{tr.understood()}"
                    }
                }
            }
        }
    }
}
