# ADR 0003 — Three-stage scripture detection

- **Status:** Accepted
- **Date:** 7 September 2026
- **PRD:** §8.3, §10.2, §15.4, §18.1

## Context

Preachers reference scripture three different ways: by exact citation ("John
3:16"), spoken out ("John chapter three verse sixteen"), and by paraphrase or
memory with no citation at all. One technique cannot cover all three within the
500 ms end-to-end budget. An LLM call on every transcript chunk would be too
slow and too expensive; regex alone misses most of what a preacher actually
does.

## Decision

Run three stages in order, cheapest first:

1. **Regex** over every chunk — standard, spoken and shorthand forms. Under 5 ms.
2. **Vector search** — the spoken phrase is embedded by a small on-device
   sentence encoder and matched by brute-force cosine against an int8-quantized
   index of all 31,102 verses. Under 5 ms, about 12 MB, shipped inside the
   binary.
3. **Claude paraphrase detection** over the rolling 60-second buffer, fired only
   after 10 seconds with no direct hit, online only. 1 to 2 seconds.

The index is built once by `scripts/build-verse-index.py` using OpenAI
`text-embedding-3-small`, reduced to 384 dimensions and quantized. That job
never runs in the shipped app.

## Consequences

**Good**
- The common case (a direct reference) resolves through the fastest path.
- Semantic detection works fully offline — no vector library, no service.
- 31K × 384 int8 vectors is small enough that brute force beats any index
  structure, and there is nothing to corrupt or rebuild.
- LLM cost stays at a few dollars per church per month.

**Costs**
- The bundled sentence encoder must stay under 25 MB and run without a GPU on
  both platforms — a real constraint on model choice (M0 deliverable).
- Quantization loses some recall versus float embeddings. Acceptable: the target
  is 80% of paraphrases surfaced in the top 3, and the operator sees a card, not
  an automatic projection (ADR 0002).
- Three sources of truth for confidence scoring means the score must be
  normalized across stages so the operator can compare them at a glance.
