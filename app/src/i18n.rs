use crate::models::Language;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct I18n {
    pub lang: Language,
}

#[allow(dead_code)]
impl I18n {
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    pub fn is_zh(&self) -> bool {
        self.lang == Language::Zh
    }

    // Common
    pub fn cancel(&self) -> &'static str {
        if self.is_zh() { "取消" } else { "Cancel" }
    }

    pub fn confirm(&self) -> &'static str {
        if self.is_zh() { "确定" } else { "Confirm" }
    }

    pub fn save(&self) -> &'static str {
        if self.is_zh() { "保存" } else { "Save" }
    }

    pub fn delete(&self) -> &'static str {
        if self.is_zh() { "删除" } else { "Delete" }
    }

    pub fn add(&self) -> &'static str {
        if self.is_zh() { "添加" } else { "Add" }
    }

    pub fn understood(&self) -> &'static str {
        if self.is_zh() { "我知道了" } else { "Got it" }
    }

    // Sidebar
    pub fn nav_servers(&self) -> &'static str {
        if self.is_zh() { "服务器" } else { "Servers" }
    }

    pub fn nav_settings(&self) -> &'static str {
        if self.is_zh() { "设置" } else { "Settings" }
    }

    // Servers Page
    pub fn servers_title(&self) -> &'static str {
        if self.is_zh() { "服务器" } else { "Servers" }
    }

    pub fn servers_empty_title(&self) -> &'static str {
        if self.is_zh() { "暂无服务器" } else { "No Servers" }
    }

    pub fn servers_empty_desc(&self) -> &'static str {
        if self.is_zh() {
            "添加一个 WebDAV 服务器后，点击服务器即可进入文件列表。"
        } else {
            "Add a WebDAV server, then click it to browse files."
        }
    }

    pub fn add_server(&self) -> &'static str {
        if self.is_zh() { "添加服务器" } else { "Add Server" }
    }

    pub fn server_qr_code(&self) -> &'static str {
        if self.is_zh() { "服务器二维码" } else { "Server QR Code" }
    }

    pub fn test_connection(&self) -> &'static str {
        if self.is_zh() { "测试连接" } else { "Test Connection" }
    }

    pub fn edit_server(&self) -> &'static str {
        if self.is_zh() { "编辑服务器" } else { "Edit Server" }
    }

    pub fn delete_server(&self) -> &'static str {
        if self.is_zh() { "删除服务器" } else { "Delete Server" }
    }

    pub fn status_testing(&self) -> &'static str {
        if self.is_zh() { "测试中..." } else { "Testing..." }
    }

    pub fn status_connected(&self) -> &'static str {
        if self.is_zh() { "连接成功" } else { "Connected" }
    }

    pub fn status_failed(&self) -> &'static str {
        if self.is_zh() { "连接失败" } else { "Failed" }
    }

    pub fn delete_server_confirm_title(&self) -> &'static str {
        if self.is_zh() { "删除服务器" } else { "Delete Server" }
    }

    pub fn delete_server_confirm_desc(&self, name: &str) -> String {
        if self.is_zh() {
            format!("确定要删除服务器 \"{name}\" 吗？该服务器的连接地址与登录凭据将被移除。")
        } else {
            format!("Are you sure you want to delete server \"{name}\"? Its address and login credentials will be removed.")
        }
    }

    // Server Modal
    pub fn modal_add_server_title(&self) -> &'static str {
        if self.is_zh() { "添加 WebDAV 服务器" } else { "Add WebDAV Server" }
    }

    pub fn modal_edit_server_title(&self) -> &'static str {
        if self.is_zh() { "编辑 WebDAV 服务器" } else { "Edit WebDAV Server" }
    }

    pub fn field_server_name(&self) -> &'static str {
        if self.is_zh() { "服务器别名" } else { "Server Name" }
    }

    pub fn placeholder_server_name(&self) -> &'static str {
        if self.is_zh() { "如：我的私有网盘" } else { "e.g. My Nextcloud" }
    }

    pub fn field_server_url(&self) -> &'static str {
        if self.is_zh() { "服务地址 (URL)" } else { "Server URL" }
    }

    pub fn field_auth_type(&self) -> &'static str {
        if self.is_zh() { "认证类型" } else { "Authentication" }
    }

    pub fn field_username(&self) -> &'static str {
        if self.is_zh() { "用户名" } else { "Username" }
    }

    pub fn placeholder_username(&self) -> &'static str {
        if self.is_zh() { "输入用户名" } else { "Enter username" }
    }

    pub fn field_password(&self) -> &'static str {
        if self.is_zh() { "密码" } else { "Password" }
    }

    pub fn placeholder_password(&self) -> &'static str {
        if self.is_zh() { "输入密码" } else { "Enter password" }
    }

    pub fn field_token(&self) -> &'static str {
        if self.is_zh() { "访问令牌 (Bearer Token)" } else { "Access Token (Bearer)" }
    }

    pub fn placeholder_token(&self) -> &'static str {
        if self.is_zh() { "输入 Bearer 凭据" } else { "Enter Bearer Token" }
    }

    pub fn field_security_options(&self) -> &'static str {
        if self.is_zh() { "安全与连接选项" } else { "Security & Connection Options" }
    }

    pub fn switch_allow_http(&self) -> &'static str {
        if self.is_zh() { "允许 HTTP" } else { "Allow HTTP" }
    }

    pub fn switch_allow_http_desc(&self) -> &'static str {
        if self.is_zh() {
            "仅建议在局域网或测试环境开启"
        } else {
            "Recommended only in LAN or testing environments"
        }
    }

    pub fn switch_trust_cert(&self) -> &'static str {
        if self.is_zh() { "信任自签名证书" } else { "Trust Self-Signed Certificate" }
    }

    pub fn switch_trust_cert_desc(&self) -> &'static str {
        if self.is_zh() { "仅对当前服务器生效" } else { "Applies only to this server" }
    }

    pub fn err_name_required(&self) -> &'static str {
        if self.is_zh() { "请输入服务器别名" } else { "Please enter a server name" }
    }

    pub fn err_url_required(&self) -> &'static str {
        if self.is_zh() { "请输入服务地址" } else { "Please enter a server URL" }
    }

    // Settings Page
    pub fn settings_title(&self) -> &'static str {
        if self.is_zh() { "设置" } else { "Settings" }
    }

    pub fn settings_lang_title(&self) -> &'static str {
        if self.is_zh() { "界面语言" } else { "Interface Language" }
    }

    pub fn settings_lang_desc(&self) -> &'static str {
        if self.is_zh() {
            "选择界面显示语言"
        } else {
            "Select interface display language"
        }
    }

    pub fn settings_about_title(&self) -> &'static str {
        if self.is_zh() { "关于 Xylos light" } else { "About Xylos light" }
    }

    pub fn settings_about_desc(&self) -> &'static str {
        if self.is_zh() {
            "Xylos light WebDAV Client 是一个基于 Rust + Dioxus 构建的跨平台客户端应用。支持多 WebDAV 服务器管理、离线浏览与 PWA 安装。"
        } else {
            "Xylos light WebDAV Client is a cross-platform client application built with Rust + Dioxus, supporting multi-server WebDAV management, offline browsing, and PWA installation."
        }
    }

    pub fn settings_version_label(&self) -> &'static str {
        if self.is_zh() { "客户端版本: " } else { "Client Version: " }
    }

    pub fn settings_cors_title(&self) -> &'static str {
        if self.is_zh() { "跨域与 CORS 说明" } else { "Cross-Origin (CORS) Notice" }
    }

    pub fn settings_cors_desc(&self) -> &'static str {
        if self.is_zh() {
            "由于浏览器安全限制，若连接外部 WebDAV 服务，目标服务器需配置允许跨域（CORS）访问，并允许 PROPFIND、MKCOL、MOVE、COPY 等 WebDAV 特殊 HTTP 方法。"
        } else {
            "Due to browser security restrictions, connecting to external WebDAV servers requires CORS to be allowed on the target server, permitting PROPFIND, MKCOL, MOVE, COPY, and other WebDAV HTTP methods."
        }
    }

    pub fn settings_pwa_title(&self) -> &'static str {
        if self.is_zh() { "添加到主屏幕 (PWA)" } else { "Add to Home Screen (PWA)" }
    }

    pub fn settings_pwa_desc(&self) -> &'static str {
        if self.is_zh() {
            "添加到主屏幕可移除地址栏，享受原生全屏体验"
        } else {
            "Add to home screen to remove address bar for fullscreen experience"
        }
    }

    pub fn settings_pwa_btn(&self) -> &'static str {
        if self.is_zh() { "安装指引" } else { "Install Guide" }
    }

    pub fn settings_cache_title(&self) -> &'static str {
        if self.is_zh() { "PWA 离线缓存与更新" } else { "PWA Cache & Reload" }
    }

    pub fn settings_cache_desc(&self) -> &'static str {
        if self.is_zh() {
            "清空离线缓存并强制刷新以获取最新版本"
        } else {
            "Clear offline cache and force reload to get latest version"
        }
    }

    pub fn settings_cache_btn(&self) -> &'static str {
        if self.is_zh() { "清理缓存" } else { "Clear Cache" }
    }

    pub fn settings_data_title(&self) -> &'static str {
        if self.is_zh() { "本地数据管理" } else { "Local Data Management" }
    }

    pub fn settings_data_desc(&self) -> &'static str {
        if self.is_zh() {
            "清除所有已保存的服务器配置及登录凭据"
        } else {
            "Clear all saved server configurations and credentials"
        }
    }

    pub fn settings_data_clear_btn(&self) -> &'static str {
        if self.is_zh() { "清空数据" } else { "Clear Data" }
    }

    pub fn settings_data_cleared_btn(&self) -> &'static str {
        if self.is_zh() { "已清空" } else { "Cleared" }
    }

    pub fn settings_clear_confirm_title(&self) -> &'static str {
        if self.is_zh() { "确认清除数据" } else { "Confirm Clear Data" }
    }

    pub fn settings_clear_confirm_desc(&self) -> &'static str {
        if self.is_zh() {
            "确定要清空本地保存的所有服务器配置及登录凭据吗？清除后需重新添加服务器，此操作无法撤销。"
        } else {
            "Are you sure you want to clear all locally saved server configurations and credentials? You will need to re-add servers. This action cannot be undone."
        }
    }

    // File Browser
    pub fn files_title_suffix(&self) -> &'static str {
        if self.is_zh() { "文件" } else { "Files" }
    }

    pub fn tooltip_sort(&self) -> &'static str {
        if self.is_zh() { "排序" } else { "Sort" }
    }

    pub fn tooltip_grid_view(&self) -> &'static str {
        if self.is_zh() { "网格视图" } else { "Grid View" }
    }

    pub fn tooltip_list_view(&self) -> &'static str {
        if self.is_zh() { "列表视图" } else { "List View" }
    }

    pub fn tooltip_upload(&self) -> &'static str {
        if self.is_zh() { "上传文件" } else { "Upload File" }
    }

    pub fn tooltip_new_folder(&self) -> &'static str {
        if self.is_zh() { "新建目录" } else { "New Folder" }
    }

    pub fn tooltip_back(&self) -> &'static str {
        if self.is_zh() { "返回上一级" } else { "Back to Parent" }
    }

    pub fn tooltip_refresh(&self) -> &'static str {
        if self.is_zh() { "刷新" } else { "Refresh" }
    }

    pub fn tooltip_actions_menu(&self) -> &'static str {
        if self.is_zh() { "操作菜单" } else { "Actions Menu" }
    }

    pub fn loading_files(&self) -> &'static str {
        if self.is_zh() { "正在加载文件列表..." } else { "Loading files..." }
    }

    pub fn empty_directory_title(&self) -> &'static str {
        if self.is_zh() { "当前目录为空" } else { "This directory is empty" }
    }

    pub fn empty_directory_desc(&self) -> &'static str {
        if self.is_zh() {
            "您可以点击右上角按钮上传文件或新建目录。"
        } else {
            "You can click the buttons in the top right to upload files or create a new folder."
        }
    }

    pub fn folder_tag(&self) -> &'static str {
        if self.is_zh() { "目录" } else { "Folder" }
    }

    pub fn folder_tag_with_date(&self, date: &str) -> String {
        if self.is_zh() {
            format!("目录 · {date}")
        } else {
            format!("Folder · {date}")
        }
    }

    pub fn menu_download(&self) -> &'static str {
        if self.is_zh() { "下载" } else { "Download" }
    }

    pub fn menu_preview(&self) -> &'static str {
        if self.is_zh() { "预览" } else { "Preview" }
    }

    pub fn preview_loading(&self) -> &'static str {
        if self.is_zh() { "正在加载预览..." } else { "Loading preview..." }
    }

    pub fn preview_unsupported_title(&self) -> &'static str {
        if self.is_zh() { "此文件暂不支持预览" } else { "Preview is not available for this file" }
    }

    pub fn preview_unsupported_desc(&self, extension: &str) -> String {
        if self.is_zh() {
            if extension.is_empty() {
                "浏览器无法直接显示这种文件类型的格式。请下载后使用本地应用打开。".into()
            } else {
                format!(".{extension} 文件无法在浏览器中安全预览。请下载后使用本地应用打开。")
            }
        } else if extension.is_empty() {
            "This file format cannot be displayed safely in the browser. Download it and open it with a local app.".into()
        } else {
            format!(".{extension} files cannot be previewed safely in the browser. Download the file and open it with a local app.")
        }
    }

    pub fn preview_too_large_title(&self) -> &'static str {
        if self.is_zh() { "文件过大，未加载预览" } else { "File is too large to preview" }
    }

    pub fn preview_too_large_desc(&self, limit: &str) -> String {
        if self.is_zh() {
            format!("为保证应用流畅运行，超过 {limit} 的文件不会自动加载。请下载后查看。")
        } else {
            format!("To keep the app responsive, files larger than {limit} are not loaded automatically. Download the file to view it.")
        }
    }

    pub fn preview_error_title(&self) -> &'static str {
        if self.is_zh() { "预览加载失败" } else { "Failed to load preview" }
    }

    pub fn preview_error_desc(&self) -> &'static str {
        if self.is_zh() {
            "文件可能已损坏、网络连接中断，或浏览器不支持此格式。您仍可以尝试下载。"
        } else {
            "The file may be damaged, the connection was interrupted, or the browser does not support this format. You can still download it."
        }
    }

    pub fn preview_download_file(&self) -> &'static str {
        if self.is_zh() { "下载文件" } else { "Download File" }
    }

    pub fn tooltip_close_preview(&self) -> &'static str {
        if self.is_zh() { "关闭预览" } else { "Close Preview" }
    }

    pub fn preview_previous_image(&self) -> &'static str {
        if self.is_zh() { "上一张图片" } else { "Previous image" }
    }

    pub fn preview_next_image(&self) -> &'static str {
        if self.is_zh() { "下一张图片" } else { "Next image" }
    }

    pub fn preview_swipe_hint(&self) -> &'static str {
        if self.is_zh() { "左右滑动切换图片" } else { "Swipe left or right to switch images" }
    }

    pub fn menu_rename(&self) -> &'static str {
        if self.is_zh() { "重命名" } else { "Rename" }
    }

    pub fn menu_delete(&self) -> &'static str {
        if self.is_zh() { "删除" } else { "Delete" }
    }

    pub fn modal_new_folder_title(&self) -> &'static str {
        if self.is_zh() { "新建目录" } else { "New Folder" }
    }

    pub fn modal_new_folder_label(&self) -> &'static str {
        if self.is_zh() { "目录名称" } else { "Folder Name" }
    }

    pub fn modal_new_folder_placeholder(&self) -> &'static str {
        if self.is_zh() { "输入新目录名称" } else { "Enter new folder name" }
    }

    pub fn modal_rename_title(&self) -> &'static str {
        if self.is_zh() { "重命名" } else { "Rename" }
    }

    pub fn modal_rename_label(&self, name: &str) -> String {
        if self.is_zh() {
            format!("原名称: {name}")
        } else {
            format!("Original Name: {name}")
        }
    }

    pub fn modal_rename_placeholder(&self) -> &'static str {
        if self.is_zh() { "输入新名称" } else { "Enter new name" }
    }

    pub fn modal_delete_confirm_title(&self) -> &'static str {
        if self.is_zh() { "确认删除" } else { "Confirm Delete" }
    }

    pub fn modal_delete_folder_desc(&self, name: &str) -> String {
        if self.is_zh() {
            format!("确定要删除目录 \"{name}\" 及其内部的所有内容吗？此操作无法撤销。")
        } else {
            format!("Are you sure you want to delete folder \"{name}\" and all of its contents? This action cannot be undone.")
        }
    }

    pub fn modal_delete_file_desc(&self, name: &str) -> String {
        if self.is_zh() {
            format!("确定要删除文件 \"{name}\" 吗？此操作无法撤销。")
        } else {
            format!("Are you sure you want to delete file \"{name}\"? This action cannot be undone.")
        }
    }

    pub fn notify_read_failed(&self) -> &'static str {
        if self.is_zh() { "无法读取所选文件" } else { "Failed to read selected file" }
    }

    pub fn notify_uploaded(&self, name: &str) -> String {
        if self.is_zh() {
            format!("文件 {name} 上传成功")
        } else {
            format!("File {name} uploaded successfully")
        }
    }

    pub fn notify_downloading(&self, name: &str) -> String {
        if self.is_zh() {
            format!("正在下载 {name}")
        } else {
            format!("Downloading {name}...")
        }
    }

    pub fn err_folder_name_empty(&self) -> &'static str {
        if self.is_zh() { "请输入目录名称" } else { "Please enter folder name" }
    }

    pub fn notify_folder_created(&self) -> &'static str {
        if self.is_zh() { "目录创建成功" } else { "Folder created successfully" }
    }

    pub fn err_new_name_empty(&self) -> &'static str {
        if self.is_zh() { "请输入新名称" } else { "Please enter new name" }
    }

    pub fn notify_renamed(&self) -> &'static str {
        if self.is_zh() { "重命名成功" } else { "Renamed successfully" }
    }

    pub fn notify_deleted(&self, name: &str) -> String {
        if self.is_zh() {
            format!("已删除 {name}")
        } else {
            format!("Deleted {name}")
        }
    }

    // PWA Guide
    pub fn pwa_banner_text(&self) -> &'static str {
        if self.is_zh() {
            "安装 Xylos light 到手机主屏幕，享受全屏原生客户端体验"
        } else {
            "Install Xylos light to your home screen for a fullscreen native experience"
        }
    }

    pub fn pwa_banner_btn(&self) -> &'static str {
        if self.is_zh() { "安装指引" } else { "Install Guide" }
    }

    pub fn pwa_banner_dismiss(&self) -> &'static str {
        if self.is_zh() { "暂不安装" } else { "Dismiss" }
    }

    pub fn pwa_guide_title(&self) -> &'static str {
        if self.is_zh() { "添加到手机主屏幕 (PWA)" } else { "Add to Home Screen (PWA)" }
    }

    pub fn pwa_guide_desc(&self) -> &'static str {
        if self.is_zh() {
            "将 Xylos light 安装到手机主屏幕后，将移除浏览器地址栏与底部工具栏，提供独立应用的高性能全屏体验。"
        } else {
            "Installing Xylos light to your home screen removes the browser address bar and toolbars, offering a fullscreen native app experience."
        }
    }

    pub fn pwa_ios_step1_prefix(&self) -> &'static str {
        if self.is_zh() { "点击 Safari 浏览器底部的 " } else { "Tap the " }
    }

    pub fn pwa_ios_step1_strong(&self) -> &'static str {
        if self.is_zh() { "「分享」按钮" } else { "Share button" }
    }

    pub fn pwa_ios_step1_suffix(&self) -> &'static str {
        if self.is_zh() { " (方框带向上箭头图标 ⎋)" } else { " at the bottom of Safari (⎋ icon)" }
    }

    pub fn pwa_ios_step2_prefix(&self) -> &'static str {
        if self.is_zh() { "在弹出菜单中向下滑动，找到并点击 " } else { "Scroll down in the share sheet and tap " }
    }

    pub fn pwa_ios_step2_strong(&self) -> &'static str {
        if self.is_zh() { "「添加到主屏幕」" } else { "Add to Home Screen" }
    }

    pub fn pwa_ios_step2_suffix(&self) -> &'static str {
        if self.is_zh() { " (Add to Home Screen)" } else { "" }
    }

    pub fn pwa_ios_step3_prefix(&self) -> &'static str {
        if self.is_zh() { "点击右上角 " } else { "Tap " }
    }

    pub fn pwa_ios_step3_strong(&self) -> &'static str {
        if self.is_zh() { "「添加」" } else { "Add" }
    }

    pub fn pwa_ios_step3_suffix(&self) -> &'static str {
        if self.is_zh() {
            "。返回手机桌面点击 Xylos light 图标，即可开启全屏无地址栏体验！"
        } else {
            " in the top right. Launch Xylos light from home screen for fullscreen experience!"
        }
    }

    pub fn pwa_other_step1_prefix(&self) -> &'static str {
        if self.is_zh() { "点击浏览器右上角菜单图标 " } else { "Tap the menu icon " }
    }

    pub fn pwa_other_step1_strong(&self) -> &'static str {
        if self.is_zh() { "「···」" } else { "「···」" }
    }

    pub fn pwa_other_step2_prefix(&self) -> &'static str {
        if self.is_zh() { "在菜单列表中选择 " } else { "Select " }
    }

    pub fn pwa_other_step2_install(&self) -> &'static str {
        if self.is_zh() { "「安装应用」" } else { "Install App" }
    }

    pub fn pwa_other_step2_or(&self) -> &'static str {
        if self.is_zh() { " 或 " } else { " or " }
    }

    pub fn pwa_other_step2_homescreen(&self) -> &'static str {
        if self.is_zh() { "「添加至主屏幕」" } else { "Add to Home screen" }
    }

    pub fn pwa_other_step3(&self) -> &'static str {
        if self.is_zh() {
            "确认添加后，从桌面图标直接启动应用，享受全屏原生体验。"
        } else {
            "Confirm the installation, then open from your home screen for native fullscreen experience."
        }
    }

    // Main Toasts
    pub fn toast_server_connected(&self, name: &str) -> String {
        if self.is_zh() {
            format!("服务器 \"{name}\" 连接成功")
        } else {
            format!("Server \"{name}\" connected successfully")
        }
    }

    pub fn toast_conn_failed(&self, err: &str) -> String {
        if self.is_zh() {
            format!("连接失败: {err}")
        } else {
            format!("Connection failed: {err}")
        }
    }

    pub fn toast_server_saved(&self, name: &str) -> String {
        if self.is_zh() {
            format!("服务器 \"{name}\" 保存成功")
        } else {
            format!("Server \"{name}\" saved successfully")
        }
    }

    pub fn toast_server_deleted(&self, name: &str) -> String {
        if self.is_zh() {
            format!("已删除服务器 \"{name}\"")
        } else {
            format!("Deleted server \"{name}\"")
        }
    }

    pub fn toast_data_cleared(&self) -> &'static str {
        if self.is_zh() {
            "已清除所有本地数据与缓存"
        } else {
            "All local data and cache have been cleared"
        }
    }
}
