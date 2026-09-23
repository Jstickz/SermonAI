# M1 latency and memory baseline

Recorded 23 September 2026, after a release-build DoD run reported **2,797 ms
p99 lag** against a 700 ms budget and **40 MB** peak memory.

Both figures were wrong, in opposite directions, and for different reasons.

---

## The memory figure was a fourfold undercount

`measure-memory.ps1` resolved processes through performance-counter *instance
names*. Several processes are all called `msedgewebview2`, so the name is
ambiguous and the lookup returned one of them.

Measured against a live app:

| Method | Processes resolved | Reported |
|---|---|---|
| Perf-counter by instance name | **1 of 9** | 6.3 MB |
| `Win32_PerfRawData` keyed by PID | **9 of 9** | **193.4 MB** |

The reported 40 MB was `sermonai.exe` on its own. It looked plausible because
the Rust process genuinely is small — 6.3 MB private here — and WebView2 carries
almost all of the app's memory.

**Fixed.** Processes are matched by PID, which cannot be confused by duplicate
names, and every sample prints how many it resolved so an undercount is visible
rather than quietly halving the answer. A run that resolves one process now says
the run is invalid.

**Re-measure needed.** A dev build idles at ~200 MB against a 300 MB budget.
That is much less headroom than 40 MB suggested, and the release figure under
capture is not yet known.

---

## The lag figure measured the wrong thing, and measured it from the wrong place

### It was finals only

Lag was recorded in `push_final` and nowhere else. So 2,797 ms described when
Deepgram **confirms** an utterance — which it does only after deciding the
speaker has stopped.

The DoD line says *"transcript appears with under 700 ms lag"*. Words **appear**
as interim results, rendered immediately. The measurement answered a different
question from the one the line asks.

### It also charged the handshake to every sample

The clock started when *capture* began. Deepgram's word timestamps are relative
to the first audio byte **it** received, so every sample carried the WebSocket
handshake — measured at **1,188 to 1,365 ms** against the live service.

Both are fixed: interim and settled are measured separately, and the clock is
anchored to the first chunk actually sent.

### Measured breakdown

38 s of speech, streamed in real time, against the live service. `ours` is the
250 ms chunk the audio spends accumulating before it can be sent; `theirs` is
the socket write to the result coming back.

| | p50 | p95 | p99 |
|---|---|---|---|
| **interim — total** | ~452 ms | ~681 ms | **~698 ms** |
| interim — ours (chunking) | 250 ms | 250 ms | 250 ms |
| interim — theirs (net + Deepgram) | 279 ms | 540 ms | 574 ms |
| **settled — total** | ~811 ms | ~2,289 ms | **~2,289 ms** |
| settled — theirs | 768 ms | 2,265 ms | 2,265 ms |

Against PRD §18.1, which budgets the stages separately:

| Stage | Budget p95 / p99 | Measured | |
|---|---|---|---|
| Audio chunk | 250 / 300 ms | 250 ms by design | pass |
| STT word return (online) | 400 / 700 ms | 540 / 574 ms | p99 pass, **p95 over by 140 ms** |
| End-to-end regex path | 500 / 800 ms | ~681 / ~698 ms | pass |

So **words appear inside the 700 ms the DoD asks for**, marginally, and the
2,797 ms was the two measurement faults above compounding.

### Two caveats on these numbers

They come from **synthesised speech**, one machine, one network, 38 seconds,
n=27 interim and n=10 settled. A real 60-minute service on a church's connection
is the number that matters, and the app now records both figures itself.

Deepgram's timestamps are also taken on trust. If its clock and ours disagree,
this measures the disagreement as well.

---

## Endpointing is not the lever for settled text

Tested against the live service at three settings:

| `endpointing` | settled p95 | settled p99 |
|---|---|---|
| default | 2,039 ms | 2,039 ms |
| 100 ms | 2,083 ms | 2,083 ms |
| 300 ms | 1,664 ms | 1,664 ms |

