use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::header::{
    ALLOW, CONTENT_LENGTH, CONTENT_TYPE, ETAG, HeaderName, LAST_MODIFIED, WWW_AUTHENTICATE,
};
use axum::http::{HeaderValue, Response, StatusCode};
use axum::routing::any;
use axum_server::tls_rustls::RustlsConfig;
use percent_encoding::{AsciiSet, CONTROLS};
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::auth::{NonceStore, authenticate, www_authenticate_values};
use crate::config::{AppConfig, DavMethod};
use crate::error::AppError;
use crate::fs::{
    CanonicalPathError, PathError, canonicalize_existing_under_root,
    canonicalize_parent_under_root, resolve_dav_path_with_root,
};
use crate::lock::{LockStore, submitted_lock_tokens, timeout_from_header};
use crate::permission::required_permission;
use crate::prop::{DeadProperty, PropStore, parse_proppatch};

const MAX_PUT_BYTES: usize = 512 * 1024 * 1024;
const MAX_PROPPATCH_BYTES: usize = 1024 * 1024;
const DESTINATION: HeaderName = HeaderName::from_static("destination");
const DEPTH: HeaderName = HeaderName::from_static("depth");
const DAV: HeaderName = HeaderName::from_static("dav");
const OVERWRITE: HeaderName = HeaderName::from_static("overwrite");
const TIMEOUT: HeaderName = HeaderName::from_static("timeout");
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
    nonce_store: Arc<NonceStore>,
    lock_store: Arc<LockStore>,
    prop_store: Arc<PropStore>,
    allowed_methods: HeaderValue,
}

pub async fn serve(config: AppConfig) -> Result<(), AppError> {
    prepare_roots(&config).await?;

    let addr = socket_addr(&config)?;
    let tls = config.server.tls.clone().filter(|tls| tls.enabled);
    let app = app(config);

    if let Some(tls) = tls {
        let cert_path = tls.cert_path.as_ref().expect("TLS cert path validated");
        let key_path = tls.key_path.as_ref().expect("TLS key path validated");
        let tls_config = RustlsConfig::from_pem_file(cert_path, key_path).await?;

        info!("xylos listening on https://{addr}");
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = TcpListener::bind(addr).await?;

        info!("xylos listening on http://{addr}");
        axum::serve(listener, app).await?;
    }

    Ok(())
}

fn app(config: AppConfig) -> Router {
    let allowed_methods = allowed_methods(&config);
    let state = AppState {
        nonce_store: Arc::new(NonceStore::new(config.auth.nonce_ttl_secs)),
        lock_store: Arc::new(LockStore::new()),
        prop_store: Arc::new(PropStore::new()),
        config: Arc::new(config),
        allowed_methods,
    };

    Router::new()
        .fallback(any(handle_request))
        .with_state(state)
}

