//! Process-wide state, registered with Tauri's manager at setup and reachable
//! from any command via `tauri::State<AppState>`.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::audio::capture::CaptureHandle;
use crate::bible::youversion::YouVersionClient;
use crate::credentials::Credentials;
use crate::detection::llm::ParaphraseStage;
use crate::detection::pipeline::Pipeline;
use crate::llm::LlmConfig;
use crate::packs::PackManager;
use crate::stt::reconnect::ResilientStream;
use crate::stt::transcript::SessionTranscript;
use crate::windows::OutputAssignments;

/// Long-lived services shared by every command.
pub struct AppState {
    /// The single SQLite connection for the app data directory.
    ///
    /// One connection behind a mutex is deliberate: SQLite in WAL mode
    /// serializes writers anyway, and a service produces a modest write rate (a
    /// transcript segment every few hundred ms). If read contention shows up
    /// during a service, this is the place to introduce a pool.
    pub db: Mutex<Connection>,
    pub packs: PackManager,
    /// Which display each output window is on. The backend owns this because
    /// the operator UI unmounts when the operator switches tabs, and a
    /// remounted panel must be able to ask what is actually on screen rather
    /// than guess.
    pub outputs: Mutex<OutputAssignments>,
    /// Online scripture. Disabled when no credential is available; the cache
    /// still serves, so that costs new lookups rather than the service.
    pub bible: YouVersionClient,
    /// The one source of vendor credentials (PRD §17). Service clients ask
    /// this rather than the environment, so the gateway swaps in behind them.
    pub credentials: Credentials,
    /// The running capture, if any. Holding it here is what keeps it alive:
    /// dropping the handle stops the device and joins its thread, so replacing
    /// this releases the old input before the new one is opened.
    pub capture: Mutex<Option<CaptureHandle>>,
    /// The live transcription stream, when capture was started with
    /// transcription on. Owned rather than shared: stopping consumes it to
    /// send `CloseStream` and collect the final results.
    pub transcript: Mutex<Option<ResilientStream>>,
    /// The three detection stages and the queue between them and the cards
    /// (PRD §10.2 layer 3). Fed from the transcript event handler; drained
    /// there too, so a card is on its way to the window before the next
    /// utterance arrives.
    pub pipeline: Mutex<Pipeline>,
    /// The Claude stage, built when transcription starts and the Anthropic
    /// credential resolves; `None` offline or without a key (FR-14 is online
    /// only). A tokio mutex because the call inside is awaited.
    pub paraphrase: tokio::sync::Mutex<Option<ParaphraseStage>>,
    pub llm: LlmConfig,
    /// The translation a card's text is fetched in. KJV until the picker
    /// (M2 deliverable 7) gives the operator the choice.
    pub default_translation: Mutex<String>,
    /// Where the bundled assets are: translation packs, the verse index, the
    /// encoder. Resolved once at startup (see `lib.rs`), because the release
    /// build finds them under Tauri's resource directory and a dev build under
    /// the source tree, and nothing else should have to know which.
    pub assets_dir: PathBuf,
    /// Everything transcribed since capture started.
    ///
    /// Here rather than in the operator window because every tab unmounts when
    /// the operator switches away, and capture deliberately keeps running. A
    /// transcript held in React would come back empty mid-sermon, which reads
    /// as a crash and invites the one action that actually loses the recording.
    pub session_transcript: Mutex<SessionTranscript>,
    /// When `run()` began. The operator window reports back once it has
    /// painted, and the difference is the cold start PRD §9.1 budgets —
    /// measured to the thing itself rather than to the end of setup.
    pub started: std::time::Instant,
    /// The same moment on the wall clock, so a timestamp the operator window
    /// takes itself (`Date.now()` at first paint) can be placed on our
    /// timeline without passing through the IPC queue first.
    pub started_wall: std::time::SystemTime,
    /// The operator window reports ready once per process; StrictMode and a
    /// reload would otherwise log it twice.
    pub operator_ready_logged: std::sync::atomic::AtomicBool,
}

impl AppState {
    // Nine long-lived services, each built in `run()` and owned here; a
    // struct of the same nine fields would only move the list.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Connection,
        packs: PackManager,
        bible: YouVersionClient,
        credentials: Credentials,
        assets_dir: PathBuf,
        pipeline: Pipeline,
        llm: LlmConfig,
        started: std::time::Instant,
        started_wall: std::time::SystemTime,
    ) -> Self {
        Self {
            started,
            started_wall,
            operator_ready_logged: std::sync::atomic::AtomicBool::new(false),
            db: Mutex::new(db),
            assets_dir,
            pipeline: Mutex::new(pipeline),
            paraphrase: tokio::sync::Mutex::new(None),
            llm,
            default_translation: Mutex::new("KJV".to_string()),
            packs,
            outputs: Mutex::new(OutputAssignments::default()),
            bible,
            credentials,
            capture: Mutex::new(None),
            transcript: Mutex::new(None),
            session_transcript: Mutex::new(SessionTranscript::new()),
        }
    }
}
