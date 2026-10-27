import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "", "hello", "hello\n", "hello\r\n", "hello\r", "one\r\ntwo", "one\rtwo",
  " ", "hello ", "\t", "\u00a0", "\uFEFF", "\uFEFFhello", "😀", "# Heading!", "hello\n\n",
  ">", "> >", "> <!-- comment -->", "<!-- comment -->", "<!-- comment", "tail -->",
  "hello <!-- comment -->", "<!-- one --><!-- two -->", "<!-- comment\ntail", "hello\n>",
  "---\ntitle: hello\n---", "---\ntitle: hello\n---\n", "```\ncode", "```\ncode\n```", "    code",
  "<div>\nhello", "<div>hello</div>", "| a |\n| - |\n| b |", "`code`", "[link](url)",
  "<!-- markdownlint-disable-next-line MD047 -->\nhello", "hello <!-- markdownlint-disable-line MD047 -->",
  "<!-- markdownlint-disable-file MD047 -->\nhello", "hello\u2028world", "\u000b", "😀 ",
];
const configurations = [true, false, { severity: "warning" }];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: { default: false, MD047: options } }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md047-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md047-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
// Literal EOF boundaries and unsupported line-separator documents are preserved.
const preserved = new Set([24, 28, 30, 31, 32, 33, 39]);
try { for (const row of observations) {
  const sample = Number(row.id.split("-")[0]);
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: { default: false, MD047: row.options } }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    const output = !fix || preserved.has(sample) ? row.input : row.output;
    assert.equal(native.stdout, output, `${row.id}: fix ${fix}: ${native.stderr}`);
    const expected = (!fix || preserved.has(sample) ? row.findings : []).map(finding => {
      // Native spans cannot point inside an astral scalar's UTF-8 bytes.
      const last = [...row.input].at(-1);
      return { line: finding.line, column: finding.column - (last?.length === 2 ? 1 : 0), severity: finding.severity };
    });
    const findings = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD047: Files should end with a single newline character$/gm)]
      .map(match => ({ line: Number(match[1]), column: Number(match[2]), severity: match[3].toLowerCase() }));
    assert.deepEqual(findings, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD047 observations and native diagnostic/safe-fix comparisons`);
