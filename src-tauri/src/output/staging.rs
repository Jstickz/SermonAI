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

/// What the projector shows beside the verse (PRD §13.6: "verse only, verse +
/// reference, verse + reference + translation"; FR-26 basic). Mirrors
/// `ProjectorLayout` in `src/lib/types.ts`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectorLayout {
    VerseOnly,
    #[default]
    VerseReference,
    /// Adds the translation's short name, small, in a corner — the
    /// operator's call of 16 Oct 2026; full attribution stays on the
    /// operator screen and in the PDF (§15.2).
    VerseReferenceTranslation,
}

impl ProjectorLayout {
    /// The `settings` value, and the string the frontend sends.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VerseOnly => "verse_only",
            Self::VerseReference => "verse_reference",
            Self::VerseReferenceTranslation => "verse_reference_translation",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "verse_only" => Some(Self::VerseOnly),
            "verse_reference" => Some(Self::VerseReference),
            "verse_reference_translation" => Some(Self::VerseReferenceTranslation),
            _ => None,
        }
    }
}

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
    pub layout: ProjectorLayout,
    /// Wall-clock ms when the live verse went live, so the projector can
    /// report its paint against it (the M3 DoD's staged → live budget).
    pub live_since_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct Staging {
    state: OutputState,
}

impl Staging {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start with the layout the operator last chose.
    pub fn with_layout(layout: ProjectorLayout) -> Self {
        Self {
            state: OutputState {
                layout,
                ..OutputState::default()
            },
        }
    }

    pub fn set_layout(&mut self, layout: ProjectorLayout) -> &OutputState {
        self.state.layout = layout;
        &self.state
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
    pub fn go_live(&mut self, now_ms: u64) -> Result<&OutputState> {
        let Some(verse) = self.state.staged.take() else {
            return Err(Error::Output(
                "Nothing is staged. Stage a verse first, then Go Live.".to_string(),
            ));
        };
        self.state.live = Some(verse);
        self.state.live_since_ms = Some(now_ms);
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

        let after = s.go_live(1_000).unwrap();
        assert_eq!(after.live.as_ref().unwrap().reference, "John 3:16");
        assert!(after.staged.is_none(), "promoted, not copied");
    }

    #[test]
    fn go_live_with_nothing_staged_is_an_error_and_changes_nothing() {
        let mut s = Staging::new();
        s.stage(verse("Romans 8:28"));
        s.go_live(1_000).unwrap();
        let before = s.state().clone();

        let err = s.go_live(2_000).unwrap_err().to_string();
        assert!(err.contains("Nothing is staged"), "{err}");
        assert_eq!(s.state(), &before, "the live verse is not disturbed");
    }

    #[test]
    fn staging_the_next_verse_leaves_the_live_one_alone() {
        let mut s = Staging::new();
        s.stage(verse("Psalm 23:1"));
        s.go_live(1_000).unwrap();
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
        s.go_live(1_000).unwrap();
        s.stage(verse("Isaiah 41:13"));
        s.clear_staged();
        assert!(s.state().staged.is_none());
        assert_eq!(s.state().live.as_ref().unwrap().reference, "Isaiah 41:10");
    }

    #[test]
    fn blanking_keeps_the_live_verse_and_go_live_unblanks() {
        let mut s = Staging::new();
        s.stage(verse("Micah 6:8"));
        s.go_live(1_000).unwrap();
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
        s.go_live(1_000).unwrap();
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

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn the_layout_round_trips_through_its_setting_string_and_json() {
        for layout in [
            ProjectorLayout::VerseOnly,
            ProjectorLayout::VerseReference,
            ProjectorLayout::VerseReferenceTranslation,
        ] {
            assert_eq!(ProjectorLayout::parse(layout.as_str()), Some(layout));
            let json = serde_json::to_string(&layout).unwrap();
            assert_eq!(
                json,
                format!("\"{}\"", layout.as_str()),
                "serde and the setting agree"
            );
        }
        assert_eq!(ProjectorLayout::parse("verse+ref"), None);
        assert_eq!(ProjectorLayout::default(), ProjectorLayout::VerseReference);
    }

    #[test]
    fn going_live_stamps_the_time_and_the_layout_survives_everything() {
        let mut s = Staging::with_layout(ProjectorLayout::VerseReferenceTranslation);
        s.stage(Verse {
            reference: "John 3:16".into(),
            translation: "KJV".into(),
            text: "For God so loved the world".into(),
        });
        assert_eq!(s.state().live_since_ms, None);
        s.go_live(1_760_000_000_000).unwrap();
        assert_eq!(s.state().live_since_ms, Some(1_760_000_000_000));
        s.set_blanked(true);
        s.clear_staged();
        assert_eq!(s.state().layout, ProjectorLayout::VerseReferenceTranslation);
        let json = serde_json::to_value(s.state()).unwrap();
        assert_eq!(json["layout"], "verse_reference_translation");
        assert_eq!(json["liveSinceMs"], 1_760_000_000_000u64);
    }
}
