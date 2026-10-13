use crate::i18n::I18n;
use crate::models::{Language, LANGUAGE_STORAGE_KEY};
use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};

#[component]
pub fn SettingsPage(
    on_clear_cache: EventHandler<()>,
    on_open_pwa_guide: EventHandler<()>,
) -> Element {
    let mut lang_signal = use_context::<Signal<Language>>();
    let lang = lang_signal();
    let tr = I18n::new(lang);

    let cleared = use_signal(|| false);
    let mut clear_confirm_open = use_signal(|| false);

    let handle_clear = move || {
        let current_l = lang_signal();
        LocalStorage::clear();
        let _ = LocalStorage::set(LANGUAGE_STORAGE_KEY, &current_l);
        let mut c = cleared;
        c.set(true);
        on_clear_cache.call(());
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "{tr.settings_title()}" }
        }
        div { class: "settings-list",
            // Language Selection Card
            div { class: "settings-card",
                div { class: "settings-item stack-mobile",
                    div { class: "settings-item-info",
                        h3 { "{tr.settings_lang_title()}" }
                        p { "{tr.settings_lang_desc()}" }
                    }
                    div { class: "lang-switch-group",
                        button {
                            class: if lang == Language::Zh { "lang-switch-btn active" } else { "lang-switch-btn" },
                            onclick: move |_| {
                                lang_signal.set(Language::Zh);
                                let _ = LocalStorage::set(LANGUAGE_STORAGE_KEY, &Language::Zh);
                            },
                            "简体中文"
                        }
                        button {
                            class: if lang == Language::En { "lang-switch-btn active" } else { "lang-switch-btn" },
                            onclick: move |_| {
                                lang_signal.set(Language::En);
                                let _ = LocalStorage::set(LANGUAGE_STORAGE_KEY, &Language::En);
                            },
                            "English"
                        }
                    }
                }
            }

            // About Card
            div { class: "settings-card",
                div { class: "settings-item-info",
                    h3 { "{tr.settings_about_title()}" }
                    p { "{tr.settings_about_desc()}" }
                    div { style: "margin-top: 10px; font-size: 13px; color: #111;",
                        strong { "{tr.settings_version_label()}" }
                        span { "0.2.0" }
                    }
                }
            }

            // CORS Card
            div { class: "settings-card",
                div { class: "settings-item-info",
                    h3 { "{tr.settings_cors_title()}" }
                    p { "{tr.settings_cors_desc()}" }
                }
            }

            // PWA Install Guide Card
            div { class: "settings-card",
                div { class: "settings-item",
                    div { class: "settings-item-info",
                        h3 { "{tr.settings_pwa_title()}" }
                        p { "{tr.settings_pwa_desc()}" }
                    }
                    button {
                        class: "settings-action-btn primary-outline",
                        onclick: move |_| on_open_pwa_guide.call(()),
                        "{tr.settings_pwa_btn()}"
                    }
                }
            }

            // Offline Cache & Reload Card
            div { class: "settings-card",
                div { class: "settings-item",
                    div { class: "settings-item-info",
                        h3 { "{tr.settings_cache_title()}" }
                        p { "{tr.settings_cache_desc()}" }
                    }
                    button {
                        class: "settings-action-btn primary-outline",
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
                        "{tr.settings_cache_btn()}"
                    }
                }
            }

            // Local Data Management Card
            div { class: "settings-card",
                div { class: "settings-item",
                    div { class: "settings-item-info",
                        h3 { "{tr.settings_data_title()}" }
                        p { "{tr.settings_data_desc()}" }
                    }
                    button {
                        class: "settings-action-btn danger-outline",
                        onclick: move |_| clear_confirm_open.set(true),
                        if cleared() { "{tr.settings_data_cleared_btn()}" } else { "{tr.settings_data_clear_btn()}" }
                    }
                }
            }
        }

        // Clear Data Confirmation Modal
        if clear_confirm_open() {
            div { class: "modal-backdrop",
                onclick: move |_| clear_confirm_open.set(false),
                div { class: "modal-dialog",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "modal-header", "{tr.settings_clear_confirm_title()}" }
                    div { class: "modal-body",
                        p { style: "font-size: 14px; color: #111; line-height: 1.6;",
                            "{tr.settings_clear_confirm_desc()}"
                        }
                    }
                    div { class: "modal-footer",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| clear_confirm_open.set(false),
                            "{tr.cancel()}"
                        }
                        button {
                            class: "btn-primary",
                            style: "background: #c84b4b;",
                            onclick: move |_| {
                                clear_confirm_open.set(false);
                                handle_clear();
                            },
                            "{tr.confirm()}"
                        }
                    }
                }
            }
        }
    }
}
