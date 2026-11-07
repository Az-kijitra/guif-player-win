import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    playQueue: vi.fn(async () => undefined),
    getPlaybackState: vi.fn(async () => ({ queue_version: 0 })),
    getQueue: vi.fn(async () => []),
    listFolder: vi.fn(),
    getAllTrackResumes: vi.fn(async () => []),
    createPlaylist: vi.fn(),
    deletePlaylist: vi.fn(async () => undefined),
    getPlaylists: vi.fn(async () => []),
    getPlaylistTracks: vi.fn(async () => []),
    addToPlaylist: vi.fn(async () => undefined),
    findMissingFiles: vi.fn(async () => []),
    getAlbumResume: vi.fn(async () => null),
  },
}));

import { api } from "./api";
import { playerState } from "./playerState.svelte";
import type { AudioTrack, FolderContent, TrackResume } from "./types";

const mocked = vi.mocked(api);
const track = (n: number) => ({ file_path: `C:\\Music\\${n}.mp3` }) as AudioTrack;
const tracks = [0, 1, 2, 3, 4].map(track);
const folder = (path: string, label = path): FolderContent =>
  ({ current_path: path, parent_path: null, items: [{ name: label }] }) as FolderContent;

function pending<T>() {
  let resolve = (_: T) => {};
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.spyOn(console, "error").mockImplementation(() => {});
  playerState.clearError();
  playerState.trackResumes = {};
});

describe("playing a whole list", () => {
  it("plays everything in order with shuffle off", async () => {
    await playerState.playAll(tracks, false);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 0, 0, false);
  });

  it("plays everything shuffled from a random track", async () => {
    vi.spyOn(Math, "random").mockReturnValue(0.65);
    await playerState.playAll(tracks, true);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 3, 0, true);
  });

  it("a partly played track starts from its position", async () => {
    playerState.trackResumes = {
      [tracks[2].file_path]: { position_ms: 42_000, is_completed: false } as TrackResume,
    };
    await playerState.playTrackInList(tracks[2], tracks);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 2, 42_000, undefined);
  });

  it("a completed track starts from the beginning", async () => {
    playerState.trackResumes = {
      [tracks[2].file_path]: { position_ms: 199_000, is_completed: true } as TrackResume,
    };
    await playerState.playTrackInList(tracks[2], tracks);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 2, 0, undefined);
  });
});

describe("resuming a folder", () => {
  it("plays the folder from where it was left off", async () => {
    playerState.albumResume = { album_path: "C:\\Music", last_file_path: tracks[3].file_path, position_ms: 61_000, updated_at: 1 };
    await playerState.playAlbumResume(tracks);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 3, 61_000, undefined);
  });

  it("plays the folder from the first track at its start", async () => {
    playerState.trackResumes = {
      [tracks[0].file_path]: { position_ms: 42_000, is_completed: false } as TrackResume,
    };
    await playerState.playAlbumFromBeginning(tracks);
    expect(mocked.playQueue).toHaveBeenCalledWith(tracks, 0, 0, undefined);
  });
});

describe("folders opened before", () => {
  it("shows a folder seen before at once and then reads it again", async () => {
    mocked.listFolder.mockResolvedValueOnce(folder("C:\\Seen", "first"));
    await playerState.navigateTo("C:\\Seen");
    mocked.listFolder.mockResolvedValueOnce(folder("C:\\Elsewhere"));
    await playerState.navigateTo("C:\\Elsewhere");

    const again = pending<FolderContent>();
    mocked.listFolder.mockReturnValueOnce(again.promise);
    const opening = playerState.navigateTo("C:\\Seen");

    expect(playerState.currentPath).toBe("C:\\Seen");
    expect(playerState.folderContent?.items[0].name).toBe("first");
    expect(playerState.isLoadingFolder).toBe(false);

    again.resolve(folder("C:\\Seen", "read again"));
    await opening;
    expect(playerState.folderContent?.items[0].name).toBe("read again");
  });

  it("remembers only the most recent folders", async () => {
    for (let n = 0; n <= 20; n++) {
      mocked.listFolder.mockResolvedValueOnce(folder(`C:\\Recent\\${n}`));
      await playerState.navigateTo(`C:\\Recent\\${n}`);
    }

    const again = pending<FolderContent>();
    mocked.listFolder.mockReturnValueOnce(again.promise);
    const opening = playerState.navigateTo("C:\\Recent\\0");
    expect(playerState.currentPath).toBe("C:\\Recent\\20");
    expect(playerState.isLoadingFolder).toBe(true);
    again.resolve(folder("C:\\Recent\\0"));
    await opening;

    const recent = pending<FolderContent>();
    mocked.listFolder.mockReturnValueOnce(recent.promise);
    const reopening = playerState.navigateTo("C:\\Recent\\20");
    expect(playerState.currentPath).toBe("C:\\Recent\\20");
    expect(playerState.isLoadingFolder).toBe(false);
    recent.resolve(folder("C:\\Recent\\20"));
    await reopening;
  });

  it("a refresh reads the folder again without the remembered result", async () => {
    mocked.listFolder.mockResolvedValueOnce(folder("C:\\Refresh", "old"));
    await playerState.navigateTo("C:\\Refresh");

    const again = pending<FolderContent>();
    mocked.listFolder.mockReturnValueOnce(again.promise);
    const refreshing = playerState.refreshFolder();
    expect(playerState.isLoadingFolder).toBe(true);
    again.resolve(folder("C:\\Refresh", "new"));
    await refreshing;

    expect(mocked.listFolder).toHaveBeenLastCalledWith("C:\\Refresh");
    expect(playerState.folderContent?.items[0].name).toBe("new");
    expect(playerState.isLoadingFolder).toBe(false);
  });
});

describe("playlists", () => {
  it("deleting the open playlist closes it", async () => {
    await playerState.selectPlaylist(7);
    await playerState.deletePlaylist(7);
    expect(playerState.activePlaylistId).toBeNull();
    expect(playerState.playlistTracks).toEqual([]);
  });

  it("a refused name adds nothing to another playlist", async () => {
    mocked.createPlaylist.mockRejectedValueOnce("「Drive」という名前のプレイリストがあります");
    await playerState.createPlaylistAndAdd("Drive", tracks[0]);
    expect(mocked.addToPlaylist).not.toHaveBeenCalled();
    expect(playerState.errorMessage).toContain("「Drive」という名前のプレイリストがあります");
  });

  it("says how many tracks of a playlist are missing", async () => {
    mocked.getPlaylistTracks.mockResolvedValueOnce(tracks);
    mocked.findMissingFiles.mockResolvedValueOnce([tracks[1].file_path, tracks[4].file_path]);

    await playerState.selectPlaylist(3);

    expect(playerState.errorMessage).toContain("見つからない曲が 2 曲あります");
    expect([...playerState.missingPlaylistPaths]).toEqual([tracks[1].file_path, tracks[4].file_path]);
  });
});
