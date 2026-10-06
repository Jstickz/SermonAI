# Vector stage paraphrase baseline — Phase 0 of the encoder upgrade

**Status: provisional.** Measured 6 October 2026 on a **draft** set that is not
yet hand-labelled (`src-tauri/tests/fixtures/paraphrases.tsv`, header says
so). The numbers move when the operator strikes lines; the method does not.
Re-run and replace this table once the set is labelled.

**Method.** `cargo run --release --example measure_vector` loads the shipped
encoder (model2vec `potion-base-8M`, 256 dims int8) and KJV index (31,102
verses), embeds each phrase, takes the top five by cosine, and records the
rank of any labelled verse. Gate edges are `llm::ParaphraseGate::default()`
(accept ≥ 0.80, call ≥ 0.55). A phrase may carry several right answers,
separated by `|`, for verses scripture quotes from itself.

## Result

| population | n | catch@1 | catch@3 | catch@5 | top-1 min / median / max | accept / call / skip |
|---|---|---|---|---|---|---|
| verbatim KJV | 8 | 7 (88%) | **8 (100%)** | 8 | 0.808 / 0.994 / 1.000 | 8 / 0 / 0 |
| paraphrase (draft) | 168 | 103 (61%) | **118 (70.2%)** | 122 (73%) | 0.339 / 0.711 / 1.000 | 44 / 104 / 20 |
| preaching, quotes nothing | 15 | — | — | — | 0.313 / 0.589 / 0.696 | **0** / 11 / 4 |

**M2 DoD line, vector stage alone: 118/168 = 70.2% at top-3 against 80%.
Not met without the Claude stage.** The 24 September figure on eight cases was
4/8; the shape has not changed, only the sample.

**False accepts on preaching: 0 of 15**, consistent with the 24 September
bands (nothing unquoted above 0.696).

## What the misses look like

Fifty paraphrases are absent from the top three. Reading them, three kinds:

1. **Vocabulary the KJV does not use.** "plans to prosper you" (0.339 against
   "thoughts of peace"), "no weapon formed against you will succeed" (0.424),
   "guard your heart" (0.397), "God looks at the heart" (0.475). A static
   token model has no path from the modern word to the archaic one. This is
   the class a transformer encoder or Claude exists for.
2. **Right idea, neighbouring verse.** "I am the bread of life" → John 6:51
   (rank 5, wanted 6:35); "every knee will bow" → Philippians 2:11 (wanted
   2:10); "he was pierced" → Isaiah 59:12 (rank 4, wanted 53:5). The stage is
   in the right chapter; a confidence that looked at top-1/top-2 gap or a
   passage-level index would recover these cheaply.
3. **Scripture quoting scripture.** "the righteous will live by faith" was a
   wrong accept until the label admitted Romans 1:17 and Galatians 3:11.
   Three more are labelled too narrowly and should be reviewed, not fixed in
   code: Psalm 27:1 → Isaiah 12:2 (0.833), Psalm 19:1 → Psalm 97:6 (0.844),
   Acts 16:31 → Acts 15:11 (0.865).

## What the gate numbers say about cost

104 of 168 paraphrases (62%) land in the call band, as do 11 of 15 preaching
lines (73%). On the real run of 4 October the stage made 58 requests in 13
minutes — the cooldown's ceiling — so **the band is hot enough that the
cooldown, not the edges, sets the call rate**: about 280 calls/hour, roughly
$0.30/hour on Haiku 4.5 at the recorded prices. Raising the floor would cut
calls but the preaching lines sit at the same scores as the paraphrases
(medians 0.589 vs 0.711, ranges overlapping), so there is no edge that drops
one without the other. That overlap is the measured reason the encoder
upgrade is being considered.

## Not measured here

- **Claude's share of the 80%.** Needs the score run with a funded key.
- **Held-out real sermon phrasings.** None supplied yet; the draft set is the
  agent's wording, which is cleaner than a preacher's.
- **The Isaiah 10:24–26 recording** ("Zion" → "Ireland"): no recording of
  that run was kept.
