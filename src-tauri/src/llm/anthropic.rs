//! HTTP client for the Anthropic Messages API.
//!
//! Small on purpose: one request shape, one response shape, retries for the
//! failures that retrying helps, and operator-facing messages for the ones it
//! does not. Both the paraphrase stage and (in M4) the summariser build on it.
//!
//! ## What is and is not retried
//!
//! 429 and 5xx are retried with backoff — the vendor is asking for a moment,
//! or having one. 401 and 403 are not: the key is wrong or lacks permission,
//! and asking again cannot fix that. 400 is not either, and its body is kept
//! in the error, because it is how a key that is not scoped to a workspace
//! announces itself ("must include the anthropic-workspace-id header"), and
//! that is a configuration the operator can supply.
//!
//! ## Timeouts are the caller's
//!
//! A paraphrase call has 1.5 s p95 to be useful (PRD §18.1) and a summary has
//! ninety seconds. One client, two deadlines, so the deadline is a parameter.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::credentials::{Access, Credentials, Secret, Service};
use crate::error::{Error, Result};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";

/// Retries after a 429 or 5xx, and the wait before each.
const RETRY_BACKOFF: [Duration; 2] = [Duration::from_millis(400), Duration::from_millis(1200)];

pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    key: Secret,
    workspace_id: Option<String>,
}

/// One completed call, with what it cost in tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub text: String,
    pub model: String,
    pub input_tokens: u64,
    pub cached_read_tokens: u64,
    pub output_tokens: u64,
}

impl AnthropicClient {
    /// Build from the credential provider. Fails now, at construction, if the
    /// key is missing — so a service without one is told at Start, not at the
    /// first paraphrase.
    pub fn from_credentials(
        credentials: &Credentials,
        workspace_id: Option<String>,
    ) -> Result<Self> {
        let key = match credentials.access(Service::Anthropic)? {
            Access::DirectKey(secret) => secret,
            Access::Gateway { .. } => {
                return Err(Error::Config(
                    "Claude through the SermonAI Gateway is not available yet.".to_string(),
                ))
            }
        };
        Ok(Self::with_key(key, workspace_id))
    }

    pub fn with_key(key: Secret, workspace_id: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            key,
            workspace_id,
        }
    }

    /// Point at a local server. Tests use this to serve canned replies and to
    /// misbehave on purpose.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// One user turn against a system prompt. `max_tokens` bounds the reply
    /// and therefore the cost; `deadline` bounds the wait.
    pub async fn message(
        &self,
        model: &str,
        system: &str,
        user: &str,
        max_tokens: u32,
        deadline: Duration,
    ) -> Result<Completion> {
        let body = Request {
            model,
            max_tokens,
            // The system prompt is the same every call and is where prompt
            // caching pays: it is marked ephemeral so the vendor keeps it
            // between calls at the cached-read price.
            system: vec![SystemBlock {
                kind: "text",
                text: system,
                cache_control: Some(CacheControl { kind: "ephemeral" }),
            }],
            messages: vec![Message {
                role: "user",
                content: user,
            }],
        };

        let mut attempt = 0usize;
        loop {
            let mut request = self
                .http
                .post(format!("{}/v1/messages", self.base_url))
                .timeout(deadline)
                .header("x-api-key", self.key.expose())
                .header("anthropic-version", API_VERSION)
                .header("content-type", "application/json")
                .json(&body);
            if let Some(ws) = &self.workspace_id {
                request = request.header("anthropic-workspace-id", ws);
            }

            let response = match request.send().await {
                Ok(r) => r,
                Err(err) if err.is_timeout() => {
                    return Err(Error::Detection(format!(
                        "Claude did not answer within {:.1} s.",
                        deadline.as_secs_f64()
                    )))
                }
                Err(err) => return Err(Error::Network(err)),
            };

            let status = response.status().as_u16();
            let text = response.text().await.map_err(Error::Network)?;

            match status {
                200..=299 => return parse_completion(&text),
                429 | 500..=599 if attempt < RETRY_BACKOFF.len() => {
                    tokio::time::sleep(RETRY_BACKOFF[attempt]).await;
                    attempt += 1;
                }
                _ => return Err(describe_failure(status, &text)),
            }
        }
    }
}

/// Turn a non-success into a message an operator can act on. The vendor's own
/// text is kept for 400, because that is where a missing workspace header is
/// explained and nothing we could write would say it better.
fn describe_failure(status: u16, body: &str) -> Error {
    let vendor = serde_json::from_str::<ErrorEnvelope>(body)
        .map(|e| e.error.message)
        .unwrap_or_default();
    match status {
        401 => Error::Config(
            "Anthropic rejected the key. Check it in Settings → Services and keys.".to_string(),
        ),
        403 => Error::Config(
            "Anthropic refused the key's permissions. Check the key in your Anthropic account."
                .to_string(),
        ),
        400 => Error::Config(format!("Anthropic refused the request: {vendor}")),
        429 => Error::Detection(
            "Claude is rate limiting this key; paraphrase detection will resume shortly."
                .to_string(),
        ),
        _ => Error::Detection(
            format!("Claude answered with status {status}. {vendor}")
                .trim()
                .to_string(),
        ),
    }
}

fn parse_completion(body: &str) -> Result<Completion> {
    let reply: Reply = serde_json::from_str(body)
        .map_err(|e| Error::Detection(format!("Claude's reply could not be read: {e}")))?;
    let text = reply
        .content
        .iter()
        .filter(|b| b.kind == "text")
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    Ok(Completion {
        text,
        model: reply.model,
        input_tokens: reply.usage.input_tokens,
        cached_read_tokens: reply.usage.cache_read_input_tokens.unwrap_or(0),
        output_tokens: reply.usage.output_tokens,
    })
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    max_tokens: u32,
    system: Vec<SystemBlock<'a>>,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct SystemBlock<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl<'a>>,
}

#[derive(Serialize)]
struct CacheControl<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct Reply {
    model: String,
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_is_read_with_its_usage() {
        // The shape the live API returned on 24 Sept, trimmed.
        let body = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-haiku-4-5-20251001",
            "content":[{"type":"text","text":"[]"}],"stop_reason":"end_turn",
            "usage":{"input_tokens":412,"output_tokens":3,"cache_read_input_tokens":380}}"#;
        let c = parse_completion(body).expect("parse");
        assert_eq!(c.text, "[]");
        assert_eq!(
            (c.input_tokens, c.cached_read_tokens, c.output_tokens),
            (412, 380, 3)
        );
    }

    #[test]
    fn the_workspace_error_reaches_the_operator_verbatim() {
        // The exact body the live API returned for this project's key. The
        // vendor's sentence names the header to add; ours would not.
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"This API key is not scoped to a workspace, so this request must include the anthropic-workspace-id header with the ID of the workspace to use."}}"#;
        let err = describe_failure(400, body).to_string();
        assert!(err.contains("anthropic-workspace-id"), "{err}");
    }

    #[test]
    fn a_bad_key_points_at_settings_and_a_forbidden_one_at_the_account() {
        assert!(describe_failure(401, "{}")
            .to_string()
            .contains("Services and keys"));
        assert!(describe_failure(403, "{}").to_string().contains("account"));
    }
}
