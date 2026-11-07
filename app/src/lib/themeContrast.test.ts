import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync("src/app.css", "utf-8");

function themes(): Record<string, Record<string, string>> {
  const result: Record<string, Record<string, string>> = {};
  for (const [, name, body] of css.matchAll(/:root\[data-theme="(\w+)"\]\s*{([^}]*)}/g)) {
    result[name] = {};
    for (const [, key, value] of body.matchAll(/--(\w+):\s*(#[0-9a-fA-F]{6})/g)) {
      result[name][key] = value;
    }
  }
  return result;
}

const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));

function luminance([r, g, b]: number[]) {
  const channel = (value: number) => {
    const c = value / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function contrast(a: number[], b: number[]) {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

const mutedShare = (name: string) => {
  const body = css.match(new RegExp(`data-theme="${name}"\\]\\s*{([^}]*)}`))?.[1] ?? "";
  if (body.includes("--ui-text-muted: var(--nord4)")) return 1;
  const match = css.match(/--ui-text-muted:\s*color-mix\(in srgb, var\(--nord4\) (\d+)%, var\(--nord0\)\)/);
  if (!match) throw new Error("--ui-text-muted is not defined as expected");
  return Number(match[1]) / 100;
};

describe("muted text", () => {
  const all = themes();

  it("finds every theme", () => {
    expect(Object.keys(all).sort()).toEqual(
      ["catppuccin", "default", "light", "monokai", "nord", "solarized"]
    );
  });

  // 4.5:1 is the WCAG AA level for normal text
  it.each(Object.keys(all))("is readable on the panels of %s", (name) => {
    const theme = all[name];
    const share = mutedShare(name);
    const muted = rgb(theme.nord4).map((c, i) => c * share + rgb(theme.nord0)[i] * (1 - share));
    expect(contrast(muted, rgb(theme.nord0))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(muted, rgb(theme.nord1))).toBeGreaterThanOrEqual(3);
  });
});
