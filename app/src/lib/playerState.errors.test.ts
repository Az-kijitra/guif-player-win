import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    playQueue: vi.fn(),
    getPlaybackState: vi.fn(),
    togglePlayPause: vi.fn(),
    playNext: vi.fn(),
    playPrevious: vi.fn(),
    listFolder: vi.fn(),
    getAllTrackResumes: vi.fn(async () => []),
    createPlaylist: vi.fn(),
    getPlaylists: vi.fn(),
  },
}));

import { api } from "./api";
import { playerState } from "./playerState.svelte";
import type { AudioTrack } from "./types";

const track = { file_path: "a.m4a" } as AudioTrack;
const mocked = vi.mocked(api);

describe("errors reach the screen", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(console, "error").mockImplementation(() => {});
    playerState.clearError();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows why a track could not be played", async () => {
    mocked.playQueue.mockRejectedValueOnce("a.m4a: DRM で保護された曲は再生できません");
    await playerState.playTrackInList(track, [track]);
    expect(playerState.errorMessage).toBe("a.m4a: DRM で保護された曲は再生できません");
  });

  it.each([
    ["togglePlayPause", () => playerState.togglePlayPause(), mocked.togglePlayPause],
    ["playNext", () => playerState.playNext(), mocked.playNext],
    ["playPrevious", () => playerState.playPrevious(), mocked.playPrevious],
  ] as const)("%s shows its failure instead of rejecting", async (_name, action, mock) => {
    mock.mockRejectedValueOnce("音声の出力先が見つかりません");
    await expect(action()).resolves.toBeUndefined();
    expect(playerState.errorMessage).toBe("音声の出力先が見つかりません");
  });

  it("says what was being done when the backend's message is technical", async () => {
    mocked.listFolder.mockRejectedValueOnce("boom");
    await playerState.navigateTo("C:\\Music");
    expect(playerState.errorMessage).toBe("フォルダーを開けません: boom");
  });

  it("goes away by itself", () => {
    playerState.showError("x");
    vi.advanceTimersByTime(7999);
    expect(playerState.errorMessage).toBe("x");
    vi.advanceTimersByTime(1);
    expect(playerState.errorMessage).toBeNull();
  });

  it("a newer message gets its full time", () => {
    playerState.showError("first");
    vi.advanceTimersByTime(7000);
    playerState.showError("second");
    vi.advanceTimersByTime(7000);
    expect(playerState.errorMessage).toBe("second");
  });

  it("can be closed", () => {
    playerState.showError("x");
    playerState.clearError();
    expect(playerState.errorMessage).toBeNull();
  });
});
