import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "alpha ", "alpha  ", "alpha   ", "alpha \n", "alpha  \n", "alpha   \nbeta\n", "alpha  \nbeta\n",
  "alpha\t\n", "alpha\u00a0\n", "alpha\uFEFF\n", "alpha\u2028\n", "\uFEFFalpha \r\n", "alpha \rbeta \r",
  "# heading \n", "# heading  \n", "# heading  ### \n", "heading \n--- \n", "*** \n", "***  \n",
  "```md \ncode \n``` \n", "```\ncode \n", "    code \n", "```\ncode  \n```\n",
  "| a | \n| - | \n| b | \n", "`alpha  \nbeta`\n", "alpha `code  \nmore` tail\n",
  "- item \n", "- item\n  \n  continuation \n", "- first\n  \n- second \n", "- item\n  \n",
  "> alpha \n> beta\n", "---\ntitle: Hi \n---\n\nalpha \n",
  "<!-- markdownlint-disable-next-line MD009 -->\nalpha \n", "<!-- markdownlint-disable-file whitespace -->\nalpha \n",
  "<!-- markdownlint-disable-next-line MD009 -->\nalpha beta gamma ",
  "> 1.\t---\n >  \t# Heading \n", " > - root \n>   - child \n", "-\troot \n  - child \n",
  "> ```md \n> code \n> ``` \n", "- one\n\t\n  continuation \n",
  "!\n-- \n", "- > a \n  > \n", "- > > a \n  > > \n", "- > a \n  > \n  \n",
  "> ```\n> x \n", "- ```\n  x \n", "> - ```\n>   x \n", "> ```\n> x \n> ``` \n",
  "- ```\n  x \n  ``` \n", "```\ncode ",
];
const configurations = [true, false, {br_spaces:0}, {br_spaces:1}, {br_spaces:3}, {strict:true},
  {code_blocks:true}, {list_item_empty_lines:true}, {strict:true,list_item_empty_lines:true}, {severity:"warning"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD009:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      length: finding.errorRange[1], detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md009-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md009-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
let preservedDifferences = 0;
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD009:row.options} }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    if (!fix) assert.equal(native.stdout, row.input, row.id);
    const expected = lint({ strings: { input: native.stdout }, config: {default:false,MD009:row.options} }).input
      .map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0], detail: finding.errorDetail, severity: finding.severity }));
    const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD009: Trailing spaces \[(.*)\]$/gm)]
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
console.log(`PASS: ${observations.length} pinned MD009/native diagnostic and idempotence cases; ${preservedDifferences} preserved upstream-fix differences`);
