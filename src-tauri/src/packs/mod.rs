//! On-demand packs. The base installer stays under 40 MB by shipping only the
//! app, KJV/WEB/ASV, the verse index and the default theme; everything heavier
//! is downloaded from the Pack CDN on request (FR-59 to FR-61, ADR 0006).

pub mod downloader;
pub mod manifest;
pub mod registry;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::error::{Error, Result};

use downloader::DownloadStatus;
use manifest::PackManifest;
use registry::{Layout, PackInfo, PackStatus};

/// CI fails the build if either installer exceeds this (FR-59, PRD §19).
pub const MAX_INSTALLER_BYTES: u64 = 40 * 1024 * 1024;

/// Progress pushed to the operator UI as a pack downloads.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackProgress {
    pub pack_id: String,
    pub status: PackStatus,
    pub progress: f64,
    pub bytes_on_disk: u64,
    pub size_bytes: u64,
}

/// Owns the catalog, the on-disk layout, and any in-flight downloads.
pub struct PackManager {
    client: reqwest::Client,
    layout: Layout,
    manifest_url: String,
    manifest: RwLock<Option<PackManifest>>,
    /// pack id -> cancel flag for the task currently downloading it.
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl PackManager {
    pub fn new(app_data_dir: &Path, manifest_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            layout: Layout::new(app_data_dir),
            manifest_url,
            manifest: RwLock::new(None),
            active: Mutex::new(HashMap::new()),
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Re-read the catalog from the CDN.
    pub async fn refresh_manifest(&self) -> Result<()> {
        let fetched = manifest::fetch(&self.client, &self.manifest_url).await?;
        *self.manifest.write().expect("manifest lock") = Some(fetched);
        Ok(())
    }

    /// Load the catalog into memory from raw JSON. Used by tests and by any
    /// future offline/bundled-catalog path.
    pub fn set_manifest_json(&self, body: &str) -> Result<()> {
        let parsed = manifest::parse(body)?;
        *self.manifest.write().expect("manifest lock") = Some(parsed);
        Ok(())
    }

    fn with_manifest<T>(&self, f: impl FnOnce(&PackManifest) -> Result<T>) -> Result<T> {
        let guard = self.manifest.read().expect("manifest lock");
        let manifest = guard.as_ref().ok_or_else(|| {
            Error::Pack("the pack catalog has not been loaded yet. Check your connection".into())
        })?;
        f(manifest)
    }

    fn is_active(&self, pack_id: &str) -> bool {
        self.active
            .lock()
            .expect("active lock")
            .contains_key(pack_id)
    }

    /// Every pack in the catalog, joined with its state on this machine.
    pub fn list(&self) -> Result<Vec<PackInfo>> {
        self.with_manifest(|manifest| {
            Ok(manifest
                .packs
                .iter()
                .map(|entry| self.layout.describe(entry, self.is_active(&entry.id)))
                .collect())
        })
    }

    pub fn info(&self, pack_id: &str) -> Result<PackInfo> {
        self.with_manifest(|manifest| {
            let entry = manifest.get(pack_id)?;
            Ok(self.layout.describe(entry, self.is_active(pack_id)))
        })
    }

    /// Ask the in-flight download for this pack to stop. The part file stays,
    /// so `download` picks up where it left off.
    pub fn pause(&self, pack_id: &str) {
        if let Some(flag) = self.active.lock().expect("active lock").get(pack_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn remove(&self, pack_id: &str) -> Result<()> {
        self.pause(pack_id);
        self.with_manifest(|manifest| {
            let entry = manifest.get(pack_id)?;
            self.layout.remove(entry)
        })
    }

    /// Download, verify and install a pack, resuming any partial download.
    ///
    /// `on_progress` is called as bytes land; the caller turns those into
    /// Tauri events.
    pub async fn download(
        &self,
        pack_id: &str,
        mut on_progress: impl FnMut(PackProgress),
    ) -> Result<PackStatus> {
        let entry = self.with_manifest(|manifest| Ok(manifest.get(pack_id)?.clone()))?;
        let url = self.with_manifest(|manifest| Ok(manifest.url_for(&entry)))?;

        if self.layout.is_installed(&entry) {
            return Ok(PackStatus::Installed);
        }

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active.lock().expect("active lock");
            if active.contains_key(pack_id) {
                return Err(Error::Pack(format!(
                    "{} is already downloading",
                    entry.name
                )));
            }
            active.insert(pack_id.to_string(), Arc::clone(&cancel));
        }

        let result = self
            .run_download(&entry, &url, Arc::clone(&cancel), &mut on_progress)
            .await;

        self.active.lock().expect("active lock").remove(pack_id);

        // Report the settled state so the UI never keeps a stale spinner.
        let settled = match &result {
            Ok(status) => *status,
            Err(_) => PackStatus::Failed,
        };
        on_progress(PackProgress {
            pack_id: pack_id.to_string(),
            status: settled,
            progress: if settled == PackStatus::Installed {
                1.0
            } else {
                0.0
            },
            bytes_on_disk: self.layout.partial_bytes(pack_id),
            size_bytes: entry.size_bytes,
        });

        result
    }

    async fn run_download(
        &self,
        entry: &manifest::PackEntry,
        url: &str,
        cancel: Arc<AtomicBool>,
        on_progress: &mut impl FnMut(PackProgress),
    ) -> Result<PackStatus> {
        let part_path = self.layout.part_path(&entry.id);
        let pack_id = entry.id.clone();
        let size_bytes = entry.size_bytes;

        let status = downloader::download_resumable(
            &self.client,
            url,
            &part_path,
            entry.size_bytes,
            cancel,
            |written, total| {
                on_progress(PackProgress {
                    pack_id: pack_id.clone(),
                    status: PackStatus::Downloading,
                    progress: if total == 0 {
                        0.0
                    } else {
                        (written as f64 / total as f64).clamp(0.0, 1.0)
                    },
                    bytes_on_disk: written,
                    size_bytes: total,
                });
            },
        )
        .await?;

        if status == DownloadStatus::Paused {
            return Ok(PackStatus::Paused);
        }

        on_progress(PackProgress {
            pack_id: entry.id.clone(),
            status: PackStatus::Verifying,
            progress: 1.0,
            bytes_on_disk: size_bytes,
            size_bytes,
        });

        let install_path = self.layout.install_path(entry);
        downloader::verify_and_install(&part_path, &install_path, &entry.sha256).await?;

        Ok(PackStatus::Installed)
    }
}
