//! The Test button: the cheapest real call that proves a key works.
//!
//! Phase 5 item 3 of `docs/SermonAI_API_Key_Strategy_Prompt.md`. "Cheapest"
//! matters — the operator may press this repeatedly while sorting out an
//! account, and a test that transcribes audio or generates tokens would bill
//! them for it. Each endpoint below is a metadata read: it authenticates, costs
//! nothing, and returns quickly.
//!
//! ## Why test at all, rather than wait for the first service
//!
//! A key that is wrong fails at the worst possible moment otherwise — the
//! moment the operator presses Transcribe with a congregation waiting. Finding
//! out on a Tuesday is the entire point.

use std::time::Duration;

use super::{Access, Credentials, Service};
use crate::error::{Error, Result};

/// A key test takes seconds at most. Longer than this and the useful answer is
/// "your connection, not your key" — which is what the timeout message says,
/// rather than leaving a spinner running until the operator gives up.
const TIMEOUT: Duration = Duration::from_secs(10);

/// What the operator is told after pressing Test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestOutcome {
    pub ok: bool,
    pub message: String,
}

impl TestOutcome {
    fn pass(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: message.into(),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
        }
    }
}

/// Make the cheapest authenticated call this service offers.
///
/// Returns `Ok(TestOutcome { ok: false, .. })` for a key the vendor rejected,
/// and `Err` only when the test could not be run at all. The difference is the
/// one the operator cares about: a rejected key is theirs to fix, and a failed
/// test is ours.
pub async fn test(credentials: &Credentials, service: Service) -> Result<TestOutcome> {
    let key = match credentials.access(service)? {
        Access::DirectKey(secret) => secret,
        // Nothing to test on this machine: the gateway holds the key, and
        // whether it works is what `/v1/status` answers. Phase 4.
        Access::Gateway { .. } => {
            return Ok(TestOutcome::pass(
                "Managed by SermonAI activation. Nothing to test on this machine.",
            ))
        }
    };

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(Error::Network)?;

    let request = match service {
        // Lists the projects the key can see. Free, and it is the call
        // Deepgram's own docs use to confirm a key.
        Service::Deepgram => client
            .get("https://api.deepgram.com/v1/projects")
            .header("Authorization", format!("Token {}", key.expose())),

        // Model list rather than a one-token message: it authenticates the
        // same way and costs nothing, where even `max_tokens: 1` is billed.
        //
        // A key not scoped to a workspace is refused without the workspace
        // header — a 400 whose body names the header, which `interpret` below
        // cannot see. The same header the live client sends is sent here, so
        // Test and the real call agree about whether the key works.
        Service::Anthropic => {
            let request = client
                .get("https://api.anthropic.com/v1/models?limit=1")
                .header("x-api-key", key.expose())
                .header("anthropic-version", "2023-06-01");
            match crate::llm::LlmConfig::load().workspace_id {
                Some(ws) => request.header("anthropic-workspace-id", ws),
                None => request,
            }
        }

        // Managed-only. Reaching here means a key was stored for a service
        // that refuses BYOK, which the provider should have prevented.
        Service::YouVersion | Service::TyndaleNlt => {
            return Ok(TestOutcome::fail(format!(
            "{} cannot be tested with a key on this machine: it comes with SermonAI activation.",
            service.label()
        )))
        }
    };

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) if err.is_timeout() => {
            return Ok(TestOutcome::fail(format!(
                "{} did not answer within {} seconds. Check this machine's internet connection.",
                service.label(),
                TIMEOUT.as_secs()
            )))
        }
        Err(err) if err.is_connect() => {
            return Ok(TestOutcome::fail(format!(
                "Could not reach {}. Check this machine's internet connection.",
                service.label()
            )))
        }
        Err(err) => return Err(Error::Network(err)),
    };

    Ok(interpret(service, response.status().as_u16()))
}

/// Turn a status code into something an operator can act on.
///
/// Split out and tested directly because the mapping is the part that is easy
/// to get wrong, and getting it wrong sends somebody to the wrong page of their
/// vendor account — 401 and 403 look alike and mean very different things.
fn interpret(service: Service, status: u16) -> TestOutcome {
    let label = service.label();
    match status {
        200..=299 => TestOutcome::pass(format!("{label} accepted this key.")),
        401 => TestOutcome::fail(format!(
            "{label} rejected this key. Check it was copied in full, then paste it again."
        )),
        403 => TestOutcome::fail(format!(
            "{label} recognised this key but refused it. Check the key's permissions in your {label} account."
        )),
        400 => TestOutcome::fail(format!(
            "{label} refused the request. If your key is not scoped to a workspace, set ANTHROPIC_WORKSPACE_ID (see Settings help)."
        )),
        402 => TestOutcome::fail(format!(
            "{label} accepted this key but the account has no credit. Add billing in your {label} account."
        )),
        429 => TestOutcome::fail(format!(
            "{label} is rate limiting this key. Wait a minute and test again."
        )),
        500..=599 => TestOutcome::fail(format!(
            "{label} is having trouble at their end ({status}). The key may be fine; test again shortly."
        )),
        other => TestOutcome::fail(format!(
            "{label} answered with an unexpected status ({other}). The key was not confirmed."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_key_is_told_apart_from_a_forbidden_one() {
        // These two are the pair that matters. 401 means the characters are
        // wrong, and the fix is to paste again; 403 means the key is real but
        // scoped wrong, and pasting it again forever will not help.
        let unauthorized = interpret(Service::Deepgram, 401);
        assert!(!unauthorized.ok);
        assert!(unauthorized.message.contains("paste it again"));

        let forbidden = interpret(Service::Deepgram, 403);
        assert!(!forbidden.ok);
        assert!(forbidden.message.contains("permissions"));
        assert!(!forbidden.message.contains("paste it again"));
    }

    #[test]
    fn a_vendor_outage_does_not_read_as_a_bad_key() {
        // Telling an operator their key is wrong when the vendor is down sends
        // them to revoke and reissue a working key, which is worse than
        // waiting.
        let outcome = interpret(Service::Anthropic, 503);
        assert!(!outcome.ok);
        assert!(outcome.message.contains("their end"), "{}", outcome.message);
        assert!(outcome.message.contains("key may be fine"));
    }

    #[test]
    fn success_is_the_only_thing_that_passes() {
        assert!(interpret(Service::Deepgram, 200).ok);
        assert!(interpret(Service::Deepgram, 204).ok);

        for status in [301, 400, 401, 402, 403, 404, 429, 500, 503] {
            assert!(
                !interpret(Service::Deepgram, status).ok,
                "{status} must not report a working key"
            );
        }
    }

    #[test]
    fn every_message_names_the_service() {
        // The panel tests services one at a time but shows the result in a
        // shared area, and an unattributed "rejected this key" is unreadable.
        for status in [200, 401, 403, 402, 429, 500, 418] {
            let outcome = interpret(Service::Anthropic, status);
            assert!(
                outcome.message.contains("Anthropic"),
                "status {status}: {}",
                outcome.message
            );
        }
    }
}
