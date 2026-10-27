import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";

const samples = [
  "***\n\n---\n", "---\n\n***\n", "***\n\n* * *\n", "  ***\n\n---\n", "***  \n\n---\n",
  "> ***\n>\n> ---\n", "- ***\n\n  ---\n", "alpha\n---\n\n***\n", "---\ntitle: Hi\n---\n\n***\n\n---\n",
  "```\n***\n---\n```\n", "| *** |\n| --- |\n| --- |\n", "\uFEFF***\r\n\r\n---\r\n",
  "<!-- markdownlint-disable-next-line MD035 -->\n***\n\n---\n", "<!-- markdownlint-disable-file MD035 -->\n***\n\n---\n",
  "*** <!-- markdownlint-disable-line MD035 -->\n\n---\n", "___\n\n- - -\n\n* * *\n", "***\n\n\t---\n",
];
const configurations = [true, false, {style:"---"}, {style:"* * *"}, {style:""}, {style:"   "},
  {style:"# heading"}, {style:"\u00a0---\u00a0"}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => ({
  id: `${sample}-${configuration}`, input, options,
  findings: lint({ strings: { input }, config: {default:false,MD035:options} }).input.map(finding => ({
    line: finding.lineNumber, detail: finding.errorDetail, severity: finding.severity,
  })),
})));
const receipt = new URL("md035-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md035-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD035:row.options} }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    if (!fix) assert.equal(native.stdout, row.input, row.id);
    const expected = lint({ strings: { input: native.stdout }, config: {default:false,MD035:row.options} }).input
      .map(finding => ({ line: finding.lineNumber, detail: finding.errorDetail, severity: finding.severity }));
    const actual = [...native.stderr.matchAll(/^-:(\d+):\d+: (Error|Warning) MD035: Horizontal rule style \[(.*)\]$/gm)]
      .map(match => ({ line: Number(match[1]), detail: match[3], severity: match[2].toLowerCase() }));
    assert.deepEqual(actual, expected, `${row.id}: fix ${fix}: ${native.stderr}`);
    assert.equal(native.status, expected.some(finding => finding.severity === "error") ? 1 : 0, native.stderr);
    if (fix) {
      const second = spawnSync(binary, ["-"], { input: native.stdout, encoding: "utf8", cwd: directory });
      assert.equal(second.stdout, native.stdout, `${row.id}: idempotence`);
      assert.equal(second.status, native.status, `${row.id}: stable status`);
    }
  }
} } finally { unlinkSync(configuration); rmdirSync(directory); }
console.log(`PASS: ${observations.length} pinned MD035/native diagnostic comparisons and native repair idempotence cases`);
