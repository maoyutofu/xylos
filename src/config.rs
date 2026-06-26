use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;
use crate::password::{digest_ha1, hash_password, validate_password_hash};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub methods: MethodsConfig,
    pub logging: LoggingConfig,
    pub users: Vec<UserConfig>,
}

#[derive(Debug, Clone)]
pub struct QuickStartConfig {
    pub host: String,
    pub port: u16,
    pub base_path: String,
    pub username: String,
    pub password: String,
    pub root_dir: PathBuf,
    pub realm: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_path: String,
    #[serde(default)]
    pub create_root_if_missing: bool,
    #[serde(default = "default_upload_idle_timeout_secs")]
    pub upload_idle_timeout_secs: u64,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_auth_enabled")]
    pub enabled: Vec<AuthScheme>,
    pub realm: String,
    pub digest_algorithm: DigestAlgorithm,
    pub nonce_ttl_secs: u64,
    pub token_header: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MethodsConfig {
    #[serde(default)]
    pub disabled: Vec<DavMethod>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    pub username: String,
    pub root_dir: PathBuf,
    pub password_hash: Option<String>,
    pub digest_ha1: Option<String>,
    pub enabled: bool,
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthScheme {
    Basic,
    Digest,
    Bearer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DigestAlgorithm {
    Md5,
    #[serde(rename = "sha-256")]
    Sha256,
}

impl DigestAlgorithm {
    pub fn as_http_name(self) -> &'static str {
        match self {
            DigestAlgorithm::Md5 => "MD5",
            DigestAlgorithm::Sha256 => "SHA-256",
        }
    }

    pub fn as_config_name(self) -> &'static str {
        match self {
            DigestAlgorithm::Md5 => "md5",
            DigestAlgorithm::Sha256 => "sha-256",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DavMethod {
    Options,
    Head,
    Get,
    Put,
    Delete,
    Mkcol,
    Copy,
    Move,
    Propfind,
    Proppatch,
    Lock,
    Unlock,
}

impl DavMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            DavMethod::Options => "OPTIONS",
            DavMethod::Head => "HEAD",
            DavMethod::Get => "GET",
            DavMethod::Put => "PUT",
            DavMethod::Delete => "DELETE",
            DavMethod::Mkcol => "MKCOL",
            DavMethod::Copy => "COPY",
            DavMethod::Move => "MOVE",
            DavMethod::Propfind => "PROPFIND",
            DavMethod::Proppatch => "PROPPATCH",
            DavMethod::Lock => "LOCK",
            DavMethod::Unlock => "UNLOCK",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "OPTIONS" => Some(DavMethod::Options),
            "HEAD" => Some(DavMethod::Head),
            "GET" => Some(DavMethod::Get),
            "PUT" => Some(DavMethod::Put),
            "DELETE" => Some(DavMethod::Delete),
            "MKCOL" => Some(DavMethod::Mkcol),
            "COPY" => Some(DavMethod::Copy),
            "MOVE" => Some(DavMethod::Move),
            "PROPFIND" => Some(DavMethod::Propfind),
            "PROPPATCH" => Some(DavMethod::Proppatch),
            "LOCK" => Some(DavMethod::Lock),
            "UNLOCK" => Some(DavMethod::Unlock),
            _ => None,
        }
    }
}

impl fmt::Display for DavMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Read,
    Write,
    Delete,
    Mkdir,
    Move,
    Copy,
}

