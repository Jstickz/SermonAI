//! The staged/live pair (FR-50, PRD §13.5, M3 deliverable 1).
//!
//! Nothing reaches an output without passing through here: a verse is
//! **staged** first, and one action — Go Live, Enter, a remote tap —
//! **promotes** it. The two slots are the operator's safety margin: a card
//! that looked right at a glance gets a second look as text before the
//! congregation sees it, and a verse on a projector cannot be recalled.
//!
//! **Auto-live** (FR-50, PRD §13.5, M3 deliverable 2) is the one exception,
//! and it is opt-in: with it on, a confirmed card whose confidence is above
//! [`AUTO_LIVE_MIN_CONFIDENCE`] skips the staged slot and goes straight to
//! live. [`auto_live_qualifies`] is the whole rule; the card emitter asks it.
//!
//! Pure state with no clock of its own, so a command can wrap it in a mutex
//! and the tests can drive it without a window. Every change returns the
//! whole [`OutputState`], which the commands broadcast as `output:changed`
//! to every window — the projector, the stage monitor and the operator's
//! own panel all render from the same struct.

use serde::{Deserialize, Serialize};

use crate::db::models::Verse;
use crate::detection::pipeline::Candidate;
use crate::error::{Error, Result};

/// Auto-live's line: a card must be **strictly above** this to skip staging
/// (wireframe §settings: "when confidence is above 90%"). The Claude stage
/// caps its confidence *at* 0.90 for exactly this reason (`pipeline.rs`):
/// a paraphrase always passes an operator.
pub const AUTO_LIVE_MIN_CONFIDENCE: f32 = 0.90;

/// Whether auto-live may send this card to the projector without the
/// operator. Three conditions, each its own reason:
///
/// - **confirmed**: a provisional card is never projectable (§18.1), by any
///   path, and auto-live is a path.
/// - **above the line**: at or below 0.90 the operator decides.
/// - **has text**: a reference with no cached words would put an empty
///   screen in front of the congregation; that card waits as a card.
pub fn auto_live_qualifies(card: &Candidate) -> bool {
    !card.provisional
        && card.confidence > AUTO_LIVE_MIN_CONFIDENCE
        && card
            .verse
            .as_ref()
            .is_some_and(|v| !v.text.trim().is_empty())
}

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

/// How one verse gives way to the next on the projector (FR-25, PRD §13.6,
/// Branding §7.1: the one place real animation budget is spent). Mirrors
/// `ProjectorTransition` in `src/lib/types.ts`.
///
/// - **fade**: the old verse fades out, then the new one fades in.
/// - **slide**: the old verse lifts away, the new one rises into place.
/// - **dissolve**: the two cross-fade at once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectorTransition {
    #[default]
    Fade,
    Slide,
    Dissolve,
}

impl ProjectorTransition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Slide => "slide",
            Self::Dissolve => "dissolve",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "fade" => Some(Self::Fade),
            "slide" => Some(Self::Slide),
            "dissolve" => Some(Self::Dissolve),
            _ => None,
        }
    }
}

/// The longest transition the setting accepts. Branding §7.1 keeps every
/// animation under 300 ms except this one; a verse that takes longer than a
/// second and a half to arrive reads as a fault, not a flourish. Zero is
/// allowed and means instant.
pub const MAX_TRANSITION_MS: u32 = 1_500;

/// What is staged and what is live. Mirrors `OutputState` in
/// `src/lib/types.ts`; the field names cross the IPC boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputState {
    pub live: Option<Verse>,
    pub staged: Option<Verse>,
    /// The projector shows nothing while blanked (FR-33); `live` is kept, so
    /// unblanking restores the verse without re-staging it.
    pub blanked: bool,
    pub layout: ProjectorLayout,
    /// Auto-live is on (§13.5). Carried here so every window can show the
    /// indicator from the one event it already listens to.
    pub auto_live: bool,
    /// How verses change on the projector (FR-25), and how long it takes.
    pub transition: ProjectorTransition,
    pub transition_ms: u32,
    /// Wall-clock ms when the live verse went live, so the projector can
    /// report its paint against it (the M3 DoD's staged → live budget).
    pub live_since_ms: Option<u64>,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            live: None,
            staged: None,
            blanked: false,
            layout: ProjectorLayout::default(),
            auto_live: false,
            transition: ProjectorTransition::default(),
            transition_ms: crate::output::VERSE_TRANSITION_MS,
            live_since_ms: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct Staging {
    state: OutputState,
}

