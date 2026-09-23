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
  it("accepts what the nearest region actually delivers", () => {
    // Frankfurt, measured over ten minutes from Abuja: 486 p95, 715 p99.
    expect(overBudget({ p95Ms: 486, p99Ms: 715 })).toBe(false);
  });

  it("accepts the far region too, which is the point of the headroom", () => {
    // Sacramento: 591 p95, 819 p99. Over the old 700 ms line at p99, inside
    // §18.1's — which is why that line had to change rather than the build.
    expect(overBudget({ p95Ms: 591, p99Ms: 819 })).toBe(false);
  });

  it("warns when most of the service is late, not just the tail", () => {
    // A p95 over budget means the median-ish case is late. That matters more
    // to an operator than one slow utterance in a hundred, so it must warn
    // even while p99 is fine.
    expect(overBudget({ p95Ms: 700, p99Ms: 900 })).toBe(true);
  });

  it("warns on a bad tail even when the bulk is fine", () => {
    expect(overBudget({ p95Ms: 400, p99Ms: 1200 })).toBe(true);
  });

  it("would have warned on the run that started all this", () => {
    // The 60-minute run: 1926 p95, 2445 p99.
    expect(overBudget({ p95Ms: 1926, p99Ms: 2445 })).toBe(true);
  });
});
