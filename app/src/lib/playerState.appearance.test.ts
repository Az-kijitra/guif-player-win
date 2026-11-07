import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    getFontScale: vi.fn(),
    saveFontScale: vi.fn(async (id: string) => id),
    setZoom: vi.fn(async () => undefined),
  },
}));

import { api } from "./api";
import { playerState } from "./playerState.svelte";

const mocked = vi.mocked(api);

beforeEach(() => {
  vi.clearAllMocks();
  playerState.clearError();
});

describe("text size", () => {
  it("the chosen font size is saved and the whole page is zoomed", async () => {
    await playerState.setFontScale("extra_large");
    expect(mocked.setZoom).toHaveBeenLastCalledWith(1.3);
    expect(mocked.saveFontScale).toHaveBeenLastCalledWith("extra_large");
    expect(playerState.fontScale).toBe("extra_large");
  });

  it("the saved font size is applied when the app starts", async () => {
    mocked.getFontScale.mockResolvedValueOnce("small");
    await playerState.loadFontScale();
    expect(playerState.fontScale).toBe("small");
    expect(mocked.setZoom).toHaveBeenLastCalledWith(0.85);
  });

  it("the zoom keys move one size at a time", async () => {
    await playerState.setFontScale("normal");
    await playerState.stepFontScale(1);
    await playerState.stepFontScale(1);
    expect(playerState.fontScale).toBe("extra_large");
    await playerState.stepFontScale(-1);
    expect(playerState.fontScale).toBe("large");
  });
});
