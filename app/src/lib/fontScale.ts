// Text size. The Android version scales the text only; on the desktop the whole page is zoomed,
// as a browser does, so the layout keeps its proportions.

export type FontScaleId = "small" | "normal" | "large" | "extra_large";

export const FONT_SCALES: { id: FontScaleId; label: string; scale: number }[] = [
  { id: "small", label: "小", scale: 0.85 },
  { id: "normal", label: "標準", scale: 1.0 },
  { id: "large", label: "大", scale: 1.15 },
  { id: "extra_large", label: "特大", scale: 1.3 },
];

/** Anything unknown, such as a value saved by a newer version, is the normal size. */
export function fontScaleFromId(id: string | null | undefined): FontScaleId {
  return FONT_SCALES.find((s) => s.id === id)?.id ?? "normal";
}

export function zoomOf(id: FontScaleId): number {
  return FONT_SCALES.find((s) => s.id === id)?.scale ?? 1;
}

/** One size up or down, stopping at both ends. */
export function stepFontScale(id: FontScaleId, step: 1 | -1): FontScaleId {
  const index = FONT_SCALES.findIndex((s) => s.id === id);
  const next = Math.min(FONT_SCALES.length - 1, Math.max(0, index + step));
  return FONT_SCALES[next].id;
}
