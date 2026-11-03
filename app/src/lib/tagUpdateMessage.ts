import type { TagUpdateResult } from "./types";

const MAX_LISTED_FAILURES = 5;

export const ARTWORK_IMAGE_TYPES = ["image/jpeg", "image/png", "image/gif"];

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function describeTagUpdate({ updated, failed }: TagUpdateResult): {
  text: string;
  type: "success" | "error";
} {
  if (failed.length === 0) {
    return { text: `✓ ${updated.length}件のプロパティを保存しました`, type: "success" };
  }

  const summary =
    updated.length > 0
      ? `${updated.length}件を保存しました。${failed.length}件は保存できず、元のままです。`
      : `${failed.length}件を保存できませんでした。ファイルは元のままです。`;
  const listed = failed
    .slice(0, MAX_LISTED_FAILURES)
    .map((f) => `${fileName(f.file_path)}: ${f.message}`);
  const hidden = failed.length - listed.length;
  if (hidden > 0) {
    listed.push(`ほか ${hidden}件`);
  }
  return { text: [summary, ...listed].join("\n"), type: "error" };
}
