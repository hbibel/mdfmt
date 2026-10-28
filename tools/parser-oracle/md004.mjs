import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { lint } from "markdownlint/sync";
import { applyFixes } from "markdownlint";

const samples = [
  "- one\n- two\n", "* one\n+ two\n", "- one\n\nparagraph\n\n+ two\n", "- one\n  - child\n",
  "- one\n  + child\n    * grandchild\n", "* one\n  * child\n    * grandchild\n", "1. one\n   - child\n   - two\n",
  "1. one\n   - child\n\n- root\n  - child\n", "> - one\n>   - child\n", "- [ ] todo\n  - [x] done\n",
  "- * *\n", "- ***\n", "---\n", "* one\n\n- two\n", "\uFEFF- 😀one\r\n  - child\r\n",
  "```\n- one\n+ two\n```\n", "| - one |\n| ----- |\n| + two |\n", "---\ntitle: Hi\n---\n\n- one\n",
  "<!-- markdownlint-disable-next-line MD004 -->\n* one\n\nparagraph\n\n- two\n",
  "<!-- markdownlint-disable-file bullet -->\n* one\n\nparagraph\n\n- two\n", "-\tone\n  - child\n",
  "- one\n\nparagraph\n\n* two\n  * child\n", "+ one\n  + child\n    + grandchild\n",
  "-\troot\n  - child\n    - grandchild\n", "> -\troot\n>   - child\n", "-\troot\n    - child\n",
  "-     root\n  - child\n", "1.\troot\n  - child\n", "1.\troot\n    - child\n", "- root\n\t- child\n",
  " >\t-\troot\n >\t  - child\n", "- one\n  + child\n- two\n  * child\n",
  "> 1.\t---\n >  \t+\n", " > - root\n>   - child\n", "> - root\n >   - child\n",
  "- > - root\n  >   - child\n", " >\t- root\n>\t  - child\n",
];
const configurations = [true, false, {style:"dash"}, {style:"plus"}, {style:"asterisk"}, {style:"sublist"},
  {severity:"warning"}, {style:"invalid"}];
const observations = samples.flatMap((input, sample) => configurations.map((options, configuration) => {
  const findings = lint({ strings: { input }, config: {default:false,MD004:options} }).input;
  const bom = input.startsWith("\uFEFF") ? "\uFEFF" : "";
  return { id: `${sample}-${configuration}`, input, options,
    findings: findings.map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0],
      detail: finding.errorDetail, severity: finding.severity })),
    output: bom + applyFixes(input.slice(bom.length), findings) };
}));
const receipt = new URL("md004-observations.json", import.meta.url);
const serialized = "[\n" + observations.map(row => "  " + JSON.stringify(row)).join(",\n") + "\n]\n";
if (process.argv.includes("--record")) writeFileSync(receipt, serialized);
else assert.equal(serialized, readFileSync(receipt, "utf8"));
const binary = fileURLToPath(new URL("../../target/debug/mdfmt", import.meta.url));
const directory = mkdtempSync("/tmp/opencode/mdfmt-md004-oracle-");
const configuration = directory + "/.markdownlint-cli2.jsonc";
let preservedDifferences = 0;
try { for (const row of observations) {
  for (const fix of [false, true]) {
    writeFileSync(configuration, JSON.stringify({ fix, config: {default:false,MD004:row.options} }));
    const native = spawnSync(binary, ["-"], { input: row.input, encoding: "utf8", cwd: directory });
    if (row.options.style === "invalid") {
      assert.equal(native.status, 2, `${row.id}: schema rejects invalid style`);
      assert.equal(native.stdout, "");
      continue;
    }
    if (!fix) assert.equal(native.stdout, row.input, row.id);
    const expected = lint({ strings: { input: native.stdout }, config: {default:false,MD004:row.options} }).input
      .map(finding => ({ line: finding.lineNumber, column: finding.errorRange[0], detail: finding.errorDetail, severity: finding.severity }));
    const actual = [...native.stderr.matchAll(/^-:(\d+):(\d+): (Error|Warning) MD004: Unordered list style \[(.*)\]$/gm)]
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
console.log(`PASS: ${observations.length} pinned MD004/native diagnostic, schema and idempotence cases; ${preservedDifferences} preserved upstream-fix differences`);
