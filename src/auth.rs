use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

use crate::config::{AppConfig, AuthScheme, DigestAlgorithm, Permission, UserConfig};
use crate::password::verify_password_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub username: String,
    pub root_dir: PathBuf,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFailure {
    Missing,
    Invalid,
}

pub struct NonceStore {
    ttl: Duration,
    counter: AtomicU64,
    nonces: Mutex<HashMap<String, Instant>>,
}

impl NonceStore {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            ttl: Duration::from_secs(ttl_secs),
            counter: AtomicU64::new(0),
            nonces: Mutex::new(HashMap::new()),
        }
    }

    pub fn issue(&self) -> String {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let nonce = md5_hex(&format!("{now}:{counter}"));
        let expires_at = Instant::now() + self.ttl;

        let mut nonces = self
            .nonces
            .lock()
            .expect("nonce store lock should not poison");
        nonces.insert(nonce.clone(), expires_at);
        nonce
    }

    fn validate(&self, nonce: &str) -> bool {
        let now = Instant::now();
        let mut nonces = self
            .nonces
            .lock()
            .expect("nonce store lock should not poison");
        nonces.retain(|_, expires_at| *expires_at > now);
        nonces
            .get(nonce)
            .is_some_and(|expires_at| *expires_at > now)
    }
}

pub fn authenticate(
    config: &AppConfig,
    headers: &HeaderMap,
    method: &str,
    request_uri: &str,
    nonce_store: &NonceStore,
) -> Result<Principal, AuthFailure> {
    let Some(header) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(AuthFailure::Missing);
    };

    if let Some(token) = header.strip_prefix("Bearer ") {
        if config.auth.enabled.contains(&AuthScheme::Bearer) {
            return authenticate_bearer(config, token.trim());
        }
    }

    if let Some(encoded) = header.strip_prefix("Basic ") {
        if config.auth.enabled.contains(&AuthScheme::Basic) {
            return authenticate_basic(config, encoded.trim());
        }
    }

    if let Some(params) = header.strip_prefix("Digest ") {
        if config.auth.enabled.contains(&AuthScheme::Digest) {
            return authenticate_digest(config, params, method, request_uri, nonce_store);
        }
    }

    Err(AuthFailure::Invalid)
}

pub fn www_authenticate_values(config: &AppConfig, nonce_store: &NonceStore) -> Vec<String> {
    let mut values = Vec::new();

    if config.auth.enabled.contains(&AuthScheme::Digest) {
        values.push(format!(
            "Digest realm=\"{}\", nonce=\"{}\", algorithm={}, qop=\"auth\"",
            config.auth.realm,
            nonce_store.issue(),
            config.auth.digest_algorithm.as_http_name()
        ));
    }

    if config.auth.enabled.contains(&AuthScheme::Basic) {
        values.push(format!("Basic realm=\"{}\"", config.auth.realm));
    }

    if config.auth.enabled.contains(&AuthScheme::Bearer) {
        values.push(format!("Bearer realm=\"{}\"", config.auth.realm));
    }

    values
}

fn authenticate_bearer(config: &AppConfig, token: &str) -> Result<Principal, AuthFailure> {
    config
        .users
        .iter()
        .find(|user| user.enabled && user.tokens.iter().any(|candidate| candidate == token))
        .map(principal_from_user)
        .ok_or(AuthFailure::Invalid)
}

fn authenticate_basic(config: &AppConfig, encoded: &str) -> Result<Principal, AuthFailure> {
    let decoded = STANDARD.decode(encoded).map_err(|_| AuthFailure::Invalid)?;
    let decoded = String::from_utf8(decoded).map_err(|_| AuthFailure::Invalid)?;
    let (username, password) = decoded.split_once(':').ok_or(AuthFailure::Invalid)?;

    let user = config
        .users
        .iter()
        .find(|user| user.enabled && user.username == username)
        .ok_or(AuthFailure::Invalid)?;

    if verify_password(user, password) {
        Ok(principal_from_user(user))
    } else {
        Err(AuthFailure::Invalid)
    }
}

