//! Resumable, checksummed pack downloads (FR-61).
//!
//! A download writes to a `.part` file and appends to it with HTTP range
//! requests, so an interrupted download — a crash, a pause, a dropped wifi
//! connection — resumes from the last byte on disk rather than starting over.
//! The `.part` file is only renamed into place after its SHA-256 matches the
//! catalog, so a partial pack is never activated (PRD §10.6).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{Error, Result};

/// Report progress at most this often, so a fast download does not flood the
/// UI with events.
const PROGRESS_STEP_BYTES: u64 = 512 * 1024;

/// Read buffer for checksumming a finished file.
const HASH_CHUNK_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Completed,
    /// The operator paused, or the app is shutting down. The `.part` file is
    /// intact and the next attempt continues from it.
    Paused,
}

/// Download `url` into `part_path`, resuming if a partial file is already there.
///
/// `on_progress` receives (bytes_on_disk, total_bytes).
pub async fn download_resumable(
    client: &reqwest::Client,
    url: &str,
    part_path: &Path,
    expected_size: u64,
    cancel: Arc<AtomicBool>,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<DownloadStatus> {
    if let Some(parent) = part_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut on_disk = match tokio::fs::metadata(part_path).await {
        Ok(meta) => meta.len(),
        Err(_) => 0,
    };

    // A part file larger than the catalog says is stale (the pack was
    // republished, or the file was truncated oddly). Start it again.
    if expected_size > 0 && on_disk > expected_size {
        tokio::fs::remove_file(part_path).await.ok();
        on_disk = 0;
    }

    if expected_size > 0 && on_disk == expected_size {
        on_progress(on_disk, expected_size);
        return Ok(DownloadStatus::Completed);
    }

    let mut request = client.get(url);
    if on_disk > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={on_disk}-"));
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(Error::Pack(format!(
            "download failed with {status} — try again, or check your connection"
        )));
    }

    // If we asked to resume but the server sent the whole file anyway, honour
    // that and rewrite from the start rather than corrupting the part file.
    let resuming = on_disk > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
    if on_disk > 0 && !resuming {
        tracing::warn!(url, "server ignored range request; restarting download");
        on_disk = 0;
    }

    let total = if expected_size > 0 {
        expected_size
    } else {
        response.content_length().unwrap_or(0) + on_disk
    };

    let mut options = OpenOptions::new();
    options.create(true).write(true);
    if resuming {
        options.append(true);
    } else {
        options.truncate(true);
    }
    let mut file = options.open(part_path).await?;

    let mut written = on_disk;
    let mut since_report = 0u64;
    let mut stream = response.bytes_stream();

    on_progress(written, total);

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            file.flush().await?;
            file.sync_all().await?;
            return Ok(DownloadStatus::Paused);
        }

        let chunk = chunk?;
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        since_report += chunk.len() as u64;

        if since_report >= PROGRESS_STEP_BYTES {
            since_report = 0;
            on_progress(written, total);
        }
    }

    // Durability matters here: the whole point of a part file is that it
    // survives a hard kill.
    file.flush().await?;
    file.sync_all().await?;
    on_progress(written, total);

    if expected_size > 0 && written != expected_size {
        return Err(Error::Pack(format!(
            "download ended early at {written} of {expected_size} bytes — try again"
        )));
    }

    Ok(DownloadStatus::Completed)
}

/// Lowercase hex SHA-256 of a file, read in chunks so a 1 GB pack does not
/// land in memory all at once.
pub async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_CHUNK_BYTES];

    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Verify a finished download and move it into place.
///
/// On mismatch the part file is deleted: a corrupt download must not be
/// resumable, or the app would append to bad bytes forever.
pub async fn verify_and_install(
    part_path: &Path,
    install_path: &Path,
    expected_sha256: &str,
) -> Result<()> {
    if expected_sha256.trim().is_empty() {
        return Err(Error::Pack(
            "the pack catalog has no checksum for this pack, so it cannot be installed safely"
                .into(),
        ));
    }

    let actual = sha256_file(part_path).await?;
    if !actual.eq_ignore_ascii_case(expected_sha256.trim()) {
        tokio::fs::remove_file(part_path).await.ok();
        return Err(Error::Pack(
            "the downloaded pack failed its integrity check and was discarded. Download it again"
                .into(),
        ));
    }

    if let Some(parent) = install_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(part_path, install_path).await?;

    Ok(())
}
