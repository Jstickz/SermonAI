# Lessons

Hard lessons, each paid for once. Every entry names the day it was learned so the session note has the detail. Add to the top; never delete, mark superseded instead.

## Hardware and networks fail silently; detect with timeouts, not errors

- **Dead socket.** A dropped network raises nothing: TCP does not learn the peer has gone, writes keep succeeding into the kernel buffer and reads pend forever. The socket task waited for an error or a close frame and got neither, so nothing reconnected, and once the send buffer filled the supervisor deadlocked on a full channel. Two timeouts replace the assumption: a write not complete in 5 s, and no message for 6 s while audio is flowing. ([[sessions/2026-09-23]])
- **Dead device.** A Bluetooth headset pulled mid-stream raises no stream error; WASAPI keeps the endpoint valid and stops delivering. A 2 s no-callback watchdog treats it as loss, which is sound because cpal delivers buffers whether or not anyone is speaking: a silent device is not a quiet room. Except for loopback, below. ([[sessions/2026-09-23]])
- **Deepgram closes idle streams** after about ten seconds of silence, so a paused capture would lose its socket during the pause. KeepAlive every 8 s. ([[sessions/2026-09-22]])

## "Nothing" and "silence" are different, and loopback sends the first

Windows WASAPI loopback stops calling back when playback stops. A quiet spell vanished from the audio timeline entirely, every later timestamp was early by that much, and the 60-minute run read it as 9,606 ms/hour of "drift". The capture thread now synthesises silence against the wall clock for loopback devices and sends it to Deepgram so both timelines count the same seconds. Loopback endpoints are exempt from the device watchdog for the same reason: a church between songs would be told its input had failed. ([[sessions/2026-09-24]])

The trigger is playback *ending*, not zeros inside a stream: on Speakers (Realtek) a file with 30 s of digital silence in the middle arrived in full. A test built on the obvious trigger passed with the fix disabled.

## Types do not cross the IPC boundary by themselves

Serde `rename_all` renames enum *variants*, not the fields inside them. `retry_in_ms` reached the frontend under that name while TypeScript read `retryInMs`; the type said `number`, the value was `undefined`, and the operator saw "Retrying in NaNs". The same mismatch silently hid the held-seconds sentence. Every tagged enum crossing the boundary now has its field names asserted on both sides by a JSON test. ([[sessions/2026-09-23]], pinned again for `Detection` on [[sessions/2026-09-24]])

## A green test can mean the test is wrong

- The reconnect harness passed twice while proving nothing: once because a flag was read at the wrong moment, once because two tests shared an endpoint and connected to each other. It had to be checked by disabling the fix and watching it fail: "1 connection(s), statuses [connected]" for 25 s, exactly what the operator saw. ([[sessions/2026-09-23]])
- The loopback pause test passed identically with the fix disabled, on an endpoint that never had the problem. Check that the endpoint under test is the one receiving the audio; two runs measured nothing because it was not. ([[sessions/2026-09-24]])
- A region probe streaming a synthetic tone reported "no interim results" for both regions and looked like a network failure. Nova-3 returns nothing for audio it does not hear as words. ([[sessions/2026-09-23]])
- The keyterm A/B against synthesised speech was byte-identical with and without the vocabulary: text-to-speech articulates too well for the bias to matter. A null result on the wrong input proves nothing about real preaching. ([[sessions/2026-09-22]])

## The measurement faults (see [[measure-the-thing-not-a-proxy]])

1. **Memory, by name.** Summing `WorkingSet` across nine processes read 583 MB where the private working set was 152 MB. Resolving by performance-counter instance name read 40 MB where the truth was 193 MB, because several processes are called `msedgewebview2` and the lookup resolved 1 of 9. Match by PID; print how many were resolved; declare a one-process run invalid. ([[sessions/2026-09-23]])
2. **Lag, wrong population.** Measuring only settled text (Deepgram deciding the speaker has stopped, ~2.3 s) against a budget written about visible text read a passing pipeline as a fourfold failure. ([[sessions/2026-09-23]])
3. **Lag, wrong anchor.** Starting the clock at capture rather than at the first chunk sent charged the 1,188 to 1,365 ms WebSocket handshake to every sample. ([[sessions/2026-09-23]])
4. **Lag, two clocks.** Wall clock since the first chunk minus Deepgram's audio timestamp: a dropped chunk removes 250 ms from one and nothing from the other, sound cards drift (+370 ms/hour measured), and both accumulate, so an hour read three times a 38-second figure. Time each result against the moment its own audio was handed over. The shape gave it away: uniform samples with p50 at the midpoint of floor and ceiling, within 3%. ([[sessions/2026-09-23]])
5. **Cold start, no clock.** "Approximately 300 ms" from a stopwatch on a VM, before the app could time itself. Properly: 716 ms, of which 554 ms is Tauri plus WebView2 (probe apps) and 162 ms ours. Two traps inside that: build output under OneDrive inflated an afternoon's figures by ~700 ms, and synchronous Tauri commands queue on the main thread ahead of the ready call, so the window must timestamp its own paint. ([[sessions/2026-10-04]])
6. **Stream clock offset per chunk.** Each reconnect starts a new Deepgram stream at t=0, so an offset is added; advancing it per chunk double-counted and 18.6 s of audio reported timings to 33.8 s. ([[sessions/2026-09-22]])
7. **Debug profile.** A hot dot-product loop is an order of magnitude slower in debug; `opt-level = "s"` made it 15.98 ms against a 5 ms budget. Budget tests assert in release only. ([[sessions/2026-09-09]], [[sessions/2026-09-24]])