async fn handle_request(State(state): State<AppState>, request: Request) -> Response<Body> {
    let method_name = request.method().as_str().to_owned();
    let path = request.uri().path().to_owned();
    let request_uri = request
        .uri()
        .path_and_query()
        .map(|path_and_query| path_and_query.as_str().to_owned())
        .unwrap_or_else(|| path.clone());
    let mut username = None;

    if !path_matches_base_path(&path, &state.config.server.base_path) {
        return finish_request(
            empty_response(StatusCode::NOT_FOUND),
            &method_name,
            &request_uri,
            username.as_deref(),
            "path_outside_base_path",
        );
    }

    let Some(method) = DavMethod::from_name(&method_name) else {
        return finish_request(
            empty_response(StatusCode::METHOD_NOT_ALLOWED),
            &method_name,
            &request_uri,
            username.as_deref(),
            "unsupported_method",
        );
    };

    if state.config.methods.disabled.contains(&method) {
        return finish_request(
            response_with_allow(
                StatusCode::METHOD_NOT_ALLOWED,
                state.allowed_methods.clone(),
            ),
            &method_name,
            &request_uri,
            username.as_deref(),
            "method_disabled",
        );
    }

    let principal = match authenticate(
        &state.config,
        request.headers(),
        &method_name,
        &request_uri,
        &state.nonce_store,
    ) {
        Ok(principal) => principal,
        Err(_) => {
            return finish_request(
                unauthorized_response(www_authenticate_values(&state.config, &state.nonce_store)),
                &method_name,
                &request_uri,
                username.as_deref(),
                "authentication_failed",
            );
        }
    };
    username = Some(principal.username.clone());

    if let Some(permission) = required_permission(method) {
        if !principal.permissions.contains(&permission) {
            return finish_request(
                empty_response(StatusCode::FORBIDDEN),
                &method_name,
                &request_uri,
                username.as_deref(),
                "permission_denied",
            );
        }
    }

    let effective_root = principal.root_dir.clone();
    let dav_path = match resolve_dav_path_with_root(&state.config, &effective_root, &path) {
        Ok(path) => path,
        Err(PathError::OutsideBasePath) => {
            return finish_request(
                empty_response(StatusCode::NOT_FOUND),
                &method_name,
                &request_uri,
                username.as_deref(),
                "resolved_path_outside_base_path",
            );
        }
        Err(PathError::InvalidEncoding | PathError::UnsafePath) => {
            return finish_request(
                empty_response(StatusCode::BAD_REQUEST),
                &method_name,
                &request_uri,
                username.as_deref(),
                "invalid_request_path",
            );
        }
    };
    let scoped_href = scoped_href(&principal.username, &dav_path.href_path);

    let destination = request
        .headers()
        .get(&DESTINATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let lock_tokens = submitted_lock_tokens(request.headers());
    let timeout = timeout_from_header(
        request
            .headers()
            .get(&TIMEOUT)
            .and_then(|value| value.to_str().ok()),
    );
    let depth = request
        .headers()
        .get(&DEPTH)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("1")
        .to_owned();
    let overwrite = request
        .headers()
        .get(&OVERWRITE)
        .and_then(|value| value.to_str().ok())
        .is_none_or(|value| !value.eq_ignore_ascii_case("F"));

    if matches!(
        method,
        DavMethod::Put
            | DavMethod::Delete
            | DavMethod::Mkcol
            | DavMethod::Copy
            | DavMethod::Move
            | DavMethod::Proppatch
    ) && state
        .lock_store
        .conflicting_lock(&scoped_href, &lock_tokens)
        .is_some()
    {
        return finish_request(
            empty_response(StatusCode::LOCKED),
            &method_name,
            &request_uri,
            username.as_deref(),
            "source_locked",
        );
    }

    let response = match method {
        DavMethod::Options => options_response(state.allowed_methods.clone()),
        DavMethod::Get => get_file_response(&effective_root, &dav_path.fs_path, false).await,
        DavMethod::Head => get_file_response(&effective_root, &dav_path.fs_path, true).await,
        DavMethod::Put => put_file_response(&effective_root, &dav_path.fs_path, request).await,
        DavMethod::Delete => delete_response(&effective_root, &dav_path.fs_path).await,
        DavMethod::Mkcol => mkcol_response(&effective_root, &dav_path.fs_path).await,
        DavMethod::Copy => {
            copy_or_move_response(
                &state.config,
                &effective_root,
                &state.lock_store,
                &dav_path.fs_path,
                &scoped_href,
                destination,
                &lock_tokens,
                overwrite,
                false,
            )
            .await
        }
        DavMethod::Move => {
            copy_or_move_response(
                &state.config,
                &effective_root,
                &state.lock_store,
                &dav_path.fs_path,
                &scoped_href,
                destination,
                &lock_tokens,
                overwrite,
                true,
            )
            .await
        }
        DavMethod::Propfind => {
            propfind_response(
                &effective_root,
                &state.prop_store,
                &principal.username,
                &dav_path.href_path,
                &dav_path.fs_path,
                &depth,
            )
            .await
        }
        DavMethod::Lock => lock_response(
            &effective_root,
            &state.lock_store,
            &dav_path.fs_path,
            &scoped_href,
            &principal.username,
            timeout,
        ),
        DavMethod::Unlock => unlock_response(&state.lock_store, &scoped_href, &lock_tokens),
        DavMethod::Proppatch => {
            proppatch_response(
                &effective_root,
                &state.prop_store,
                &scoped_href,
                &dav_path.fs_path,
                &dav_path.href_path,
                request,
            )
            .await
        }
    };

    let reason = default_reason_for_status(method, response.status());
    finish_request(
        response,
        &method_name,
        &request_uri,
        username.as_deref(),
        reason,
    )
}

fn path_matches_base_path(path: &str, base_path: &str) -> bool {
    if base_path == "/" {
        return path.starts_with('/');
    }

    path == base_path
        || path
            .strip_prefix(base_path)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn allowed_methods(config: &AppConfig) -> HeaderValue {
    const SUPPORTED: [DavMethod; 12] = [
        DavMethod::Options,
        DavMethod::Head,
        DavMethod::Get,
        DavMethod::Put,
        DavMethod::Delete,
        DavMethod::Mkcol,
        DavMethod::Copy,
        DavMethod::Move,
        DavMethod::Propfind,
        DavMethod::Proppatch,
        DavMethod::Lock,
        DavMethod::Unlock,
    ];

    let methods = SUPPORTED
        .iter()
        .copied()
        .filter(|method| !config.methods.disabled.contains(method))
        .map(DavMethod::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    HeaderValue::from_str(&methods).expect("DAV method names are valid header values")
}

fn socket_addr(config: &AppConfig) -> Result<SocketAddr, AppError> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    addr.parse()
        .map_err(|source| AppError::AddrParse { addr, source })
}

async fn prepare_roots(config: &AppConfig) -> Result<(), AppError> {
    for user in &config.users {
        prepare_root_dir(&user.root_dir, config.server.create_root_if_missing).await?;
    }

    Ok(())
}

async fn prepare_root_dir(
    root_dir: &std::path::Path,
    create_root_if_missing: bool,
) -> Result<(), AppError> {
    if create_root_if_missing {
        tokio::fs::create_dir_all(root_dir)
            .await
            .map_err(|source| AppError::RootDirCreate {
                path: root_dir.display().to_string(),
                source,
            })?;
    }

    let metadata = tokio::fs::metadata(root_dir)
        .await
        .map_err(|_| AppError::RootDirMissing {
            path: root_dir.display().to_string(),
        })?;

    if !metadata.is_dir() {
        return Err(AppError::RootDirNotDirectory {
            path: root_dir.display().to_string(),
        });
    }

    std::fs::canonicalize(root_dir).map_err(|source| AppError::RootDirCanonicalize {
        path: root_dir.display().to_string(),
        source,
    })?;

    Ok(())
}

fn empty_response(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .expect("empty response should be valid")
}

fn response_with_allow(status: StatusCode, allow: HeaderValue) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(ALLOW, allow)
        .body(Body::empty())
        .expect("response with Allow header should be valid")
}

fn options_response(allow: HeaderValue) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header(ALLOW, allow)
        .header(DAV, "1")
        .body(Body::empty())
        .expect("OPTIONS response should be valid")
}

