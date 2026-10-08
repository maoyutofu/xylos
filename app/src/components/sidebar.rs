use crate::components::icons::{IconDns, IconSettings};
use crate::models::NavTab;
use dioxus::prelude::*;

#[component]
pub fn Sidebar(current_tab: NavTab, on_select_tab: EventHandler<NavTab>) -> Element {
    let servers_active = current_tab == NavTab::Servers;
    let settings_active = current_tab == NavTab::Settings;

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-brand",
                img {
                    class: "sidebar-logo",
                    src: "/app/assets/icon.svg",
                    alt: "Xylos",
                }
            }
            button {
                class: if servers_active { "nav-item active" } else { "nav-item" },
                onclick: move |_| on_select_tab.call(NavTab::Servers),
                div { class: "nav-icon", IconDns {} }
                span { "服务器" }
            }
            button {
                class: if settings_active { "nav-item active" } else { "nav-item" },
                onclick: move |_| on_select_tab.call(NavTab::Settings),
                div { class: "nav-icon", IconSettings {} }
                span { "设置" }
            }
        }
    }
}

