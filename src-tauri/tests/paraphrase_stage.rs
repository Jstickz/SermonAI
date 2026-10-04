//! The paraphrase stage against a local stand-in for the Anthropic API.
//!
//! The live service needs a key scoped to a workspace, which this project's
//! key is not (see `llm::LlmConfig::workspace_id`), so what can be verified
//! without one is verified here: the request the client actually sends, the
//! headers it carries, the retry it performs on a 429, and the way the stage
//! turns a reply into references. The one thing this cannot verify is that
//! Claude's judgement is any good — that is the 100-paraphrase DoD measure,
//! and it needs the real thing.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;

use sermonai_lib::credentials::Secret;
use sermonai_lib::detection::llm::{Decision, ParaphraseStage};
use sermonai_lib::llm::anthropic::AnthropicClient;
use sermonai_lib::llm::ParaphraseGate;

#[derive(Default)]
struct Seen {
    calls: AtomicUsize,
    last_headers: std::sync::Mutex<Option<HeaderMap>>,
    last_body: std::sync::Mutex<Option<String>>,
}

/// Serves `reply` for every call after the first `fail_first` calls, which
/// get a 429.
async fn serve(reply: &'static str, fail_first: usize) -> (SocketAddr, Arc<Seen>) {
    serve_with_status(reply, fail_first, StatusCode::OK).await
}

/// `status` is what every call after the first `fail_first` gets.
async fn serve_with_status(
    reply: &'static str,
    fail_first: usize,
    status: StatusCode,
) -> (SocketAddr, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let state = (Arc::clone(&seen), reply, fail_first, status);

    let app = Router::new().route(
        "/v1/messages",
        post(
            |State((seen, reply, fail_first, status)): State<(
                Arc<Seen>,
                &'static str,
                usize,
                StatusCode,
            )>,
             headers: HeaderMap,
             body: Bytes| async move {
                let n = seen.calls.fetch_add(1, Ordering::SeqCst);
                *seen.last_headers.lock().unwrap() = Some(headers);
                *seen.last_body.lock().unwrap() =
                    Some(String::from_utf8_lossy(&body).into_owned());
                if n < fail_first {
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
                    );
                }
                (status, reply)
            },
        ),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.with_state(state)).await.unwrap();
    });
    (addr, seen)
}

const TWO_REFERENCES: &str = r#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-haiku-4-5-20251001",
 "content":[{"type":"text","text":"[{\"reference\":\"Jeremiah 29:11\",\"evidence\":\"plans to prosper you\",\"confidence\":0.92},{\"reference\":\"John 30:1\",\"evidence\":\"made up\",\"confidence\":0.9}]"}],
 "stop_reason":"end_turn","usage":{"input_tokens":512,"output_tokens":48,"cache_read_input_tokens":400}}"#;

fn stage_at(addr: SocketAddr, workspace: Option<&str>) -> ParaphraseStage {
    let client = AnthropicClient::with_key(
        Secret::new("sk-test-key-0000"),
        workspace.map(str::to_string),
    )
    .with_base_url(format!("http://{addr}"));
    ParaphraseStage::new(
        client,
        "claude-haiku-4-5-20251001",
        ParaphraseGate::default(),
    )
}

#[tokio::test]
async fn the_request_carries_the_headers_and_prompt_the_api_expects() {
    let (addr, seen) = serve(TWO_REFERENCES, 0).await;
    let mut stage = stage_at(addr, Some("wrkspc_test"));

    assert_eq!(stage.consider(0.70), Decision::Call);
    let hits = stage
        .detect("I know the plans I have for you")
        .await
        .expect("reply");

    let headers = seen.last_headers.lock().unwrap().clone().unwrap();
    assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-key-0000");
    assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
    // The header this project's key requires. Its absence was the whole of
    // the 400 the live API returned.
    assert_eq!(
        headers.get("anthropic-workspace-id").unwrap(),
        "wrkspc_test"
    );

    let body: serde_json::Value =
        serde_json::from_str(seen.last_body.lock().unwrap().as_deref().unwrap()).unwrap();
    assert_eq!(body["model"], "claude-haiku-4-5-20251001");
    assert_eq!(
        body["messages"][0]["content"],
        "I know the plans I have for you"
    );
    // The system prompt is marked for caching: it is identical every call.
    assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
    assert!(body["system"][0]["text"]
        .as_str()
        .unwrap()
        .contains("JSON array"));

    // The good reference survived; the impossible one did not.
    let ids: Vec<String> = hits.iter().map(|h| h.reference.passage_id()).collect();
    assert_eq!(ids, ["JER.29.11"]);
    assert_eq!(hits[0].evidence, "plans to prosper you");
}

