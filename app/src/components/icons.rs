use dioxus::prelude::*;

#[component]
pub fn IconDns() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "20",
            "height": "20",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            rect { "x": "2", "y": "2", "width": "20", "height": "8", "rx": "2", "ry": "2" }
            rect { "x": "2", "y": "14", "width": "20", "height": "8", "rx": "2", "ry": "2" }
            line { "x1": "6", "y1": "6", "x2": "6.01", "y2": "6" }
            line { "x1": "6", "y1": "18", "x2": "6.01", "y2": "18" }
        }
    }
}

#[component]
pub fn IconTransfers() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "20",
            "height": "20",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            polyline { "points": "17 1 21 5 17 9" }
            path { "d": "M3 11V9a4 4 0 0 1 4-4h14" }
            polyline { "points": "7 23 3 19 7 15" }
            path { "d": "M21 13v2a4 4 0 0 1-4 4H3" }
        }
    }
}

#[component]
pub fn IconOffline() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "20",
            "height": "20",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M20 6L9 17l-5-5" }
        }
    }
}

#[component]
pub fn IconSettings() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "20",
            "height": "20",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            line { "x1": "4", "y1": "21", "x2": "4", "y2": "14" }
            line { "x1": "4", "y1": "10", "x2": "4", "y2": "3" }
            line { "x1": "12", "y1": "21", "x2": "12", "y2": "12" }
            line { "x1": "12", "y1": "8", "x2": "12", "y2": "3" }
            line { "x1": "20", "y1": "21", "x2": "20", "y2": "16" }
            line { "x1": "20", "y1": "12", "x2": "20", "y2": "3" }
            line { "x1": "1", "y1": "14", "x2": "7", "y2": "14" }
            line { "x1": "9", "y1": "8", "x2": "15", "y2": "8" }
            line { "x1": "17", "y1": "16", "x2": "23", "y2": "16" }
        }
    }
}

#[component]
pub fn IconStorage() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            ellipse { "cx": "12", "cy": "5", "rx": "9", "ry": "3" }
            path { "d": "M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" }
            path { "d": "M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" }
        }
    }
}

#[component]
pub fn IconWifi() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M5 12.55a11 11 0 0 1 14.08 0" }
            path { "d": "M1.42 9a16 16 0 0 1 21.16 0" }
            path { "d": "M8.53 16.11a6 6 0 0 1 6.95 0" }
            line { "x1": "12", "y1": "20", "x2": "12.01", "y2": "20" }
        }
    }
}

#[component]
pub fn IconPencil() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" }
        }
    }
}

#[component]
pub fn IconTrash() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            polyline { "points": "3 6 5 6 21 6" }
            path { "d": "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
        }
    }
}

#[component]
pub fn IconQr() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            rect { "x": "3", "y": "3", "width": "7", "height": "7" }
            rect { "x": "14", "y": "3", "width": "7", "height": "7" }
            rect { "x": "3", "y": "14", "width": "7", "height": "7" }
            line { "x1": "14", "y1": "14", "x2": "14", "y2": "14.01" }
            line { "x1": "17", "y1": "17", "x2": "17", "y2": "17.01" }
            line { "x1": "20", "y1": "14", "x2": "20", "y2": "14.01" }
            line { "x1": "20", "y1": "20", "x2": "20", "y2": "20.01" }
        }
    }
}

#[component]
pub fn IconPlus() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            line { "x1": "12", "y1": "5", "x2": "12", "y2": "19" }
            line { "x1": "5", "y1": "12", "x2": "19", "y2": "12" }
        }
    }
}

#[component]
pub fn IconFolder(#[props(default = false)] is_large: bool) -> Element {
    let size = if is_large { "48" } else { "20" };
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "{size}",
            "height": "{size}",
            "fill": "currentColor",
            path { "d": "M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" }
        }
    }
}

#[component]
pub fn IconFile(#[props(default = false)] is_large: bool) -> Element {
    let size = if is_large { "48" } else { "20" };
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "{size}",
            "height": "{size}",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" }
            polyline { "points": "13 2 13 9 20 9" }
        }
    }
}

