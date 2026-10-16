# Deepgram EU endpoint is the default

**Decided:** 23 September 2026 · **Record:** `docs/testing/network-latency.md`

`api.deepgram.com` resolves to Sacramento for everyone. `api.eu.deepgram.com` (Frankfurt) takes the same keys; only the base URL changes. Measured from one home connection in Abuja, ten minutes per region at real-time pace with real speech: Frankfurt 208/486/715 ms (p50/p95/p99) against Sacramento 320/591/819 ms, about 110 ms cheaper at every percentile. TCP connect alone: 140 ms vs 294 ms, so it is distance, not anything Deepgram does differently. Per-minute medians vary under 16 ms, which rules out our pipeline as a source of lag.

So `stt::deepgram::Region` defaults to EU. Caveat recorded: one home connection, not a church wifi during a service; §18.1 says to measure that at M4's first real Sunday.

Also recorded: the first probe streamed a synthetic tone and reported "no interim results" for both regions. Nova-3 returns nothing for audio it does not hear as words. Real speech is not optional in a latency probe ([[lessons]]).

Related: [[two-latency-budgets]], [[sessions/2026-09-23]]
