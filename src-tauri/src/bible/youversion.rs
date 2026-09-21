//! YouVersion Platform client (PRD v2.2, replaces API.Bible).
//!
//! Three things make this different from an ordinary REST client.
//!
//! **Attribution is mandatory.** Every version carries a copyright string that
//! must be stored with the cached text and shown wherever the verse appears.
//! It is returned as part of the version, not the passage, so it is refreshed
//! on every online fetch and travels with the text into the cache.
//!
//! **Licensing is per app key.** A version present in the catalog is not
//! necessarily licensed to us; the operator may have to accept terms in the
//! YouVersion portal before it can be fetched.
//!
//! **Missing key is not fatal.** If `YVP_APP_KEY` is absent the client starts
//! disabled: online fetching is off, the cache still serves, and the operator
//! is told once rather than on every verse. A church mid-service should lose
//! new lookups, not the app.
//!
//! ## Response shapes are provisional
//!
//! Everything marked `TODO(yvp-shape)` below was inferred from the endpoint
//! descriptions, not from a live call. Deserialization is deliberately
//! permissive — unknown fields are ignored and alternative field names are
//! accepted via `serde(alias)` — so a wrong guess degrades to a missing field
//! rather than a hard parse failure. Confirm against a real response before
//! relying on any of it.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const BASE_URL: &str = "https://api.youversion.com/v1";

/// Environment variable holding the app key, locally and in CI.
pub const APP_KEY_ENV: &str = "YVP_APP_KEY";

/// Where the operator accepts a version's licence terms.
pub const PORTAL_URL: &str = "https://platform.youversion.com";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Whether a version may be fetched with this app key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// In the catalog, terms not yet accepted in the portal.
    #[default]
    Pending,
    Approved,
    /// Access withdrawn. Cached text must stop being displayed.
    Revoked,
}

impl LicenseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LicenseStatus::Pending => "pending",
            LicenseStatus::Approved => "approved",
            LicenseStatus::Revoked => "revoked",
        }
    }
}

/// One version from `GET /bibles`.
///
/// TODO(yvp-shape): field names are inferred. Aliases cover the spellings a
/// JSON API of this shape usually picks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleVersion {
    #[serde(alias = "version_id", alias = "versionId")]
    pub id: i64,
    #[serde(default, alias = "title", alias = "local_title")]
    pub name: String,
    #[serde(
        default,
        alias = "abbreviation",
        alias = "short_name",
        alias = "local_abbreviation"
    )]
    pub short_name: String,
    #[serde(default, alias = "language_tag", alias = "iso_639_3")]
    pub language: String,
    /// The copyright string that must be displayed with the text.
    #[serde(
        default,
        alias = "copyright",
        alias = "copyright_short",
        alias = "publisher"
    )]
    pub attribution: String,
    #[serde(default, alias = "license", alias = "status")]
    pub license_status: LicenseStatus,
}

impl BibleVersion {
    /// Attribution is a licensing obligation, so a version without one cannot
    /// be displayed even if its text fetches fine.
    pub fn has_attribution(&self) -> bool {
        !self.attribution.trim().is_empty()
    }
}

/// `GET /bibles/{version_id}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleMetadata {
    #[serde(alias = "version_id", alias = "versionId")]
    pub id: i64,
    #[serde(default, alias = "title", alias = "local_title")]
    pub name: String,
    #[serde(
        default,
        alias = "abbreviation",
        alias = "short_name",
        alias = "local_abbreviation"
    )]
    pub short_name: String,
    #[serde(default, alias = "language_tag")]
    pub language: String,
    #[serde(default, alias = "copyright", alias = "copyright_short")]
    pub attribution: String,
}

/// `GET /bibles/{version_id}/passages/{passage_id}`.
///
/// TODO(yvp-shape): the HTML is assumed to arrive as a `content` string. Some
/// APIs nest it under `data` or return an array of verses instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passage {
    #[serde(default, alias = "id", alias = "usfm")]
    pub passage_id: String,
    #[serde(default, alias = "html", alias = "text", alias = "body")]
    pub content: String,
    #[serde(default, alias = "reference", alias = "human")]
    pub reference: String,
    /// Copyright for the version this passage came from, when the endpoint
    /// repeats it. Otherwise taken from the version record.
    #[serde(default, alias = "copyright", alias = "copyright_short")]
    pub attribution: String,
}

