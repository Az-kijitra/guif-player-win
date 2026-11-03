import { beforeEach, describe, expect, it } from "vitest";
import { playerState } from "./playerState.svelte";
import type { AudioTrack } from "./types";

function track(n: number): AudioTrack {
  return {
    file_path: `${n}.mp3`,
    file_name: `${n}.mp3`,
    title: `Track ${n}`,
    artist: "Artist",
    album: "Album",
    duration_ms: 1000,
    track_number: null,
    has_artwork: false,
  };
}

const tracks = [track(0), track(1), track(2), track(3)];
const selectedPaths = () => playerState.selectedTracks.map((t) => t.file_path);

describe("extendSelection", () => {
  beforeEach(() => playerState.clearSelection());

  it("includes the row the movement started from", () => {
    playerState.extendSelection(tracks[1], tracks[2], tracks);
    expect(selectedPaths()).toEqual(["1.mp3", "2.mp3"]);
  });

  it("keeps growing as the movement continues", () => {
    playerState.extendSelection(tracks[0], tracks[1], tracks);
    playerState.extendSelection(tracks[1], tracks[2], tracks);
    expect(selectedPaths()).toEqual(["0.mp3", "1.mp3", "2.mp3"]);
  });

  it("does not deselect a row that was already selected", () => {
    playerState.selectSingleTrack(tracks[1]);
    playerState.extendSelection(tracks[1], tracks[2], tracks);
    expect(selectedPaths()).toEqual(["1.mp3", "2.mp3"]);
  });

  it("keeps the selection when a folder row is passed through", () => {
    playerState.extendSelection(null, tracks[0], tracks);
    playerState.extendSelection(tracks[0], null, tracks);
    expect(selectedPaths()).toEqual(["0.mp3"]);
  });
});
