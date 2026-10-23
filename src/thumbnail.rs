use std::collections::HashSet;
use std::fmt;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use image::{ImageFormat, ImageReader, Limits, imageops::FilterType};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};
use tracing::{info, warn};

use crate::config::ThumbnailConfig;
use crate::fs::metadata_etag;

const ENCODER_VERSION: &str = "v1";
const CACHE_DIR_NAME: &str = ".xylos-cache";
const THUMBNAIL_DIR_NAME: &str = "thumbnails";
const CLEANUP_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const TEMP_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum ThumbnailError {
    Disabled,
    Unsupported,
    TooLarge,
    NotFound,
    Busy,
    Io(String),
    Decode(String),
}

impl fmt::Display for ThumbnailError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => formatter.write_str("server thumbnails are disabled"),
            Self::Unsupported => formatter.write_str("unsupported image format"),
            Self::TooLarge => formatter.write_str("image is too large for thumbnails"),
            Self::NotFound => formatter.write_str("source image was not found"),
            Self::Busy => {
                formatter.write_str("source image changed repeatedly during thumbnail generation")
            }
            Self::Io(error) => write!(formatter, "thumbnail I/O error: {error}"),
            Self::Decode(error) => write!(formatter, "thumbnail decode error: {error}"),
        }
    }
}

impl std::error::Error for ThumbnailError {}

impl From<std::io::Error> for ThumbnailError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::Io(error.to_string())
        }
    }
}

