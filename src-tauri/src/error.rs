//! One error type crossing the command boundary. Messages are operator-facing:
//! name what failed and offer one action, never "Something went wrong"
//! (Branding §9.3).

use serde::Serialize;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Audio device unavailable: {0}")]
    Audio(String),

    #[error("Transcription failed: {0}")]
    Stt(String),

    #[error("Scripture lookup failed: {0}")]
    Bible(String),

    #[error("Summary generation failed: {0}")]
    Summary(String),

    #[error("Pack download failed: {0}")]
    Pack(String),

    #[error("Display error: {0}")]
    Window(String),

    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("File error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Tauri(#[from] tauri::Error),
}

// Commands return errors to the frontend as a plain string.
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
