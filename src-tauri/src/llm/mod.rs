//! The one Claude client, shared by two layers (PRD §11.3, §15.3).
//!
//! Scripture detection (layer 3) asks Claude whether a paraphrase is a verse;
//! sermon intelligence (layer 7) asks it for a summary. Same endpoint, same
//! credential, same retry and error handling — so one client rather than one
//! per layer, justified the way `credentials/` is: by the external service it
//! fronts, not by a layer of the architecture. PRD §11.3 records that.
//!
//! Nothing in here reads a key. The client is built from [`Credentials`], so
//! when the gateway lands (key strategy Phase 3) managed mode slots in behind
//! it and neither caller changes.

pub mod anthropic;

/// Which model does which job. Configuration, not constants: these are read
/// once at startup and will come from settings when M8 gives settings a home.
///
/// Two models on purpose (PRD §15.3, amended 24 Sept 2026). Paraphrase
/// detection is a short prompt fired many times a service with a 1.5 s p95
/// budget, where latency and per-call cost dominate; a summary is one long
/// structured call per service where quality dominates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    pub paraphrase: String,
    pub summary: String,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            paraphrase: "claude-haiku-4-5-20251001".to_string(),
            summary: "claude-sonnet-5".to_string(),
        }
    }
}

/// When the paraphrase stage is worth a call (FR-14, amended 24 Sept 2026).
///
/// Not a timer. The vector stage runs first on every settled utterance and
/// its top score says how sure it is; Claude is only asked in the band where
/// that score is neither convincing nor dismissible. The edges are measured,
/// not round — see `docs/testing/vector-score-bands.md`:
///
/// | population | top-1 min | median | max |
/// |---|---|---|---|
/// | verbatim KJV | 0.808 | 0.997 | 1.000 |
/// | paraphrase | 0.339 | 0.751 | 0.843 |
/// | preaching, no scripture | 0.313 | 0.589 | 0.696 |
///
/// Above `accept_at_or_above` every verbatim quote lands and nothing said
/// unquoted does. Below it paraphrases and plain speech overlap, which is what
/// Claude is for. `call_at_or_above` is the trade: at 0.55 it spares roughly a
/// quarter of unquoted speech from a call but also drops the NIV-worded
/// "plans to prosper you" (0.339 against a KJV index). Both were set on eight
/// paraphrases and fifteen sentences and are **validated against the
/// 100-paraphrase DoD set**, which is why they are configuration.
///
/// `cooldown_secs` bounds cost inside the band. FR-14 sends the 60-second
/// buffer, so one call already covers many utterances; asking again inside
/// the cooldown would mostly re-read the same words.
#[derive(Debug, Clone, PartialEq)]
pub struct ParaphraseGate {
    pub accept_at_or_above: f32,
    pub call_at_or_above: f32,
    pub cooldown_secs: u64,
}

impl Default for ParaphraseGate {
    fn default() -> Self {
        Self {
            accept_at_or_above: 0.80,
            call_at_or_above: 0.55,
            cooldown_secs: 10,
        }
    }
}

/// Everything the LLM layer is configured with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LlmConfig {
    pub models: Models,
    pub gate: ParaphraseGate,
    /// Required by keys that are not scoped to a workspace: the API refuses
    /// them without an `anthropic-workspace-id` header. `None` for a scoped
    /// key. Comes from `ANTHROPIC_WORKSPACE_ID` in a development `.env`; a
    /// release build has no way to set it until M8's settings exist, which is
    /// recorded in Parked.
    pub workspace_id: Option<String>,
}

impl LlmConfig {
    /// Defaults, with development-only overrides from the environment.
    ///
    /// The overrides exist so a model or a threshold can be tried without a
    /// rebuild. They are compiled out of release builds for the same reason
    /// `.env` loading is: a church's machine must not have its behaviour
    /// changed by a stray environment variable.
    pub fn load() -> Self {
        let mut config = Self::default();

        #[cfg(debug_assertions)]
        {
            let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
            if let Some(m) = var("SERMONAI_LLM_PARAPHRASE_MODEL") {
                config.models.paraphrase = m;
            }
            if let Some(m) = var("SERMONAI_LLM_SUMMARY_MODEL") {
                config.models.summary = m;
            }
            if let Some(v) = var("SERMONAI_GATE_ACCEPT").and_then(|v| v.parse().ok()) {
                config.gate.accept_at_or_above = v;
            }
            if let Some(v) = var("SERMONAI_GATE_CALL").and_then(|v| v.parse().ok()) {
                config.gate.call_at_or_above = v;
            }
            config.workspace_id = var("ANTHROPIC_WORKSPACE_ID");
        }

        config
    }
}

/// Per-million-token prices, US dollars, from <https://claude.com/pricing> as
/// read on 24 September 2026. For estimating cost per service from measured
/// token counts — never for billing, which is the vendor's.
///
/// Recorded with the date because they change, and a stale figure that looks
/// authoritative is worse than none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cached_read_per_mtok: f64,
}

impl Pricing {
    pub const HAIKU_4_5: Pricing = Pricing {
        input_per_mtok: 1.00,
        output_per_mtok: 5.00,
        cached_read_per_mtok: 0.10,
    };
    pub const SONNET_5: Pricing = Pricing {
        input_per_mtok: 2.00,
        output_per_mtok: 10.00,
        cached_read_per_mtok: 0.20,
    };

    /// Prices for a model ID, by family prefix. Unknown IDs get Sonnet's
    /// figures, so an estimate errs high rather than low.
    pub fn for_model(model: &str) -> Pricing {
        if model.starts_with("claude-haiku") {
            Pricing::HAIKU_4_5
        } else {
            Pricing::SONNET_5
        }
    }

    /// Dollars for a call, given its token counts.
    pub fn cost(&self, input_tokens: u64, cached_read_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 * self.input_per_mtok
            + cached_read_tokens as f64 * self.cached_read_per_mtok
            + output_tokens as f64 * self.output_per_mtok)
            / 1_000_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_has_a_middle_band() {
        // A gate whose edges cross has no band, and the stage would either
        // always call or never call. Cheap to check, expensive to discover.
        let g = ParaphraseGate::default();
        assert!(g.call_at_or_above < g.accept_at_or_above);
        // And the measured populations fall where the doc says: every
        // verbatim quote above accept, every unquoted sentence below it.
        assert!(0.808 >= g.accept_at_or_above, "verbatim min must accept");
        assert!(
            0.696 < g.accept_at_or_above,
            "preaching max must not accept"
        );
    }

    #[test]
    fn a_call_costs_what_the_page_says() {
        // 600 input, 100 output on Haiku: $0.0006 + $0.0005.
        let cost = Pricing::HAIKU_4_5.cost(600, 0, 100);
        assert!((cost - 0.0011).abs() < 1e-9, "{cost}");
        // Unknown model estimates as Sonnet, the dearer of the two in use.
        assert_eq!(Pricing::for_model("claude-future-9"), Pricing::SONNET_5);
        assert_eq!(
            Pricing::for_model("claude-haiku-4-5-20251001"),
            Pricing::HAIKU_4_5
        );
    }
}