impl AppConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|source| AppError::ConfigRead {
            path: path.display().to_string(),
            source,
        })?;

        let config: AppConfig = toml::from_str(&raw).map_err(|source| AppError::ConfigParse {
            path: path.display().to_string(),
            source,
        })?;

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        validate_server(&self.server)?;
        validate_auth(&self.auth)?;
        validate_logging(&self.logging)?;
        validate_users(&self.auth, &self.users)?;
        validate_methods(&self.methods)?;
        Ok(())
    }

    pub fn from_quick_start(options: QuickStartConfig) -> Result<Self, AppError> {
        let QuickStartConfig {
            host,
            port,
            base_path,
            username,
            password,
            root_dir,
            realm,
            log_level,
        } = options;

        if username.trim().is_empty() {
            return Err(AppError::QuickStartInvalid(
                "`--username` cannot be empty".into(),
            ));
        }

        if password.is_empty() {
            return Err(AppError::QuickStartInvalid(
                "`--password` cannot be empty".into(),
            ));
        }

        if root_dir.as_os_str().is_empty() {
            return Err(AppError::QuickStartInvalid(
                "`--root-dir` cannot be empty".into(),
            ));
        }

        let password_hash = hash_password(&password).map_err(|source| {
            AppError::QuickStartInvalid(format!("failed to hash `--password`: {source}"))
        })?;
        let digest_ha1 = digest_ha1(DigestAlgorithm::Md5, &username, &realm, &password);

        let config = AppConfig {
            server: ServerConfig {
                host,
                port,
                base_path,
                create_root_if_missing: true,
                upload_idle_timeout_secs: default_upload_idle_timeout_secs(),
                tls: None,
            },
            auth: AuthConfig {
                enabled: vec![AuthScheme::Basic],
                realm: realm.clone(),
                digest_algorithm: DigestAlgorithm::Md5,
                nonce_ttl_secs: 300,
                token_header: "Authorization".into(),
            },
            methods: MethodsConfig { disabled: vec![] },
            logging: LoggingConfig { level: log_level },
            users: vec![UserConfig {
                username,
                root_dir,
                password_hash: Some(password_hash),
                digest_ha1: Some(digest_ha1),
                enabled: true,
                permissions: vec![
                    Permission::Read,
                    Permission::Write,
                    Permission::Delete,
                    Permission::Mkdir,
                    Permission::Move,
                    Permission::Copy,
                ],
                tokens: vec![],
            }],
        };

        config.validate()?;
        Ok(config)
    }

    pub fn startup_summary(&self) -> String {
        let protocol = match self.server.tls.as_ref() {
            Some(tls) if tls.enabled => "https",
            _ => "http",
        };
        let auth_enabled = self
            .auth
            .enabled
            .iter()
            .map(|scheme| scheme.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let disabled_methods = if self.methods.disabled.is_empty() {
            "none".to_owned()
        } else {
            self.methods
                .disabled
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        };
        let enabled_users = self.users.iter().filter(|user| user.enabled).count();
        let tls_summary = match self.server.tls.as_ref() {
            Some(tls) if tls.enabled => format!(
                "enabled(cert_path={}, key_path={})",
                tls.cert_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<unset>".into()),
                tls.key_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<unset>".into())
            ),
            _ => "disabled".into(),
        };

        format!(
            concat!(
                "server.protocol={protocol}, ",
                "server.host={host}, ",
                "server.port={port}, ",
                "server.base_path={base_path}, ",
                "server.create_root_if_missing={create_root_if_missing}, ",
                "server.upload_idle_timeout_secs={upload_idle_timeout_secs}, ",
                "server.tls={tls_summary}, ",
                "auth.enabled=[{auth_enabled}], ",
                "auth.realm={realm}, ",
                "auth.digest_algorithm={digest_algorithm}, ",
                "auth.nonce_ttl_secs={nonce_ttl_secs}, ",
                "auth.token_header={token_header}, ",
                "logging.level={log_level}, ",
                "methods.disabled=[{disabled_methods}], ",
                "users.total={users_total}, ",
                "users.enabled={users_enabled}"
            ),
            protocol = protocol,
            host = self.server.host,
            port = self.server.port,
            base_path = self.server.base_path,
            create_root_if_missing = self.server.create_root_if_missing,
            upload_idle_timeout_secs = self.server.upload_idle_timeout_secs,
            tls_summary = tls_summary,
            auth_enabled = auth_enabled,
            realm = self.auth.realm,
            digest_algorithm = self.auth.digest_algorithm.as_config_name(),
            nonce_ttl_secs = self.auth.nonce_ttl_secs,
            token_header = self.auth.token_header,
            log_level = self.logging.level,
            disabled_methods = disabled_methods,
            users_total = self.users.len(),
            users_enabled = enabled_users,
        )
    }
}

impl AuthScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthScheme::Basic => "basic",
            AuthScheme::Digest => "digest",
            AuthScheme::Bearer => "bearer",
        }
    }
}

fn default_auth_enabled() -> Vec<AuthScheme> {
    vec![AuthScheme::Basic]
}

fn default_upload_idle_timeout_secs() -> u64 {
    120
}

