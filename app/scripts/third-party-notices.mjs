// Writes THIRD_PARTY_NOTICES.md (the list) and THIRD_PARTY_LICENSES.txt (the texts, shipped in the installer)
// at the repository root from the locked dependencies.
// Run it with `npm run notices` after changing dependencies. Node built-ins only.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const APP_DIR = join(dirname(fileURLToPath(import.meta.url)), "..");
const ROOT = join(APP_DIR, "..");
const WINDOWS_TARGET = "x86_64-pc-windows-msvc";
const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;

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
  return [...seen].map((id) => {
    const p = byId.get(id);
    return { ...p, folder: dirname(p.manifest_path) };
  });
}

function npmPackages() {
  const lock = JSON.parse(readFileSync(join(APP_DIR, "package-lock.json"), "utf-8"));
  return Object.entries(lock.packages)
    .filter(([path, p]) => path && !p.dev)
    .map(([path, p]) => ({
      name: path.replace(/^.*node_modules\//, ""),
      version: p.version,
      license: p.license,
      folder: join(APP_DIR, path),
    }));
}

// SvelteKit's and Svelte's runtime code ends up in the bundle, although they are build tools to npm
function bundledBuildTools() {
  return ["svelte", "@sveltejs/kit"].map((name) => {
    const folder = join(APP_DIR, "node_modules", name);
    const p = JSON.parse(readFileSync(join(folder, "package.json"), "utf-8"));
    return { name, version: p.version, license: p.license, folder };
  });
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

function licenseTexts(packages) {
  const byText = new Map();
  const missing = [];
  for (const p of packages) {
    const files = existsSync(p.folder)
      ? readdirSync(p.folder).filter((name) => LICENSE_FILE.test(name) && !/\.(rs|toml|json|js|ts)$/i.test(name))
      : [];
    if (files.length === 0) missing.push(p);
    for (const file of files) {
      let text;
      try {
        text = readFileSync(join(p.folder, file), "utf-8").replaceAll("\r\n", "\n").trim();
      } catch {
        continue; // a folder of that name
      }
      if (!byText.has(text)) byText.set(text, new Set());
      byText.get(text).add(`${p.name} ${p.version}`);
    }
  }
  return { byText, missing };
}

const rust = rustPackages();
const npm = npmPackages();
const shipped = [...rust, ...npm, ...bundledBuildTools()];
const mplNames = rust
  .filter((p) => p.license?.includes("MPL"))
  .map((p) => p.name)
  .sort();

writeFileSync(
  join(ROOT, "THIRD_PARTY_NOTICES.md"),
  `# 第三者ソフトウェアの一覧

このファイルは \`npm run notices\` が依存関係から生成します。直接編集しないでください。

guif-player 本体のライセンスは [MIT License](LICENSE) です。配布物には次のソフトウェアが含まれ、それぞれ記載のライセンスに従います。各ライセンスの本文は [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) にあり、インストール先にも置いています。

## MPL-2.0 のソフトウェア

次のソフトウェアは Mozilla Public License 2.0 に従います。変更せずに利用しており、ソースコードは下の表の「ソース」から入手できます。

${mplNames.map((name) => `- ${name}`).join("\n")}

## Rust のソフトウェア

${table(rust)}

## フロントエンドのソフトウェア

${table([...npm, ...bundledBuildTools()])}
`
);

const { byText, missing } = licenseTexts(shipped);
const LINE = "=".repeat(78);
const sections = [...byText.entries()]
  .map(([text, names]) => ({ names: [...names].sort(), text }))
  .sort((a, b) => a.names[0].localeCompare(b.names[0]))
  .map(({ names, text }) => `${LINE}\n${names.join("\n")}\n${LINE}\n\n${text}\n`);
const withoutFile = missing.length
  ? `${LINE}\nThese packages ship no license file; their declared licenses are:\n${LINE}\n\n${missing
      .map((p) => `${p.name} ${p.version}: ${p.license ?? "unknown"}`)
      .sort()
      .join("\n")}\n`
  : "";

writeFileSync(
  join(ROOT, "THIRD_PARTY_LICENSES.txt"),
  `guif-player includes the following third-party software.\n\n${sections.join("\n")}\n${withoutFile}`
);
console.log(`${shipped.length} packages, ${byText.size} license texts, ${missing.length} without a license file`);
