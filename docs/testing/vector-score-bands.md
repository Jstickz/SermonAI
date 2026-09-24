# Vector score bands — what gates the Claude paraphrase call

Measured 24 September 2026 with `src-tauri/examples/score_bands.rs` against the
shipped KJV index (model2vec `potion-base-8M`, 256 dims int8). Top-1 cosine of
the vector stage for three populations.

## The populations

| population | n | top-1 min | median | max |
|---|---|---|---|---|
| **Verbatim KJV quotes** | 8 | **0.808** | 0.997 | 1.000 |
| **Modern paraphrases** (the recall test's eight) | 8 | 0.339 | 0.751 | 0.843 |
| **Preaching that quotes nothing** (Nova-3 transcript lines) | 15 | 0.313 | 0.589 | **0.696** |

Verbatim quotes and unquoted preaching **separate cleanly**: nothing unquoted
scored above 0.696, and every verbatim quote scored at least 0.808. Seven of
eight verbatim quotes came top-1; John 3:16 came second to 1 John 4:9 by 0.003,
which is the KJV's own wording overlapping.

Paraphrases and preaching **overlap**: paraphrases 0.64–0.76, preaching
0.56–0.70. A static bag-of-tokens model cannot tell "trust in the Lord with all
your heart" (0.738, missed) from "God is not surprised by your situation"
(0.584). That overlap is the band Claude exists for.

The top-1/top-2 **gap** is a second signal for the accept side: real hits had
gaps of 0.15–0.33; both paraphrase misses and preaching had gaps under 0.06.
It does not help on the low side — both "no clear verse" populations are flat —
so the gate uses top-1 alone and the gap is left for the confidence score
(FR-16, deliverable 4).

## The gate (`llm::ParaphraseGate`)

| score | decision | why |
|---|---|---|
| ≥ 0.80 | accept the vector match, no call | above every unquoted sentence, at or below every verbatim quote |
| 0.55 – 0.80 | **call Claude** (once per 10 s cooldown) | the overlap |
| < 0.55 | skip | plainly not scripture — 4 of 15 preaching lines land here |

**Cost of the floor.** The NIV-worded "I know the plans I have for you, plans
to prosper you" scores 0.339 against a KJV index that says "thoughts of peace",
and is skipped. That is the wireframe's own example verse. Lowering the floor to
catch it would call Claude on 14 of 15 unquoted sentences and defeat the gate.
This is a KJV-index problem as much as a threshold one: `build-verse-index.py`
records that a WEB index beat KJV on paraphrases and is meant to return once
the WEB pack is rebuilt.

**Cooldown.** Inside the band, roughly two in three unquoted utterances would
call. At ~40 settled utterances a minute that is unaffordable per utterance and
unnecessary: FR-14 sends the whole 60-second buffer, so one call reads
everything since the last. Ten seconds between calls bounds a 45-minute
sermon at 270 calls worst case; the counted figure from real services replaces
that estimate.

## What this does not establish

**n=8 and n=15.** Enough to see the shape, not to place the edges to two
decimals. Both edges and the cooldown are configuration, and the M2 Definition
of Done — 80% of 100 hand-labelled paraphrases in top-3 by vector or Claude —
is where they get set properly. Expect the floor to move.

**Nothing here says whether Claude's answers are good.** The live API refused
this project's key for want of a workspace header, so the stage is verified
against a local stand-in (`tests/paraphrase_stage.rs`): the request it sends,
the retry it performs, the references it drops. The judgement is measured when
a scoped key or the workspace ID is available.

## Reproduce

```powershell
cd src-tauri
cargo run --release --example score_bands
```
