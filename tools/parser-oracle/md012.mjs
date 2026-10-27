import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "", "\n", "\n\n", "alpha\n", "alpha\n\n", "alpha\n\n\n", "\n\nalpha\n",
  "alpha\n\n\n# beta\n", "alpha\n\n\nbeta\n", "alpha\r\n\r\n\r\n# beta\r\n",
  "alpha\r\r\r# beta\r", "alpha\n\n\u00a0\n\n# beta", "alpha\n \n\t\n# beta",
  "> alpha\n>\n>\n> beta", "```\n\n\n```\n\n\n", "    alpha\n\n\n    beta\n",
  "- alpha\n\n- beta\n", "- alpha\n\n\n- beta\n", "| a |\n| - |\n| b |\n\n\n# beta\n",
  "<div>\n\n\nhello\n</div>\n", "---\ntitle: hi\n---\n\n\nhello", "\uFEFFalpha\n\n\n# beta",
  "<!-- markdownlint-disable-file MD012 -->\n\n\nalpha",
  "alpha\n\n<!-- markdownlint-disable-next-line MD012 -->\n\n\n# beta",
  "alpha\n\n\n<!-- markdownlint-disable-line MD012 -->\n# beta",
];
const configurations = [true, false, { maximum: 0 }, { maximum: 2 }, { severity: "warning" }];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: { default: false, MD012: options } }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md012-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md012-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
const preserved = new Set([17, 19, 24]);
try { for (const row of observations) {
  const sample = Number(row.id.split("-")[0]);
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: { default: false, MD012: row.options } }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    if (row.options.maximum === 0) {
      assert.equal(native.status, 2, `${row.id}: schema rejects maximum zero`);
      assert.equal(native.stdout, "");
      continue;
    }
    const preserve = preserved.has(sample) || (sample === 11 && row.options.maximum !== 2);
    const output = !fix || preserve ? row.input : row.output;
    assert.equal(native.stdout, output, `${row.id}: fix ${fix}: ${native.stderr}`);
    const expected = lint({ strings: { input: output }, config: { default: false, MD012: row.options } }).input
      .map(finding => ({ line: finding.lineNumber, detail: finding.errorDetail, severity: finding.severity }));
    const actual = [...native.stderr.matchAll(/^-:(\d+):\d+: (Error|Warning) MD012: Multiple consecutive blank lines \[(.*)\]$/gm)]
      .map(match => ({ line: Number(match[1]), detail: match[3], severity: match[2].toLowerCase() }));
    assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD012 observations and native diagnostics/safe-fix/schema comparisons`);