#[derive(Debug, Clone)]
pub struct RootThumbnailCache {
    pub root_dir: PathBuf,
    pub cache_dir: PathBuf,
    format_dir: PathBuf,
    temp_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ThumbnailOutput {
    pub bytes: Vec<u8>,
    pub etag: String,
    pub width: u32,
}

pub struct ThumbnailService {
    config: ThumbnailConfig,
    config_dir: PathBuf,
    protected_roots: Vec<PathBuf>,
    generation: Arc<Semaphore>,
    warm_slots: Arc<Semaphore>,
    stripes: Vec<AsyncMutex<()>>,
    roots: Mutex<HashSet<PathBuf>>,
}

impl ThumbnailService {
    pub fn new(
        config: ThumbnailConfig,
        config_dir: PathBuf,
        protected_roots: Vec<PathBuf>,
    ) -> Self {
        let concurrency = config.max_concurrency;
        Self {
            config,
            config_dir,
            protected_roots,
            generation: Arc::new(Semaphore::new(concurrency)),
            warm_slots: Arc::new(Semaphore::new(1)),
            stripes: (0..64).map(|_| AsyncMutex::new(())).collect(),
            roots: Mutex::new(HashSet::new()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn config(&self) -> &ThumbnailConfig {
        &self.config
    }

    pub async fn root_cache(&self, root_dir: &Path) -> Result<RootThumbnailCache, ThumbnailError> {
        if !self.config.enabled {
            return Err(ThumbnailError::Disabled);
        }

        let root_dir = tokio::fs::canonicalize(root_dir).await?;
        let cache_dir = self.cache_dir_for_root(&root_dir);
        let format_dir = cache_dir.join(THUMBNAIL_DIR_NAME).join(ENCODER_VERSION);
        let temp_dir = format_dir.join("tmp");

        Ok(RootThumbnailCache {
            root_dir,
            cache_dir,
            format_dir,
            temp_dir,
        })
    }

    pub async fn prepare_root(
        &self,
        root_dir: &Path,
    ) -> Result<RootThumbnailCache, ThumbnailError> {
        let cache = self.root_cache(root_dir).await?;
        self.ensure_dirs(&cache).await?;

        if let Ok(mut roots) = self.roots.lock() {
            roots.insert(cache.root_dir.clone());
        }

        info!(
            root = %cache.root_dir.display(),
            cache = %cache.cache_dir.display(),
            "thumbnail cache ready"
        );
        Ok(cache)
    }

    pub async fn load_or_generate(
        &self,
        cache: &RootThumbnailCache,
        username: &str,
        href: &str,
        source: &Path,
        requested_width: Option<u32>,
    ) -> Result<ThumbnailOutput, ThumbnailError> {
        if !self.config.enabled {
            return Err(ThumbnailError::Disabled);
        }
        self.ensure_dirs(cache).await?;

        let width = self.quantize_width(requested_width);
        for _ in 0..2 {
            let metadata = tokio::fs::metadata(source).await?;
            if !metadata.is_file() {
                return Err(ThumbnailError::Unsupported);
            }
            if metadata.len() > self.config.max_source_bytes {
                return Err(ThumbnailError::TooLarge);
            }
            if !is_supported_image_path(source) {
                return Err(ThumbnailError::Unsupported);
            }

            let source_etag = metadata_etag(&metadata);
            let cache_path = cache_path(cache, username, href, &source_etag, width);
            let etag = format!("\"{}\"", cache_key(username, href, &source_etag, width));
            if let Ok(bytes) = tokio::fs::read(&cache_path).await {
                return Ok(ThumbnailOutput { bytes, etag, width });
            }

            let _permit = self
                .generation
                .acquire()
                .await
                .map_err(|_| ThumbnailError::Busy)?;
            let stripe = self.stripe(username, href, &source_etag, width);
            let _guard = stripe.lock().await;

            let metadata = tokio::fs::metadata(source).await?;
            if metadata_etag(&metadata) != source_etag {
                continue;
            }
            if let Ok(bytes) = tokio::fs::read(&cache_path).await {
                return Ok(ThumbnailOutput { bytes, etag, width });
            }

            if let Some(parent) = cache_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
                set_private_permissions(parent).await?;
            }
            let temp_path = cache.temp_dir.join(format!(
                "{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let source_for_task = source.to_path_buf();
            let temp_for_task = temp_path.clone();
            let max_pixels = self.config.max_pixels;
            let generated = tokio::task::spawn_blocking(move || {
                generate_webp(&source_for_task, &temp_for_task, width, max_pixels)
            })
            .await
            .map_err(|error| ThumbnailError::Decode(error.to_string()))?;

            if let Err(error) = generated {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(error);
            }

            let current_metadata = tokio::fs::metadata(source).await?;
            if metadata_etag(&current_metadata) != source_etag {
                let _ = tokio::fs::remove_file(&temp_path).await;
                continue;
            }

            let bytes = tokio::fs::read(&temp_path).await?;
            commit_cache_file(&temp_path, &cache_path).await;
            return Ok(ThumbnailOutput { bytes, etag, width });
        }

        Err(ThumbnailError::Busy)
    }

    pub fn prewarm(
        self: &Arc<Self>,
        root_dir: PathBuf,
        username: String,
        href: String,
        source: PathBuf,
    ) {
        if !self.config.enabled
            || !self.config.prewarm_on_upload
            || !is_supported_image_path(&source)
        {
            return;
        }

        let Ok(permit) = self.warm_slots.clone().try_acquire_owned() else {
            return;
        };
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let _permit = permit;
            let Ok(cache) = service.root_cache(&root_dir).await else {
                return;
            };
            if let Err(error) = service
                .load_or_generate(&cache, &username, &href, &source, None)
                .await
            {
                if !matches!(
                    error,
                    ThumbnailError::Unsupported
                        | ThumbnailError::TooLarge
                        | ThumbnailError::NotFound
                ) {
                    warn!(
                        source = %source.display(),
                        error = %error,
                        "thumbnail prewarm failed"
                    );
                }
            }
        });
    }

    pub fn start_cleanup_task(self: &Arc<Self>) {
        if !self.config.enabled {
            return;
        }

        let service = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(CLEANUP_INTERVAL).await;
                service.cleanup_all().await;
            }
        });
    }

