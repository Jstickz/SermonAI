# ADR 0005: Structured summary schema with a hard validator

**Decided:** 7 September 2026 · **Status:** Accepted, not yet built (M4) · **Full record:** `docs/adr/0005-summary-schema-and-validator.md`

The summary is generated as structured JSON against a fixed schema (PRD §26.3), never free prose, and every summary passes a validator before storage: every scripture reference must be in the accepted log or a regex hit in the transcript; every quote must be a substring of the transcript; dropped content is logged, not hidden.

Why: the PDF is why a church pays. An invented reference or a polished quote, handed to a small group under the pastor's name, is the highest-consequence failure in the product. A strict validator will sometimes drop a legitimate uncited paraphrase; a missing entry is recoverable in Content Studio, an invented one is not.

The same schema makes the offline template renderer (FR-45) possible: same shape, no model. The one Claude client in `llm/anthropic.rs` (built 24 September) is shared with this summariser; Sonnet is configured for summaries, Haiku for paraphrase.
