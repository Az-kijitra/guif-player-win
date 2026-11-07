import { describe, expect, it } from "vitest";
import { PLAYBACK_SPEEDS, formatSpeed, speedLabel, stepSpeed } from "./speed";

describe("playback speed", () => {
  it("offers the same choices as the backend", () => {
    expect(PLAYBACK_SPEEDS.map(formatSpeed)).toEqual(["0.8x", "1.0x", "1.2x", "1.4x", "1.6x", "1.8x", "2.0x"]);
  });

  it("the bar shows the speed only when it is not normal", () => {
    expect(speedLabel(1)).toBe("");
    expect(speedLabel(1.4)).toBe("1.4x");
    expect(speedLabel(0.8)).toBe("0.8x");
  });

  it("the speed keys move one step at a time and stop at both ends", () => {
    expect(stepSpeed(1, 1)).toBe(1.2);
    expect(stepSpeed(1, -1)).toBe(0.8);
    expect(stepSpeed(2, 1)).toBe(2);
    expect(stepSpeed(0.8, -1)).toBe(0.8);
    expect(stepSpeed(1.4000001, 1)).toBe(1.6);
  });
});
