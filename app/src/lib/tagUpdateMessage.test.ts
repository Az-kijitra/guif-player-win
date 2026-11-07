import { describe, expect, it } from "vitest";
import { describeTagUpdate } from "./tagUpdateMessage";
import type { AudioTrack, TagUpdateFailure } from "./types";

const track = { file_path: "a.mp3" } as AudioTrack;
const failure = (n: number): TagUpdateFailure => ({
  file_path: `C:\\Music\\${n}.mp3`,
  message: "読み取り専用のファイルです",
});

describe("describeTagUpdate", () => {
  it("reports success when every file was saved", () => {
    expect(describeTagUpdate({ updated: [track, track], failed: [] })).toEqual({
      text: "✓ 2件のプロパティを保存しました",
      type: "success",
    });
  });

  it("reports both counts and names the failed file when some were saved", () => {
    const { text, type } = describeTagUpdate({ updated: [track], failed: [failure(1)] });
    expect(type).toBe("error");
    expect(text).toBe(
      "1件を保存しました。1件は保存できず、元のままです。\n1.mp3: 読み取り専用のファイルです"
    );
  });

  it("says nothing changed when every file failed", () => {
    const { text, type } = describeTagUpdate({ updated: [], failed: [failure(1)] });
    expect(type).toBe("error");
    expect(text).toContain("1件を保存できませんでした。ファイルは元のままです。");
  });

  it("shortens a long list of failures", () => {
    const failed = [1, 2, 3, 4, 5, 6, 7].map(failure);
    const { text } = describeTagUpdate({ updated: [], failed });
    expect(text).toContain("5.mp3");
    expect(text).not.toContain("6.mp3");
    expect(text).toContain("ほか 2件");
  });
});
