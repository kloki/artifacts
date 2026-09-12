use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
};

use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{error::AppError, models::Meta};

/// Filesystem-backed artifact store.
///
/// Layout: `$data_dir/{uuid}/{index.html,meta.json,versions/index.v{N}.html}`.
/// Artifacts under construction live in `$data_dir/.tmp-{uuid}` and are renamed
/// into place, so a crash never leaves a half-built artifact visible.
#[derive(Clone)]
pub struct Storage {
    root: PathBuf,
    locks: Arc<StdMutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
}

impl Storage {
    pub fn new(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            locks: Arc::new(StdMutex::new(HashMap::new())),
        })
    }

    fn lock_for(&self, id: Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("lock map poisoned");
        locks.entry(id).or_default().clone()
    }

    fn dir(&self, id: Uuid) -> PathBuf {
        self.root.join(id.to_string())
    }

    pub async fn create(
        &self,
        html: &[u8],
        title: Option<String>,
        description: Option<String>,
    ) -> Result<Meta, AppError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let meta = Meta {
            id,
            title,
            description,
            created_at: now,
            updated_at: now,
            version: 1,
            size_bytes: html.len() as u64,
            sha256: sha256_hex(html),
        };

        let staging = self.root.join(format!(".tmp-{id}"));
        let final_dir = self.dir(id);
        let html = html.to_vec();
        let meta_for_write = meta.clone();

        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            if staging.exists() {
                fs::remove_dir_all(&staging)?;
            }
            fs::create_dir_all(staging.join("versions"))?;
            fs::write(staging.join("index.html"), &html)?;
            write_meta(&staging, &meta_for_write)?;
            fs::rename(&staging, &final_dir)
        })
        .await
        .map_err(join_err)??;

        Ok(meta)
    }

    pub async fn update(
        &self,
        id: Uuid,
        html: &[u8],
        title: Option<String>,
        description: Option<String>,
    ) -> Result<Meta, AppError> {
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;

        let mut meta = self.read_meta(id).await?;
        let old_version = meta.version;

        meta.version += 1;
        meta.updated_at = OffsetDateTime::now_utc();
        meta.size_bytes = html.len() as u64;
        meta.sha256 = sha256_hex(html);
        // Metadata fields are only touched when the caller supplied them.
        if title.is_some() {
            meta.title = title;
        }
        if description.is_some() {
            meta.description = description;
        }

        let dir = self.dir(id);
        let html = html.to_vec();
        let meta_for_write = meta.clone();

        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let versions = dir.join("versions");
            fs::create_dir_all(&versions)?;
            // Copy rather than rename so the current version stays readable
            // throughout, then swap in the new content and metadata.
            fs::copy(
                dir.join("index.html"),
                versions.join(format!("index.v{old_version}.html")),
            )?;
            write_atomic(&dir.join("index.html"), &html)?;
            write_meta(&dir, &meta_for_write)
        })
        .await
        .map_err(join_err)??;

        Ok(meta)
    }

    /// Update only the metadata fields. `updated_at` changes but `version` does
    /// not, since no new content is published. Use `Some(Some(...))` to set,
    /// `Some(None)` to clear, and `None` to leave a field untouched.
    pub async fn update_metadata(
        &self,
        id: Uuid,
        title: Option<Option<String>>,
        description: Option<Option<String>>,
    ) -> Result<Meta, AppError> {
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;

        let mut meta = self.read_meta(id).await?;
        if let Some(t) = title {
            meta.title = t;
        }
        if let Some(d) = description {
            meta.description = d;
        }
        meta.updated_at = OffsetDateTime::now_utc();

        let dir = self.dir(id);
        let meta_for_write = meta.clone();
        tokio::task::spawn_blocking(move || write_meta(&dir, &meta_for_write))
            .await
            .map_err(join_err)??;

        Ok(meta)
    }

    pub async fn read_meta(&self, id: Uuid) -> Result<Meta, AppError> {
        let path = self.dir(id).join("meta.json");
        let bytes = tokio::task::spawn_blocking(move || fs::read(path))
            .await
            .map_err(join_err)?
            .map_err(|_| AppError::NotFound)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            AppError::Internal(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("corrupt meta.json for {id}: {e}"),
            ))
        })
    }

    /// Reads the HTML for `version` (defaults to current). Any version that
    /// ever existed stays addressable; anything else is a 404.
    pub async fn read_html(&self, id: Uuid, version: Option<u32>) -> Result<Vec<u8>, AppError> {
        let meta = self.read_meta(id).await?;
        let dir = self.dir(id);

        let path = match version {
            None => dir.join("index.html"),
            Some(v) if v == meta.version => dir.join("index.html"),
            Some(v) if v >= 1 && v < meta.version => {
                dir.join("versions").join(format!("index.v{v}.html"))
            }
            Some(_) => return Err(AppError::NotFound),
        };

        tokio::task::spawn_blocking(move || fs::read(path))
            .await
            .map_err(join_err)?
            .map_err(|_| AppError::NotFound)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;

        // Confirms existence so a missing artifact is a 404, not a silent 204.
        self.read_meta(id).await?;

        let dir = self.dir(id);
        tokio::task::spawn_blocking(move || fs::remove_dir_all(dir))
            .await
            .map_err(join_err)??;

        self.locks.lock().expect("lock map poisoned").remove(&id);
        Ok(())
    }

    /// Returns one page of artifacts sorted by `updated_at` descending, plus
    /// the total count.
    pub async fn list(&self, limit: usize, offset: usize) -> Result<(Vec<Meta>, usize), AppError> {
        let root = self.root.clone();

        let mut metas = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<Meta>> {
            let mut metas = Vec::new();
            for entry in fs::read_dir(&root)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();
                // Skips `.tmp-*` staging dirs and any stray files.
                if name.starts_with('.') || !entry.file_type()?.is_dir() {
                    continue;
                }
                match fs::read(entry.path().join("meta.json"))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Meta>(&b).ok())
                {
                    Some(meta) => metas.push(meta),
                    None => {
                        tracing::warn!(dir = %name, "skipping artifact with unreadable meta.json")
                    }
                }
            }
            Ok(metas)
        })
        .await
        .map_err(join_err)??;

        metas.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
        let total = metas.len();
        let page = metas.into_iter().skip(offset).take(limit).collect();
        Ok((page, total))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_meta(dir: &Path, meta: &Meta) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(meta).map_err(std::io::Error::other)?;
    write_atomic(&dir.join("meta.json"), &json)
}

/// Writes via a sibling temp file + rename so readers only ever observe the
/// complete old or complete new file.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

fn join_err(e: tokio::task::JoinError) -> AppError {
    AppError::Internal(std::io::Error::other(e))
}
