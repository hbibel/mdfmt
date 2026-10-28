import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "alpha\tbeta\n", "alpha\t\tbeta\n", "alpha \t beta\n", "alpha\t\t\n", "alpha\t\nbeta\n", "\talpha\n",
  "`code\tvalue`\n", "`code`\toutside\n", "`code` \toutside\n", "alpha `code\n\tmore` end\n",
  "```js\n\tcode\n```\n", "```JS\ncode\tvalue\n```\n", "```js extra\n\tcode\n```\n",
  "```make\n\tcode\n```\n", "```\n\tcode\n```\n", "```j&#115;\n\tcode\n```\n", "```js\t\ncode\n```\t\n",
  "```js\n\tcode", "    code\tvalue\n", "| a\tb |\n| --- |\n| data |\n", "[label\tvalue](url)\n",
  "-\titem\n  - child\n", "> \talpha\n", "\uFEFF😀\talpha\r\n", "alpha\tbeta\rnext\tline\r",
  "---\ntitle:\tHi\n---\n\nalpha\tbeta\n", "<!-- markdownlint-disable-next-line MD010 -->\nalpha\tbeta\n",
  "<!-- markdownlint-disable-file hard_tab -->\nalpha\tbeta\n", "- * *\t\n",
  "> ```js\n> code\tvalue\n", "> ```js\n> code\tvalue\n> ```\t\n",
  "- ```js\n  code\tvalue\n", "> - ```js\n>   code\tvalue\n",
  "    \tcode\n", "\tcode\tvalue\n", "- item\n\n      code\tvalue\n",
  "`code`\t\t\n", "\t`code`\n", "alpha `a\tb`\tend\n",
  "```JS extra\ncode\tvalue\n```\n", "```js\ncode\tvalue\n", "~~~make\ncode\tvalue\n~~~\n",
];
const configurations = [true, false, {spaces_per_tab:0}, {spaces_per_tab:2}, {spaces_per_tab:4},
  {code_blocks:false}, {ignore_code_languages:["js"]}, {code_blocks:false,ignore_code_languages:["make"]},
  {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD010:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md010-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md010-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
let preservedDifferences = 0;
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD010:row.options} }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    if (!fix) assert.equal(native.stdout, row.input, row.id);
    const expected = lint({ strings: { input: native.stdout }, config: {default:false,MD010:row.options} }).input
      .map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0], detail: finding.errorDetail, severity: finding.severity }));
    const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD010: Hard tabs \[(.*)\]$/gm)]
      .map(match => ({ line: Number(match[1]), column: Number(match[2]), detail: match[4], severity: match[3].toLowerCase() }));
    assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    if (fix) {
      if (native.stdout !== row.output) preservedDifferences++;
      const second = spawnSync(binary, ["-"], { input: native.stdout, encoding: "utf8", cwd: directory });
      assert.equal(second.stdout, native.stdout, `${row.id}: idempotence`);
      assert.equal(second.status, native.status, `${row.id}: stable status`);
    }
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD010/native diagnostic and idempotence cases; ${preservedDifferences} preserved upstream-fix differences`);
