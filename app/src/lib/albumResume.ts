// Where an album's "resume" starts. The same rules as ResumePolicy in the Android version.
import type { AlbumResume, AudioTrack, TrackResume } from "./types";

export interface ResumePoint {
  index: number;
  positionMs: number;
}

/**
 * The last track left unfinished resumes at its position. A finished one is followed by the next
 * track from its start, and the end of the album starts it over.
 */
export function albumResumePoint(
  trackCount: number,
  lastIndex: number,
  lastIsCompleted: boolean,
  positionMs: number
): ResumePoint {
  if (lastIndex < 0 || lastIndex >= trackCount) return { index: 0, positionMs: 0 };
  if (!lastIsCompleted) return { index: lastIndex, positionMs: Math.max(0, positionMs) };
  if (lastIndex + 1 < trackCount) return { index: lastIndex + 1, positionMs: 0 };
  return { index: 0, positionMs: 0 };
}

/**
 * Where the folder's tracks resume from, or null when nothing in them has been played
 * (or the track played last is no longer there).
 */
export function folderResumePoint(
  tracks: AudioTrack[],
  albumResume: AlbumResume | null,
  trackResumes: Record<string, TrackResume>
): ResumePoint | null {
  if (!albumResume) return null;
  const lastIndex = tracks.findIndex((t) => t.file_path === albumResume.last_file_path);
  if (lastIndex < 0) return null;
  const completed = trackResumes[albumResume.last_file_path]?.is_completed ?? false;
  return albumResumePoint(tracks.length, lastIndex, completed, albumResume.position_ms);
}
