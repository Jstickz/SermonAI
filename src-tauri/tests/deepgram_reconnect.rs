//! Reconnection, against a server that fails the way a real network does.
//!
//! These exist because reasoning about a dropped connection was not enough.
//! The reconnect shipped, passed a live happy-path check, and then failed on
//! real hardware in a way no unit test could have found: when wifi goes, TCP
//! reports nothing. Writes keep succeeding into the kernel buffer and reads
//! pend forever, so a socket that is dead still looks open. The client waited
//! on it indefinitely and never reconnected.
//!
//! So the server here does not politely close. It **goes silent while holding
//! the connection open**, which is the case that broke, and then refuses new
//! connections for a while so the backoff is exercised for real.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use sermonai_lib::credentials::{Access, CredentialStatus};
use sermonai_lib::credentials::{Credentials, Provider, Secret, Service};
use sermonai_lib::stt::reconnect::{ResilientStream, SttStatus};

/// A provider that hands out a fixed key, so these tests need no real
/// credential and never touch the network beyond localhost.
struct FakeKey;

impl Provider for FakeKey {
    fn access(&self, _service: Service) -> sermonai_lib::error::Result<Access> {
        Ok(Access::DirectKey(Secret::new("test-key")))
    }
    fn status(&self, _service: Service) -> CredentialStatus {
        CredentialStatus::ManagedActive
    }
}

#[derive(Clone)]
struct Server {
    /// Connections accepted so far, so a test can tell a reconnect happened.
    connections: Arc<AtomicUsize>,
    /// While true, a connection is accepted and then goes silent without
    /// closing — a dead socket that still looks alive.
    go_silent: Arc<AtomicBool>,
    /// Chunks of audio received across every connection.
    audio_chunks: Arc<AtomicUsize>,
}

async fn handler(ws: WebSocketUpgrade, State(server): State<Server>) -> Response {
    ws.on_upgrade(move |socket| serve(socket, server))
}

async fn serve(mut socket: WebSocket, server: Server) {
    server.connections.fetch_add(1, Ordering::SeqCst);

    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Binary(bytes) => {
                server.audio_chunks.fetch_add(1, Ordering::SeqCst);
                // Read per message, not once at connect. Reading it once meant
                // a connection opened before the switch was flipped carried on
                // replying, so the test never went silent at all and reported
                // a product bug that was its own.
                if server.go_silent.load(Ordering::SeqCst) {
                    // The failure being reproduced: keep the socket open, keep
                    // reading, answer nothing. A real dropped network does not
                    // send a close frame.
                    continue;
                }
                // Otherwise behave like Deepgram: acknowledge audio with a
                // result, so the client's liveness check is satisfied.
                let reply = format!(
                    r#"{{"type":"Results","is_final":true,"speech_final":true,
                        "channel":{{"alternatives":[{{"transcript":"chunk {}",
                        "words":[{{"word":"chunk","start":0.0,"end":0.1,"confidence":0.9}}]}}]}}}}"#,
                    bytes.len()
                );
                if socket.send(Message::Text(reply)).await.is_err() {
                    return;
                }
            }
            Message::Text(_) => {}
            Message::Close(_) => return,
            _ => {}
        }
    }
}

/// Start the stand-in, returning its address and a handle to steer it.
async fn start_server() -> (SocketAddr, Server, tokio::task::JoinHandle<()>) {
    let server = Server {
        connections: Arc::new(AtomicUsize::new(0)),
        go_silent: Arc::new(AtomicBool::new(false)),
        audio_chunks: Arc::new(AtomicUsize::new(0)),
    };

    let app = Router::new()
        .route("/listen", get(handler))
        .with_state(server.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    (addr, server, task)
}

fn chunk() -> Vec<i16> {
    vec![0i16; 4000]
}

/// The bug, reproduced: a connection that goes silent without closing.
///
/// Before the fix this hung indefinitely — the client sat in `read.next()`
/// waiting for an error that TCP never delivers, so no reconnect was ever
/// attempted and nothing was transcribed again.
#[tokio::test(flavor = "multi_thread")]
async fn a_silent_connection_is_detected_and_replaced() {
    let (addr, server, _task) = start_server().await;
    let endpoint = format!("ws://{addr}/listen");

    let credentials = Credentials::new(Box::new(FakeKey));
    let statuses: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&statuses);

    let stream = ResilientStream::connect_at(
        &endpoint,
        &credentials,
        Vec::new(),
        Box::new(|_| {}),
        Box::new(move |status| {
            seen.lock().unwrap().push(match status {
                SttStatus::Connected => "connected".to_string(),
                SttStatus::Reconnecting { attempt, .. } => format!("reconnecting/{attempt}"),
                SttStatus::AudioDropped { .. } => "dropped".to_string(),
            });
        }),
    )
    .await
    .expect("the first connection should succeed");

    // Healthy for a moment, so the test is not measuring startup.
    for _ in 0..4 {
        stream.send(chunk());
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(server.connections.load(Ordering::SeqCst), 1);

    // From here the server answers nothing, and never closes.
    server.go_silent.store(true, Ordering::SeqCst);

    // Keep speaking, as the operator did. The client must notice within its
    // response timeout and reopen the connection by itself.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    while tokio::time::Instant::now() < deadline {
        stream.send(chunk());
        tokio::time::sleep(Duration::from_millis(100)).await;
        if server.connections.load(Ordering::SeqCst) > 1 {
            break;
        }
    }

    let connections = server.connections.load(Ordering::SeqCst);
    let statuses = statuses.lock().unwrap().clone();

    assert!(
        connections > 1,
        "a silent connection was never replaced: {connections} connection(s), statuses {statuses:?}"
    );
    assert!(
        statuses.iter().any(|s| s.starts_with("reconnecting")),
        "the operator was never told: {statuses:?}"
    );
}

/// Audio spoken during an outage is held and replayed, not lost.
#[tokio::test(flavor = "multi_thread")]
async fn audio_during_an_outage_is_held_and_replayed() {
    let (addr, server, task) = start_server().await;
    let endpoint = format!("ws://{addr}/listen");

    let credentials = Credentials::new(Box::new(FakeKey));
    let stream = ResilientStream::connect_at(
        &endpoint,
        &credentials,
        Vec::new(),
        Box::new(|_| {}),
        Box::new(|_| {}),
    )
    .await
    .expect("the first connection should succeed");

    for _ in 0..4 {
        stream.send(chunk());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let before = server.audio_chunks.load(Ordering::SeqCst);
    assert!(before > 0, "the server should have received audio");

    // The whole server goes away: connections are refused, not merely silent.
    task.abort();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Twenty chunks spoken while there is nothing to send them to.
    for _ in 0..20 {
        stream.send(chunk());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // The same port comes back, as a router rebooting would.
    let app = Router::new()
        .route("/listen", get(handler))
        .with_state(server.clone());
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let _revived = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // Long enough for the backoff to come round again.
    tokio::time::sleep(Duration::from_secs(12)).await;

    let after = server.audio_chunks.load(Ordering::SeqCst);
    assert!(
        after > before,
        "held audio was never replayed: {before} chunks before the outage, {after} after"
    );
}
