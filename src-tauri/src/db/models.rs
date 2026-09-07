//! Row structs for the tables in PRD §14.2. These serialize straight to the
//! frontend, so field names must match src/lib/types.ts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SermonStatus {
    Live,
    Processing,
    Ready,
    NeedsAttention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionSource {
    Regex,
    Vector,
    Llm,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sermon {
    pub id: i64,
    pub title: Option<String>,
    pub preacher: Option<String>,
    pub date: String,
    pub duration_seconds: Option<i64>,
    pub default_translation: String,
    pub series_id: Option<i64>,
    pub status: SermonStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: i64,
    pub sermon_id: i64,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
    pub text: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedScripture {
    pub id: i64,
    pub sermon_id: i64,
    pub segment_id: Option<i64>,
    pub book: String,
    pub chapter: i64,
    pub verse: i64,
    pub end_verse: Option<i64>,
    pub translation: String,
    pub confidence: Option<f64>,
    pub source: DetectionSource,
    pub accepted_by_operator: bool,
    pub went_live: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verse {
    pub reference: String,
    pub translation: String,
    pub text: String,
}
