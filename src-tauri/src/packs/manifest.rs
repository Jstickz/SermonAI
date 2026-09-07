//! The pack catalog served from the Pack CDN (PRD §15.5).
//!
//! The manifest lists every downloadable pack with its version, size and
//! SHA-256. It is the only place a pack URL comes from, so a pack can never be
//! fetched from an address the catalog did not name.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// What a pack contains, which decides where it is installed (PRD §14.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackKind {
    Speech,
    Translation,
    Intelligence,
    Theme,
}

impl PackKind {
    /// Directory under `<app data>/packs/` that holds this kind.
    pub fn dir_name(self) -> &'static str {
        match self {
            PackKind::Speech => "speech",
            PackKind::Translation => "translations",
            PackKind::Intelligence => "intelligence",
            PackKind::Theme => "themes",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackEntry {
    pub id: String,
    pub kind: PackKind,
    pub name: String,
    pub description: String,
    pub version: String,
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 of the downloaded file. Required: a pack with no
    /// checksum is never activated (FR-61).
    pub sha256: String,
    /// Path relative to the manifest's `baseUrl`.
    pub path: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    pub manifest_version: u32,
    #[serde(default)]
    pub generated_at: String,
    pub base_url: String,
    pub packs: Vec<PackEntry>,
}

impl PackManifest {
    pub fn get(&self, pack_id: &str) -> Result<&PackEntry> {
        self.packs
            .iter()
            .find(|p| p.id == pack_id)
            .ok_or_else(|| Error::Pack(format!("no pack called '{pack_id}' in the catalog")))
    }

    /// Absolute download URL for a pack.
    pub fn url_for(&self, entry: &PackEntry) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            entry.path.trim_start_matches('/')
        )
    }
}

/// Fetch and parse the catalog.
///
/// TODO(M0, blocked on deliverable 6): verify the manifest's detached signature
/// before trusting it. Until the signing key is provisioned, every pack is
/// still checksum-verified against this manifest, but the manifest itself is
/// trusted on TLS alone.
pub async fn fetch(client: &reqwest::Client, manifest_url: &str) -> Result<PackManifest> {
    let response = client.get(manifest_url).send().await?;
    if !response.status().is_success() {
        return Err(Error::Pack(format!(
            "pack catalog returned {} — check your connection and try again",
            response.status()
        )));
    }

    let body = response.text().await?;
    parse(&body)
}

pub fn parse(body: &str) -> Result<PackManifest> {
    serde_json::from_str(body).map_err(|e| Error::Pack(format!("pack catalog is unreadable: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "manifestVersion": 1,
        "baseUrl": "https://packs.example.com/v1/",
        "packs": [{
            "id": "speech-whisper-base-en",
            "kind": "speech",
            "name": "Offline Speech Pack",
            "description": "Transcribe without internet.",
            "version": "1.0.0",
            "sizeBytes": 78643200,
            "sha256": "abc123",
            "path": "/speech/whisper-base-en.bin",
            "optional": true
        }]
    }"#;

    #[test]
    fn parses_and_builds_urls_without_double_slashes() {
        let manifest = parse(SAMPLE).expect("parse");
        let entry = manifest.get("speech-whisper-base-en").expect("entry");

        assert_eq!(entry.kind, PackKind::Speech);
        assert_eq!(entry.size_bytes, 78_643_200);
        assert_eq!(
            manifest.url_for(entry),
            "https://packs.example.com/v1/speech/whisper-base-en.bin"
        );
    }

    #[test]
    fn unknown_pack_names_the_pack_in_the_error() {
        let manifest = parse(SAMPLE).expect("parse");
        let err = manifest.get("nope").unwrap_err().to_string();
        assert!(err.contains("nope"), "error should name the pack: {err}");
    }
}
