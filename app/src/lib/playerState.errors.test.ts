import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    playQueue: vi.fn(),
    getPlaybackState: vi.fn(),
    togglePlayPause: vi.fn(),
    playNext: vi.fn(),
    playPrevious: vi.fn(),
    listFolder: vi.fn(),
    listAlbums: vi.fn(),
    getQueue: vi.fn(),
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

  it("keeps the old folder and path when a folder cannot be opened", async () => {
    mocked.listFolder.mockResolvedValueOnce({ current_path: "C:\\A", parent_path: null, items: [] });
    await playerState.navigateTo("C:\\A");
    mocked.listFolder.mockRejectedValueOnce("gone");
    await playerState.navigateTo("D:\\");
    expect(playerState.currentPath).toBe("C:\\A");
    expect(playerState.folderContent?.current_path).toBe("C:\\A");
    expect(playerState.isLoadingFolder).toBe(false);
  });

  it("ignores a slow answer to an older folder request", async () => {
    let answerOld = (_: unknown) => {};
    mocked.listFolder.mockImplementationOnce(() => new Promise((resolve) => (answerOld = resolve)) as never);
    const old = playerState.navigateTo("C:\\Old");
    expect(playerState.isLoadingFolder).toBe(true);

    mocked.listFolder.mockResolvedValueOnce({ current_path: "C:\\New", parent_path: null, items: [] });
    await playerState.navigateTo("C:\\New");
    answerOld({ current_path: "C:\\Old", parent_path: null, items: [] });
    await old;

    expect(playerState.currentPath).toBe("C:\\New");
    expect(playerState.folderContent?.current_path).toBe("C:\\New");
    expect(playerState.isLoadingFolder).toBe(false);
  });

  it("ignores a failure of an older folder request", async () => {
    let failOld = (_: unknown) => {};
    mocked.listFolder.mockImplementationOnce(() => new Promise((_, reject) => (failOld = reject)) as never);
    const old = playerState.navigateTo("C:\\Old");
    mocked.listFolder.mockResolvedValueOnce({ current_path: "C:\\New", parent_path: null, items: [] });
    await playerState.navigateTo("C:\\New");
    failOld("gone");
    await old;
    expect(playerState.errorMessage).toBeNull();
  });

  it("ignores a slow answer to an older album request", async () => {
    mocked.listFolder.mockResolvedValueOnce({ current_path: "C:\\New", parent_path: null, items: [] });
    await playerState.navigateTo("C:\\New");
    let answerOld = (_: unknown) => {};
    mocked.listAlbums.mockImplementationOnce(() => new Promise((resolve) => (answerOld = resolve)) as never);
    const old = playerState.loadAlbums();
    mocked.listAlbums.mockResolvedValueOnce([]);
    await playerState.loadAlbums();
    answerOld([{ album_name: "stale" }]);
    await old;
    expect(playerState.albums).toEqual([]);
    expect(playerState.isLoadingAlbums).toBe(false);
  });

  it("fetches the queue only when its version changes", async () => {
    const state = { queue_version: 7 } as never;
    mocked.playQueue.mockResolvedValue(undefined as never);
    mocked.getPlaybackState.mockResolvedValue(state);
    mocked.getQueue.mockResolvedValue([track]);

    await playerState.playTrackInList(track, [track]);
    await playerState.playTrackInList(track, [track]);

    expect(mocked.getQueue).toHaveBeenCalledTimes(1);
    expect(playerState.queue).toEqual([track]);
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