    pub async fn cleanup_all(&self) {
        let roots = match self.roots.lock() {
            Ok(roots) => roots.iter().cloned().collect::<Vec<_>>(),
            Err(_) => return,
        };

        for root in roots {
            let Ok(cache) = self.root_cache(&root).await else {
                continue;
            };
            let format_dir = cache.format_dir.clone();
            let temp_dir = cache.temp_dir.clone();
            let ttl = Duration::from_secs(self.config.ttl_days.saturating_mul(24 * 60 * 60));
            let max_bytes = self.config.max_cache_bytes;
            let result = tokio::task::spawn_blocking(move || {
                cleanup_cache_dir(&format_dir, &temp_dir, ttl, max_bytes)
            })
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!(
                    cache = %cache.cache_dir.display(),
                    error = %error,
                    "thumbnail cache cleanup failed"
                ),
                Err(error) => warn!(
                    cache = %cache.cache_dir.display(),
                    error = %error,
                    "thumbnail cache cleanup task failed"
                ),
            }
        }
    }

    async fn ensure_dirs(&self, cache: &RootThumbnailCache) -> Result<(), ThumbnailError> {
        let candidate = canonicalize_missing_path(&cache.cache_dir).await?;
        if self
            .protected_roots
            .iter()
            .chain(std::iter::once(&cache.root_dir))
            .any(|root| candidate.starts_with(root) || root.starts_with(&candidate))
        {
            return Err(ThumbnailError::Io(
                "thumbnail cache must be outside every WebDAV root".into(),
            ));
        }
        tokio::fs::create_dir_all(&cache.cache_dir).await?;
        let canonical_cache_dir = tokio::fs::canonicalize(&cache.cache_dir).await?;
        if self
            .protected_roots
            .iter()
            .chain(std::iter::once(&cache.root_dir))
            .any(|root| {
                canonical_cache_dir.starts_with(root) || root.starts_with(&canonical_cache_dir)
            })
        {
            return Err(ThumbnailError::Io(
                "thumbnail cache must be outside every WebDAV root".into(),
            ));
        }
        tokio::fs::create_dir_all(&cache.temp_dir).await?;
        for directory in [
            &cache.cache_dir,
            &cache.cache_dir.join(THUMBNAIL_DIR_NAME),
            &cache.format_dir,
            &cache.temp_dir,
        ] {
            set_private_permissions(directory).await?;
        }
        Ok(())
    }

    fn quantize_width(&self, requested: Option<u32>) -> u32 {
        let max_width = self.config.width;
        let requested = requested.unwrap_or(max_width).clamp(64, max_width);
        if requested >= max_width.saturating_mul(3) / 4 {
            max_width
        } else if requested >= 160 {
            160.min(max_width)
        } else {
            96.min(max_width)
        }
    }

    fn stripe(&self, username: &str, href: &str, etag: &str, width: u32) -> &AsyncMutex<()> {
        let digest = cache_key(username, href, etag, width);
        let index =
            u8::from_str_radix(&digest[..2], 16).unwrap_or_default() as usize % self.stripes.len();
        &self.stripes[index]
    }

    fn cache_dir_for_root(&self, root_dir: &Path) -> PathBuf {
        if let Some(cache_dir) = &self.config.cache_dir {
            let base = if cache_dir.is_absolute() {
                cache_dir.clone()
            } else {
                self.config_dir.join(cache_dir)
            };
            return base.join(root_cache_id(root_dir));
        }

        let parent = root_dir.parent().unwrap_or(root_dir);
        parent.join(CACHE_DIR_NAME).join(root_cache_id(root_dir))
    }
}

async fn canonicalize_missing_path(path: &Path) -> Result<PathBuf, ThumbnailError> {
    let mut ancestor = path;
    while tokio::fs::metadata(ancestor).await.is_err() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| ThumbnailError::Io("thumbnail cache has no existing parent".into()))?;
    }
    let canonical = tokio::fs::canonicalize(ancestor).await?;
    Ok(canonical.join(path.strip_prefix(ancestor).unwrap_or(Path::new(""))))
}

fn root_cache_id(root_dir: &Path) -> String {
    let raw_name = root_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("root");
    let mut name = raw_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if name.is_empty() {
        name.push_str("root");
    }

    let digest = Sha256::digest(root_dir.as_os_str().as_encoded_bytes());
    let suffix = digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{name}-{suffix}")
}

