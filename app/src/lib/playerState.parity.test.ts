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
    setPlaybackSpeed: vi.fn(async (speed: number) => speed),
    getAlbumResume: vi.fn(async () => null),
    listAlbums: vi.fn(async () => []),
    updateTracksMetadata: vi.fn(),
  },
}));

import { api } from "./api";
import { playerState } from "./playerState.svelte";
import type { AlbumResume, AlbumSummary, AudioTrack, FolderContent, TrackResume } from "./types";

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

describe("moving to another folder", () => {
  it("the album view lists the albums of the folder it moves to", async () => {
    mocked.listFolder.mockImplementation(async (path: string) => folder(path));
    mocked.listAlbums.mockImplementation(
      async (path: string) => [{ folder_path: `${path}\\A` }] as never,
    );
    playerState.setViewMode("ARTWORK");
    await playerState.navigateTo("D:\\Music\\Jazz");
    await vi.waitFor(() =>
      expect(playerState.albums[0]?.folder_path).toBe("D:\\Music\\Jazz\\A"),
    );
    expect(mocked.listAlbums).toHaveBeenLastCalledWith("D:\\Music\\Jazz");
  });

  it("opening a folder in the album view stays in the album view", async () => {
    mocked.listFolder.mockImplementation(async (path: string) => folder(path));
    playerState.setViewMode("ARTWORK");
    await playerState.openAlbumViewItem({
      folder_path: "D:\\Music\\Rock",
      is_folder: true,
    } as AlbumSummary);
    expect(playerState.viewMode).toBe("ARTWORK");
    expect(playerState.currentPath).toBe("D:\\Music\\Rock");
    await vi.waitFor(() => expect(mocked.listAlbums).toHaveBeenLastCalledWith("D:\\Music\\Rock"));
  });

  it("the folder view does not look for albums", async () => {
    mocked.listFolder.mockImplementation(async (path: string) => folder(path));
    playerState.setViewMode("FOLDER");
    await playerState.navigateTo("D:\\Music\\Rock");
    expect(mocked.listAlbums).not.toHaveBeenCalled();
  });
});

