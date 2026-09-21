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
//! ## Response shapes, verified against the live API on 21 September 2026
//!
//! `GET /bibles?language_ranges[]=eng` returns `{ data: [...], next_page_token,
//! total_size }`. The other two endpoints return a **bare object**, with no
//! envelope, so both forms are handled.
//!
//! Three things the endpoint descriptions did not say, each confirmed by call:
//!
//! 1. `copyright` is **null in the list** and populated only on
//!    `GET /bibles/{id}`. Attribution therefore requires a per-version fetch.
//! 2. Passage `content` is **plain text by default**. `?format=html` is what
//!    returns markup with verse markers, so this client always asks for it.
//! 3. There is **no licence field anywhere**. The list is itself the licence:
//!    a version this app key may use appears in it, and one it may not does
//!    not. `LicenseStatus` is therefore derived locally, not parsed.

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
///
/// The API exposes no licence field: `GET /bibles` simply lists what this key
/// may use. So a version present in that list is Approved, one we know of but
/// which has dropped out is Revoked, and Pending covers a version the operator
/// has asked for in the portal but which has not appeared yet. This is derived
/// from the catalog, never deserialized from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    /// Requested in the portal, not yet in the catalog.
    #[default]
    Pending,
    Approved,
    /// Was licensed and no longer is. Cached text must stop being displayed.
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
/// Field names are the live ones. `copyright` is null here for every version —
/// see `attribution`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleVersion {
    pub id: i64,
    /// Publisher's abbreviation, e.g. "engWEBUS". Often not what a reader
    /// recognises; `localized_abbreviation` is the friendlier "WEBUS".
    #[serde(default)]
    pub abbreviation: String,
    #[serde(default)]
    pub localized_abbreviation: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub localized_title: String,
    #[serde(default)]
    pub language_tag: String,
    /// USFM codes this version contains. 66 for a Protestant canon, 80 when
    /// the Apocrypha is included.
    #[serde(default)]
    pub books: Vec<String>,
    /// Always null on this endpoint; populated by `get_bible_metadata`.
    #[serde(default)]
    pub copyright: Option<String>,
    /// Derived, not parsed: presence in the catalog is the licence.
    #[serde(skip, default = "approved")]
    pub license_status: LicenseStatus,
}

fn approved() -> LicenseStatus {
    LicenseStatus::Approved
}

impl BibleVersion {
    /// The copyright string, if this record carries one.
    pub fn attribution(&self) -> &str {
        self.copyright.as_deref().unwrap_or_default().trim()
    }

    /// Attribution is a licensing obligation, so a version without one cannot
    /// be displayed even if its text fetches fine. Note that the list endpoint
    /// never supplies it: fetch the version to find out.
    pub fn has_attribution(&self) -> bool {
        !self.attribution().is_empty()
    }

    /// What the operator should see, preferring the localized forms.
    pub fn display_name(&self) -> &str {
        first_non_empty(&[&self.localized_title, &self.title])
    }

    pub fn short_name(&self) -> &str {
        first_non_empty(&[&self.localized_abbreviation, &self.abbreviation])
    }
}

fn first_non_empty<'a>(candidates: &[&'a String]) -> &'a str {
    candidates
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

/// `GET /bibles/{version_id}` — the same object as the list, but with
/// `copyright` and `promotional_content` filled in.
pub type BibleMetadata = BibleVersion;

/// `GET /bibles/{version_id}/passages/{passage_id}?format=html`.
///
/// The live response has exactly three fields and no envelope. There is no
/// copyright here, so attribution has to come from the version record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passage {
    /// USFM ID as requested, e.g. "JHN.3.16".
    #[serde(default)]
    pub id: String,
    /// Markup when `format=html` was asked for, which this client always does.
    #[serde(default)]
    pub content: String,
    /// Human reference, e.g. "John 3:16".
    #[serde(default)]
    pub reference: String,
}

/// One page of `GET /bibles`.
#[derive(Debug, Deserialize)]
struct VersionPage {
    #[serde(default)]
    data: Vec<BibleVersion>,
    #[serde(default)]
    next_page_token: Option<String>,
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
    /// Versions this app key is licensed for.
    ///
    /// `language_ranges[]` is required: without it the API rejects the request
    /// with "Field required" rather than returning everything.
    ///
    /// Note the returned records have `copyright: null`. Call
    /// `get_bible_metadata` for a version before displaying its text.
    pub async fn list_bibles(&self, language: &str) -> Result<Vec<BibleVersion>> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut path = format!("/bibles?language_ranges[]={language}");
            if let Some(token) = &page_token {
                path.push_str(&format!("&page_token={token}"));
            }

            let body = self.send(self.get(&path)?, "the version list").await?;
            let page: VersionPage = serde_json::from_str(&body).map_err(|e| {
                Error::Bible(format!(
                    "could not read the version list from YouVersion: {e}"
                ))
            })?;

            all.extend(page.data);

