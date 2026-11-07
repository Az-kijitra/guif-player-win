import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    addBookmark: vi.fn(),
    getBookmarks: vi.fn(async () => []),
    deleteBookmark: vi.fn(async () => undefined),
    listFolder: vi.fn(),
    playQueue: vi.fn(async () => undefined),
    getPlaybackState: vi.fn(async () => ({ queue_version: 0 })),
    getQueue: vi.fn(async () => []),
  },
}));

import { api } from "./api";
import { playerState } from "./playerState.svelte";
import type { AudioTrack, Bookmark, FolderContent } from "./types";

const mocked = vi.mocked(api);
const bookmark = (file: string, position: number): Bookmark => ({
  file_path: `C:\\Album\\${file}`,
  album_path: "C:\\Album",
  title: file,
  album: "Album",
  position_ms: position,
  duration_ms: 300_000,
  created_at: 1,
});
const tracks = ["1.mp3", "2.mp3", "3.mp3"].map((f) => ({ file_path: `C:\\Album\\${f}` }) as AudioTrack);
const albumFolder = { current_path: "C:\\Album", parent_path: null, items: tracks.map((track) => ({ track })) } as FolderContent;

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(console, "error").mockImplementation(() => {});
  playerState.clearError();
});

describe("bookmarks", () => {
  it("bookmarking a track lists it and says where", async () => {
    const made = bookmark("2.mp3", 83_000);
    mocked.addBookmark.mockResolvedValueOnce(made);
    mocked.getBookmarks.mockResolvedValueOnce([made]);

    await playerState.addBookmark();

    expect(playerState.bookmarks).toEqual([made]);
    expect(playerState.errorMessage).toBe("しおりを挟みました: 2.mp3 (01:23)");
    expect(playerState.noticeKind).toBe("info");
  });

  it("deleting a bookmark removes it from the list", async () => {
    mocked.getBookmarks.mockResolvedValueOnce([bookmark("1.mp3", 1_000)]);
    await playerState.deleteBookmark("C:\\Album\\2.mp3");
    expect(mocked.deleteBookmark).toHaveBeenCalledWith("C:\\Album\\2.mp3");
    expect(playerState.bookmarks.map((b) => b.title)).toEqual(["1.mp3"]);
  });

  it("playing a bookmark starts its track at the bookmark and keeps the album queued", async () => {
    mocked.listFolder.mockResolvedValueOnce(albumFolder);
    await playerState.playBookmark(bookmark("2.mp3", 83_000));
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 1, 83_000, undefined);
  });

  it("playing a bookmark whose track is gone shows an error", async () => {
    mocked.listFolder.mockResolvedValueOnce(albumFolder);
    await playerState.playBookmark(bookmark("gone.mp3", 83_000));
    mocked.listFolder.mockRejectedValueOnce("フォルダーがありません");
    await playerState.playBookmark(bookmark("1.mp3", 1_000));

    expect(mocked.playQueue).not.toHaveBeenCalled();
    expect(mocked.deleteBookmark).not.toHaveBeenCalled();
    expect(playerState.errorMessage).toContain("しおりの曲が見つかりません: 1.mp3");
    expect(playerState.noticeKind).toBe("error");
  });

  it("opening the bookmarks closes the open playlist", async () => {
    playerState.activePlaylistId = 4;
    await playerState.openBookmarks();
    expect(playerState.showBookmarks).toBe(true);
    expect(playerState.activePlaylistId).toBeNull();
  });
});
