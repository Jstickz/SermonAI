import { describe, expect, it } from "vitest";
import { isMissingKeyError } from "./LiveTranscript";

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
