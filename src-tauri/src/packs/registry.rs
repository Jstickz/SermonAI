//! What is installed on disk, what it costs, and removing it.
//!
//! Layout follows PRD §14.3: `<app data>/packs/<kind dir>/<pack id>/<file>`,
//! with in-flight downloads parked in `<app data>/packs/.downloads`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::manifest::{PackEntry, PackKind};

/// Where a pack is in its lifecycle, as the Packs screen shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackStatus {
    Available,
    Downloading,
    Paused,
    Verifying,
    Installed,
    Failed,
}

/// A catalog entry joined with its state on this machine — what the Settings →
/// Packs screen renders (FR-60).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub id: String,
    pub kind: PackKind,
    pub name: String,
    pub description: String,
    pub version: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub optional: bool,
    pub tier: Option<String>,
    pub status: PackStatus,
    /// 0.0 to 1.0.
    pub progress: f64,
    pub bytes_on_disk: u64,
}

pub struct Layout {
    root: PathBuf,
}

impl Layout {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            root: app_data_dir.join("packs"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join(".downloads")
    }

    pub fn part_path(&self, pack_id: &str) -> PathBuf {
        self.downloads_dir().join(format!("{pack_id}.part"))
    }

    /// Directory that holds one installed pack.
    pub fn install_dir(&self, entry: &PackEntry) -> PathBuf {
        self.root.join(entry.kind.dir_name()).join(&entry.id)
    }

    /// Final resting place of the pack file itself.
    pub fn install_path(&self, entry: &PackEntry) -> PathBuf {
        let file_name = entry
            .path
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or(entry.id.as_str());
        self.install_dir(entry).join(file_name)
    }

    pub fn is_installed(&self, entry: &PackEntry) -> bool {
        self.install_path(entry).is_file()
    }

    /// Bytes already fetched for an unfinished download, 0 if there is none.
    pub fn partial_bytes(&self, pack_id: &str) -> u64 {
        std::fs::metadata(self.part_path(pack_id))
            .map(|meta| meta.len())
            .unwrap_or(0)
    }

    /// Delete an installed pack and free the space immediately (PRD §14.3).
    /// Also clears any leftover part file so a later download starts clean.
    pub fn remove(&self, entry: &PackEntry) -> Result<()> {
        let dir = self.install_dir(entry);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }

        let part = self.part_path(&entry.id);
        if part.exists() {
            std::fs::remove_file(part)?;
        }

        Ok(())
    }

    /// Join a catalog entry with what is on disk. `active` marks packs the
    /// download task is working on right now.
    pub fn describe(&self, entry: &PackEntry, active: bool) -> PackInfo {
        let installed = self.is_installed(entry);
        let on_disk = if installed {
            entry.size_bytes
        } else {
            self.partial_bytes(&entry.id)
        };

        let status = if installed {
            PackStatus::Installed
        } else if active {
            PackStatus::Downloading
        } else if on_disk > 0 {
            PackStatus::Paused
        } else {
            PackStatus::Available
        };

        let progress = match (installed, entry.size_bytes) {
            (true, _) => 1.0,
            (false, 0) => 0.0,
            (false, total) => (on_disk as f64 / total as f64).clamp(0.0, 1.0),
        };

        PackInfo {
            id: entry.id.clone(),
            kind: entry.kind,
            name: entry.name.clone(),
            description: entry.description.clone(),
            version: entry.version.clone(),
            size_bytes: entry.size_bytes,
            sha256: entry.sha256.clone(),
            optional: entry.optional,
            tier: entry.tier.clone(),
            status,
            progress,
            bytes_on_disk: on_disk,
        }
    }
}
