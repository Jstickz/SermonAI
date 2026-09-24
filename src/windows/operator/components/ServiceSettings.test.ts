import { describe, expect, it } from "vitest";
import { isMissingKeyError, overBudget } from "./LiveTranscript";

/**
 * The link from a capture failure to the panel that fixes it.
 *
 * The strings below are the literal messages `credentials::local` produces, and
 * they are asserted on the Rust side too — see
 * `the_message_for_a_missing_key_names_where_to_set_it` in
 * `src-tauri/src/credentials/local.rs`. Both ends pin the same wording on
 * purpose: the match is on the message text, so a reworded error would
 * otherwise stop offering the button with nothing failing anywhere.
 *
 * That is the same class of bug as "Retrying in NaNs" — a value crossing the
 * IPC boundary under a shape one side no longer expects, with every type still
 * satisfied.
 */
const MISSING_DEEPGRAM_KEY =
  "No Deepgram key is set. Add one in Settings → Services and keys.";

const MANAGED_ONLY =
  "Tyndale NLT is not available yet. It comes with SermonAI activation, which is not built.";

describe("isMissingKeyError", () => {
  it("offers the button when a key is what is missing", () => {
    expect(isMissingKeyError(MISSING_DEEPGRAM_KEY)).toBe(true);
  });

  it("stays quiet for a service the operator cannot fix themselves", () => {
    // Tyndale and YouVersion are managed-only: the licence is SermonAI's, so
    // there is no key to paste. Sending the operator to a box they cannot use
    // is worse than saying the feature is not built.
    expect(isMissingKeyError(MANAGED_ONLY)).toBe(false);
  });

  it("stays quiet for unrelated capture failures", () => {
    // These share a banner. A device error that offered "Open Services and
    // keys" would send the operator hunting through the wrong screen while a
    // service is running.
    for (const message of [
      "Headset (oraimo OpenArc) is no longer available. Choose another input.",
      "Deepgram is unreachable. Retrying in 4s (attempt 3).",
      "The audio device stopped sending data.",
    ]) {
      expect(isMissingKeyError(message)).toBe(false);
    }
  });
});

/**
 * The appear-latency warning threshold.
 *
 * PRD §18.1 budgets 900 ms p95 / 1300 ms p99 from spoken word to a provisional
 * candidate in staging. The footer shows only the part it measures — chunk sent
 * to result back — so the 250 ms a chunk spends accumulating comes off first.
 *
 * Pinned because the old 700 ms figure was carried in three places at once and
 * they drifted: the DoD line, the Rust log field and this component. It now
 * derives from one number in one file.
 */
describe("overBudget", () => {
  it("passes the run that closed M1", () => {
    // 20 minutes, EU region, n=835: p50 187, p95 501. Its p99 was 1404 — over
    // the old p99 line — and that is exactly why the tail is no longer what
    // this warns on. §18.1 governs the tail by episode conduct instead.
    expect(overBudget({ p50Ms: 187, p95Ms: 501 })).toBe(false);
  });

  it("warns on the far region, which is why EU is the default", () => {
    // Sacramento, ten minutes from Abuja: 320 p50, 591 p95. With the 250 ms
    // chunk on top that is a 570 ms median against a 500 ms budget. The old
    // p95/p99 budget let Sacramento through; the steady-state one does not,
    // and it should not — a church left on the global default would see this.
    expect(overBudget({ p50Ms: 320, p95Ms: 591 })).toBe(true);
  });

  it("warns when the median is late", () => {
    // A slow median is the operator's whole experience, not a tail event.
    expect(overBudget({ p50Ms: 300, p95Ms: 500 })).toBe(true);
  });

  it("warns when most of the service is late even if the median is fine", () => {
    expect(overBudget({ p50Ms: 200, p95Ms: 700 })).toBe(true);
  });

  it("does not warn on a bad tail alone", () => {
    // A single wifi wobble owning the top 1% is an episode, reported through
    // dropped audio, reconnects and the 6 s ceiling — not through this colour.
    // Making it amber here would teach the operator to ignore amber.
    expect(overBudget({ p50Ms: 187, p95Ms: 501 })).toBe(false);
  });

  it("would have warned on the run that started all this", () => {
    // The 60-minute run: 1490 p50, 1926 p95.
    expect(overBudget({ p50Ms: 1490, p95Ms: 1926 })).toBe(true);
  });
});
