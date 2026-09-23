import { describe, expect, it } from "vitest";
import { reconnectingMessage } from "./LiveTranscript";
import type { SttStatus } from "@/lib/types";

/**
 * The reconnect banner, tested against the JSON the backend actually emits.
 *
 * The fixture below is the literal asserted by
 * `the_status_json_matches_what_the_banner_reads` in
 * `src-tauri/src/stt/reconnect.rs`. It is parsed rather than hand-written as
 * an object, so this test sees the same bytes that cross the IPC boundary —
 * including the field *names*, which is where the bug was.
 *
 * `retry_in_ms` reached the frontend under that name because `rename_all`
 * renames enum variants, not the fields inside them. TypeScript read
 * `retryInMs`, got `undefined`, and `undefined / 1000` is `NaN` — so the
 * operator saw "Retrying in NaNs" while every type said `number` and nothing
 * threw.
 */
const BACKEND_JSON = `{"kind":"reconnecting","attempt":3,"retryInMs":2000,"bufferedSeconds":12.0}`;

describe("reconnectingMessage", () => {
  it("formats the real payload the backend sends", () => {
    const status = JSON.parse(BACKEND_JSON) as Extract<SttStatus, { kind: "reconnecting" }>;

    // 2000 ms is BACKOFF_MS[2] in reconnect.rs — a real delay from the table,
    // not an invented round number.
    expect(status.retryInMs).toBe(2000);
    expect(status.bufferedSeconds).toBe(12);

    const text = reconnectingMessage(status);
    expect(text).toBe(
      "Deepgram is unreachable. Retrying in 2s (attempt 3). " +
        "Still recording — 12s of speech held, and it will be transcribed when the connection returns.",
    );
    expect(text).not.toContain("NaN");
  });

  it("says how much speech is held, which is the question being asked", () => {
    const text = reconnectingMessage({ attempt: 5, retryInMs: 8000, bufferedSeconds: 31.5 });
    expect(text).toContain("32s of speech held");
    expect(text).toContain("Retrying in 8s");
  });

  it("does not claim held speech before any has accumulated", () => {
    // The first attempt fires before anything has been buffered, and
    // "0s of speech held" reads as a loss rather than a reassurance.
    const text = reconnectingMessage({ attempt: 1, retryInMs: 500, bufferedSeconds: 0 });
    expect(text).toContain("Nothing spoken is being lost");
    expect(text).not.toContain("0s of speech");
  });

  it("never prints NaN, whatever arrives", () => {
    // The guard that would have caught the original bug at the boundary
    // rather than on the operator's screen. A field renamed on the Rust side
    // arrives as undefined, and the type system cannot help.
    const broken = { attempt: 2 } as unknown as {
      attempt: number;
      retryInMs: number;
      bufferedSeconds: number;
    };

    const text = reconnectingMessage(broken);
    expect(text).not.toContain("NaN");
    expect(text).toContain("attempt 2");
    // Degrades to a weaker but true sentence rather than a broken one.
    expect(text).toContain("Retrying");
  });

  it("stops promising recovery once the buffer has overflowed", () => {
    // Seen on a real outage: the amber banner said "60s of speech held, and it
    // will be transcribed when the connection returns" directly above a red
    // one saying "about 25s of speech was not transcribed". Both were true and
    // together they were incoherent — the operator cannot tell which to
    // believe, and the reassurance is the one that is misleading.
    const text = reconnectingMessage(
      { attempt: 8, retryInMs: 30000, bufferedSeconds: 60 },
      25,
    );

    expect(text).toContain("about 25s of speech has already been lost");
    expect(text).toContain("most recent 60s is still held");
    expect(text).not.toContain("will be transcribed when the connection returns");
    expect(text).toContain("Retrying in 30s (attempt 8)");
  });

  it("keeps the reassurance while the buffer still fits the outage", () => {
    const text = reconnectingMessage(
      { attempt: 2, retryInMs: 1000, bufferedSeconds: 8 },
      0,
    );
    expect(text).toContain("8s of speech held");
    expect(text).toContain("will be transcribed when the connection returns");
    expect(text).not.toContain("lost");
  });

  it("rounds a sub-second delay up rather than to zero", () => {
    // BACKOFF_MS[0] is 500 ms. "Retrying in 0s" reads as stuck.
    const text = reconnectingMessage({ attempt: 1, retryInMs: 500, bufferedSeconds: 4 });
    expect(text).toContain("Retrying in 1s");
  });
});
