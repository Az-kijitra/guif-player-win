// Builds the NSIS installer and records its checksum. Run with `npm run release:build`.
// The first build needs the network: Tauri downloads NSIS into %LOCALAPPDATA%\tauri\NSIS.
// The installer is only built here, never executed. Node built-ins and versions.mjs only.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkVersions, readVersions } from "./versions.mjs";

const appDir = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const tauriDir = join(appDir, "src-tauri");
const tauriJs = join(appDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const nsisDir = join(process.env.CARGO_TARGET_DIR ?? join(tauriDir, "target"), "release", "bundle", "nsis");

function fail(message) {
  console.error(`ERROR ${message}`);
  process.exit(1);
}

const errors = checkVersions(readVersions(appDir));
if (errors.length > 0) fail(errors.join("\n"));

const conf = JSON.parse(readFileSync(join(tauriDir, "tauri.conf.json"), "utf-8"));
const expected = `${conf.productName}_${conf.version}_x64-setup.exe`;

// Leave exactly one installer behind
if (existsSync(nsisDir)) {
  for (const file of readdirSync(nsisDir)) {
    if (/-setup\.exe(\.sha256)?$/.test(file)) unlinkSync(join(nsisDir, file));
  }
}

execFileSync(process.execPath, [tauriJs, "build", "--bundles", "nsis"], { cwd: appDir, stdio: "inherit" });

const installers = existsSync(nsisDir) ? readdirSync(nsisDir).filter((f) => f.endsWith("-setup.exe")) : [];
if (installers.length !== 1 || installers[0] !== expected) {
  fail(`expected only ${expected} in ${nsisDir}, found ${JSON.stringify(installers)}`);
}

const installer = join(nsisDir, expected);
const hex = createHash("sha256").update(readFileSync(installer)).digest("hex");
const line = `${hex} *${expected}`;
writeFileSync(`${installer}.sha256`, `${line}\n`, "utf-8");
console.log(`installer: ${installer}`);
console.log(`sha256: ${line}`);