fn authenticate_digest(
    config: &AppConfig,
    params: &str,
    method: &str,
    request_uri: &str,
    nonce_store: &NonceStore,
) -> Result<Principal, AuthFailure> {
    let params = parse_digest_params(params);
    let username = params.get("username").ok_or(AuthFailure::Invalid)?;
    let realm = params.get("realm").ok_or(AuthFailure::Invalid)?;
    let nonce = params.get("nonce").ok_or(AuthFailure::Invalid)?;
    let uri = params.get("uri").ok_or(AuthFailure::Invalid)?;
    let response = params.get("response").ok_or(AuthFailure::Invalid)?;
    let expected_algorithm = config.auth.digest_algorithm.as_http_name();

    if let Some(algorithm) = params.get("algorithm") {
        if !algorithm.eq_ignore_ascii_case(expected_algorithm) {
            return Err(AuthFailure::Invalid);
        }
    }

    if realm != &config.auth.realm || uri != request_uri || !nonce_store.validate(nonce) {
        return Err(AuthFailure::Invalid);
    }

    let user = config
        .users
        .iter()
        .find(|user| user.enabled && &user.username == username)
        .ok_or(AuthFailure::Invalid)?;
    let ha1 = user.digest_ha1.as_deref().ok_or(AuthFailure::Invalid)?;
    let digest_algorithm = config.auth.digest_algorithm;

    let ha2 = digest_hex(digest_algorithm, &format!("{method}:{uri}"));
    let expected = if let Some(qop) = params.get("qop") {
        let nc = params.get("nc").ok_or(AuthFailure::Invalid)?;
        let cnonce = params.get("cnonce").ok_or(AuthFailure::Invalid)?;
        digest_hex(
            digest_algorithm,
            &format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}"),
        )
    } else {
        digest_hex(digest_algorithm, &format!("{ha1}:{nonce}:{ha2}"))
    };

    if response.eq_ignore_ascii_case(&expected) {
        Ok(principal_from_user(user))
    } else {
        Err(AuthFailure::Invalid)
    }
}

fn verify_password(user: &UserConfig, password: &str) -> bool {
    user.password_hash
        .as_deref()
        .is_some_and(|password_hash| verify_password_hash(password_hash, password))
}

fn principal_from_user(user: &UserConfig) -> Principal {
    Principal {
        username: user.username.clone(),
        root_dir: user.root_dir.clone(),
        permissions: user.permissions.clone(),
    }
}

fn parse_digest_params(input: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let mut rest = input.trim();

    while !rest.is_empty() {
        let Some((key, after_key)) = rest.split_once('=') else {
            break;
        };
        let key = key
            .trim()
            .trim_start_matches(',')
            .trim()
            .to_ascii_lowercase();
        let after_key = after_key.trim_start();

        let (value, remaining) = if let Some(quoted) = after_key.strip_prefix('"') {
            let mut escaped = false;
            let mut end = None;
            for (index, char) in quoted.char_indices() {
                if escaped {
                    escaped = false;
                } else if char == '\\' {
                    escaped = true;
                } else if char == '"' {
                    end = Some(index);
                    break;
                }
            }

            let Some(end) = end else {
                break;
            };
            let value = quoted[..end].replace("\\\"", "\"").replace("\\\\", "\\");
            (value, &quoted[end + 1..])
        } else if let Some((value, remaining)) = after_key.split_once(',') {
            (value.trim().to_owned(), remaining)
        } else {
            (after_key.trim().to_owned(), "")
        };

        if !key.is_empty() {
            params.insert(key, value);
        }

        rest = remaining.trim_start();
        if let Some(stripped) = rest.strip_prefix(',') {
            rest = stripped.trim_start();
        }
    }

    params
}

