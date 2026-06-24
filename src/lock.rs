use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use axum::http::header::HeaderName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLock {
    pub href_path: String,
    pub token: String,
    pub owner: String,
    pub expires_at: Instant,
}

pub struct LockStore {
    counter: AtomicU64,
    locks: Mutex<HashMap<String, ActiveLock>>,
}

impl LockStore {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            locks: Mutex::new(HashMap::new()),
        }
    }

    pub fn lock(
        &self,
        href_path: &str,
        owner: &str,
        timeout: Duration,
    ) -> Result<ActiveLock, ActiveLock> {
        let now = Instant::now();
        let mut locks = self.locks.lock().expect("lock store should not poison");
        locks.retain(|_, active_lock| active_lock.expires_at > now);

        if let Some(conflict) = locks
            .values()
            .find(|active_lock| paths_overlap(&active_lock.href_path, href_path))
        {
            return Err(conflict.clone());
        }

        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let token = format!(
            "opaquelocktoken:{:x}",
            md5::compute(format!("{href_path}:{owner}:{timestamp}:{counter}"))
        );
        let active_lock = ActiveLock {
            href_path: href_path.to_owned(),
            token: token.clone(),
            owner: owner.to_owned(),
            expires_at: now + timeout,
        };

        locks.insert(token, active_lock.clone());
        Ok(active_lock)
    }

    pub fn unlock(&self, href_path: &str, token: &str) -> bool {
        let mut locks = self.locks.lock().expect("lock store should not poison");
        let Some(active_lock) = locks.get(token) else {
            return false;
        };

        if !paths_overlap(&active_lock.href_path, href_path) {
            return false;
        }

        locks.remove(token);
        true
    }

    pub fn conflicting_lock(
        &self,
        href_path: &str,
        submitted_tokens: &[String],
    ) -> Option<ActiveLock> {
        let now = Instant::now();
        let mut locks = self.locks.lock().expect("lock store should not poison");
        locks.retain(|_, active_lock| active_lock.expires_at > now);

        locks
            .values()
            .find(|active_lock| {
                paths_overlap(&active_lock.href_path, href_path)
                    && !submitted_tokens
                        .iter()
                        .any(|token| token == &active_lock.token)
            })
            .cloned()
    }
}

impl Default for LockStore {
    fn default() -> Self {
        Self::new()
    }
}

pub fn submitted_lock_tokens(headers: &HeaderMap) -> Vec<String> {
    const IF: HeaderName = HeaderName::from_static("if");
    const LOCK_TOKEN: HeaderName = HeaderName::from_static("lock-token");

    let mut tokens = Vec::new();
    for header in [IF, LOCK_TOKEN] {
        for value in headers.get_all(header) {
            if let Ok(value) = value.to_str() {
                extract_angle_tokens(value, &mut tokens);
            }
        }
    }

    tokens
}

pub fn timeout_from_header(value: Option<&str>) -> Duration {
    const DEFAULT_SECS: u64 = 300;
    const MAX_SECS: u64 = 3600;

    let Some(value) = value else {
        return Duration::from_secs(DEFAULT_SECS);
    };

    value
        .split(',')
        .find_map(|part| part.trim().strip_prefix("Second-"))
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map(|seconds| seconds.clamp(1, MAX_SECS))
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_SECS))
}

fn extract_angle_tokens(value: &str, tokens: &mut Vec<String>) {
    let mut rest = value;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let token = rest[..end].trim();
        if token.starts_with("opaquelocktoken:") {
            tokens.push(token.to_owned());
        }
        rest = &rest[end + 1..];
    }
}

fn paths_overlap(left: &str, right: &str) -> bool {
    path_contains(left, right) || path_contains(right, left)
}

fn path_contains(parent: &str, child: &str) -> bool {
    let parent = parent.trim_end_matches('/');
    let child = child.trim_end_matches('/');

    child == parent
        || child
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::HeaderValue;

    #[test]
    fn extracts_lock_tokens_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("if"),
            HeaderValue::from_static("(<opaquelocktoken:abc>)"),
        );
        headers.insert(
            HeaderName::from_static("lock-token"),
            HeaderValue::from_static("<opaquelocktoken:def>"),
        );

        assert_eq!(
            submitted_lock_tokens(&headers),
            vec!["opaquelocktoken:abc", "opaquelocktoken:def"]
        );
    }

    #[test]
    fn detects_overlapping_paths() {
        assert!(paths_overlap("/dav/a", "/dav/a/b"));
        assert!(paths_overlap("/dav/a/b", "/dav/a"));
        assert!(!paths_overlap("/dav/a", "/dav/ab"));
    }
}