## Endpointing is not the lever for settled text

Deepgram's `endpointing` at default, 100 ms and 300 ms gave settled p95 of 2,039 / 2,083 / 1,664 ms, within run-to-run variance. About 2 s is the cost of a confirmed utterance. This is why [[two-stage-detection]] exists. ([[sessions/2026-09-23]])

## Real calls before trusting docs

- YouVersion's serde structs were guessed from documentation and pinned to the live API afterwards; the guesses were wrong. ([[sessions/2026-09-21]])
- Three separate Anthropic refusals (missing workspace header, unscoped key, no credit) each read as "Claude doesn't work" in the log. 57 calls, every one a 400, zero tokens. The key was never the blocker; the account was. The stage now disables itself for the service on a configuration error and names which one. ([[sessions/2026-10-04]], [[sessions/2026-10-09]])
- Loopback device formats come from the *output* config; `supported_input_configs()` is empty for an output device on Windows. Reading the input side would have made every loopback device look unusable. ([[sessions/2026-09-21]])

## Decimation is not resampling

48 kHz to 16 kHz is exactly one in three, which makes dropping samples tempting. Without band-limiting it folds a 12 kHz sibilant into a 4 kHz tone nobody spoke, damaging recognition where consonants are distinguished, while passing every obvious check. `rubato` band-limits; a test measures that the 12 kHz tone arrives at under a tenth the energy of a real 4 kHz one. ([[sessions/2026-09-21]])

## Interim results are revisions, not drafts

"...if you will to join" became "...to John chapter three" one result later. A consumer appending every interim prints the same half-sentence five times; one acting on interim text detects the wrong reference. Hence the two-variant `TranscriptEvent`, the interim-free rolling buffer, and [[two-stage-detection]]. ([[sessions/2026-09-22]])

## Seed test corpora from what the vendor actually produces

The regex corpus was seeded from Nova-3's own output, and three things would have been guessed wrong: references arrive split across settled utterances ("Jeremiah." / "Chapter 29." / "Verse 11,"); digits after a keyword but spelled-out run-together numbers ("Romans eight twenty eight"); "John writes in his first epistle" must not become 1 John. ([[sessions/2026-09-24]])

## A label can be wrong in the stage's favour

Three "wrong accepts" in the paraphrase set were scripture quoting scripture (Habakkuk 2:4 / Romans 1:17 / Galatians 3:11). The fixture now holds alternatives, and the operator reviewed every line against the KJV text before any number was trusted. ([[sessions/2026-10-09]])

## Measure the stage, then measure what the next stage adds

Vector alone 70% → 79% with synonyms → 92% with Claude as gated → 99% if every line were asked. Each number told a different decision: keep the synonyms; the gate's floor is where the remaining misses live; a bigger encoder is now a question of cost rather than of passing. ([[sessions/2026-10-15]])

## Two components keeping their own copy of a selection will disagree

Display assignments in React state lost track of the live screen on a tab switch (moved to Rust, [[sessions/2026-09-08]]). The Live tab and Settings each kept their own device selection, so choosing a replacement input changed a value the running capture never read (moved to a shared store, [[sessions/2026-09-23]]). The output windows ask for state on mount because the Live tab unmounts on a tab switch ([[sessions/2026-10-16]]).

## A window built inside a synchronous command deadlocks on Windows

Tauri runs a synchronous command on the main thread. Creating a webview window there waits for the main thread's own event loop, which is blocked by the command: the first "Use as projector" froze the Settings page for five minutes and logged nothing, because the log line sits after the build. The window creation had moved from startup into the assignment command on 4 October for cold-start reasons and had not been exercised on the operator's machine since. Rule: any command that may create a window is `async`, and the doc comment on it says why. ([[sessions/2026-10-16]])

## Operational

- A leftover `sermonai.exe` holds the release binary and fails every cargo build with "Access is denied (os error 5)". [[close-leftover-instances]].
- Keep the build tree out of any sync root. The project moved from OneDrive on 4 October. ([[sessions/2026-10-04]])
- Moving the project to another drive leaves cargo's cached build-script output pointing at the old path: every build fails with "failed to read plugin permissions … C:\Projects\… (os error 3)" even though the project is on D:. The fix is to delete every `target/*/build/*/` whose `root-output` names the old path (136 of them on 16 October), not `cargo clean`. ([[sessions/2026-10-16]])
- A `.foglamp/scan.lock.json` holds a public URL's edit token; it is gitignored. Secrets have never entered git history; keep it so. ([[people-and-accounts]])
