// Checks that the version is the same everywhere it is written. Run with `npm run release:check`.
// Node built-ins only.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SEMVER = /^\d+\.\d+\.\d+$/;
const CARGO_PACKAGE = "guif-player";

function cargoTomlVersion(text) {
  let inPackage = false;
  for (const line of text.split(/\r?\n/)) {
    const table = /^\s*\[([^\]]+)\]\s*$/.exec(line);
    if (table) {
      inPackage = table[1].trim() === "package";
      continue;
    }
    if (inPackage) {
      const m = /^\s*version\s*=\s*"([^"]*)"/.exec(line);
      if (m) return m[1];
    }
  }
  return null;
}

function cargoLockVersion(text) {
  const m = new RegExp(`name\\s*=\\s*"${CARGO_PACKAGE}"\\s*\\r?\\nversion\\s*=\\s*"([^"]*)"`).exec(text);
  return m ? m[1] : null;
}

/** The newest version heading in CHANGELOG.md, ignoring an "unreleased" one. */
function changelogVersion(text) {
  const m = /^## (\d+\.\d+\.\d+)/m.exec(text);
  return m ? m[1] : null;
}

export function readVersions(appDir) {
  const tauri = join(appDir, "src-tauri");
  const json = (p) => JSON.parse(readFileSync(p, "utf-8"));
  return {
    "tauri.conf.json": json(join(tauri, "tauri.conf.json")).version ?? null,
    "package.json": json(join(appDir, "package.json")).version ?? null,
    "package-lock.json": json(join(appDir, "package-lock.json")).version ?? null,
    "Cargo.toml": cargoTomlVersion(readFileSync(join(tauri, "Cargo.toml"), "utf-8")),
    "Cargo.lock": cargoLockVersion(readFileSync(join(tauri, "Cargo.lock"), "utf-8")),
    "CHANGELOG.md": changelogVersion(readFileSync(join(appDir, "..", "CHANGELOG.md"), "utf-8")),
  };
}

export function checkVersions(versions) {
  const errors = [];
  const reference = versions["tauri.conf.json"];
  for (const [file, version] of Object.entries(versions)) {
    if (typeof version !== "string" || !SEMVER.test(version)) {
      errors.push(`${file}: version ${JSON.stringify(version)} is not MAJOR.MINOR.PATCH`);
    } else if (version !== reference) {
      errors.push(`${file}: version ${version} differs from tauri.conf.json ${reference}`);
    }
  }
  return errors;
}

function main() {
  const appDir = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
  const versions = readVersions(appDir);
  for (const [file, version] of Object.entries(versions)) console.log(`${file}: ${version}`);
  const errors = checkVersions(versions);
  if (errors.length > 0) {
    for (const e of errors) console.error(`ERROR ${e}`);
    process.exit(1);
  }
  console.log(`OK all versions agree: ${versions["tauri.conf.json"]}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
