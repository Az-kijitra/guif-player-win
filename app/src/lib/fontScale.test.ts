import { describe, expect, it } from "vitest";
import { FONT_SCALES, fontScaleFromId, stepFontScale, zoomOf } from "./fontScale";

describe("font size", () => {
  it("font sizes go from small to extra large", () => {
    expect(FONT_SCALES.map((s) => [s.label, s.scale])).toEqual([
      ["小", 0.85],
      ["標準", 1.0],
      ["大", 1.15],
      ["特大", 1.3],
    ]);
  });

  it("an unknown saved font size falls back to normal", () => {
    expect(fontScaleFromId("large")).toBe("large");
    expect(fontScaleFromId("huge")).toBe("normal");
    expect(fontScaleFromId(null)).toBe("normal");
    expect(zoomOf(fontScaleFromId(undefined))).toBe(1);
  });

  it("zoom keys step through the sizes and stop at both ends", () => {
    expect(stepFontScale("normal", 1)).toBe("large");
    expect(stepFontScale("extra_large", 1)).toBe("extra_large");
    expect(stepFontScale("normal", -1)).toBe("small");
    expect(stepFontScale("small", -1)).toBe("small");
  });
});
