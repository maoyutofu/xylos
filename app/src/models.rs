use serde::{Deserialize, Serialize};

pub const SERVERS_STORAGE_KEY: &str = "xylos.webdav.servers";
pub const ACTIVE_SERVER_ID_KEY: &str = "xylos.webdav.active_server_id";
pub const LEGACY_CONNECTION_KEY: &str = "xylos.webdav.connection";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ServerAccount {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub allow_http: bool,
    #[serde(default)]
    pub trust_self_signed_cert: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    #[default]
    Basic,
    Digest,
    Bearer,
}

impl AuthType {
    pub fn label(&self) -> &'static str {
        match self {
            AuthType::Basic => "Basic",
            AuthType::Digest => "Digest",
            AuthType::Bearer => "Bearer",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum NavTab {
    #[default]
    Servers,
    Settings,
}


#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ViewMode {
    #[default]
    List,
    Grid,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SortField {
    #[default]
    Name,
    Type,
    Size,
    Modified,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub href: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: String,
    pub etag: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransferItem {
    pub id: String,
    pub filename: String,
    pub is_upload: bool,
    pub size: u64,
    pub status: TransferStatus,
    pub time: String,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Success,
    Failed,
    Running,
}
