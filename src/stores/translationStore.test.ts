import { describe, expect, it } from "vitest";
import { filterChoices } from "./translationStore";
import type { TranslationChoice } from "@/lib/types";

const choices: TranslationChoice[] = [
  { code: "KJV", name: "King James Version", language: "eng", cached: true, source: "bundled" },
  { code: "WEB", name: "World English Bible", language: "eng", cached: true, source: "bundled" },
  { code: "NIV", name: "New International Version", language: "eng", cached: false, source: "youversion" },
];

describe("filterChoices", () => {
  it("returns everything for an empty query", () => {
    expect(filterChoices(choices, "")).toEqual(choices);
    expect(filterChoices(choices, "   ")).toEqual(choices);
  });

  it("matches the code or the name, ignoring case", () => {
    expect(filterChoices(choices, "niv").map((c) => c.code)).toEqual(["NIV"]);
    expect(filterChoices(choices, "king").map((c) => c.code)).toEqual(["KJV"]);
    expect(filterChoices(choices, "VERSION").map((c) => c.code)).toEqual(["KJV", "NIV"]);
  });

  it("keeps the list's order", () => {
    expect(filterChoices(choices, "e").map((c) => c.code)).toEqual(["KJV", "WEB", "NIV"]);
  });
});
