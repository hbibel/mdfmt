import assert from "node:assert/strict";
import { readFileSync, mkdtempSync, writeFileSync, unlinkSync, rmdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { lint } from "markdownlint/sync";

const fixtures = JSON.parse(readFileSync(new URL("../../tests/fixtures/reflow.json", import.meta.url)));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const blockAndInline = new Set([
  "content", "paragraph", "atxHeading", "setextHeading", "listOrdered", "listUnordered",
  "blockQuote", "codeFenced", "codeIndented", "htmlFlow", "table", "definition",
  "link", "image", "codeText", "emphasis", "strong", "htmlText", "hardBreakEscape", "hardBreakTrailing",
  "mathText", "mathFlow", "directiveContainer", "directiveLeaf", "directiveText"
]);
const atomic = new Set(["link", "image", "codeText", "emphasis", "strong", "htmlText", "htmlFlow",
  "codeFenced", "codeIndented", "table", "definition", "hardBreakEscape", "hardBreakTrailing",
  "mathText", "mathFlow", "directiveContainer", "directiveLeaf", "directiveText"]);

function observe(text, fixture) {
  let structure;
  const findings = lint({ strings: { text }, noInlineConfig: true,
    config: { default: false, MD013: { line_length: fixture.width, strict: fixture.strict ?? false }, capture: true },
    customRules: [{ names: ["capture"], description: "Capture reflow structure", tags: ["observation"], parser: "micromark",
      function(params) {
        function visit(tokens) {
          return tokens.flatMap(token => {
            if (atomic.has(token.type)) return [[token.type, token.text]];
            if (token.type === "data") return [["data", token.text.replace(/\s+/gu, " ").trim()]];
            const children = visit(token.children);
            return blockAndInline.has(token.type) ? [[token.type, "open"], ...children, [token.type, "close"]] : children;
          });
        }
        structure = [];
        for (const item of visit(params.parsers.micromark.tokens)) {
          const last = structure.at(-1);
          if (item[0] === "data" && last?.[0] === "data") last[1] += " " + item[1];
          else structure.push(item);
        }
      }
    }]
  }).text;
  assert.ok(structure.length, fixture.id);
  return { structure, lines: findings.map(finding => finding.lineNumber) };
}

const directory = mkdtempSync("/tmp/opencode/mdfmt-reflow-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
try { for (const fixture of fixtures) {
  writeFileSync(configuration, JSON.stringify({ config: { default: false, MD013: { line_length: fixture.width, strict: fixture.strict ?? false } } }));
  const formatted = spawnSync(binary, ["-"], { input: fixture.input, encoding: "utf8", cwd: directory });
  assert.equal(formatted.status, fixture.after.length ? 1 : 0, formatted.stderr);
  assert.equal(formatted.stdout, fixture.output, fixture.id);
  const nativeLines = [...formatted.stderr.matchAll(/^-:(\d+):\d+: (?:Error|Warning) MD013:/gm)].map(match => Number(match[1]));
  assert.deepEqual(nativeLines, fixture.after, `${fixture.id}: native remaining MD013`);
  const second = spawnSync(binary, ["-"], { input: fixture.output, encoding: "utf8", cwd: directory });
  assert.equal(second.status, formatted.status, second.stderr);
  assert.equal(second.stdout, fixture.output, `${fixture.id}: idempotence`);
  const before = observe(fixture.input, fixture);
  const after = observe(fixture.output, fixture);
  assert.deepEqual(after.structure, before.structure, `${fixture.id}: pinned parser structure`);
  assert.deepEqual(before.lines, fixture.before, `${fixture.id}: original MD013`);
  assert.deepEqual(after.lines, fixture.after, `${fixture.id}: remaining MD013`);
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${fixtures.length} native reflow outputs, idempotence, and pinned upstream parser/MD013 comparisons`);