fn validate_server(server: &ServerConfig) -> Result<(), AppError> {
    if server.host.trim().is_empty() {
        return Err(AppError::ConfigInvalid(
            "server.host cannot be empty".into(),
        ));
    }

    if server.base_path.is_empty() || !server.base_path.starts_with('/') {
        return Err(AppError::ConfigInvalid(
            "server.base_path must start with `/`".into(),
        ));
    }

    if server.base_path.len() > 1 && server.base_path.ends_with('/') {
        return Err(AppError::ConfigInvalid(
            "server.base_path must not end with `/` unless it is `/`".into(),
        ));
    }

    if let Some(tls) = &server.tls
        && tls.enabled
    {
        if tls
            .cert_path
            .as_ref()
            .is_none_or(|path| path.as_os_str().is_empty())
        {
            return Err(AppError::ConfigInvalid(
                "server.tls.cert_path must be set when TLS is enabled".into(),
            ));
        }

        if tls
            .key_path
            .as_ref()
            .is_none_or(|path| path.as_os_str().is_empty())
        {
            return Err(AppError::ConfigInvalid(
                "server.tls.key_path must be set when TLS is enabled".into(),
            ));
        }
    }

    Ok(())
}

fn validate_auth(auth: &AuthConfig) -> Result<(), AppError> {
    if auth.enabled.is_empty() {
        return Err(AppError::ConfigInvalid(
            "auth.enabled must contain at least one auth scheme".into(),
        ));
    }

    if auth.enabled.contains(&AuthScheme::Digest) && auth.realm.trim().is_empty() {
        return Err(AppError::ConfigInvalid(
            "auth.realm cannot be empty when digest auth is enabled".into(),
        ));
    }

    if auth.nonce_ttl_secs == 0 {
        return Err(AppError::ConfigInvalid(
            "auth.nonce_ttl_secs must be greater than 0".into(),
        ));
    }

    if auth.token_header.trim().is_empty() {
        return Err(AppError::ConfigInvalid(
            "auth.token_header cannot be empty".into(),
        ));
    }

    Ok(())
}

fn validate_logging(logging: &LoggingConfig) -> Result<(), AppError> {
    if logging.level.trim().is_empty() {
        return Err(AppError::ConfigInvalid(
            "logging.level cannot be empty".into(),
        ));
    }

    Ok(())
}

fn validate_methods(methods: &MethodsConfig) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for method in &methods.disabled {
        if !seen.insert(*method) {
            return Err(AppError::ConfigInvalid(format!(
                "duplicate disabled method: {method:?}"
            )));
        }
    }

    Ok(())
}

fn validate_users(auth: &AuthConfig, users: &[UserConfig]) -> Result<(), AppError> {
    if users.is_empty() {
        return Err(AppError::ConfigInvalid(
            "at least one user must be configured".into(),
        ));
    }

    let mut usernames = HashSet::new();
    let mut tokens = HashSet::new();

    for user in users {
        if user.username.trim().is_empty() {
            return Err(AppError::ConfigInvalid(
                "user.username cannot be empty".into(),
            ));
        }

        if user.root_dir.as_os_str().is_empty() {
            return Err(AppError::ConfigInvalid(format!(
                "user `{}` root_dir cannot be empty",
                user.username
            )));
        }

        if !usernames.insert(user.username.as_str()) {
            return Err(AppError::ConfigInvalid(format!(
                "duplicate username: {}",
                user.username
            )));
        }

        if user.permissions.is_empty() {
            return Err(AppError::ConfigInvalid(format!(
                "user `{}` must have at least one permission",
                user.username
            )));
        }

        if auth.enabled.contains(&AuthScheme::Basic) && is_blank(user.password_hash.as_deref()) {
            return Err(AppError::ConfigInvalid(format!(
                "user `{}` must configure password_hash when basic auth is enabled",
                user.username
            )));
        }

        if let Some(password_hash) = user.password_hash.as_deref() {
            if !validate_password_hash(password_hash) {
                return Err(AppError::ConfigInvalid(format!(
                    "user `{}` password_hash must be a valid Argon2 PHC password hash",
                    user.username
                )));
            }
        }

        if auth.enabled.contains(&AuthScheme::Digest) && is_blank(user.digest_ha1.as_deref()) {
            return Err(AppError::ConfigInvalid(format!(
                "user `{}` must configure digest_ha1 when digest auth is enabled",
                user.username
            )));
        }

        if let Some(digest_ha1) = user.digest_ha1.as_deref() {
            validate_digest_ha1(auth, user, digest_ha1)?;
        }

        for token in &user.tokens {
            if token.trim().is_empty() {
                return Err(AppError::ConfigInvalid(format!(
                    "user `{}` has an empty bearer token",
                    user.username
                )));
            }

            if !tokens.insert(token.as_str()) {
                return Err(AppError::ConfigInvalid(format!(
                    "duplicate bearer token found for user `{}`",
                    user.username
                )));
            }
        }

        if auth.enabled.contains(&AuthScheme::Bearer) && user.tokens.is_empty() {
            return Err(AppError::ConfigInvalid(format!(
                "user `{}` must configure at least one bearer token when bearer auth is enabled",
                user.username
            )));
        }
    }

    Ok(())
}