fn unauthorized_response(www_authenticate: Vec<String>) -> Response<Body> {
    let mut builder = Response::builder().status(StatusCode::UNAUTHORIZED);
    for value in www_authenticate {
        builder = builder.header(WWW_AUTHENTICATE, value);
    }

    builder
        .body(Body::empty())
        .expect("unauthorized response should be valid")
}

fn finish_request(
    response: Response<Body>,
    method: &str,
    request_uri: &str,
    username: Option<&str>,
    reason: &'static str,
) -> Response<Body> {
    let status = response.status();
    let user = username.unwrap_or("-");

    if status.is_client_error() || status.is_server_error() {
        warn!(
            method,
            request_uri,
            username = user,
            status = status.as_u16(),
            reason,
            "request failed"
        );
    } else {
        info!(
            method,
            request_uri,
            username = user,
            status = status.as_u16(),
            reason,
            "request completed"
        );
    }

    response
}

fn default_reason_for_status(method: DavMethod, status: StatusCode) -> &'static str {
    match status {
        StatusCode::OK => match method {
            DavMethod::Get => "get_ok",
            DavMethod::Lock => "lock_acquired",
            _ => "request_ok",
        },
        StatusCode::CREATED => match method {
            DavMethod::Put => "file_created",
            DavMethod::Mkcol => "collection_created",
            DavMethod::Copy => "copy_created",
            DavMethod::Move => "move_created",
            _ => "resource_created",
        },
        StatusCode::NO_CONTENT => match method {
            DavMethod::Options => "options_ok",
            DavMethod::Head => "head_ok",
            DavMethod::Put => "file_updated",
            DavMethod::Delete => "resource_deleted",
            DavMethod::Move => "move_replaced",
            DavMethod::Copy => "copy_replaced",
            DavMethod::Unlock => "lock_released",
            _ => "request_no_content",
        },
        StatusCode::MULTI_STATUS => match method {
            DavMethod::Propfind => "propfind_ok",
            DavMethod::Proppatch => "proppatch_ok",
            _ => "multi_status",
        },
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "authentication_failed",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::CONFLICT => "conflict",
        StatusCode::LOCKED => "locked",
        StatusCode::PRECONDITION_FAILED => "precondition_failed",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::BAD_GATEWAY => "invalid_destination",
        StatusCode::INTERNAL_SERVER_ERROR => "internal_error",
        _ => "request_completed",
    }
}

