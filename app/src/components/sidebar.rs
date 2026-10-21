use crate::components::icons::{IconDns, IconSettings};
use crate::i18n::I18n;
use crate::models::{Language, NavTab};
use dioxus::prelude::*;

#[component]
pub fn Sidebar(current_tab: NavTab, on_select_tab: EventHandler<NavTab>) -> Element {
    let lang = use_context::<Signal<Language>>()();
    let tr = I18n::new(lang);

    let servers_active = current_tab == NavTab::Servers;
    let settings_active = current_tab == NavTab::Settings;

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-brand",
                img {
                    class: "sidebar-logo",
                    src: "/app/assets/icon.svg",
                    alt: "Xylos light",
                }
            }
            button {
                class: if servers_active { "nav-item active" } else { "nav-item" },
                onclick: move |_| on_select_tab.call(NavTab::Servers),
                div { class: "nav-icon", IconDns {} }
                span { "{tr.nav_servers()}" }
            }
            button {
                class: if settings_active { "nav-item active" } else { "nav-item" },
                onclick: move |_| on_select_tab.call(NavTab::Settings),
                div { class: "nav-icon", IconSettings {} }
                span { "{tr.nav_settings()}" }
            }
        }
    }
}