fn validate_digest_ha1(
    auth: &AuthConfig,
    user: &UserConfig,
    digest_ha1: &str,
) -> Result<(), AppError> {
    let expected_len = match auth.digest_algorithm {
        DigestAlgorithm::Md5 => 32,
        DigestAlgorithm::Sha256 => 64,
    };

    if digest_ha1.len() != expected_len || !digest_ha1.chars().all(|char| char.is_ascii_hexdigit())
    {
        return Err(AppError::ConfigInvalid(format!(
            "user `{}` digest_ha1 must be a {}-character hex {} HA1 hash",
            user.username,
            expected_len,
            auth.digest_algorithm.as_http_name()
        )));
    }

    Ok(())
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".into(),
                port: 8080,
                base_path: "/dav".into(),
                create_root_if_missing: true,
                upload_idle_timeout_secs: default_upload_idle_timeout_secs(),
                tls: None,
            },
            auth: AuthConfig {
                enabled: vec![AuthScheme::Basic, AuthScheme::Digest, AuthScheme::Bearer],
                realm: "xylos".into(),
                digest_algorithm: DigestAlgorithm::Md5,
                nonce_ttl_secs: 300,
                token_header: "Authorization".into(),
            },
            methods: MethodsConfig {
                disabled: vec![DavMethod::Lock],
            },
            logging: LoggingConfig {
                level: "info".into(),
            },
            users: vec![UserConfig {
                username: "admin".into(),
                root_dir: PathBuf::from("/tmp/xylos-admin"),
                password_hash: Some(crate::password::hash_password_for_test("secret")),
                digest_ha1: Some(crate::password::digest_ha1(
                    DigestAlgorithm::Md5,
                    "admin",
                    "xylos",
                    "secret",
                )),
                enabled: true,
                permissions: vec![Permission::Read, Permission::Write],
                tokens: vec!["token-1".into()],
            }],
        }
    }

    #[test]
    fn validates_example_shape() {
        valid_config().validate().expect("config should be valid");
    }

    #[test]
    fn loads_example_config_file() {
        AppConfig::load_from_path("config.example.toml").expect("example config should load");
    }

    #[test]
    fn defaults_auth_enabled_to_basic() {
        let raw = r#"
[server]
host = "127.0.0.1"
port = 8080
base_path = "/dav"

[auth]
realm = "xylos"
digest_algorithm = "md5"
nonce_ttl_secs = 300
token_header = "Authorization"

[methods]
disabled = []

[logging]
level = "info"

[[users]]
username = "admin"
root_dir = "/tmp/xylos-admin"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZS1hZG1pbi1zYWx0$cmVwbGFjZS10aGlzLWFkbWluLWhhc2g"
enabled = true
permissions = ["read"]
"#;

        let config = toml::from_str::<AppConfig>(raw).expect("config should parse");

        assert_eq!(config.auth.enabled, vec![AuthScheme::Basic]);
        assert_eq!(
            config.server.upload_idle_timeout_secs,
            default_upload_idle_timeout_secs()
        );
    }

    #[test]
    fn rejects_plaintext_password_field() {
        let raw = format!(
            r#"
[server]
host = "127.0.0.1"
port = 8080
base_path = "/dav"

[auth]
enabled = ["bearer"]
realm = "xylos"
digest_algorithm = "md5"
nonce_ttl_secs = 300
token_header = "Authorization"

[methods]
disabled = []

[logging]
level = "info"

[[users]]
username = "admin"
root_dir = "/tmp/xylos-admin"
{} = "secret"
enabled = true
permissions = ["read"]
tokens = ["token-1"]
"#,
            "password"
        );

        let err = toml::from_str::<AppConfig>(&raw).expect_err("config should reject password");
        assert!(err.to_string().contains("unknown field `password`"));
    }

    #[test]
    fn rejects_missing_digest_password() {
        let mut config = valid_config();
        config.auth.enabled = vec![AuthScheme::Digest];
        config.users[0].digest_ha1 = None;

        let err = config.validate().expect_err("config should be invalid");
        assert!(
            err.to_string()
                .contains("must configure digest_ha1 when digest auth is enabled")
        );
    }

    #[test]
    fn rejects_missing_basic_password_hash() {
        let mut config = valid_config();
        config.auth.enabled = vec![AuthScheme::Basic];
        config.users[0].password_hash = None;

        let err = config.validate().expect_err("config should be invalid");
        assert!(
            err.to_string()
                .contains("must configure password_hash when basic auth is enabled")
        );
    }

    #[test]
    fn rejects_invalid_password_hash() {
        let mut config = valid_config();
        config.users[0].password_hash = Some("secret".into());

        let err = config.validate().expect_err("config should be invalid");
        assert!(
            err.to_string()
                .contains("password_hash must be a valid Argon2 PHC")
        );
    }

    #[test]
    fn rejects_invalid_digest_ha1() {
        let mut config = valid_config();
        config.users[0].digest_ha1 = Some("not-a-hash".into());

        let err = config.validate().expect_err("config should be invalid");
        assert!(
            err.to_string()
                .contains("digest_ha1 must be a 32-character hex MD5 HA1 hash")
        );
    }

    #[test]
    fn rejects_duplicate_bearer_tokens() {
        let mut config = valid_config();
        config.users.push(UserConfig {
            username: "guest".into(),
            root_dir: PathBuf::from("/tmp/xylos-guest"),
            password_hash: Some(crate::password::hash_password_for_test("secret")),
            digest_ha1: Some(crate::password::digest_ha1(
                DigestAlgorithm::Md5,
                "guest",
                "xylos",
                "secret",
            )),
            enabled: true,
            permissions: vec![Permission::Read],
            tokens: vec!["token-1".into()],
        });

        let err = config.validate().expect_err("config should be invalid");
        assert!(err.to_string().contains("duplicate bearer token"));
    }

    #[test]
    fn rejects_invalid_base_path() {
        let mut config = valid_config();
        config.server.base_path = "dav".into();

        let err = config.validate().expect_err("config should be invalid");
        assert!(err.to_string().contains("server.base_path"));
    }

    #[test]
    fn rejects_enabled_tls_without_certificate_paths() {
        let mut config = valid_config();
        config.server.tls = Some(TlsConfig {
            enabled: true,
            cert_path: None,
            key_path: Some(PathBuf::from("/tmp/key.pem")),
        });

        let err = config.validate().expect_err("config should be invalid");
        assert!(err.to_string().contains("server.tls.cert_path"));
    }

    #[test]
    fn rejects_empty_user_root_dir() {
        let mut config = valid_config();
        config.users[0].root_dir = PathBuf::new();

        let err = config.validate().expect_err("config should be invalid");
        assert!(err.to_string().contains("root_dir cannot be empty"));
    }

    #[test]
    fn maps_dav_methods_from_http_names() {
        assert_eq!(DavMethod::from_name("GET"), Some(DavMethod::Get));
        assert_eq!(DavMethod::from_name("PROPFIND"), Some(DavMethod::Propfind));
        assert_eq!(DavMethod::from_name("PATCH"), None);
    }

    #[test]
    fn builds_quick_start_config() {
        let config = AppConfig::from_quick_start(QuickStartConfig {
            host: "0.0.0.0".into(),
            port: 8080,
            base_path: "/dav".into(),
            username: "admin".into(),
            password: "secret".into(),
            root_dir: PathBuf::from("/tmp/xylos-admin"),
            realm: "xylos".into(),
            log_level: "info".into(),
        })
        .expect("quick start config should be valid");

        assert_eq!(config.auth.enabled, vec![AuthScheme::Basic]);
        assert_eq!(config.users.len(), 1);
        assert_eq!(config.users[0].username, "admin");
        assert!(config.users[0].password_hash.is_some());
        assert_ne!(config.users[0].password_hash.as_deref(), Some("secret"));
        assert_eq!(
            config.users[0].digest_ha1.as_deref(),
            Some("3f107222b3c92793b26ffcc29bd3672d")
        );
    }

    #[test]
    fn startup_summary_redacts_user_secrets() {
        let config = valid_config();

        let summary = config.startup_summary();

        assert!(summary.contains("server.host=0.0.0.0"));
        assert!(summary.contains("auth.enabled=[basic,digest,bearer]"));
        assert!(summary.contains("users.total=1"));
        assert!(!summary.contains("$argon2"));
        assert!(!summary.contains("3f107222b3c92793b26ffcc29bd3672d"));
        assert!(!summary.contains("token-1"));
        assert!(!summary.contains("/tmp/xylos-admin"));
    }
}
