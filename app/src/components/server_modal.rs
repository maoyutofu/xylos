use crate::components::icons::IconEye;
use crate::models::{AuthType, ServerAccount};
use dioxus::prelude::*;

#[component]
pub fn ServerModal(
    server: Option<ServerAccount>,
    on_save: EventHandler<ServerAccount>,
    on_cancel: EventHandler<()>,
) -> Element {
    let initial = server.clone().unwrap_or_else(|| ServerAccount {
        id: format!("srv_{}", js_sys::Date::now() as u64),
        name: String::new(),
        base_url: "http://127.0.0.1:8080/dav/".into(),
        auth_type: AuthType::Basic,
        username: String::new(),
        password: String::new(),
        token: String::new(),
        allow_http: true,
        trust_self_signed_cert: false,
    });

    let is_edit = server.is_some();
    let mut name = use_signal(|| initial.name.clone());
    let mut base_url = use_signal(|| initial.base_url.clone());
    let mut auth_type = use_signal(|| initial.auth_type);
    let mut username = use_signal(|| initial.username.clone());
    let mut password = use_signal(|| initial.password.clone());
    let mut token = use_signal(|| initial.token.clone());
    let mut allow_http = use_signal(|| initial.allow_http);
    let mut trust_cert = use_signal(|| initial.trust_self_signed_cert);
    let mut show_password = use_signal(|| false);
    let mut form_error = use_signal(|| None::<String>);

    let id = initial.id;

    let handle_save = move |_| {
        let n = name().trim().to_string();
        let u = base_url().trim().to_string();
        if n.is_empty() {
            form_error.set(Some("请输入服务器别名".into()));
            return;
        }
        if u.is_empty() {
            form_error.set(Some("请输入服务地址".into()));
            return;
        }

        let updated = ServerAccount {
            id: id.clone(),
            name: n,
            base_url: u,
            auth_type: auth_type(),
            username: username().trim().to_string(),
            password: password().clone(),
            token: token().trim().to_string(),
            allow_http: allow_http(),
            trust_self_signed_cert: trust_cert(),
        };

        on_save.call(updated);
    };

    let auth_str = match auth_type() {
        AuthType::Basic => "basic",
        AuthType::Digest => "digest",
        AuthType::Bearer => "bearer",
    };

    rsx! {
        div { class: "modal-backdrop",
            onclick: move |_| on_cancel.call(()),
            div { class: "modal-dialog",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    if is_edit { "编辑服务器" } else { "添加服务器" }
                }
                div { class: "modal-body",
                    div { class: "form-field",
                        label { class: "form-label", "别名" }
                        input {
                            class: "form-input",
                            placeholder: "如：Default 或 个人网盘",
                            value: "{name()}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div { class: "form-field",
                        label { class: "form-label", "服务地址" }
                        input {
                            class: "form-input",
                            placeholder: "http://192.168.0.26:8080/dav/",
                            value: "{base_url()}",
                            oninput: move |e| base_url.set(e.value()),
                        }
                    }
                    div { class: "form-field",
                        label { class: "form-label", "认证方式" }
                        select {
                            class: "form-input",
                            value: "{auth_str}",
                            onchange: move |e| {
                                let val = e.value();
                                if val == "bearer" {
                                    auth_type.set(AuthType::Bearer);
                                } else if val == "digest" {
                                    auth_type.set(AuthType::Digest);
                                } else {
                                    auth_type.set(AuthType::Basic);
                                }
                            },
                            option { value: "basic", "Basic" }
                            option { value: "digest", "Digest" }
                            option { value: "bearer", "Bearer Token" }
                        }
                    }
                    if auth_type() != AuthType::Bearer {
                        div { class: "form-field",
                            label { class: "form-label", "用户名" }
                            input {
                                class: "form-input",
                                placeholder: "用户名",
                                value: "{username()}",
                                oninput: move |e| username.set(e.value()),
                            }
                        }
                        div { class: "form-field",
                            label { class: "form-label", "密码" }
                            div { class: "form-input-wrap",
                                input {
                                    class: "form-input",
                                    r#type: if show_password() { "text" } else { "password" },
                                    placeholder: "密码",
                                    value: "{password()}",
                                    oninput: move |e| password.set(e.value()),
                                }
                                button {
                                    class: "input-suffix-btn",
                                    r#type: "button",
                                    onclick: move |_| show_password.set(!show_password()),
                                    IconEye { visible: show_password() }
                                }
                            }
                        }
                    } else {
                        div { class: "form-field",
                            label { class: "form-label", "Bearer Token" }
                            input {
                                class: "form-input",
                                r#type: "password",
                                placeholder: "输入 Token",
                                value: "{token()}",
                                oninput: move |e| token.set(e.value()),
                            }
                        }
                    }

                    div { class: "switch-field",
                        div { class: "switch-label",
                            div { class: "switch-label-title", "允许 HTTP" }
                            div { class: "switch-label-desc", "仅建议在局域网或测试环境开启" }
                        }
                        label { class: "switch",
                            input {
                                r#type: "checkbox",
                                checked: "{allow_http()}",
                                onchange: move |e| allow_http.set(e.value() == "true"),
                            }
                            span { class: "switch-slider" }
                        }
                    }

                    div { class: "switch-field",
                        div { class: "switch-label",
                            div { class: "switch-label-title", "信任自签名证书" }
                            div { class: "switch-label-desc", "仅对当前服务器生效" }
                        }
                        label { class: "switch",
                            input {
                                r#type: "checkbox",
                                checked: "{trust_cert()}",
                                onchange: move |e| trust_cert.set(e.value() == "true"),
                            }
                            span { class: "switch-slider" }
                        }
                    }

                    if let Some(err) = form_error() {
                        div { style: "color: #c84b4b; font-size: 13px;", "{err}" }
                    }
                }
                div { class: "modal-footer",
                    button {
                        class: "btn-secondary",
                        r#type: "button",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    button {
                        class: "btn-primary",
                        r#type: "button",
                        onclick: handle_save,
                        "保存"
                    }
                }
            }
        }
    }
}
