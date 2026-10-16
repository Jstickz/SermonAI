import { describe, expect, it } from "vitest";
import { enterAction, isBlankHotkey, stepDelta } from "./stagingKeys";

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

  it("B blanks, bare and outside a text field only (FR-33)", () => {
    const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({
      key: k,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      ...mods,
    });
    expect(isBlankHotkey(key("b"), null)).toBe(true);
    expect(isBlankHotkey(key("B"), null)).toBe(true);
    expect(isBlankHotkey(key("b", { ctrlKey: true }), null)).toBe(false);
    expect(isBlankHotkey(key("b", { metaKey: true }), null)).toBe(false);
    expect(isBlankHotkey(key("x"), null)).toBe(false);
    const input = document.createElement("input");
    expect(isBlankHotkey(key("b"), input)).toBe(false);
  });

  it("arrows step the live passage, bare and outside a text field only (FR-51)", () => {
    const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({
      key: k,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      ...mods,
    });
    expect(stepDelta(key("ArrowRight"), null)).toBe(1);
    expect(stepDelta(key("ArrowLeft"), null)).toBe(-1);
    expect(stepDelta(key("ArrowDown"), null)).toBe(0);
    expect(stepDelta(key("ArrowRight", { ctrlKey: true }), null)).toBe(0);
    const input = document.createElement("input");
    expect(stepDelta(key("ArrowRight"), input)).toBe(0);
  });
});