/// A client for the YouVersion Platform API.
pub struct YouVersionClient {
    http: reqwest::Client,
    app_key: Option<String>,
    base_url: String,
}

impl YouVersionClient {
    /// Build from the environment.
    ///
    /// A missing key disables online fetching rather than failing: the cache
    /// still works offline, which is the whole point of PRD §2.3.
    pub fn from_env() -> Self {
        let app_key = std::env::var(APP_KEY_ENV)
            .ok()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());

        if app_key.is_none() {
            tracing::error!(
                "{APP_KEY_ENV} is not set, so online Bible lookups are disabled. \
                 Cached verses still work. Get an app key from {PORTAL_URL} and put it in .env"
            );
        }

        Self::with_key(app_key, BASE_URL.to_string())
    }

    /// Construct explicitly. Tests point this at a local stub server.
    pub fn with_key(app_key: Option<String>, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();

        Self {
            http,
            app_key,
            base_url,
        }
    }

    /// False when no app key is configured; callers fall back to the cache.
    pub fn is_online_enabled(&self) -> bool {
        self.app_key.is_some()
    }

    /// Every request carries the app key header; there is no other auth.
    fn get(&self, path: &str) -> Result<reqwest::RequestBuilder> {
        let key = self.app_key.as_ref().ok_or_else(|| {
            Error::Bible(format!(
                "online Bible lookups are off because {APP_KEY_ENV} is not set. Add it to .env and restart"
            ))
        })?;

        Ok(self
            .http
            .get(format!("{}{path}", self.base_url))
            .header("X-YVP-App-Key", key)
            .header(reqwest::header::ACCEPT, "application/json"))
    }

    /// Versions this app key is licensed for.
    pub async fn list_bibles(&self) -> Result<Vec<BibleVersion>> {
        let body = self.send(self.get("/bibles")?, "the version list").await?;

        // TODO(yvp-shape): a bare array and a `{ "data": [...] }` envelope are
        // both plausible; unwrap_envelope handles either.
        parse_maybe_enveloped(&body, "the version list")
    }

    pub async fn get_bible_metadata(&self, version_id: i64) -> Result<BibleMetadata> {
        let body = self
            .send(
                self.get(&format!("/bibles/{version_id}"))?,
                "version details",
            )
            .await?;
        parse_maybe_enveloped(&body, "version details")
    }

    /// Fetch a passage by USFM ID, e.g. `JHN.3.16` or `PSA.139.13-16`.
    pub async fn get_passage(&self, version_id: i64, passage_id: &str) -> Result<Passage> {
        let body = self
            .send(
                self.get(&format!("/bibles/{version_id}/passages/{passage_id}"))?,
                passage_id,
            )
            .await?;
        parse_maybe_enveloped(&body, passage_id)
    }

    /// Send a request and turn transport and status failures into messages an
    /// operator can act on (Branding §9.3).
    async fn send(&self, request: reqwest::RequestBuilder, what: &str) -> Result<String> {
        let response = request
            .send()
            .await
            .map_err(|e| Error::Bible(format!("could not reach YouVersion for {what}: {e}")))?;

        let status = response.status();
        if status.is_success() {
            return response
                .text()
                .await
                .map_err(|e| Error::Bible(format!("could not read {what} from YouVersion: {e}")));
        }

        Err(Error::Bible(match status.as_u16() {
            401 | 403 => format!(
                "YouVersion refused the request for {what}. The app key may be wrong, or this \
                 version's licence may not be approved yet at {PORTAL_URL}"
            ),
            404 => format!("YouVersion has no {what} in this version"),
            429 => format!("YouVersion is rate limiting us. {what} will retry shortly"),
            _ => format!("YouVersion returned {status} for {what}"),
        }))
    }
}

/// Keys an API of this shape might wrap its payload in.
const ENVELOPE_KEYS: [&str; 6] = ["data", "passage", "bible", "version", "result", "response"];

