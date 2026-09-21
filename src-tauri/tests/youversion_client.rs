//! YouVersion Platform client tests.
//!
//! Two layers, because they answer different questions:
//!
//! * A local stub server proves the request plumbing — that every call carries
//!   `X-YVP-App-Key`, that responses are parsed, and that failures produce
//!   messages an operator can act on. These run everywhere, offline.
//! * One live test hits the real API when `YVP_APP_KEY` is present, which is
//!   the only thing that can confirm the response shapes are right. It skips
//!   itself on forks rather than failing a build nobody can fix.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::Router;
use sermonai_lib::bible::youversion::{YouVersionClient, APP_KEY_ENV};

const TEST_KEY: &str = "test-app-key-123";

#[derive(Default)]
struct Seen {
    app_keys: Mutex<Vec<Option<String>>>,
}

fn record(state: &Arc<Seen>, headers: &HeaderMap) {
    let key = headers
        .get("X-YVP-App-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    state.app_keys.lock().unwrap().push(key);
}

async fn bibles(State(state): State<Arc<Seen>>, headers: HeaderMap) -> (StatusCode, String) {
    record(&state, &headers);
    (
        StatusCode::OK,
        r#"{"data":[
             {"id":111,"title":"New International Version","abbreviation":"NIV",
              "language_tag":"eng","copyright":"The Holy Bible, NIV (c) Biblica","status":"approved"},
             {"id":59,"title":"English Standard Version","abbreviation":"ESV",
              "language_tag":"eng","copyright":"(c) Crossway","status":"pending"}
           ]}"#
            .to_string(),
    )
}

async fn passage(
    State(state): State<Arc<Seen>>,
    Path((_version, passage_id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> (StatusCode, String) {
    record(&state, &headers);
    (
        StatusCode::OK,
        format!(
            r#"{{"data":{{"id":"{passage_id}","content":"<p><span class=\"verse v16\" data-usfm=\"JHN.3.16\">For God so loved the world.</span></p>","reference":"John 3:16","copyright":"(c) Biblica"}}}}"#
        ),
    )
}

async fn forbidden(State(state): State<Arc<Seen>>, headers: HeaderMap) -> (StatusCode, String) {
    record(&state, &headers);
    (
        StatusCode::FORBIDDEN,
        r#"{"error":"not licensed"}"#.to_string(),
    )
}

async fn spawn_stub() -> (SocketAddr, Arc<Seen>) {
    let state = Arc::new(Seen::default());
    let app = Router::new()
        .route("/bibles", get(bibles))
        .route("/bibles/:version/passages/:passage_id", get(passage))
        .route("/forbidden", get(forbidden))
        .with_state(Arc::clone(&state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, state)
}

fn client_for(addr: SocketAddr) -> YouVersionClient {
    YouVersionClient::with_key(Some(TEST_KEY.to_string()), format!("http://{addr}"))
}

#[tokio::test]
async fn every_request_carries_the_app_key_header() {
    let (addr, seen) = spawn_stub().await;
    let client = client_for(addr);

    client.list_bibles().await.expect("list");
    client.get_passage(111, "JHN.3.16").await.expect("passage");

    let keys = seen.app_keys.lock().unwrap();
    assert_eq!(keys.len(), 2, "both calls should have reached the server");
    for key in keys.iter() {
        assert_eq!(
            key.as_deref(),
            Some(TEST_KEY),
            "X-YVP-App-Key missing or wrong on a request"
        );
    }
}

#[tokio::test]
async fn versions_parse_with_their_attribution_and_licence_state() {
    let (addr, _) = spawn_stub().await;
    let versions = client_for(addr).list_bibles().await.expect("list");

    assert_eq!(versions.len(), 2);

    let niv = &versions[0];
    assert_eq!(niv.id, 111);
    assert_eq!(niv.short_name, "NIV");
    assert_eq!(niv.language, "eng");
    assert!(
        niv.has_attribution(),
        "attribution is a licensing obligation"
    );
    assert!(niv.attribution.contains("Biblica"));

    // A version in the catalog is not necessarily licensed to this app key.
    assert_eq!(
        versions[1].license_status,
        sermonai_lib::bible::youversion::LicenseStatus::Pending
    );
}

#[tokio::test]
async fn a_passage_comes_back_with_content_and_attribution() {
    let (addr, _) = spawn_stub().await;
    let passage = client_for(addr)
        .get_passage(111, "JHN.3.16")
        .await
        .expect("passage");

    assert_eq!(passage.passage_id, "JHN.3.16");
    assert!(passage.content.contains("For God so loved"));
    assert!(!passage.attribution.is_empty());

    // The HTML is sanitized before it reaches a screen.
    let sanitized = sermonai_lib::bible::sanitize::sanitize(&passage.content);
    assert_eq!(sanitized.text, "For God so loved the world.");
    assert_eq!(
        sanitized.verses,
        vec![(16, "For God so loved the world.".to_string())]
    );
}

#[tokio::test]
async fn a_refused_request_tells_the_operator_what_to_do() {
    let (addr, _) = spawn_stub().await;
    let client = YouVersionClient::with_key(Some(TEST_KEY.into()), format!("http://{addr}"));

    // /forbidden stands in for an unlicensed version.
    let err = client
        .get_passage(999, "../../forbidden")
        .await
        .map(|_| ())
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("licence")
            || err.contains("app key")
            || err.contains("no ")
            || err.contains("YouVersion"),
        "unhelpful message: {err}"
    );
}

#[tokio::test]
async fn without_a_key_the_client_is_disabled_but_usable() {
    let client = YouVersionClient::with_key(None, "http://127.0.0.1:1".to_string());
    assert!(!client.is_online_enabled());

    let err = client.list_bibles().await.unwrap_err().to_string();
    assert!(err.contains(APP_KEY_ENV), "should name the variable: {err}");
}

/// The only test that can confirm YouVersion's real response shapes.
///
/// Skips rather than fails when the key is absent, so forks and outside PRs
/// stay green. Everything it asserts is a licensing requirement: content must
/// be non-empty, and so must the copyright string.
#[tokio::test]
async fn live_api_returns_content_and_attribution() {
    let Ok(key) = std::env::var(APP_KEY_ENV) else {
        eprintln!("SKIPPED: {APP_KEY_ENV} is not set, so the live YouVersion test cannot run.");
        return;
    };
    if key.trim().is_empty() {
        eprintln!("SKIPPED: {APP_KEY_ENV} is empty.");
        return;
    }

    let client = YouVersionClient::from_env();
    assert!(client.is_online_enabled());

    let versions = client
        .list_bibles()
        .await
        .expect("listing versions should succeed with a valid app key");
    assert!(!versions.is_empty(), "no versions licensed to this app key");

    let licensed = versions
        .iter()
        .find(|v| v.has_attribution())
        .unwrap_or(&versions[0]);

    let passage = client
        .get_passage(licensed.id, "JHN.3.16")
        .await
        .expect("John 3:16 should fetch from a licensed version");

    assert!(
        !passage.content.trim().is_empty(),
        "passage content was empty for version {}",
        licensed.id
    );

    let attribution = if passage.attribution.trim().is_empty() {
        licensed.attribution.clone()
    } else {
        passage.attribution.clone()
    };
    assert!(
        !attribution.trim().is_empty(),
        "no copyright string for version {} — the text cannot legally be displayed",
        licensed.id
    );

    // Print the real shapes so the TODO(yvp-shape) guesses can be checked.
    eprintln!(
        "live version: id={} short={} attribution={attribution:?}",
        licensed.id, licensed.short_name
    );
    eprintln!("live passage first 200 chars: {:.200}", passage.content);
}