#[tokio::test]
async fn a_key_without_a_workspace_sends_no_workspace_header() {
    let (addr, seen) = serve(TWO_REFERENCES, 0).await;
    let mut stage = stage_at(addr, None);
    stage.consider(0.70);
    stage.detect("anything").await.expect("reply");
    let headers = seen.last_headers.lock().unwrap().clone().unwrap();
    assert!(headers.get("anthropic-workspace-id").is_none());
}

#[tokio::test]
async fn a_rate_limit_is_retried_and_the_tokens_are_counted_once() {
    let (addr, seen) = serve(TWO_REFERENCES, 1).await;
    let mut stage = stage_at(addr, None);
    stage.consider(0.70);
    let hits = stage
        .detect("anything")
        .await
        .expect("second attempt succeeds");
    assert_eq!(hits.len(), 1);
    assert_eq!(seen.calls.load(Ordering::SeqCst), 2, "one 429, one success");

    // Usage counts the one call that answered, and its tokens once.
    let usage = stage.usage();
    assert_eq!(usage.calls, 1);
    assert_eq!(
        (
            usage.input_tokens,
            usage.cached_read_tokens,
            usage.output_tokens
        ),
        (512, 400, 48)
    );

    // And the cost estimate follows the priced tokens.
    let usd = usage.estimated_cost_usd(stage.model());
    assert!(usd > 0.0 && usd < 0.01, "{usd}");
}

#[tokio::test]
async fn the_gate_keeps_a_sermon_from_calling_on_every_utterance() {
    let (addr, seen) = serve(TWO_REFERENCES, 0).await;
    let mut stage = stage_at(addr, None);

    // The fifteen unquoted sentences and eight paraphrases measured for the
    // bands, in the order a service might produce them. Only utterances in
    // the band, outside the cooldown, reach the network.
    let scores = [
        0.615, 0.592, 0.463, 0.665, 0.997, 0.620, 0.560, 0.699, 0.572, 0.432, 0.843, 0.584, 0.696,
        0.589, 0.339, 0.664, 0.621, 0.759, 0.422, 0.313,
    ];
    for score in scores {
        if stage.consider(score) == Decision::Call {
            stage.detect("...").await.expect("reply");
        }
    }

    let usage = stage.usage();
    // Everything above 0.80 was accepted from the vector stage alone.
    assert_eq!(usage.accepted_vector, 2);
    // Everything below 0.55 never cost anything.
    assert_eq!(usage.skipped, 5);
    // Of the thirteen in the band, the first called and the rest fell inside
    // its ten-second cooldown — this loop runs in milliseconds.
    assert_eq!(usage.calls, 1);
    assert_eq!(usage.cooled_down, 12);
    assert_eq!(seen.calls.load(Ordering::SeqCst), 1);
}

const WORKSPACE_400: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"This API key is not scoped to a workspace, so this request must include the anthropic-workspace-id header with the ID of the workspace to use."}}"#;

#[tokio::test]
async fn a_configuration_error_disables_the_stage_for_the_service() {
    // The first live run: 57 calls, every one this 400, zero tokens. A
    // configuration error cannot be fixed by asking again ten seconds later,
    // so the first one has to be the last one until the next Start.
    let (addr, seen) = serve_with_status(WORKSPACE_400, 0, StatusCode::BAD_REQUEST).await;
    let mut stage = stage_at(addr, None);

    stage.consider(0.70);
    let err = stage
        .detect("plans to prosper you")
        .await
        .expect_err("a 400 is an error");
    assert!(err.to_string().contains("anthropic-workspace-id"), "{err}");

    // Disabled, with the vendor's own sentence as the reason.
    assert!(stage.disabled_reason().is_some());
    assert_eq!(stage.usage().calls, 1);
    assert_eq!(stage.usage().failed, 1);

    // A second request does not reach the network at all.
    stage.consider(0.70);
    stage
        .detect("more words")
        .await
        .expect_err("still disabled");
    assert_eq!(
        seen.calls.load(Ordering::SeqCst),
        1,
        "no second call was made"
    );
    assert_eq!(
        stage.usage().calls,
        1,
        "a refused call is not counted as an attempt"
    );
}

#[tokio::test]
async fn a_rate_limit_that_persists_does_not_disable_the_stage() {
    // 429 is the vendor asking for a moment, not a misconfiguration. The call
    // fails after its retries, is counted as failed, and the stage stays
    // available for the next window.
    let (addr, _seen) = serve_with_status(TWO_REFERENCES, 99, StatusCode::OK).await;
    let mut stage = stage_at(addr, None);
    stage.consider(0.70);
    stage
        .detect("anything")
        .await
        .expect_err("exhausted retries");
    assert_eq!(stage.usage().failed, 1);
    assert!(
        stage.disabled_reason().is_none(),
        "a 429 must not disable the stage"
    );
}