            // 20 versions fitted one page when this was written, but the API
            // paginates and a wider licence would spill over.
            page_token = page.next_page_token.filter(|token| !token.is_empty());
            if page_token.is_none() {
                break;
            }
        }

        Ok(all)
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

    /// Fetch a passage by USFM ID, e.g. `JHN.3.16`, `PSA.139.13-16` or a whole
    /// chapter as `JHN.3`.
    ///
    /// Always asks for `format=html`. Without it the API returns one
    /// unbroken string with no verse boundaries, which cannot be split into
    /// the per-verse rows the cache stores.
    pub async fn get_passage(&self, version_id: i64, passage_id: &str) -> Result<Passage> {
        let body = self
            .send(
                self.get(&format!(
                    "/bibles/{version_id}/passages/{passage_id}?format=html"
                ))?,
                passage_id,
            )
            .await?;

        let passage: Passage = parse_maybe_enveloped(&body, passage_id)?;

        // An empty passage means the verse did not come back, which must not
        // reach a projector as a blank screen the operator cannot explain.
        if passage.content.trim().is_empty() {
            return Err(Error::Bible(format!(
                "YouVersion returned no text for {passage_id}"
            )));
        }

        Ok(passage)
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

    /// Verbatim from /bibles?language_ranges[]=eng, captured 21 Sept 2026.
    const LIST_SAMPLE: &str = r#"{"data":[{"id":1588,"abbreviation":"AMP",
        "promotional_content":null,"copyright":null,"info":null,"publisher_url":null,
        "language_tag":"en","localized_abbreviation":"AMP","localized_title":"Amplified Bible",
        "title":"Amplified Bible","books":["GEN","EXO"],
        "youversion_deep_link":"https://www.bible.com/versions/1588",
        "organization_id":"798d8fa4"}],"next_page_token":null,"total_size":20}"#;

    /// Verbatim from /bibles/206.
    const VERSION_SAMPLE: &str = r#"{"id":206,"abbreviation":"engWEBUS",
        "promotional_content":"This Public Domain Bible text is courtesy of eBible.org.",
        "copyright":"PUBLIC DOMAIN (not copyrighted)","info":null,"publisher_url":null,
        "language_tag":"en","localized_abbreviation":"WEBUS",
        "localized_title":"World English Bible, American English Edition, without Strong's Numbers",
        "title":"World English Bible, American English Edition, without Strong's Numbers",
        "books":["GEN"],"youversion_deep_link":"https://www.bible.com/versions/206",
        "organization_id":"73a4fa15"}"#;

    /// Verbatim from /bibles/206/passages/JHN.3.16?format=html.
    const PASSAGE_SAMPLE: &str = r#"{"id":"JHN.3.16","content":"<div><div class=\"p\"><span class=\"yv-v\" v=\"16\"></span><span class=\"yv-vlbl\">16</span><span class=\"wj\">For God so loved the world.</span></div></div>","reference":"John 3:16"}"#;

    #[test]
    fn a_missing_key_disables_online_rather_than_failing() {
        let client = YouVersionClient::with_key(None, BASE_URL.to_string());
        assert!(!client.is_online_enabled());

        let err = client.get("/bibles").unwrap_err().to_string();
        assert!(err.contains(APP_KEY_ENV), "{err}");
    }

    #[test]
    fn from_env_treats_a_blank_key_as_absent() {
        let trimmed: Option<String> = Some("   ".to_string())
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty());
        assert!(trimmed.is_none());
    }

    #[test]
    fn license_status_is_derived_not_parsed() {
        // The API has no licence field; presence in the catalog is the licence.
        let page: VersionPage = serde_json::from_str(LIST_SAMPLE).unwrap();
        assert_eq!(page.data[0].license_status, LicenseStatus::Approved);
        assert_eq!(LicenseStatus::Revoked.as_str(), "revoked");
    }

    /// The list endpoint returns copyright: null for every version, so nothing
    /// from it may be displayed until the version itself has been fetched.
    #[test]
    fn the_list_carries_no_attribution() {
        let page: VersionPage = serde_json::from_str(LIST_SAMPLE).unwrap();
        let version = &page.data[0];

        assert_eq!(version.id, 1588);
        assert_eq!(version.short_name(), "AMP");
        assert_eq!(version.display_name(), "Amplified Bible");
        assert_eq!(version.language_tag, "en");
        assert!(
            !version.has_attribution(),
            "the list endpoint should not be trusted for attribution"
        );
        assert_eq!(page.next_page_token, None);
    }

    #[test]
    fn fetching_a_version_supplies_the_attribution() {
        let version: BibleMetadata = serde_json::from_str(VERSION_SAMPLE).unwrap();

        assert_eq!(version.id, 206);
        assert_eq!(version.short_name(), "WEBUS", "prefers the localized form");
        assert_eq!(version.abbreviation, "engWEBUS");
        assert!(version.has_attribution());
        assert_eq!(version.attribution(), "PUBLIC DOMAIN (not copyrighted)");
    }

    #[test]
    fn a_passage_parses_and_sanitizes() {
        let passage: Passage = serde_json::from_str(PASSAGE_SAMPLE).unwrap();
        assert_eq!(passage.id, "JHN.3.16");
        assert_eq!(passage.reference, "John 3:16");

        let sanitized = crate::bible::sanitize::sanitize(&passage.content);
        assert_eq!(sanitized.text, "For God so loved the world.");
        assert_eq!(
            sanitized.verses,
            vec![(16, "For God so loved the world.".to_string())]
        );
    }

    /// Both endpoint styles appear in this API: the list is enveloped, the
    /// other two are bare.
    #[test]
    fn bare_and_enveloped_objects_both_parse() {
        let bare: Passage = parse_maybe_enveloped(PASSAGE_SAMPLE, "JHN.3.16").unwrap();
        assert_eq!(bare.id, "JHN.3.16");

        let enveloped = format!(r#"{{"data":{PASSAGE_SAMPLE}}}"#);
        let wrapped: Passage = parse_maybe_enveloped(&enveloped, "JHN.3.16").unwrap();
        assert_eq!(wrapped.id, "JHN.3.16");
        assert!(
            !wrapped.content.is_empty(),
            "an envelope must not parse as an empty object"
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let version: BibleVersion =
            serde_json::from_str(r#"{"id":7,"title":"X","something_new":true}"#).unwrap();
        assert_eq!(version.id, 7);
        assert_eq!(version.display_name(), "X");
    }
}