impl Staging {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start with what the operator last chose: the layout, and whether
    /// auto-live was on. Auto-live survives a restart deliberately — a church
    /// that runs hands-off should not find the setting silently off on
    /// Sunday — and the indicator makes the state visible either way.
    pub fn with_settings(
        layout: ProjectorLayout,
        auto_live: bool,
        transition: ProjectorTransition,
        transition_ms: u32,
    ) -> Self {
        Self {
            state: OutputState {
                layout,
                auto_live,
                transition,
                transition_ms: transition_ms.min(MAX_TRANSITION_MS),
                ..OutputState::default()
            },
        }
    }

    /// Choose the transition and its length (FR-25). The length is checked
    /// here, not in the command, so no path can store one the projector
    /// would make a spectacle of.
    pub fn set_transition(
        &mut self,
        transition: ProjectorTransition,
        transition_ms: u32,
    ) -> Result<&OutputState> {
        if transition_ms > MAX_TRANSITION_MS {
            return Err(Error::Output(format!(
                "{transition_ms} ms is too long for a transition. Choose up to {MAX_TRANSITION_MS} ms."
            )));
        }
        self.state.transition = transition;
        self.state.transition_ms = transition_ms;
        Ok(&self.state)
    }

    pub fn set_layout(&mut self, layout: ProjectorLayout) -> &OutputState {
        self.state.layout = layout;
        &self.state
    }

    pub fn set_auto_live(&mut self, enabled: bool) -> &OutputState {
        self.state.auto_live = enabled;
        &self.state
    }

    pub fn auto_live(&self) -> bool {
        self.state.auto_live
    }

    /// Auto-live: put a verse straight into the live slot. The staged slot is
    /// left alone — whatever the operator lined up is still theirs to send —
    /// and so is blank: blanking is an explicit act, and a verse arriving on
    /// its own must not undo it. The verse is live underneath, so unblank
    /// shows it. Refused while auto-live is off, so a caller cannot bypass
    /// staging by naming the wrong method.
    pub fn auto_live_verse(&mut self, verse: Verse, now_ms: u64) -> Result<&OutputState> {
        if !self.state.auto_live {
            return Err(Error::Output(
                "Auto-live is off. Stage the verse and press Go Live.".to_string(),
            ));
        }
        self.state.live = Some(verse);
        self.state.live_since_ms = Some(now_ms);
        Ok(&self.state)
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

    /// Next or previous verse (FR-51): the live verse is replaced directly.
    /// The operator is stepping through a passage they have already chosen
    /// to show, and a staged slot between every step would be the thing in
    /// the way. Unblanks, like Go Live: stepping is an explicit "show this".
    /// The staged slot is left alone.
    pub fn step_live(&mut self, verse: Verse, now_ms: u64) -> &OutputState {
        self.state.live = Some(verse);
        self.state.live_since_ms = Some(now_ms);
        self.state.blanked = false;
        &self.state
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
        assert_eq!(json["autoLive"], false);
    }
}

#[cfg(test)]
mod auto_live_tests {
    use super::*;
    use crate::db::models::DetectionSource;

    fn verse(reference: &str) -> Verse {
        Verse {
            reference: reference.to_string(),
            translation: "KJV".to_string(),
            text: format!("text of {reference}"),
        }
    }

    fn card(confidence: f32, provisional: bool, verse: Option<Verse>) -> Candidate {
        Candidate {
            id: 1,
            passage_id: "JHN.3.16".into(),
            reference: "John 3:16".into(),
            source: DetectionSource::Regex,
            confidence,
            provisional,
            evidence: None,
            detected_at_ms: 0,
            verse,
            went_live: false,
            pinned: false,
        }
    }

    #[test]
    fn only_a_confirmed_card_with_text_above_the_line_qualifies() {
        let v = Some(verse("John 3:16"));
        assert!(auto_live_qualifies(&card(0.97, false, v.clone())));
        assert!(
            !auto_live_qualifies(&card(0.90, false, v.clone())),
            "at the line is not above it: Claude's cap must keep a paraphrase out"
        );
        assert!(
            !auto_live_qualifies(&card(0.97, true, v.clone())),
            "provisional is never projectable, by any path"
        );
        assert!(
            !auto_live_qualifies(&card(0.97, false, None)),
            "no text means an empty screen"
        );
        let blank = Verse {
            text: "  ".into(),
            ..verse("John 3:16")
        };
        assert!(!auto_live_qualifies(&card(0.97, false, Some(blank))));
    }