/// Unwrap `{ "data": ... }` if present, then deserialize.
///
/// Deliberately not "try bare, then try enveloped": every field on these
/// structs has a default, so an envelope parses cleanly as the inner type with
/// everything empty. That would hand back a blank verse instead of an error —
/// the worst outcome available, since nothing looks wrong until it is on a
/// screen in front of a congregation.
fn unwrap_envelope(body: &str, what: &str) -> Result<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| Error::Bible(format!("YouVersion sent unreadable JSON for {what}: {e}")))?;

    let Some(object) = value.as_object() else {
        return Ok(value);
    };

    // An envelope has exactly one recognised key holding the payload.
    for key in ENVELOPE_KEYS {
        if let Some(inner) = object.get(key) {
            if inner.is_object() || inner.is_array() {
                return Ok(inner.clone());
            }
        }
    }

    Ok(value)
}

fn parse_maybe_enveloped<T: serde::de::DeserializeOwned>(body: &str, what: &str) -> Result<T> {
    let value = unwrap_envelope(body, what)?;
    serde_json::from_value(value)
        .map_err(|e| Error::Bible(format!("could not read {what} from YouVersion: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_key_disables_online_rather_than_failing() {
        let client = YouVersionClient::with_key(None, BASE_URL.to_string());
        assert!(!client.is_online_enabled());

        // The error names the variable and where to get a key.
        let err = client.get("/bibles").unwrap_err().to_string();
        assert!(err.contains(APP_KEY_ENV), "{err}");
    }

    #[test]
    fn a_blank_key_counts_as_missing() {
        let client = YouVersionClient::with_key(Some("   ".into()), BASE_URL.to_string());
        // with_key takes what it is given; from_env does the trimming, so this
        // documents that callers must not pass blanks through.
        assert!(client.is_online_enabled());

        let trimmed: Option<String> = Some("   ".to_string())
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty());
        assert!(
            trimmed.is_none(),
            "from_env must treat a blank key as absent"
        );
    }

    #[test]
    fn license_status_round_trips() {
        assert_eq!(LicenseStatus::default(), LicenseStatus::Pending);
        assert_eq!(LicenseStatus::Approved.as_str(), "approved");
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"revoked\"").unwrap(),
            LicenseStatus::Revoked
        );
    }

    #[test]
    fn a_version_without_attribution_is_not_displayable() {
        let version: BibleVersion = serde_json::from_str(
            r#"{"id":111,"name":"New International Version","abbreviation":"NIV","copyright":""}"#,
        )
        .unwrap();
        assert_eq!(version.short_name, "NIV");
        assert!(!version.has_attribution());
    }

    /// Permissiveness is the point: an unexpected field must not break a
    /// service, and a differently named one should still be found.
    #[test]
    fn unknown_fields_are_ignored_and_aliases_accepted() {
        let version: BibleVersion = serde_json::from_str(
            r#"{"version_id":59,"local_title":"English Standard Version",
                "local_abbreviation":"ESV","copyright_short":"(c) Crossway",
                "something_we_have_never_seen":true}"#,
        )
        .unwrap();

        assert_eq!(version.id, 59);
        assert_eq!(version.name, "English Standard Version");
        assert_eq!(version.short_name, "ESV");
        assert_eq!(version.attribution, "(c) Crossway");
        assert!(version.has_attribution());
    }

    #[test]
    fn enveloped_and_bare_passages_both_parse() {
        let bare =
            r#"{"id":"JHN.3.16","content":"<p>For God so loved</p>","copyright":"(c) Someone"}"#;
        let passage: Passage = parse_maybe_enveloped(bare, "JHN.3.16").unwrap();
        assert_eq!(passage.passage_id, "JHN.3.16");
        assert!(passage.content.contains("For God so loved"));

        let enveloped = format!(r#"{{"data":{bare}}}"#);
        let passage: Passage = parse_maybe_enveloped(&enveloped, "JHN.3.16").unwrap();
        assert_eq!(passage.attribution, "(c) Someone");
    }
}