#[component]
pub fn IconImage() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "20",
            "height": "20",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            rect { "x": "3", "y": "3", "width": "18", "height": "18", "rx": "2", "ry": "2" }
            circle { "cx": "8.5", "cy": "8.5", "r": "1.5" }
            polyline { "points": "21 15 16 10 5 21" }
        }
    }
}


#[component]
pub fn IconArrowBack() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            line { "x1": "19", "y1": "12", "x2": "5", "y2": "12" }
            polyline { "points": "12 19 5 12 12 5" }
        }
    }
}

#[component]
pub fn IconRefresh() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            polyline { "points": "23 4 23 10 17 10" }
            path { "d": "M20.49 15a9 9 0 1 1-2.12-9.36L23 10" }
        }
    }
}

#[component]
pub fn IconSort() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            line { "x1": "4", "y1": "6", "x2": "20", "y2": "6" }
            line { "x1": "4", "y1": "12", "x2": "14", "y2": "12" }
            line { "x1": "4", "y1": "18", "x2": "8", "y2": "18" }
        }
    }
}

#[component]
pub fn IconList() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            line { "x1": "8", "y1": "6", "x2": "21", "y2": "6" }
            line { "x1": "8", "y1": "12", "x2": "21", "y2": "12" }
            line { "x1": "8", "y1": "18", "x2": "21", "y2": "18" }
            line { "x1": "3", "y1": "6", "x2": "3.01", "y2": "6" }
            line { "x1": "3", "y1": "12", "x2": "3.01", "y2": "12" }
            line { "x1": "3", "y1": "18", "x2": "3.01", "y2": "18" }
        }
    }
}

#[component]
pub fn IconGrid() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            rect { "x": "3", "y": "3", "width": "7", "height": "7" }
            rect { "x": "14", "y": "3", "width": "7", "height": "7" }
            rect { "x": "14", "y": "14", "width": "7", "height": "7" }
            rect { "x": "3", "y": "14", "width": "7", "height": "7" }
        }
    }
}

#[component]
pub fn IconUpload() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { "points": "17 8 12 3 7 8" }
            line { "x1": "12", "y1": "3", "x2": "12", "y2": "15" }
        }
    }
}

#[component]
pub fn IconDownload() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
            polyline { "points": "7 10 12 15 17 10" }
            line { "x1": "12", "y1": "15", "x2": "12", "y2": "3" }
        }
    }
}


#[component]
pub fn IconNewFolder() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "none",
            "stroke": "currentColor",
            "stroke_width": "2",
            "stroke_linecap": "round",
            "stroke_linejoin": "round",
            path { "d": "M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" }
            line { "x1": "12", "y1": "11", "x2": "12", "y2": "17" }
            line { "x1": "9", "y1": "14", "x2": "15", "y2": "14" }
        }
    }
}

#[component]
pub fn IconMore() -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            "width": "18",
            "height": "18",
            "fill": "currentColor",
            circle { "cx": "12", "cy": "5", "r": "2" }
            circle { "cx": "12", "cy": "12", "r": "2" }
            circle { "cx": "12", "cy": "19", "r": "2" }
        }
    }
}

#[component]
pub fn IconEye(#[props(default = true)] visible: bool) -> Element {
    if visible {
        rsx! {
            svg {
                "viewBox": "0 0 24 24",
                "width": "18",
                "height": "18",
                "fill": "none",
                "stroke": "currentColor",
                "stroke_width": "2",
                "stroke_linecap": "round",
                "stroke_linejoin": "round",
                path { "d": "M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" }
                circle { "cx": "12", "cy": "12", "r": "3" }
            }
        }
    } else {
        rsx! {
            svg {
                "viewBox": "0 0 24 24",
                "width": "18",
                "height": "18",
                "fill": "none",
                "stroke": "currentColor",
                "stroke_width": "2",
                "stroke_linecap": "round",
                "stroke_linejoin": "round",
                path { "d": "M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" }
                line { "x1": "1", "y1": "1", "x2": "23", "y2": "23" }
            }
        }
    }
}
