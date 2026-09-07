//! Covers the M0 definition-of-done line for packs:
//! "Download a 30 MB test pack, kill the app at 50%, relaunch, download
//! resumes and verifies." (FR-61)
//!
//! A local range-capable server stands in for the Pack CDN so the whole cycle
//! runs in CI with no network access.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use sermonai_lib::packs::downloader::{self, DownloadStatus};
use sermonai_lib::packs::registry::PackStatus;
use sermonai_lib::packs::PackManager;
use sha2::{Digest, Sha256};

/// Smaller than the 30 MB of the manual DoD run so the suite stays fast. The
/// resume logic is identical at either size.
const PACK_BYTES: usize = 4 * 1024 * 1024;

struct ServerState {
    body: Vec<u8>,
    /// Range header of every request served, so a test can prove that a resume
    /// really did continue rather than start over.
    ranges: Mutex<Vec<Option<String>>>,
}

async fn serve_pack(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    state.ranges.lock().unwrap().push(range.clone());

    let total = state.body.len();
    let start = range
        .as_deref()
        .and_then(|r| r.strip_prefix("bytes="))
        .and_then(|r| r.strip_suffix('-'))
        .and_then(|n| n.parse::<usize>().ok());

    match start {
        Some(offset) if offset < total => {
            let mut out = HeaderMap::new();
            let value = format!("bytes {}-{}/{}", offset, total - 1, total);
            out.insert(header::CONTENT_RANGE, value.parse().unwrap());
            (
                StatusCode::PARTIAL_CONTENT,
                out,
                state.body[offset..].to_vec(),
            )
        }
        Some(_) => (
            StatusCode::RANGE_NOT_SATISFIABLE,
            HeaderMap::new(),
            Vec::new(),
        ),
        None => (StatusCode::OK, HeaderMap::new(), state.body.clone()),
    }
}

async fn spawn_server(body: Vec<u8>) -> (SocketAddr, Arc<ServerState>) {
    let state = Arc::new(ServerState {
        body,
        ranges: Mutex::new(Vec::new()),
    });

    let app = Router::new()
        .route("/packs/test-pack.bin", get(serve_pack))
        .with_state(Arc::clone(&state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, state)
}

fn manifest_json(addr: SocketAddr, sha256: &str, size: usize) -> String {
    format!(
        concat!(
            r#"{{"manifestVersion":1,"baseUrl":"http://{addr}","packs":[{{"#,
            r#""id":"test-pack","kind":"speech","name":"Test Pack","#,
            r#""description":"Stand-in for the Offline Speech Pack.","#,
            r#""version":"1.0.0","sizeBytes":{size},"sha256":"{sha}","#,
            r#""path":"/packs/test-pack.bin","optional":true}}]}}"#
        ),
        addr = addr,
        size = size,
        sha = sha256
    )
}

fn deterministic_body(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sermonai-packs-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The DoD line: half the pack is already on disk from a killed run, and a
/// freshly constructed manager finishes it without re-fetching what it has.
#[tokio::test]
async fn resumes_a_half_finished_pack_after_a_restart() {
    let body = deterministic_body(PACK_BYTES);
    let digest = sha256_hex(&body);
    let (addr, server) = spawn_server(body.clone()).await;
    let temp = temp_dir("resume");

    // Leave behind exactly what a kill at 50% leaves behind.
    let part_path = temp.join("packs/.downloads/test-pack.part");
    std::fs::create_dir_all(part_path.parent().unwrap()).unwrap();
    std::fs::write(&part_path, &body[..PACK_BYTES / 2]).unwrap();

    // A brand new manager, as after relaunching the app.
    let manager = PackManager::new(&temp, format!("http://{addr}/manifest.json"));
    manager
        .set_manifest_json(&manifest_json(addr, &digest, PACK_BYTES))
        .unwrap();

    let status = manager
        .download("test-pack", |_| {})
        .await
        .expect("resume should succeed");
    assert_eq!(status, PackStatus::Installed);

    let ranges = server.ranges.lock().unwrap();
    let sent = ranges.first().cloned().flatten();
    assert_eq!(
        sent,
        Some(format!("bytes={}-", PACK_BYTES / 2)),
        "resume must ask only for the bytes it does not already have"
    );

    let install_path = temp.join("packs/speech/test-pack/test-pack.bin");
    let installed = std::fs::read(&install_path).expect("pack should be installed");
    assert_eq!(installed.len(), PACK_BYTES);
    assert_eq!(
        sha256_hex(&installed),
        digest,
        "installed pack must match the catalog"
    );
    assert!(!part_path.exists(), "part file is consumed on install");

    std::fs::remove_dir_all(&temp).ok();
}

/// Pausing reports Paused rather than an error, leaves the part file in place
/// for a later resume, and that resume then completes to full length.
///
/// Byte-level continuation is asserted by the restart test above; here the
/// cancel fires before the first chunk, so the part file is empty.
#[tokio::test]
async fn pausing_is_not_an_error_and_the_download_can_continue_after() {
    let body = deterministic_body(PACK_BYTES);
    let (addr, _server) = spawn_server(body.clone()).await;
    let temp = temp_dir("pause");
    let part_path = temp.join("test.part");
    std::fs::create_dir_all(&temp).unwrap();

    // Cancel before the first chunk lands: the strictest version of "the
    // operator hit Pause immediately".
    let cancel = Arc::new(AtomicBool::new(true));
    let client = reqwest::Client::new();

    let status = downloader::download_resumable(
        &client,
        &format!("http://{addr}/packs/test-pack.bin"),
        &part_path,
        PACK_BYTES as u64,
        Arc::clone(&cancel),
        |_, _| {},
    )
    .await
    .expect("pause is not an error");

    assert_eq!(status, DownloadStatus::Paused);
    assert!(part_path.exists(), "a paused download keeps its part file");

    // Resuming from that part file completes to the full length.
    cancel.store(false, Ordering::Relaxed);
    let status = downloader::download_resumable(
        &client,
        &format!("http://{addr}/packs/test-pack.bin"),
        &part_path,
        PACK_BYTES as u64,
        cancel,
        |_, _| {},
    )
    .await
    .expect("resume after pause");

    assert_eq!(status, DownloadStatus::Completed);
    assert_eq!(
        std::fs::metadata(&part_path).unwrap().len(),
        PACK_BYTES as u64
    );

    std::fs::remove_dir_all(&temp).ok();
}

/// FR-61: verified by checksum before activation. A pack whose bytes do not
/// match the catalog is never installed, and the bad part file is discarded so
/// a retry cannot append to corrupt data.
#[tokio::test]
async fn a_corrupt_download_is_rejected_and_discarded() {
    let body = deterministic_body(64 * 1024);
    let (addr, _server) = spawn_server(body.clone()).await;
    let temp = temp_dir("corrupt");

    let wrong = sha256_hex(b"not the pack you are looking for");
    let manager = PackManager::new(&temp, format!("http://{addr}/manifest.json"));
    manager
        .set_manifest_json(&manifest_json(addr, &wrong, body.len()))
        .unwrap();

    let err = manager
        .download("test-pack", |_| {})
        .await
        .expect_err("a checksum mismatch must fail");

    let message = err.to_string();
    assert!(
        message.contains("integrity check"),
        "operator-facing message should explain the failure: {message}"
    );

    assert!(!temp.join("packs/speech/test-pack/test-pack.bin").exists());
    assert!(!temp.join("packs/.downloads/test-pack.part").exists());

    std::fs::remove_dir_all(&temp).ok();
}
