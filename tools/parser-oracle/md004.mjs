import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
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
console.log(`PASS: ${observations.length} pinned MD004 observations (native implementation pending)`);
