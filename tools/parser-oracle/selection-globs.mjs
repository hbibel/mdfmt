import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
const modules = fileURLToPath(new URL("node_modules/", import.meta.url));
const require = createRequire(path.join(modules, "../package.json"));
assert.equal(JSON.parse(readFileSync(path.join(modules, "markdownlint-cli2/package.json"))).version, "0.24.0");
const cliFile = path.join(modules, "markdownlint-cli2/markdownlint-cli2.mjs");
let source = readFileSync(cliFile, "utf8").replace("import { lint, extendConfig, readConfig }", "import { lint as originalLint, extendConfig, readConfig }");
source = source.replace(/(from\s+|import\(\s*)(["'])([^"']+)\2/g, (_, prefix, quote, specifier) => `${prefix}${quote}${specifier.startsWith("node:") ? specifier : specifier.startsWith(".") ? new URL(specifier, pathToFileURL(cliFile)).href : pathToFileURL(require.resolve(specifier)).href}${quote}`);
source += "\nconst lint = (options) => { globalThis.__mdfmtSelectionCalls.push(...options.files); return originalLint(options); };\n";
const { main } = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
const generated = JSON.parse(readFileSync(new URL("../../tests/fixtures/configuration/generated-globs.json", import.meta.url)));
const files = ["a.md", "aa.md", "ab.md", "b.md", ".hidden.md", "1.md", "2.md", "é.md", "🦀.md", "*.md", "[a].md", "a/b.md", "a/x.md", "a/b/x.md", "b/a.md", "x/a.md", "foo.test.md", "foo.md", "a..md", "a+b.md", "aaaa.md", 'a"b".md'];
const root = mkdtempSync("/tmp/opencode/mdfmt-selection-");
const cases = [];
function literalPattern(pattern) {
  let escaped = false;
  for (const ch of pattern) {
    if (escaped) { escaped = false; continue; }
    if (ch === "\\") { escaped = true; continue; }
    if ("*?[{(".includes(ch)) return false;
  }
  return !pattern.startsWith("!");
}
try {
  for (const file of files) { const name = path.join(root, file); mkdirSync(path.dirname(name), { recursive: true }); writeFileSync(name, "text\n"); }
  for (const { pattern } of generated.cases) {
    globalThis.__mdfmtSelectionCalls = [];
    await main({ directory: root, argv: [pattern], optionsOverride: { fix: false, outputFormatters: [], noBanner: true, noProgress: true } });
    const selected = [...new Set(globalThis.__mdfmtSelectionCalls.map(file => path.relative(root, file).split(path.sep).join("/")))].sort();
    cases.push({ pattern, files: selected, missingNamedInput: selected.length === 0 && literalPattern(pattern) });
  }
} finally { rmSync(root, { recursive: true, force: true }); }
const output = { files, cases };
const file = new URL("../../tests/fixtures/configuration/selection-globs.json", import.meta.url);
if (process.argv.includes("--record")) writeFileSync(file, JSON.stringify(output, null, 2) + "\n");
else assert.deepEqual(output, JSON.parse(readFileSync(file)));
console.log(`PASS: ${cases.length} actual cli2 selection cases over ${files.length} files (${cases.filter(c => c.missingNamedInput).length} explicit missing-input policy cases)`);
