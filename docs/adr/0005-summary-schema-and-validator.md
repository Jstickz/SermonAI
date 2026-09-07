# ADR 0005 — Structured summary schema with a hard validator

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §8.8 (FR-40 to FR-46), §10.5, §13.11, §26.3

## Context

The summary PDF is the reason a church pays for SermonAI rather than using a
free competitor. Its value collapses entirely if it is ever wrong about
scripture. A model that invents a plausible reference, or polishes a quote the
preacher did not say, produces a document a pastor might hand to a small group —
attributed to them. That is the highest-consequence failure in the product.

## Decision

The summary is generated as **structured JSON against a fixed schema**
(PRD §26.3), never as free prose, and every generated summary passes a validator
before it is stored or rendered:

- Every scripture reference must appear in the accepted scripture log or match a
  regex hit in the transcript. Anything else is dropped and logged.
- Every quote must be a substring of the transcript, allowing only light removal
  of filler words. Quotes are never rewritten.
- Dropped content is recorded so prompt quality can be measured, not hidden.

The same schema serves all four templates and Content Studio, so editing and
regeneration operate on data rather than on a rendered document.

## Consequences

**Good**
- Hallucination becomes a caught error rather than a shipped document.
- The offline template renderer produces the *same shape* with no model at all,
  which is what makes a fully offline summary PDF possible (FR-45).
- Versioning, editing and derivative generation are straightforward over JSON.
- Golden-file tests across templates become meaningful.

**Costs**
- A strict validator will sometimes drop a legitimate paraphrased reference the
  preacher made without a citation. Accepted: a missing entry is recoverable in
  Content Studio, an invented one is not recoverable at all.
- The schema is now a compatibility surface. Changes need a migration path for
  stored summaries.