fn md5_hex(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

fn sha256_hex(input: &str) -> String {
    format!("{:x}", Sha256::digest(input.as_bytes()))
}

fn digest_hex(algorithm: DigestAlgorithm, input: &str) -> String {
    match algorithm {
        DigestAlgorithm::Md5 => md5_hex(input),
        DigestAlgorithm::Sha256 => sha256_hex(input),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;
    use crate::config::{
        AuthConfig, DavMethod, DigestAlgorithm, LoggingConfig, MethodsConfig, ServerConfig,
    };

    fn config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                base_path: "/dav".into(),
                create_root_if_missing: true,
                upload_idle_timeout_secs: 120,
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
            users: vec![
                UserConfig {
                    username: "admin".into(),
                    root_dir: "/tmp/xylos-admin".into(),
                    password_hash: Some(crate::password::hash_password_for_test("secret")),
                    digest_ha1: Some(digest_hex(DigestAlgorithm::Md5, "admin:xylos:secret")),
                    enabled: true,
                    permissions: vec![Permission::Read, Permission::Write],
                    tokens: vec!["token-1".into()],
                },
                UserConfig {
                    username: "disabled".into(),
                    root_dir: "/tmp/xylos-disabled".into(),
                    password_hash: Some(crate::password::hash_password_for_test("secret")),
                    digest_ha1: Some(digest_hex(DigestAlgorithm::Md5, "disabled:xylos:secret")),
                    enabled: false,
                    permissions: vec![Permission::Read],
                    tokens: vec!["token-2".into()],
                },
            ],
        }
    }

    #[test]
    fn authenticates_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-1"));

        let nonce_store = NonceStore::new(300);
        let principal = authenticate(&config(), &headers, "GET", "/dav", &nonce_store)
            .expect("token should authenticate");
        assert_eq!(principal.username, "admin");
    }

    #[test]
    fn authenticates_basic_password() {
        let encoded = STANDARD.encode("admin:secret");
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
        );

        let nonce_store = NonceStore::new(300);
        let principal = authenticate(&config(), &headers, "GET", "/dav", &nonce_store)
            .expect("password should authenticate");
        assert_eq!(principal.username, "admin");
    }

    #[test]
    fn authenticates_digest_password() {
        let config = config();
        let nonce_store = NonceStore::new(300);
        let nonce = nonce_store.issue();
        let uri = "/dav/file.txt";
        let ha1 = digest_hex(DigestAlgorithm::Md5, "admin:xylos:secret");
        let ha2 = digest_hex(DigestAlgorithm::Md5, &format!("GET:{uri}"));
        let response = digest_hex(
            DigestAlgorithm::Md5,
            &format!("{ha1}:{nonce}:00000001:client-nonce:auth:{ha2}"),
        );
        let header = format!(
            r#"Digest username="admin", realm="xylos", nonce="{nonce}", uri="{uri}", algorithm=MD5, qop=auth, nc=00000001, cnonce="client-nonce", response="{response}""#
        );
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&header).unwrap());

        let principal = authenticate(&config, &headers, "GET", uri, &nonce_store)
            .expect("digest should authenticate");
        assert_eq!(principal.username, "admin");
    }

    #[test]
    fn authenticates_digest_password_with_sha256() {
        let mut config = config();
        config.auth.digest_algorithm = DigestAlgorithm::Sha256;
        config.users[0].digest_ha1 =
            Some(digest_hex(DigestAlgorithm::Sha256, "admin:xylos:secret"));

        let nonce_store = NonceStore::new(300);
        let nonce = nonce_store.issue();
        let uri = "/dav/file.txt";
        let ha1 = digest_hex(DigestAlgorithm::Sha256, "admin:xylos:secret");
        let ha2 = digest_hex(DigestAlgorithm::Sha256, &format!("GET:{uri}"));
        let response = digest_hex(
            DigestAlgorithm::Sha256,
            &format!("{ha1}:{nonce}:00000001:client-nonce:auth:{ha2}"),
        );
        let header = format!(
            r#"Digest username="admin", realm="xylos", nonce="{nonce}", uri="{uri}", algorithm=SHA-256, qop=auth, nc=00000001, cnonce="client-nonce", response="{response}""#
        );
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&header).unwrap());

        let principal = authenticate(&config, &headers, "GET", uri, &nonce_store)
            .expect("digest should authenticate");
        assert_eq!(principal.username, "admin");
    }

    #[test]
    fn advertises_configured_digest_algorithm() {
        let mut config = config();
        config.auth.enabled = vec![AuthScheme::Digest];
        config.auth.digest_algorithm = DigestAlgorithm::Sha256;

        let values = www_authenticate_values(&config, &NonceStore::new(300));
        assert_eq!(values.len(), 1);
        assert!(values[0].contains("algorithm=SHA-256"));
    }

    #[test]
    fn rejects_disabled_users() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token-2"));

        let nonce_store = NonceStore::new(300);
        assert_eq!(
            authenticate(&config(), &headers, "GET", "/dav", &nonce_store),
            Err(AuthFailure::Invalid)
        );
    }
}
