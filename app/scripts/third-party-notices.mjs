// Writes THIRD_PARTY_NOTICES.md at the repository root from the locked dependencies.
// Run it with `npm run notices` after changing dependencies. Node built-ins only.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const APP_DIR = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUTPUT = join(APP_DIR, "..", "THIRD_PARTY_NOTICES.md");
const WINDOWS_TARGET = "x86_64-pc-windows-msvc";

function rustPackages() {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--locked", "--format-version", "1", "--filter-platform", WINDOWS_TARGET],
      { cwd: join(APP_DIR, "src-tauri"), maxBuffer: 256 * 1024 * 1024, encoding: "utf-8" }
    )
  );
  // Follow normal and build dependencies only: dev dependencies are not distributed
  const byId = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set();
  const visit = (id) => {
    if (seen.has(id)) return;
    seen.add(id);
    for (const dep of nodes.get(id).deps) {
      if (dep.dep_kinds.some((kind) => kind.kind !== "dev")) visit(dep.pkg);
    }
  };
  visit(metadata.resolve.root);
  seen.delete(metadata.resolve.root);
  return [...seen].map((id) => byId.get(id));
}

function npmPackages() {
  const lock = JSON.parse(readFileSync(join(APP_DIR, "package-lock.json"), "utf-8"));
  return Object.entries(lock.packages)
    .filter(([path, p]) => path && !p.dev)
    .map(([path, p]) => ({
      name: path.replace(/^.*node_modules\//, ""),
      version: p.version,
      license: p.license,
    }));
}

function table(packages) {
  const rows = packages
    .map((p) => {
      const license = (p.license ?? "不明").replaceAll("|", "/");
      const source = p.repository ?? "";
      return `| ${p.name} | ${p.version} | ${license} | ${source} |`;
    })
    .sort((a, b) => a.localeCompare(b));
  return ["| 名前 | 版 | ライセンス | ソース |", "|---|---|---|---|", ...rows].join("\n");
}

const rust = rustPackages();
const mplNames = rust
  .filter((p) => p.license?.includes("MPL"))
  .map((p) => p.name)
  .sort();

writeFileSync(
  OUTPUT,
  `# 第三者ソフトウェアの一覧

このファイルは \`npm run notices\` が依存関係から生成します。直接編集しないでください。

guif-player 本体のライセンスは [MIT License](LICENSE) です。配布物には次のソフトウェアが含まれ、それぞれ記載のライセンスに従います。

## MPL-2.0 のソフトウェア

次のソフトウェアは Mozilla Public License 2.0 に従います。変更せずに利用しており、ソースコードは下の表の「ソース」から入手できます。

${mplNames.map((name) => `- ${name}`).join("\n")}

## Rust のソフトウェア

${table(rust)}

## フロントエンドのソフトウェア

${table(npmPackages())}
`
);
console.log(`Wrote ${OUTPUT}`);
