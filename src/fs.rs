use std::io;
use std::path::{Component, Path, PathBuf};

use percent_encoding::percent_decode_str;

use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DavPath {
    pub href_path: String,
    pub fs_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    OutsideBasePath,
    InvalidEncoding,
    UnsafePath,
}

#[derive(Debug)]
pub enum CanonicalPathError {
    Root(io::Error),
    Path(io::Error),
    MissingParent,
    OutsideRoot,
}

pub fn resolve_dav_path_with_root(
    config: &AppConfig,
    root_dir: &Path,
    request_path: &str,
) -> Result<DavPath, PathError> {
    let relative = strip_base_path(request_path, &config.server.base_path)
        .ok_or(PathError::OutsideBasePath)?;
    let decoded = percent_decode_str(relative)
        .decode_utf8()
        .map_err(|_| PathError::InvalidEncoding)?;

    if decoded.contains('\0') {
        return Err(PathError::UnsafePath);
    }

    let mut fs_path = root_dir.to_path_buf();
    let mut href_segments = Vec::new();

    for component in Path::new(decoded.as_ref()).components() {
        match component {
            Component::Normal(segment) => {
                fs_path.push(segment);
                href_segments.push(segment.to_string_lossy().into_owned());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(PathError::UnsafePath);
            }
        }
    }

    let href_path = if href_segments.is_empty() {
        config.server.base_path.clone()
    } else if config.server.base_path == "/" {
        format!("/{}", href_segments.join("/"))
    } else {
        format!("{}/{}", config.server.base_path, href_segments.join("/"))
    };

    Ok(DavPath { href_path, fs_path })
}

pub fn canonicalize_existing_under_root(
    root_dir: &Path,
    path: &Path,
) -> Result<PathBuf, CanonicalPathError> {
    let root = canonicalize_root(root_dir)?;
    let path = std::fs::canonicalize(path).map_err(CanonicalPathError::Path)?;

    if path.starts_with(&root) {
        Ok(path)
    } else {
        Err(CanonicalPathError::OutsideRoot)
    }
}

pub fn canonicalize_parent_under_root(
    root_dir: &Path,
    path: &Path,
) -> Result<PathBuf, CanonicalPathError> {
    let root = canonicalize_root(root_dir)?;
    let parent = path.parent().ok_or(CanonicalPathError::MissingParent)?;
    let parent = std::fs::canonicalize(parent).map_err(CanonicalPathError::Path)?;

    if parent.starts_with(&root) {
        Ok(parent)
    } else {
        Err(CanonicalPathError::OutsideRoot)
    }
}

fn canonicalize_root(root_dir: &Path) -> Result<PathBuf, CanonicalPathError> {
    std::fs::canonicalize(root_dir).map_err(CanonicalPathError::Root)
}

fn strip_base_path<'a>(path: &'a str, base_path: &str) -> Option<&'a str> {
    if base_path == "/" {
        return path.strip_prefix('/');
    }

    if path == base_path {
        return Some("");
    }

    path.strip_prefix(base_path)
        .and_then(|rest| rest.strip_prefix('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AppConfig, AuthConfig, AuthScheme, DigestAlgorithm, LoggingConfig, MethodsConfig,
        ServerConfig, UserConfig,
    };

    fn config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                base_path: "/dav".into(),
                create_root_if_missing: true,
                tls: None,
            },
            auth: AuthConfig {
                enabled: vec![AuthScheme::Bearer],
                realm: "xylos".into(),
                digest_algorithm: DigestAlgorithm::Md5,
                nonce_ttl_secs: 300,
                token_header: "Authorization".into(),
            },
            methods: MethodsConfig { disabled: vec![] },
            logging: LoggingConfig {
                level: "info".into(),
            },
            users: vec![UserConfig {
                username: "admin".into(),
                root_dir: "/tmp/xylos-root".into(),
                password_hash: None,
                digest_ha1: None,
                enabled: true,
                permissions: vec![],
                tokens: vec![],
            }],
        }
    }

    #[test]
    fn resolves_root_and_child_paths() {
        let config = config();
        let root_dir = PathBuf::from("/tmp/xylos-root");

        let root =
            resolve_dav_path_with_root(&config, &root_dir, "/dav").expect("root should resolve");
        assert_eq!(root.href_path, "/dav");
        assert_eq!(root.fs_path, PathBuf::from("/tmp/xylos-root"));

        let child = resolve_dav_path_with_root(&config, &root_dir, "/dav/docs/readme.txt")
            .expect("child should resolve");
        assert_eq!(child.href_path, "/dav/docs/readme.txt");
        assert_eq!(
            child.fs_path,
            PathBuf::from("/tmp/xylos-root/docs/readme.txt")
        );
    }

    #[test]
    fn rejects_paths_outside_base_or_with_parent_components() {
        let config = config();
        let root_dir = PathBuf::from("/tmp/xylos-root");

        assert_eq!(
            resolve_dav_path_with_root(&config, &root_dir, "/other"),
            Err(PathError::OutsideBasePath)
        );
        assert_eq!(
            resolve_dav_path_with_root(&config, &root_dir, "/dav/../secret"),
            Err(PathError::UnsafePath)
        );
        assert_eq!(
            resolve_dav_path_with_root(&config, &root_dir, "/dav/%2e%2e/secret"),
            Err(PathError::UnsafePath)
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonicalize_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("xylos-canon-test-{suffix}"));
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "secret").unwrap();
        symlink(outside.join("secret.txt"), root.join("linked-secret.txt")).unwrap();
        symlink(&outside, root.join("linked-outside-dir")).unwrap();

        assert!(matches!(
            canonicalize_existing_under_root(&root, &root.join("linked-secret.txt")),
            Err(CanonicalPathError::OutsideRoot)
        ));
        assert!(matches!(
            canonicalize_parent_under_root(&root, &root.join("linked-outside-dir/new.txt")),
            Err(CanonicalPathError::OutsideRoot)
        ));

        std::fs::remove_dir_all(base).unwrap();
    }
}
