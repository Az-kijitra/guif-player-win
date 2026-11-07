// Playback speed choices. The backend (speed.rs) holds the same list and the speed itself.

export const PLAYBACK_SPEEDS = [0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0];

export function formatSpeed(speed: number): string {
  return `${speed.toFixed(1)}x`;
}

/** What the playback bar shows: nothing at normal speed. */
export function speedLabel(speed: number): string {
  return speed === 1 ? "" : formatSpeed(speed);
}

/** One step slower or faster, stopping at both ends. */
export function stepSpeed(speed: number, step: 1 | -1): number {
  const index = PLAYBACK_SPEEDS.findIndex((s) => Math.abs(s - speed) < 0.01);
  const from = index < 0 ? PLAYBACK_SPEEDS.indexOf(1.0) : index;
  const next = Math.min(PLAYBACK_SPEEDS.length - 1, Math.max(0, from + step));
  return PLAYBACK_SPEEDS[next];
}
