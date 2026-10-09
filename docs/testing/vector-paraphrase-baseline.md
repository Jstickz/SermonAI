# Vector stage paraphrase baseline — Phase 0 of the encoder upgrade

**Status: measured on the hand-labelled set, 9 October 2026.** First run 6
October on the agent's draft; the operator then reviewed every line against
the KJV text (`docs/testing/paraphrases-labelling.xlsx` is the record), kept
191 of 191 and added three alternative references. The re-run on the labelled
set gave the same figures to the case — the alternatives were not in the top
three either — so the table below stands as the baseline.

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
| paraphrase | 168 | 103 (61%) | **118 (70.2%)** | 122 (73%) | 0.339 / 0.711 / 1.000 | 44 / 104 / 20 |
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
   Three more looked that way — Psalm 27:1 → Isaiah 12:2 (0.833), Psalm 19:1
   → Psalm 97:6 (0.844), Acts 16:31 → Acts 15:11 (0.865) — and the operator
   reviewed each against the text and **kept the narrow label**: those are
   the stage's misses, not the set's. The operator added three genuine
   parallels (Matthew 16:26, Isaiah 61:1, Mark 10:43); none changed a rank.

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

## Phase 1 — synonym expansion and the neighbouring-verse search (9 October 2026)

Two separate changes to the query side, each behind a `SearchOptions` flag,
each measured with `measure_vector --synonyms blend|max` and `--neighbours`
on the labelled set above, same encoder, same index, same gate edges.

**Synonyms** (`detection/synonyms.rs`, our own table): contractions
expanded, then modern words and idioms replaced by the KJV's word for the
idea ("worry" → "careful", "wounds" → "stripes", "plans" → "thoughts"), then
"you/your" → "thee/thy". *Blend* embeds every variant, averages the vectors
and runs one search; *Max* searches each variant and keeps a verse's best
score. **Neighbours** averages each verse's vector with the next verse of its
chapter at load (about 7.7 MB in memory, nothing new shipped) and searches
those two-verse passages alongside the single verses.

| configuration | catch@1 | **catch@3** | catch@5 | wrong accepts | preaching false accepts | preaching top-1 median / max |
|---|---|---|---|---|---|---|
| baseline | 103 (61%) | **118 (70.2%)** | 122 | 3 | 0 | 0.589 / 0.696 |
| synonyms Blend | 115 (68%) | **133 (79.2%)** | 138 | 5 | 0 | 0.619 / 0.719 |
| synonyms Max | 118 (70%) | **133 (79.2%)** | 136 | 10 | 0 | — |
| neighbours | 106 (63%) | **119 (70.8%)** | 125 | 3 | 0 | — |
| Blend + neighbours | 116 (69%) | **131 (78.0%)** | 138 | 4 | 0 | — |
| Max + neighbours | 117 (70%) | **131 (78.0%)** | 137 | 11 | 0 | — |

**Kept: Blend.** +15 paraphrases at top-3 (70.2% → 79.2%) for one search
per query and up to two extra embeddings at 0.04 ms each. Preaching false
accepts stay at zero. The wireframe's example, "plans to prosper you" at
0.339 against "thoughts of peace", is now caught: "plans" → "thoughts" and
"prosper you" → "give you peace" put it in the KJV's bag of tokens.

**Not kept: Max.** Same catch rate, but wrong accepts go 3 → 10: taking the
best of three variants lets a rewrite that happens to match some other verse
cross the accept edge. Blend averages the variants, so one stray match is
diluted.

**Not kept: neighbours.** +1 line alone; combined with Blend it *costs* two
(79.2% → 78.0%), because a two-verse passage can outrank the single verse
the label names. The code stays behind its flag for Phase 3, where a
transformer encoder may change the picture; the pipeline does not use it.

**Costs, stated.** Wrong accepts on labelled lines rise 3 → 5: Romans 10:9
→ Matthew 26:63 (0.834) and Psalm 24:1 → Jeremiah 22:29 (0.822) now cross
0.80 with the wrong verse. And the preaching band is hotter — top-1 median
0.589 → 0.619, max 0.696 → 0.719, 12 of 15 lines in the call band against
11 — so the gate edges measured on 24 September are now measured against
Blend queries and should be re-read from the next real service. Neither
cost reaches the projector: a wrong accept is a card the operator sees, and
a hotter band is Claude calls, which the 10 s cooldown already caps.

**Still short of the DoD alone: 79.2% against 80%**, one line. Claude's
share on top of Blend is the next measurement, blocked on account credit.
