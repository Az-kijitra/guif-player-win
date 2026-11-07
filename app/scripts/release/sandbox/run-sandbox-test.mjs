// Tests the installer for real inside Windows Sandbox, leaving this PC and its data alone.
// Usage: npm run release:sandbox [-- <the 0.1.1 installer, to test updating from it>]
// Needs the "Windows Sandbox" Windows feature, and `npm run release:build` first.
// A sandbox window appears while it runs and closes by itself. Node built-ins only.
import { execFileSync, spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const appDir = resolve(HERE, "..", "..", "..");
const nsisDir = join(appDir, "src-tauri", "target", "release", "bundle", "nsis");
const TIMEOUT_MS = 8 * 60 * 1000;

const installer = readdirSync(nsisDir).find((f) => f.endsWith("-setup.exe"));
if (!installer) throw new Error(`no installer in ${nsisDir}: run npm run release:build first`);

const host = mkdtempSync(join(tmpdir(), "guif-player-sandbox-"));
mkdirSync(join(host, "installers"));
mkdirSync(join(host, "results"));
copyFileSync(join(nsisDir, installer), join(host, "installers", installer));
const oldInstaller = process.argv[2];
if (oldInstaller) copyFileSync(oldInstaller, join(host, "installers", basename(oldInstaller)));
copyFileSync(join(HERE, "sandbox-test.ps1"), join(host, "sandbox-test.ps1"));

// A database as 0.1.1 left it: no schema version
const legacy = new DatabaseSync(join(host, "legacy.db"));
legacy.exec(`
  CREATE TABLE album_resume (album_path TEXT PRIMARY KEY, last_file_path TEXT NOT NULL, position_ms INTEGER NOT NULL, updated_at INTEGER NOT NULL);
  CREATE TABLE track_resume (file_path TEXT PRIMARY KEY, album_path TEXT NOT NULL, position_ms INTEGER NOT NULL, duration_ms INTEGER NOT NULL, is_completed INTEGER NOT NULL, updated_at INTEGER NOT NULL);
  CREATE TABLE playlists (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at INTEGER NOT NULL);
  CREATE TABLE playlist_tracks (id INTEGER PRIMARY KEY AUTOINCREMENT, playlist_id INTEGER NOT NULL, file_path TEXT NOT NULL, title TEXT NOT NULL, artist TEXT NOT NULL, album TEXT NOT NULL, duration_ms INTEGER NOT NULL, sort_order INTEGER NOT NULL, FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE);
  CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
  INSERT INTO playlists (name, created_at) VALUES ('old favourites', 1);
`);
legacy.close();

const inside = "C:\\Users\\WDAGUtilityAccount\\Desktop\\work";
const config = join(host, "test.wsb");
writeFileSync(config, `<Configuration>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>${host}</HostFolder>
      <SandboxFolder>${inside}</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>powershell.exe -NoProfile -ExecutionPolicy Bypass -File ${inside}\\sandbox-test.ps1 -Work ${inside}</Command>
  </LogonCommand>
</Configuration>
`);

spawn("WindowsSandbox.exe", [config], { detached: true, stdio: "ignore" }).unref();

const done = join(host, "results", "done.txt");
const log = join(host, "results", "log.txt");
const started = Date.now();
while (!existsSync(done) && Date.now() - started < TIMEOUT_MS) {
  await new Promise((r) => setTimeout(r, 2000));
}
const finished = existsSync(done);

for (const name of ["WindowsSandboxRemoteSession.exe", "WindowsSandboxServer.exe", "WindowsSandbox.exe", "WindowsSandboxClient.exe"]) {
  try { execFileSync("taskkill", ["/IM", name, "/F"], { stdio: "ignore" }); } catch {}
}

console.log(existsSync(log) ? readFileSync(log, "utf-8").replace(/^\uFEFF/, "") : "no log was written");
console.log(finished ? "finished" : "TIMED OUT");
console.log(`results: ${join(host, "results")}`);
if (!finished || /^FAIL/m.test(existsSync(log) ? readFileSync(log, "utf-8") : "FAIL")) process.exit(1);
rmSync(join(host, "installers"), { recursive: true, force: true });
