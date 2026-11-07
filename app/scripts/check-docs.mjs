// Checks that the documents stay true: every relative link points at something that exists,
// and every test named in specs/ is really in the code. Node built-ins only.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const SKIPPED = new Set(["node_modules", "target", "build", ".svelte-kit", ".git", "gen"]);

function walk(dir, matches, out = []) {
  for (const name of readdirSync(dir)) {
    if (SKIPPED.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path, matches, out);
    else if (matches(name)) out.push(path);
  }
  return out;
}

const problems = [];
const documents = walk(ROOT, (name) => name.endsWith(".md") && name !== "THIRD_PARTY_NOTICES.md");
const code = walk(join(ROOT, "app"), (name) => name.endsWith(".rs") || name.endsWith(".test.ts"))
  .map((path) => readFileSync(path, "utf-8"))
  .join("\n");

for (const document of documents) {
  const text = readFileSync(document, "utf-8");
  const where = relative(ROOT, document);

  for (const [, target] of text.matchAll(/\]\(([^)\s]+)\)/g)) {
    if (/^[a-z]+:/i.test(target) || target.startsWith("#")) continue;
    const path = target.split("#")[0];
    if (!existsSync(resolve(dirname(document), path))) {
      problems.push(`${where}: broken link: ${target}`);
    }
  }

  if (where.startsWith("specs") && !where.endsWith("README.md")) {
    for (const line of text.split("\n")) {
      const cells = line.split("|");
      if (cells.length < 5 || !/^[A-Z]-\d+$/.test(cells[1].trim())) continue;
      const verification = cells[3];
      const tests = [...verification.matchAll(/`([^`]+)`/g)].map((m) => m[1]);
      if (tests.length === 0 && !verification.includes("手動")) {
        problems.push(`${where}: ${cells[1].trim()} says nothing about how it is checked`);
      }
      for (const test of tests) {
        if (!code.includes(test)) {
          problems.push(`${where}: ${cells[1].trim()}: no such test: ${test}`);
        }
      }
    }
  }
}

if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`${documents.length} documents checked`);
