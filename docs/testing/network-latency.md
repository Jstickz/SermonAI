# Where the lag actually goes

Measured 23 September 2026 from the development machine in Nigeria, against the
live Deepgram service, with `src-tauri/examples/region_probe.rs`.

## Method

The probe streams `sermonai-test-sermon.wav` — the same file the DoD run
plays — at **real-time pace**, 250 ms at a time, exactly as capture does. For
each interim result it records how long ago the chunk carrying that result's
last word was sent. That is the same quantity `SessionTranscript` measures, so
the two are directly comparable.

Two things this method gets right that a `ping` would not: it includes
Deepgram's own processing, and it reflects a long-lived WebSocket rather than a
fresh connection.

One thing to watch: the first version streamed a **synthetic tone** and reported
"no interim results came back" for both regions. Nova-3 returns nothing for
audio it does not hear as words, so that run measured nothing at all while
looking like a network failure. Real speech is not optional here.

## Round trip is the dominant term, and it is not ours

`api.deepgram.com` resolves to **`api.sac1.deepgram.com`** — Sacramento,
California — for everyone, wherever they are. Deepgram also runs
`api.eu.deepgram.com` (Frankfurt), which takes the **same API keys**; only the
base URL changes.

Ten minutes per region, chunk sent → result back:

| | n | p50 | p95 | p99 | max |
|---|---|---|---|---|---|
| Sacramento | 400 | 320 ms | 591 ms | 819 ms | 838 ms |
| **Frankfurt** | 414 | **208 ms** | **486 ms** | **715 ms** | 1,080 ms |

Frankfurt is about **110 ms cheaper at every percentile**. Its worst single
sample is larger, but its p99 is still 100 ms better.

TCP connect alone, 20 samples each: Sacramento p50 294 ms, Frankfurt p50
140 ms — consistent with the above, and confirming the gap is distance rather
than anything Deepgram does differently per region.

## It is flat, which rules out our pipeline

Per-minute median over the ten minutes:

| minute | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| Sacramento | 311 | 315 | 320 | 323 | 318 | 327 | 317 | 319 | 324 | 327 |
| Frankfurt | 204 | 209 | 207 | 209 | 208 | 216 | 207 | 208 | 208 | 210 |

**No trend.** Sacramento moves within 16 ms across ten minutes, Frankfurt
within 12 ms. Nothing is backing up, warming up or leaking.

This matters for the 60-minute DoD run, which reported a p50 of 1,490 ms for
the same quantity this measures at 320 ms. The network did not do that. The old
lag metric was accumulating dropped audio and sound-card drift into the figure;
see `m1-latency-baseline.md`.

## Interim cadence does not change on long continuous speech

Results per minute held steady — Sacramento 37–42, Frankfurt 36–45 — across ten
unbroken minutes. Deepgram returns interim results at the same rate late in a
long utterance as early in a short one, so nothing about the 250 ms chunk size
or the result cadence behaves differently at length. The short-clip measurement
was not flattering itself in that respect.

## What this leaves for the budget

Adding the 250 ms a chunk spends accumulating (fixed by FR-03), **spoken word →
words on screen**:

| | p50 | p95 | p99 |
|---|---|---|---|
| Sacramento | ~570 ms | ~841 ms | ~1,069 ms |
| **Frankfurt** | ~458 ms | ~736 ms | ~965 ms |

**The M1 DoD line's 700 ms p99 is not reachable from here at any region.** The
network alone spends 715 ms at p99 on the closest one, before the chunk is even
counted. That number predates the two-stage detection decision and is not
derived from any measurement; PRD §18.1's provisional budget of 900/1300 ms,
which was set with the network in mind, is met comfortably on Frankfurt and
just met on Sacramento.

## What would actually move it

| Lever | Worth | Ours? |
|---|---|---|
| EU region instead of the global default | ~110 ms at every percentile | **yes** — done |
| Smaller audio chunks (125 ms) | up to 125 ms, at 2x the message rate | ours, untested |
| Self-hosted Deepgram near the church | most of the remaining round trip | enterprise pricing |
| A different vendor with an African PoP | unknown | not investigated |

Nothing else on our side is worth more than a few milliseconds: regex detection
is budgeted at 5 ms and the staging render at 50.