    #[test]
    fn auto_live_goes_straight_to_live_and_leaves_staged_alone() {
        let mut s = Staging::with_settings(
            ProjectorLayout::default(),
            true,
            ProjectorTransition::Fade,
            400,
        );
        s.stage(verse("Psalm 23:1"));
        let after = s.auto_live_verse(verse("John 3:16"), 5_000).unwrap();
        assert_eq!(after.live.as_ref().unwrap().reference, "John 3:16");
        assert_eq!(after.live_since_ms, Some(5_000));
        assert_eq!(
            after.staged.as_ref().unwrap().reference,
            "Psalm 23:1",
            "the operator's staged verse is still theirs to send"
        );
    }

    #[test]
    fn auto_live_respects_a_blank_and_is_refused_while_off() {
        let mut s = Staging::with_settings(
            ProjectorLayout::default(),
            true,
            ProjectorTransition::Fade,
            400,
        );
        s.set_blanked(true);
        s.auto_live_verse(verse("John 3:16"), 1).unwrap();
        assert!(s.state().blanked, "blanking is the operator's act");
        s.set_blanked(false);
        assert_eq!(s.state().live.as_ref().unwrap().reference, "John 3:16");

        s.set_auto_live(false);
        let err = s
            .auto_live_verse(verse("John 3:17"), 2)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Auto-live is off"), "{err}");
        assert_eq!(s.state().live.as_ref().unwrap().reference, "John 3:16");
        assert_eq!(serde_json::to_value(s.state()).unwrap()["autoLive"], false);
    }
}

#[cfg(test)]
mod step_tests {
    use super::*;

    fn verse(reference: &str) -> Verse {
        Verse {
            reference: reference.to_string(),
            translation: "KJV".to_string(),
            text: format!("text of {reference}"),
        }
    }

    #[test]
    fn stepping_replaces_live_unblanks_and_leaves_staged_alone() {
        let mut s = Staging::new();
        s.stage(verse("John 3:16"));
        s.go_live(1_000).unwrap();
        s.stage(verse("Romans 8:28"));
        s.set_blanked(true);

        let after = s.step_live(verse("John 3:17"), 2_000);
        assert_eq!(after.live.as_ref().unwrap().reference, "John 3:17");
        assert_eq!(
            after.live_since_ms,
            Some(2_000),
            "a step is a go-live for the paint report"
        );
        assert!(!after.blanked, "stepping means show it");
        assert_eq!(after.staged.as_ref().unwrap().reference, "Romans 8:28");
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
        let mut s = Staging::with_settings(
            ProjectorLayout::VerseReferenceTranslation,
            false,
            ProjectorTransition::Fade,
            400,
        );
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

#[cfg(test)]
mod transition_tests {
    use super::*;

    #[test]
    fn the_transition_round_trips_and_defaults_to_a_400ms_fade() {
        for t in [
            ProjectorTransition::Fade,
            ProjectorTransition::Slide,
            ProjectorTransition::Dissolve,
        ] {
            assert_eq!(ProjectorTransition::parse(t.as_str()), Some(t));
            assert_eq!(
                serde_json::to_string(&t).unwrap(),
                format!("\"{}\"", t.as_str())
            );
        }
        assert_eq!(ProjectorTransition::parse("wipe"), None);
        let state = OutputState::default();
        assert_eq!(state.transition, ProjectorTransition::Fade);
        assert_eq!(state.transition_ms, 400, "FR-25's default");
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["transition"], "fade");
        assert_eq!(json["transitionMs"], 400);
    }

    #[test]
    fn the_length_is_bounded_and_zero_is_instant() {
        let mut s = Staging::new();
        s.set_transition(ProjectorTransition::Slide, 0).unwrap();
        assert_eq!(s.state().transition_ms, 0);
        s.set_transition(ProjectorTransition::Dissolve, MAX_TRANSITION_MS)
            .unwrap();
        let err = s
            .set_transition(ProjectorTransition::Fade, MAX_TRANSITION_MS + 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("too long"), "{err}");
        assert_eq!(
            s.state().transition,
            ProjectorTransition::Dissolve,
            "a refused change changes nothing"
        );
        let s = Staging::with_settings(
            ProjectorLayout::default(),
            false,
            ProjectorTransition::Fade,
            9_000,
        );
        assert_eq!(
            s.state().transition_ms,
            MAX_TRANSITION_MS,
            "a stored value past the cap is clamped on load, not refused"
        );
    }
}
