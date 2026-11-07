import { describe, expect, it } from "vitest";
import { albumResumePoint, folderResumePoint } from "./albumResume";
import type { AlbumResume, AudioTrack, TrackResume } from "./types";

describe("albumResumePoint", () => {
  it("album resumes at the position of an unfinished track", () => {
    expect(albumResumePoint(5, 2, false, 42_000)).toEqual({ index: 2, positionMs: 42_000 });
  });

  it("album resumes at the next track after a completed track", () => {
    expect(albumResumePoint(5, 2, true, 199_000)).toEqual({ index: 3, positionMs: 0 });
  });

  it("album starts over after its last track is completed", () => {
    expect(albumResumePoint(5, 4, true, 199_000)).toEqual({ index: 0, positionMs: 0 });
  });

  it("album starts over when the saved track is gone", () => {
    expect(albumResumePoint(5, -1, false, 42_000)).toEqual({ index: 0, positionMs: 0 });
    expect(albumResumePoint(5, 9, false, 42_000)).toEqual({ index: 0, positionMs: 0 });
  });
});

describe("folderResumePoint", () => {
  const tracks = [1, 2, 3].map((n) => ({ file_path: `C:\\A\\${n}.mp3` }) as AudioTrack);
  const resume = (file: string, position: number) =>
    ({ album_path: "C:\\A", last_file_path: file, position_ms: position, updated_at: 1 }) as AlbumResume;
  const done = (file: string) => ({ [file]: { is_completed: true } as TrackResume });

  it("folder shows the track its album resumes from", () => {
    expect(folderResumePoint(tracks, resume("C:\\A\\2.mp3", 30_000), {})).toEqual({ index: 1, positionMs: 30_000 });
    expect(folderResumePoint(tracks, resume("C:\\A\\2.mp3", 199_000), done("C:\\A\\2.mp3"))).toEqual({ index: 2, positionMs: 0 });
  });

  it("offers nothing to resume in a folder never played or whose track is gone", () => {
    expect(folderResumePoint(tracks, null, {})).toBeNull();
    expect(folderResumePoint(tracks, resume("C:\\A\\gone.mp3", 30_000), {})).toBeNull();
  });
});
