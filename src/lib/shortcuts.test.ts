import { describe, expect, it } from "vitest";
import prd from "../../docs/PRD.md?raw";
import sheet from "../../docs/testing/cheat-sheet.html?raw";
import { SHORTCUTS } from "./shortcuts";

/**
 * The PRD's §13.18 table and `SHORTCUTS` are the same list (M3 polish item
 * 7). This parses the Markdown table and compares every cell, so a key
 * added in one place and not the other is a failing test rather than a
 * cheat sheet that lies. The printed sheet is checked more loosely: every
 * key must appear on it. Both documents arrive as raw text through Vite.
 */
function prdRows(): { keys: string; does: string; where: string; fr: string }[] {
  const start = prd.indexOf("### 13.18 Keyboard Shortcuts");
  expect(start, "PRD §13.18 exists").toBeGreaterThan(-1);
  const section = prd.slice(start, prd.indexOf("\n## 14.", start));
  return section
    .split("\n")
    .filter((line) => line.startsWith("| ") && !line.startsWith("| Key |") && !line.startsWith("|---"))
    .map((line) => {
      const cells = line
        .slice(1, -1)
        .split("|")
        .map((c) => c.trim());
      return { keys: cells[0] ?? "", does: cells[1] ?? "", where: cells[2] ?? "", fr: cells[3] ?? "" };
    });
}

describe("shortcuts", () => {
  it("match PRD §13.18 row for row", () => {
    expect(prdRows()).toEqual(SHORTCUTS.map((s) => ({ ...s })));
  });

  it("every key is on the printed cheat sheet", () => {
    const tokens = ["Enter", "Shift", "Esc", "B", "→", "←", "P", "1", "9", "Ctrl", "K", "Tab"];
    for (const t of tokens) {
      expect(sheet, `cheat sheet mentions ${t}`).toContain(`<kbd>${t}</kbd>`);
    }
  });
});