describe("an album opened in the album view", () => {
  const loud = { folder_path: "D:\\Music\\Rock\\Loud", is_folder: false } as AlbumSummary;
  const albumFolder = (path: string): FolderContent =>
    ({
      current_path: path,
      parent_path: null,
      items: tracks.slice(0, 3).map((t) => ({ name: t.file_path, track: t })),
    }) as FolderContent;
  const resume: AlbumResume = {
    album_path: loud.folder_path,
    last_file_path: tracks[1].file_path,
    position_ms: 42_000,
    updated_at: 0,
  };

  beforeEach(async () => {
    mocked.listFolder.mockImplementation(async (path: string) => albumFolder(path));
    mocked.getAlbumResume.mockResolvedValue(resume);
    await playerState.navigateTo("D:\\Music\\Rock");
    playerState.setViewMode("ARTWORK");
    mocked.listAlbums.mockClear();
  });

  it("opening an album shows its tracks in the album view", async () => {
    await playerState.openAlbumViewItem(loud);
    expect(playerState.viewMode).toBe("ARTWORK");
    expect(playerState.currentPath).toBe("D:\\Music\\Rock");
    expect(playerState.openedAlbum?.tracks).toEqual(tracks.slice(0, 3));
    expect(playerState.openedAlbum?.resume).toEqual(resume);
    expect(mocked.getAlbumResume).toHaveBeenCalledWith(loud.folder_path);
    expect(mocked.listAlbums).not.toHaveBeenCalled();
  });

  it("an opened album plays from where it was left off", async () => {
    await playerState.openAlbumViewItem(loud);
    const opened = playerState.openedAlbum!;
    await playerState.playAlbumResume(opened.tracks, opened.resume);
    expect(mocked.playQueue).toHaveBeenCalledWith(opened.tracks, 1, 42_000, undefined);
  });

  it("going back, moving or leaving the album view closes the album", async () => {
    await playerState.openAlbumViewItem(loud);
    playerState.closeAlbum();
    expect(playerState.openedAlbum).toBeNull();

    await playerState.openAlbumViewItem(loud);
    await playerState.navigateTo("D:\\Music");
    expect(playerState.openedAlbum).toBeNull();

    await playerState.openAlbumViewItem(loud);
    playerState.setViewMode("ARTWORK");
    expect(playerState.openedAlbum).not.toBeNull();
    playerState.setViewMode("FOLDER");
    expect(playerState.openedAlbum).toBeNull();
  });

  it("the properties follow the opened album", async () => {
    playerState.selectSingleTrack(tracks[3]);
    await playerState.openAlbumViewItem(loud);
    expect(playerState.selectedTracks).toEqual(tracks.slice(0, 3));

    playerState.closeAlbum();
    expect(playerState.selectedTracks).toEqual([]);

    await playerState.openAlbumViewItem(loud);
    await playerState.openAlbumInFolderView(loud);
    expect(playerState.selectedTracks).toEqual(tracks.slice(0, 3));
  });

  it("tags edited in the properties show in the opened album", async () => {
    await playerState.openAlbumViewItem(loud);
    const renamed = { ...tracks[0], title: "New title", album: "New album" };
    mocked.updateTracksMetadata.mockResolvedValue({ updated: [renamed], failed: [] });
    mocked.listAlbums.mockResolvedValue([{ ...loud, album_name: "New album" }]);

    await playerState.updateTracksMetadata({ file_paths: [renamed.file_path] } as never);

    expect(playerState.openedAlbum?.tracks[0].title).toBe("New title");
    expect(playerState.openedAlbum?.album.album_name).toBe("New album");
    expect(playerState.selectedTracks[0].title).toBe("New title");
  });

  it("the folder button opens the album in the folder view", async () => {
    await playerState.openAlbumViewItem(loud);
    await playerState.openAlbumInFolderView(loud);
    expect(playerState.viewMode).toBe("FOLDER");
    expect(playerState.currentPath).toBe(loud.folder_path);
    expect(mocked.listAlbums).not.toHaveBeenCalled();
  });
});

describe("playing a list with missing tracks", () => {
  beforeEach(() => {
    playerState.missingPlaylistPaths = new Set([tracks[0].file_path, tracks[3].file_path]);
  });

  it("does not start on a missing track", async () => {
    await playerState.playAll(tracks, false);
    expect(mocked.playQueue).toHaveBeenLastCalledWith(tracks, 1, 0, false);

    for (const random of [0, 0.4, 0.99]) {
      vi.spyOn(Math, "random").mockReturnValue(random);
      await playerState.playAll(tracks, true);
      const start = mocked.playQueue.mock.lastCall![1];
      expect([1, 2, 4]).toContain(start);
    }
  });

  it("still tries when every track is missing", async () => {
    playerState.missingPlaylistPaths = new Set(tracks.map((t) => t.file_path));
    await playerState.playAll(tracks, true);
    expect(mocked.playQueue).toHaveBeenLastCalledWith(tracks, 0, 0, true);
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

describe("playback speed", () => {
  it("the speed is sent to the player and shown at once", async () => {
    await playerState.setPlaybackSpeed(1.4);
    expect(mocked.setPlaybackSpeed).toHaveBeenLastCalledWith(1.4);
    expect(playerState.playback.speed).toBe(1.4);
    await playerState.stepPlaybackSpeed(1);
    expect(mocked.setPlaybackSpeed).toHaveBeenLastCalledWith(1.6);
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

describe("keeping in step with the player", () => {
  it("with nothing loaded, the resume points are not read again on every poll", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("window", globalThis);
    try {
      playerState.startSync();
      await vi.advanceTimersByTimeAsync(2_000);
      expect(mocked.getAllTrackResumes.mock.calls.length).toBeLessThanOrEqual(1);
    } finally {
      clearInterval((playerState as unknown as { syncInterval: number }).syncInterval);
      vi.unstubAllGlobals();
      vi.useRealTimers();
    }
  });
});