fn scoped_href(username: &str, href_path: &str) -> String {
    format!("{username}\0{href_path}")
}

fn scoped_destination_href(source_scoped_href: &str, destination_href: &str) -> String {
    let username = source_scoped_href
        .split_once('\0')
        .map(|(username, _)| username)
        .unwrap_or_default();
    scoped_href(username, destination_href)
}

fn existing_path_or_response(
    root_dir: &Path,
    path: &Path,
    not_found_status: StatusCode,
) -> Result<PathBuf, Response<Body>> {
    canonicalize_existing_under_root(root_dir, path)
        .map_err(|error| canonical_error_response(error, not_found_status))
}

fn parent_path_or_response(root_dir: &Path, path: &Path) -> Result<PathBuf, Response<Body>> {
    canonicalize_parent_under_root(root_dir, path)
        .map_err(|error| canonical_error_response(error, StatusCode::CONFLICT))
}

fn existing_state_or_response(root_dir: &Path, path: &Path) -> Result<bool, Response<Body>> {
    match canonicalize_existing_under_root(root_dir, path) {
        Ok(_) => Ok(true),
        Err(CanonicalPathError::Path(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            if std::fs::symlink_metadata(path).is_ok() {
                Err(empty_response(StatusCode::FORBIDDEN))
            } else {
                Ok(false)
            }
        }
        Err(error) => Err(canonical_error_response(error, StatusCode::NOT_FOUND)),
    }
}

fn lockable_path_or_response(root_dir: &Path, path: &Path) -> Result<(), Response<Body>> {
    match canonicalize_existing_under_root(root_dir, path) {
        Ok(_) => Ok(()),
        Err(CanonicalPathError::Path(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            if std::fs::symlink_metadata(path).is_ok() {
                return Err(empty_response(StatusCode::FORBIDDEN));
            }
            parent_path_or_response(root_dir, path).map(|_| ())
        }
        Err(error) => Err(canonical_error_response(error, StatusCode::NOT_FOUND)),
    }
}

fn canonical_error_response(
    error: CanonicalPathError,
    missing_status: StatusCode,
) -> Response<Body> {
    match error {
        CanonicalPathError::Path(error) if error.kind() == std::io::ErrorKind::NotFound => {
            empty_response(missing_status)
        }
        CanonicalPathError::MissingParent => empty_response(StatusCode::CONFLICT),
        CanonicalPathError::OutsideRoot => empty_response(StatusCode::FORBIDDEN),
        CanonicalPathError::Root(_) | CanonicalPathError::Path(_) => {
            empty_response(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn get_file_response(
    root_dir: &Path,
    path: &std::path::Path,
    head_only: bool,
) -> Response<Body> {
    let path = match existing_path_or_response(root_dir, path, StatusCode::NOT_FOUND) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let metadata = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_response(StatusCode::NOT_FOUND);
        }
        Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if metadata.is_dir() {
        return empty_response(StatusCode::METHOD_NOT_ALLOWED);
    }

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_LENGTH, metadata.len().to_string())
        .header(ETAG, etag(&metadata));
    if let Ok(modified) = metadata.modified() {
        builder = builder.header(LAST_MODIFIED, httpdate::fmt_http_date(modified));
    }

    if head_only {
        return builder
            .body(Body::empty())
            .expect("HEAD response should be valid");
    }

    match tokio::fs::read(&path).await {
        Ok(contents) => builder
            .body(Body::from(contents))
            .expect("file response should be valid"),
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn put_file_response(
    root_dir: &Path,
    path: &std::path::Path,
    request: Request,
) -> Response<Body> {
    let existed = match existing_state_or_response(root_dir, path) {
        Ok(existed) => existed,
        Err(response) => return response,
    };

    let secure_parent = match parent_path_or_response(root_dir, path) {
        Ok(parent) => parent,
        Err(response) => return response,
    };
    let Some(file_name) = path.file_name() else {
        return empty_response(StatusCode::CONFLICT);
    };
    let secure_path = secure_parent.join(file_name);

    let body = match to_bytes(request.into_body(), MAX_PUT_BYTES).await {
        Ok(body) => body,
        Err(_) => return empty_response(StatusCode::PAYLOAD_TOO_LARGE),
    };

    match tokio::fs::write(secure_path, body).await {
        Ok(()) if existed => empty_response(StatusCode::NO_CONTENT),
        Ok(()) => empty_response(StatusCode::CREATED),
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_response(root_dir: &Path, path: &std::path::Path) -> Response<Body> {
    let secure_path = match existing_path_or_response(root_dir, path, StatusCode::NOT_FOUND) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_response(StatusCode::NOT_FOUND);
        }
        Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let result = if metadata.file_type().is_symlink() {
        tokio::fs::remove_file(path).await
    } else if metadata.is_dir() {
        tokio::fs::remove_dir_all(secure_path).await
    } else {
        tokio::fs::remove_file(secure_path).await
    };

    match result {
        Ok(()) => empty_response(StatusCode::NO_CONTENT),
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn mkcol_response(root_dir: &Path, path: &std::path::Path) -> Response<Body> {
    match existing_state_or_response(root_dir, path) {
        Ok(true) => return empty_response(StatusCode::METHOD_NOT_ALLOWED),
        Ok(false) => {}
        Err(response) => return response,
    }

    let secure_parent = match parent_path_or_response(root_dir, path) {
        Ok(parent) => parent,
        Err(response) => return response,
    };
    let Some(file_name) = path.file_name() else {
        return empty_response(StatusCode::CONFLICT);
    };
    let secure_path = secure_parent.join(file_name);

    match tokio::fs::create_dir(secure_path).await {
        Ok(()) => empty_response(StatusCode::CREATED),
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn copy_or_move_response(
    config: &AppConfig,
    root_dir: &Path,
    lock_store: &LockStore,
    source: &Path,
    source_scoped_href: &str,
    destination: Option<String>,
    lock_tokens: &[String],
    overwrite: bool,
    move_source: bool,
) -> Response<Body> {
    let Some(destination) = destination else {
        return empty_response(StatusCode::BAD_REQUEST);
    };

    let destination_path = destination
        .parse::<axum::http::Uri>()
        .ok()
        .and_then(|uri| uri.path_and_query().map(|path| path.path().to_owned()))
        .unwrap_or_else(|| destination.to_owned());

    let destination = match resolve_dav_path_with_root(config, root_dir, &destination_path) {
        Ok(path) => path,
        Err(PathError::OutsideBasePath) => return empty_response(StatusCode::BAD_GATEWAY),
        Err(PathError::InvalidEncoding | PathError::UnsafePath) => {
            return empty_response(StatusCode::BAD_REQUEST);
        }
    };

    let destination_scoped_href =
        scoped_destination_href(source_scoped_href, &destination.href_path);
    if lock_store
        .conflicting_lock(&destination_scoped_href, lock_tokens)
        .is_some()
    {
        return empty_response(StatusCode::LOCKED);
    }

    let secure_source = match existing_path_or_response(root_dir, source, StatusCode::NOT_FOUND) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let secure_parent = match parent_path_or_response(root_dir, &destination.fs_path) {
        Ok(parent) => parent,
        Err(response) => return response,
    };
    let Some(destination_name) = destination.fs_path.file_name() else {
        return empty_response(StatusCode::CONFLICT);
    };
    let secure_destination = secure_parent.join(destination_name);

    if secure_source == secure_destination {
        return empty_response(StatusCode::FORBIDDEN);
    }

    let metadata = match tokio::fs::metadata(&secure_source).await {
        Ok(metadata) => metadata,
        Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if metadata.is_dir() && secure_destination.starts_with(&secure_source) {
        return empty_response(StatusCode::FORBIDDEN);
    }

    let existed = match existing_state_or_response(root_dir, &destination.fs_path) {
        Ok(existed) => existed,
        Err(response) => return response,
    };
    if existed && !overwrite {
        return empty_response(StatusCode::PRECONDITION_FAILED);
    }

    if existed {
        match remove_path(root_dir, &destination.fs_path).await {
            Ok(()) => {}
            Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    let source = secure_source;
    let destination = secure_destination;
    let result = if move_source {
        tokio::fs::rename(&source, &destination).await
    } else if metadata.is_dir() {
        tokio::task::spawn_blocking(move || copy_dir_recursive(&source, &destination))
            .await
            .unwrap_or_else(|_| Err(std::io::Error::other("directory copy task failed")))
    } else {
        tokio::fs::copy(&source, &destination).await.map(|_| ())
    };

    match result {
        Ok(()) if existed => empty_response(StatusCode::NO_CONTENT),
        Ok(()) => empty_response(StatusCode::CREATED),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            empty_response(StatusCode::CONFLICT)
        }
        Err(_) => empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn remove_path(root_dir: &Path, path: &Path) -> std::io::Result<()> {
    let secure_path = canonicalize_existing_under_root(root_dir, path).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "path outside root")
    })?;
    let metadata = tokio::fs::symlink_metadata(path).await?;
    if metadata.file_type().is_symlink() {
        tokio::fs::remove_file(path).await
    } else if metadata.is_dir() {
        tokio::fs::remove_dir_all(secure_path).await
    } else {
        tokio::fs::remove_file(secure_path).await
    }
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir(destination)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let child_source = entry.path();
        let child_destination: PathBuf = destination.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&child_source, &child_destination)?;
        } else if file_type.is_file() {
            std::fs::copy(&child_source, &child_destination)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "copying symlinks or special files is not supported",
            ));
        }
    }

    Ok(())
}

fn lock_response(
    root_dir: &Path,
    lock_store: &LockStore,
    fs_path: &Path,
    href_path: &str,
    owner: &str,
    timeout: std::time::Duration,
) -> Response<Body> {
    if let Err(response) = lockable_path_or_response(root_dir, fs_path) {
        return response;
    }

    let active_lock = match lock_store.lock(href_path, owner, timeout) {
        Ok(active_lock) => active_lock,
        Err(_) => return empty_response(StatusCode::LOCKED),
    };
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><D:prop xmlns:D="DAV:"><D:lockdiscovery><D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>infinity</D:depth><D:owner>{}</D:owner><D:timeout>Second-{}</D:timeout><D:locktoken><D:href>{}</D:href></D:locktoken></D:activelock></D:lockdiscovery></D:prop>"#,
        xml_escape(owner),
        timeout.as_secs(),
        xml_escape(&active_lock.token)
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .header("Lock-Token", format!("<{}>", active_lock.token))
        .body(Body::from(body))
        .expect("LOCK response should be valid")
}

fn unlock_response(
    lock_store: &LockStore,
    href_path: &str,
    lock_tokens: &[String],
) -> Response<Body> {
    let Some(token) = lock_tokens.first() else {
        return empty_response(StatusCode::BAD_REQUEST);
    };

    if lock_store.unlock(href_path, token) {
        empty_response(StatusCode::NO_CONTENT)
    } else {
        empty_response(StatusCode::CONFLICT)
    }
}

async fn proppatch_response(
    root_dir: &Path,
    prop_store: &PropStore,
    scoped_href: &str,
    fs_path: &Path,
    href_path: &str,
    request: Request,
) -> Response<Body> {
    if let Err(response) = existing_path_or_response(root_dir, fs_path, StatusCode::NOT_FOUND) {
        return response;
    }

    let body = match to_bytes(request.into_body(), MAX_PROPPATCH_BYTES).await {
        Ok(body) => body,
        Err(_) => return empty_response(StatusCode::PAYLOAD_TOO_LARGE),
    };
    let body = match String::from_utf8(body.to_vec()) {
        Ok(body) => body,
        Err(_) => return empty_response(StatusCode::BAD_REQUEST),
    };
    let ops = match parse_proppatch(&body) {
        Ok(ops) => ops,
        Err(_) => return empty_response(StatusCode::BAD_REQUEST),
    };

    prop_store.apply(scoped_href, &ops);

    let propstats = ops
        .iter()
        .map(|op| {
            let name = match op {
                crate::prop::PropPatchOp::Set(property) => &property.name,
                crate::prop::PropPatchOp::Remove(name) => name,
            };
            format!(
                "<D:propstat><D:prop>{}</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>",
                dead_property_empty_xml(name)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>{}</D:href>{}</D:response></D:multistatus>"#,
        xml_escape(href_path),
        propstats
    );

    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(body))
        .expect("PROPPATCH response should be valid")
}

async fn propfind_response(
    root_dir: &Path,
    prop_store: &PropStore,
    username: &str,
    href_path: &str,
    fs_path: &std::path::Path,
    depth: &str,
) -> Response<Body> {
    let secure_path = match existing_path_or_response(root_dir, fs_path, StatusCode::NOT_FOUND) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let metadata = match tokio::fs::metadata(&secure_path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_response(StatusCode::NOT_FOUND);
        }
        Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let mut entries = Vec::new();
    let current_scoped_href = scoped_href(username, href_path);
    entries.push(prop_response(
        href_path,
        &metadata,
        &prop_store.get(&current_scoped_href),
    ));

    if metadata.is_dir() && depth != "0" {
        let mut read_dir = match tokio::fs::read_dir(&secure_path).await {
            Ok(read_dir) => read_dir,
            Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
        };

        loop {
            match read_dir.next_entry().await {
                Ok(Some(entry)) => {
                    let child_path = entry.path();
                    if canonicalize_existing_under_root(root_dir, &child_path).is_err() {
                        continue;
                    }
                    let Ok(metadata) = entry.metadata().await else {
                        continue;
                    };
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let child_href = join_href(href_path, &name, metadata.is_dir());
                    let child_scoped_href = scoped_href(username, &child_href);
                    entries.push(prop_response(
                        &child_href,
                        &metadata,
                        &prop_store.get(&child_scoped_href),
                    ));
                }
                Ok(None) => break,
                Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
    }

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:">{}</D:multistatus>"#,
        entries.join("")
    );

    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header(CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(Body::from(body))
        .expect("PROPFIND response should be valid")
}

fn prop_response(
    href_path: &str,
    metadata: &std::fs::Metadata,
    dead_properties: &[DeadProperty],
) -> String {
    let href = if metadata.is_dir() {
        collection_href(href_path)
    } else {
        href_path.to_owned()
    };
    let resource_type = if metadata.is_dir() {
        "<D:resourcetype><D:collection/></D:resourcetype>".to_owned()
    } else {
        "<D:resourcetype/>".to_owned()
    };
    let content_length = if metadata.is_dir() { 0 } else { metadata.len() };
    let display_name = display_name(href_path);
    let last_modified = metadata
        .modified()
        .map(httpdate::fmt_http_date)
        .unwrap_or_default();
    let etag = if metadata.is_dir() {
        String::new()
    } else {
        etag(metadata)
    };
    let dead_properties = dead_properties
        .iter()
        .map(dead_property_xml)
        .collect::<Vec<_>>()
        .join("");

    format!(
        "<D:response><D:href>{}</D:href><D:propstat><D:prop><D:displayname>{}</D:displayname>{}<D:getcontentlength>{}</D:getcontentlength><D:getlastmodified>{}</D:getlastmodified><D:getetag>{}</D:getetag><D:supportedlock/><D:lockdiscovery/>{}</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>",
        xml_escape(&href),
        xml_escape(&display_name),
        resource_type,
        content_length,
        xml_escape(&last_modified),
        xml_escape(&etag),
        dead_properties
    )
}

fn dead_property_xml(property: &DeadProperty) -> String {
    let value = xml_escape(&property.value);
    if property.name.namespace.is_empty() {
        format!("<{}>{}</{}>", property.name.name, value, property.name.name)
    } else {
        format!(
            r#"<X:{} xmlns:X="{}">{}</X:{}>"#,
            property.name.name,
            xml_escape(&property.name.namespace),
            value,
            property.name.name
        )
    }
}

fn dead_property_empty_xml(name: &crate::prop::DeadPropertyName) -> String {
    if name.namespace.is_empty() {
        format!("<{}/>", name.name)
    } else {
        format!(
            r#"<X:{} xmlns:X="{}"/>"#,
            name.name,
            xml_escape(&name.namespace)
        )
    }
}

fn display_name(href_path: &str) -> String {
    href_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn etag(metadata: &std::fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    format!("\"{:x}-{:x}\"", metadata.len(), modified)
}

fn join_href(parent: &str, child: &str, is_dir: bool) -> String {
    let parent = collection_href(parent);
    let href = format!("{parent}{}", percent_encode_path_segment(child));
    if is_dir { collection_href(&href) } else { href }
}

fn collection_href(path: &str) -> String {
    if path.ends_with('/') {
        path.to_owned()
    } else {
        format!("{path}/")
    }
}

fn percent_encode_path_segment(segment: &str) -> String {
    percent_encoding::utf8_percent_encode(segment, PATH_SEGMENT_ENCODE_SET).to_string()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_path_matches_exact_path_and_children() {
        assert!(path_matches_base_path("/dav", "/dav"));
        assert!(path_matches_base_path("/dav/file.txt", "/dav"));
        assert!(!path_matches_base_path("/davish/file.txt", "/dav"));
        assert!(!path_matches_base_path("/file.txt", "/dav"));
    }

    #[test]
    fn root_base_path_matches_all_absolute_paths() {
        assert!(path_matches_base_path("/", "/"));
        assert!(path_matches_base_path("/file.txt", "/"));
    }

    #[test]
    fn maps_status_to_default_log_reason() {
        assert_eq!(
            default_reason_for_status(DavMethod::Put, StatusCode::CREATED),
            "file_created"
        );
        assert_eq!(
            default_reason_for_status(DavMethod::Propfind, StatusCode::MULTI_STATUS),
            "propfind_ok"
        );
        assert_eq!(
            default_reason_for_status(DavMethod::Get, StatusCode::NOT_FOUND),
            "not_found"
        );
        assert_eq!(
            default_reason_for_status(DavMethod::Copy, StatusCode::BAD_GATEWAY),
            "invalid_destination"
        );
    }
}
