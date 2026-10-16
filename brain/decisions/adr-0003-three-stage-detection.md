# ADR 0003: Three-stage scripture detection

**Decided:** 7 September 2026 · **Status:** Accepted, with amendments the ADR text has not caught up with · **Full record:** `docs/adr/0003-three-stage-detection.md`

Run three stages in order, cheapest first: regex over every chunk (standard, spoken, shorthand forms), brute-force cosine over an int8 index of all 31,102 verses with a small on-device encoder, then Claude paraphrase detection over the rolling 60-second buffer, online only.

## Amendments since

- The index is built with model2vec `potion-base-8M` (256 dims, static embeddings, no ONNX, no GPU), not OpenAI `text-embedding-3-small` at 384 dims. The same model answers runtime queries, so Rust queries share the Python-built index's space. Decided 8 to 9 September; PRD §15.4 amended 24 September.
- The Claude stage is gated on the **vector stage's score** (call only in the 0.55 to 0.80 band, 10 s cooldown), not on "10 seconds with no direct hit". Decided 24 September (FR-14 amended); edges in `docs/testing/vector-score-bands.md`.
- "Regex" is a linear word walker that reads spoken numbers; it keeps the PRD's name for the stage.
- A modern-to-KJV synonym blend (`detection/synonyms.rs`) sits in front of the vector search since 9 October.

## Numbers of record (15 Oct)

Regex 426/426 on 459 cases. Vector alone 79.2% of 168 paraphrases at top-3 with the synonym blend. As gated with Claude 91.7%; ceiling if every line were asked 99.4%. 1.10 ms per vector query, 242 µs per regex window.

Related: [[two-stage-detection]], [[sessions/2026-09-24]], [[sessions/2026-10-09]], [[sessions/2026-10-15]]