The spread is within run-to-run variance on ten samples. Deepgram decides when
an utterance is settled and this parameter does not meaningfully move it, so
**~2 s p95 should be treated as the cost of a confirmed utterance** rather than
something to tune away.

That matters for M2 rather than M1. Detection runs on settled text, so a spoken
reference cannot reach the projector in under about two seconds however fast the
regex stage is. PRD §18.1's "end-to-end regex path, 800 ms p99" is unreachable
if it is measured from the spoken word to the projector; it is reachable if
measured from settled text onward. **That line needs amending or the detection
stage needs to run on interim text**, which the interim/final split exists to
prevent. Parked for M2.

---

## What was not investigated

- **Whether a reconnect inflated the original run.** The app now counts
  reconnects and excludes a catch-up window after each, so the next run will
  say. The original figure predates that and cannot be re-examined.
- **Network round trip in isolation.** The handshake (1,188–1,365 ms) is a rough
  ceiling for several round trips including DNS and TLS, not a clean RTT.
- **macOS.** Nothing here has run on a Mac.

---

# Second round, 23 September 2026: the metric was still wrong

A 60-minute DoD run on a release build reported:

| | p50 | p95 | p99 |
|---|---|---|---|
| appear | 1,490 ms | 1,926 ms | **2,445 ms** |
| settle | 1,905 ms | 2,366 ms | 3,409 ms |

Peak memory **212 MB** against a 300 MB budget — line 4 passes.

The lag figures are three times the 38-second run of the same pipeline
(452/681/698 ms). Nothing about the pipeline changed between them. What changed
was the length of the run, and that turned out to be the whole story.

## Lag was measured between two different clocks

```
lag = (wall clock since the first chunk was sent) − (Deepgram's word timestamp)
```

Deepgram timestamps the audio it has actually *received*. The wall clock counts
everything. Any gap between those two counts as latency, and unlike latency it
**never recovers**:

- **A dropped chunk** removes 250 ms from Deepgram's timeline and nothing from
  the wall clock. `AudioFeed::send` used `try_send` and logged a warning, so
  each drop quietly added 250 ms to every subsequent measurement.
- **Sound cards drift.** Measured directly (`examples/capture_rate.rs`, ten
  minutes on this machine's microphone array): **+370 ms per hour**, at ±120 ms
  resolution. Real, and about a fifth of what needed explaining.

Over 38 seconds neither is visible. Over an hour they dominate.

The shape of the numbers says the same thing: for lag growing linearly from a
floor to a ceiling, the samples are roughly uniform and p50 sits at the
midpoint. Taking 450 ms as the floor and the observed 2,445 ms as the ceiling
predicts a p50 of 1,448 ms against 1,490 measured — within 3%.

**This is the second time this measurement has been wrong in a way that looked
plausible**, and both times it was the anchor. First it charged the WebSocket
handshake to every sample; now it charged every dropped chunk and every tick of
clock drift.

## The fix

Each result is now timed against **the moment its own audio was handed over**,
found by looking up the chunk that carried its last word. One clock measured
against itself, so neither drift nor drops can reach it.

Dropped chunks are counted and shown in the footer as *"Ns of audio lost"* —
in red, because this is missing sermon rather than slow sermon, and a
transcript with holes in it reads as complete.

## What is still unknown

**How much of the 1,490 ms was real.** The decomposition above is arithmetic
and a drift measurement, not a controlled re-run. The next 60-minute run
answers it directly: the footer will report dropped audio, and the lag figure
will no longer accumulate.

Worth checking in the run's log either way:

```powershell
Select-String -Path <log> -Pattern "not keeping up"
```

Roughly seven or eight of those across the hour would account for the rest.

## Why chunks would be dropped at all

The queue holds 40 chunks — 10 seconds. Filling it means the supervisor was not
draining for ten seconds, which a healthy connection should never do. If the
next run shows drops, the question is what blocks that task: a slow socket
write, a reconnect, or the 45-minute transcript making each event more expensive
to process. Not yet investigated.
