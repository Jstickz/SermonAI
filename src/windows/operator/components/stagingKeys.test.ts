import { describe, expect, it } from "vitest";
import { enterAction } from "./stagingKeys";

describe("enterAction", () => {
  it("stages the top card when nothing is staged", () => {
    expect(enterAction(false, true, false)).toBe("stage");
  });

  it("goes live when something is staged, whatever cards are waiting", () => {
    expect(enterAction(true, true, false)).toBe("go-live");
    expect(enterAction(true, false, false)).toBe("go-live");
  });

  it("does nothing with nothing to act on", () => {
    expect(enterAction(false, false, false)).toBe("none");
    expect(enterAction(false, false, true)).toBe("none");
  });

  it("shift+enter stages the top card and goes live in one", () => {
    expect(enterAction(false, true, true)).toBe("stage-and-go-live");
    // A staged verse is replaced by the top card, then sent: the operator
    // asked for the card, not for what was waiting.
    expect(enterAction(true, true, true)).toBe("stage-and-go-live");
  });

  it("shift+enter with no card but a staged verse just goes live", () => {
    expect(enterAction(true, false, true)).toBe("go-live");
  });
});