fn cache_key(username: &str, href: &str, etag: &str, width: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(username.as_bytes());
    hasher.update([0]);
    hasher.update(href.as_bytes());
    hasher.update([0]);
    hasher.update(etag.as_bytes());
    hasher.update([0]);
    hasher.update(width.to_be_bytes());
    hasher.update([0]);
    hasher.update(ENCODER_VERSION.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cache_path(
    cache: &RootThumbnailCache,
    username: &str,
    href: &str,
    etag: &str,
    width: u32,
) -> PathBuf {
    let digest = cache_key(username, href, etag, width);
    cache
        .format_dir
        .join(&digest[..2])
        .join(format!("{digest}-{width}.webp"))
}

pub fn is_supported_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

fn generate_webp(
    source: &Path,
    destination: &Path,
    target_width: u32,
    max_pixels: u64,
) -> Result<(), ThumbnailError> {
    let dimensions = ImageReader::open(source)
        .map_err(ThumbnailError::from)?
        .with_guessed_format()
        .map_err(ThumbnailError::from)?
        .into_dimensions()
        .map_err(|error| ThumbnailError::Decode(error.to_string()))?;

    let pixels = u64::from(dimensions.0) * u64::from(dimensions.1);
    if dimensions.0 == 0 || dimensions.1 == 0 || pixels == 0 || pixels > max_pixels {
        return Err(ThumbnailError::TooLarge);
    }

    let mut limits = Limits::default();
    limits.max_alloc = Some(max_pixels.saturating_mul(4));
    let mut reader = ImageReader::open(source).map_err(ThumbnailError::from)?;
    reader = reader.with_guessed_format().map_err(ThumbnailError::from)?;
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|error| ThumbnailError::Decode(error.to_string()))?;

    let (target_width, target_height) = if dimensions.0 > target_width {
        let height = (u64::from(dimensions.1) * u64::from(target_width) / u64::from(dimensions.0))
            .max(1) as u32;
        (target_width, height)
    } else {
        dimensions
    };

    let thumbnail = if target_width != dimensions.0 || target_height != dimensions.1 {
        image.resize_exact(target_width, target_height, FilterType::Triangle)
    } else {
        image
    };

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(destination)?;
    let mut writer = BufWriter::new(file);
    thumbnail
        .write_to(&mut writer, ImageFormat::WebP)
        .map_err(|error| ThumbnailError::Decode(error.to_string()))?;
    writer.flush()?;
    let file = writer
        .into_inner()
        .map_err(|error| ThumbnailError::Io(error.to_string()))?;
    file.sync_all()?;
    Ok(())
}

async fn commit_cache_file(temp_path: &Path, cache_path: &Path) {
    if let Err(error) = tokio::fs::rename(temp_path, cache_path).await {
        if tokio::fs::metadata(cache_path).await.is_ok() {
            let _ = tokio::fs::remove_file(temp_path).await;
        } else {
            warn!(
                temp = %temp_path.display(),
                cache = %cache_path.display(),
                error = %error,
                "failed to commit thumbnail cache file"
            );
        }
    }
}

fn cleanup_cache_dir(
    format_dir: &Path,
    temp_dir: &Path,
    ttl: Duration,
    max_bytes: u64,
) -> std::io::Result<()> {
    let now = SystemTime::now();

    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_file() && older_than(&metadata, now, TEMP_MAX_AGE) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    let mut files = Vec::new();
    collect_cache_files(format_dir, temp_dir, &mut files)?;

    let mut total_bytes = 0_u64;
    for file in &mut files {
        if !ttl.is_zero() && time_older_than(file.modified, now, ttl) {
            let _ = std::fs::remove_file(&file.path);
            continue;
        }
        total_bytes = total_bytes.saturating_add(file.size);
    }

    if total_bytes > max_bytes {
        files.sort_by_key(|file| file.modified);
        for file in files {
            if total_bytes <= max_bytes {
                break;
            }
            if std::fs::remove_file(&file.path).is_ok() {
                total_bytes = total_bytes.saturating_sub(file.size);
            }
        }
    }

    Ok(())
}

struct CacheFile {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn collect_cache_files(
    directory: &Path,
    temp_dir: &Path,
    files: &mut Vec<CacheFile>,
) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path == temp_dir {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_cache_files(&path, temp_dir, files)?;
        } else if metadata.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("webp")
        {
            files.push(CacheFile {
                path,
                size: metadata.len(),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }
    }

    Ok(())
}

fn older_than(metadata: &std::fs::Metadata, now: SystemTime, age: Duration) -> bool {
    metadata
        .modified()
        .ok()
        .is_some_and(|modified| time_older_than(modified, now, age))
}

fn time_older_than(modified: SystemTime, now: SystemTime, age: Duration) -> bool {
    now.duration_since(modified)
        .is_ok_and(|elapsed| elapsed > age)
}

async fn set_private_permissions(path: &Path) -> Result<(), ThumbnailError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = tokio::fs::metadata(path).await?.permissions();
        if permissions.mode() & 0o777 != 0o700 {
            permissions.set_mode(0o700);
            tokio::fs::set_permissions(path, permissions).await?;
        }
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xylos-thumbnail-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn supports_expected_image_extensions() {
        assert!(is_supported_image_path(Path::new("photo.JPG")));
        assert!(is_supported_image_path(Path::new("photo.webp")));
        assert!(!is_supported_image_path(Path::new("image.svg")));
        assert!(!is_supported_image_path(Path::new("archive.zip")));
    }

    #[test]
    fn resolves_default_cache_next_to_root() {
        let service = ThumbnailService::new(ThumbnailConfig::default(), PathBuf::from("."), vec![]);
        let root = if cfg!(windows) {
            PathBuf::from(r"C:\data\admin")
        } else {
            PathBuf::from("/data/admin")
        };
        let cache = service.cache_dir_for_root(&root);
        assert_eq!(cache.parent().unwrap().file_name().unwrap(), CACHE_DIR_NAME);
    }

    #[test]
    fn generates_webp_thumbnail() {
        let directory = test_dir("generate");
        let source = directory.join("source.png");
        let destination = directory.join("thumbnail.webp");
        let image = image::RgbaImage::from_pixel(640, 320, image::Rgba([20, 40, 60, 255]));
        image.save(&source).unwrap();

        generate_webp(&source, &destination, 320, 60_000_000).unwrap();
        let generated = image::ImageReader::open(&destination)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert_eq!(generated, (320, 160));

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn cache_hit_and_width_have_distinct_etags() {
        let directory = test_dir("cache");
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        image::RgbaImage::from_pixel(640, 320, image::Rgba([20, 40, 60, 255]))
            .save(&source)
            .unwrap();
        let service = ThumbnailService::new(ThumbnailConfig::default(), directory.clone(), vec![]);
        let cache = service.prepare_root(&root).await.unwrap();

        let first = service
            .load_or_generate(&cache, "user", "/dav/source.png", &source, Some(320))
            .await
            .unwrap();
        let second = service
            .load_or_generate(&cache, "user", "/dav/source.png", &source, Some(320))
            .await
            .unwrap();
        let narrow = service
            .load_or_generate(&cache, "user", "/dav/source.png", &source, Some(96))
            .await
            .unwrap();
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.etag, second.etag);
        assert_ne!(first.etag, narrow.etag);
        assert_eq!(first.width, 320);
        assert_eq!(narrow.width, 96);

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn refuses_cache_inside_root_before_creating_it() {
        let directory = test_dir("unsafe-cache");
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let mut config = ThumbnailConfig::default();
        config.cache_dir = Some(root.join("visible"));
        let service = ThumbnailService::new(config, directory.clone(), vec![]);

        assert!(service.prepare_root(&root).await.is_err());
        assert!(!root.join("visible").exists());

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn refuses_cache_inside_another_users_root() {
        let directory = test_dir("other-root");
        let first_root = directory.join("first");
        let second_root = directory.join("second");
        std::fs::create_dir_all(&first_root).unwrap();
        std::fs::create_dir_all(&second_root).unwrap();
        let mut config = ThumbnailConfig::default();
        config.cache_dir = Some(second_root.join("visible"));
        let service = ThumbnailService::new(
            config,
            directory.clone(),
            vec![std::fs::canonicalize(&second_root).unwrap()],
        );

        assert!(service.prepare_root(&first_root).await.is_err());
        assert!(!second_root.join("visible").exists());

        std::fs::remove_dir_all(directory).unwrap();
    }
}
