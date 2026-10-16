//! The staged/live pair (FR-50, PRD §13.5, M3 deliverable 1).
//!
//! Nothing reaches an output without passing through here: a verse is
//! **staged** first, and one action — Go Live, Enter, a remote tap —
//! **promotes** it. The two slots are the operator's safety margin: a card
//! that looked right at a glance gets a second look as text before the
//! congregation sees it, and a verse on a projector cannot be recalled.
//!
//! Pure state with no clock of its own, so a command can wrap it in a mutex
//! and the tests can drive it without a window. Every change returns the
//! whole [`OutputState`], which the commands broadcast as `output:changed`
//! to every window — the projector, the stage monitor and the operator's
//! own panel all render from the same struct.

use serde::{Deserialize, Serialize};

use crate::db::models::Verse;
use crate::error::{Error, Result};

/// What is staged and what is live. Mirrors `OutputState` in
/// `src/lib/types.ts`; the field names cross the IPC boundary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputState {
    pub live: Option<Verse>,
    pub staged: Option<Verse>,
    /// The projector shows nothing while blanked (FR-33); `live` is kept, so
    /// unblanking restores the verse without re-staging it.
    pub blanked: bool,
}

#[derive(Debug, Default)]
pub struct Staging {
    state: OutputState,
}

impl Staging {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &OutputState {
        &self.state
    }

    /// Put a verse in the staged slot, replacing whatever was there. Staging
    /// never touches the live slot: the congregation keeps reading the
    /// current verse while the operator lines up the next.
    pub fn stage(&mut self, verse: Verse) -> &OutputState {
        self.state.staged = Some(verse);
        &self.state
    }

    /// Promote staged to live. The staged slot empties, so a second Enter
    /// cannot send the same verse twice. Unblanks: Go Live is an explicit
    /// "show this", and a projector that stayed dark after it would read as
    /// a fault.
    pub fn go_live(&mut self) -> Result<&OutputState> {
        let Some(verse) = self.state.staged.take() else {
            return Err(Error::Output(
                "Nothing is staged. Stage a verse first, then Go Live.".to_string(),
            ));
        };
        self.state.live = Some(verse);
        self.state.blanked = false;
        Ok(&self.state)
    }

    /// Escape: empty the staged slot. Live is untouched.
    pub fn clear_staged(&mut self) -> &OutputState {
        self.state.staged = None;
        &self.state
    }

    /// Blank or unblank the projector (FR-33). State is kept either way.
    pub fn set_blanked(&mut self, blanked: bool) -> &OutputState {
        self.state.blanked = blanked;
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verse(reference: &str) -> Verse {
        Verse {
            reference: reference.to_string(),
            translation: "KJV".to_string(),
            text: format!("text of {reference}"),
        }
    }

    #[test]
    fn a_verse_is_staged_then_promoted_and_the_staged_slot_empties() {
        let mut s = Staging::new();
        assert_eq!(s.state(), &OutputState::default());

        s.stage(verse("John 3:16"));
        assert_eq!(s.state().staged.as_ref().unwrap().reference, "John 3:16");
        assert!(
            s.state().live.is_none(),
            "staging shows nothing to the congregation"
        );

        let after = s.go_live().unwrap();
        assert_eq!(after.live.as_ref().unwrap().reference, "John 3:16");
        assert!(after.staged.is_none(), "promoted, not copied");
    }

    #[test]
    fn go_live_with_nothing_staged_is_an_error_and_changes_nothing() {
        let mut s = Staging::new();
        s.stage(verse("Romans 8:28"));
        s.go_live().unwrap();
        let before = s.state().clone();

        let err = s.go_live().unwrap_err().to_string();
        assert!(err.contains("Nothing is staged"), "{err}");
        assert_eq!(s.state(), &before, "the live verse is not disturbed");
    }

    #[test]
    fn staging_the_next_verse_leaves_the_live_one_alone() {
        let mut s = Staging::new();
        s.stage(verse("Psalm 23:1"));
        s.go_live().unwrap();
        s.stage(verse("Psalm 23:2"));
        assert_eq!(s.state().live.as_ref().unwrap().reference, "Psalm 23:1");
        assert_eq!(s.state().staged.as_ref().unwrap().reference, "Psalm 23:2");
        s.stage(verse("Psalm 23:3"));
        assert_eq!(
            s.state().staged.as_ref().unwrap().reference,
            "Psalm 23:3",
            "replaced"
        );
    }

    #[test]
    fn escape_clears_staged_only() {
        let mut s = Staging::new();
        s.stage(verse("Isaiah 41:10"));
        s.go_live().unwrap();
        s.stage(verse("Isaiah 41:13"));
        s.clear_staged();
        assert!(s.state().staged.is_none());
        assert_eq!(s.state().live.as_ref().unwrap().reference, "Isaiah 41:10");
    }

    #[test]
    fn blanking_keeps_the_live_verse_and_go_live_unblanks() {
        let mut s = Staging::new();
        s.stage(verse("Micah 6:8"));
        s.go_live().unwrap();
        s.set_blanked(true);
        assert!(s.state().blanked);
        assert_eq!(
            s.state().live.as_ref().unwrap().reference,
            "Micah 6:8",
            "no lost state"
        );
        s.set_blanked(false);
        assert!(!s.state().blanked);

        s.set_blanked(true);
        s.stage(verse("Micah 6:9"));
        s.go_live().unwrap();
        assert!(!s.state().blanked, "Go Live means show it");
    }

    #[test]
    fn the_json_matches_what_the_windows_read() {
        // Field names cross the IPC boundary; the last mismatch there read
        // "Retrying in NaNs".
        let mut s = Staging::new();
        s.stage(verse("John 3:16"));
        let json = serde_json::to_value(s.state()).unwrap();
        assert_eq!(json["live"], serde_json::Value::Null);
        assert_eq!(json["staged"]["reference"], "John 3:16");
        assert_eq!(json["staged"]["translation"], "KJV");
        assert_eq!(json["blanked"], false);
    }
}
